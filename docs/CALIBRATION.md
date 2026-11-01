# Controller calibration

Task 09.03 is complete. The native Bevy preview and the game now share controller calibration for center drift, asymmetric travel, dead zones, full-lock thresholds, response curves, inversion and trigger thresholds. This is an input preference layer, not a claim about original Road Rash physics. Acceleration/gearing calibration starts in 09.04.

## Open and use the preview

Choose **Road Rash Controller Calibration** in this machine's application menu, or run:

```sh
cargo run --locked --release -p rr-game --bin rr-calibrate
# A tested release build can also be launched through:
./tools/input/calibrate.sh
```

No commercial game data is needed by the calibration tool. The default file is `local/controller-calibration.json`; it is created only when you save. A custom destination is supported:

```sh
cargo run --locked --release -p rr-game --bin rr-calibrate -- --profile local/my-controller.json
```

1. Choose your controller with **Next** or Tab. The window shows both raw input and calibrated steering, plus the exact response curve used by the game.
2. Release the stick and press **Capture center** or Enter. The tool samples for 1.5 seconds, estimates the center and a noise-safe dead zone, and rejects excessive movement.
3. Press **Start sweep** or M, move fully left and right, then press **Finish sweep** or M. Both sides must have enough travel. Disconnecting cancels a capture and preserves the previous draft.
4. Select a setting with the mouse or Up/Down; use its minus/plus buttons or Left/Right to adjust. Shift uses a larger step. Linear, Square and Cubic response curves trade center sensitivity for finer control. Full deflection still reaches full steering. **Invert stick** changes analog wiring direction; D-pad directions keep their usual meaning.
5. Press **Save profile** or S. Unsaved drafts survive controller selection changes. **Reset draft** or R copies the file's default settings into the selected draft; it does not write to disk until Save. Escape closes the tool without auto-saving.

The game automatically loads `local/controller-calibration.json` on its next launch if that file exists. It otherwise uses factory defaults. An explicit file overrides that location:

```sh
cargo run --locked --release -p rr-game -- --install-root local/game-install \
  --catalog local/catalog --course CITY --calibration local/my-controller.json
```

Profiles load once at startup; restart an already running game after saving. Invalid settings fail clearly rather than silently applying an unintended steering curve. An explicitly supplied missing file fails startup. Calibration does not change remappable keyboard bindings. The game package keeps `rr-game` as `default-run`, so existing launch commands still open the game.

## Processing rules

The Bevy/Gilrs bridge supplies normalized physical values. On controller connection/reconnection, its axis filter is configured to pass values through: bounds -1/1, zero built-in dead zone and zero change threshold. Trigger axes similarly use 0/1 bounds and no extra rounding threshold. Calibration is applied exactly once after that, so small measured drift is not hidden by a second filter. Digital face-button handling stays with Bevy.

For finite raw steering `r`, clamp it to [-1,1]. Subtract the saved center and divide by that side's saved travel (`center - minimum` on the left, `maximum - center` on the right). Clamp the resulting signed value `u` to [-1,1]. Then:

```text
magnitude = clamp((abs(u) - dead_zone) / (saturation - dead_zone), 0, 1)
response  = magnitude, magnitude² or magnitude³
steering  = sign(u) × response × optional_inversion
Q15       = round(32767 × steering)
```

Factory defaults are center 0, travel -1..1, 15% dead zone, 98% full-lock threshold, Linear response, normal polarity and a 50% trigger threshold. The 98% outer threshold accommodates devices whose normalized endpoints fall slightly short of exactly one, including the one-count kernel endpoint observed in the previous step. Finite out-of-range input clamps; non-finite steering becomes neutral. Trigger thresholds affect the binary driving action; proportional source throttle/braking is later handling work.

Validation requires finite values, at least 0.05 travel on each side, dead zone 0..0.5, full-lock threshold 0.5..1 with a gap of at least 0.05, and trigger threshold 0.05..0.95. Unknown fields/schema/curve names are rejected. Validated profiles feed the same Q15 command structure as keyboard/gamepad input in 09.02. Both replay schemas and all recorded hashes remain unchanged: calibration happens before commands are recorded, never during playback.

Center capture requires at least 16 valid frame samples and no more than 0.12 raw movement across the capture. It uses the sample mean, normalizes observed noise to the learned center/travel, and adds a 2% margin, bounded to a valid dead zone. Range capture uses observed extrema and requires at least 0.2 raw travel on both sides of the current center. The accumulator uses constant memory and continues tracking late extrema during long sweeps. These are declared modern calibration policies; they are not reverse-engineered Windows calibration behavior. Frame samples are the backend's current normalized value, not a measurement of the controller's internal ADC sampling frequency.

## Profile storage and controller identity

[Default schema](fixtures/default-calibration.json) is version 1, separate from binding-file and replay schemas. It contains a default calibration and at most 32 model overrides. A model key combines USB vendor/product IDs when available with a normalized, bounded controller name; missing IDs use `----`. No transient ECS entity ID or USB port is persisted. Data is bounded to 64 KiB, validated before edits/writes, and replaced through an exclusive temporary file, file sync and same-directory rename on the supported Linux platform. Bindings reuse that storage helper and retain their existing schema.

Identical controller models share a saved profile because this Bevy API exposes model identity rather than a unique physical serial number. The preview still lists and selects their **separate live sessions**: capture samples can never mix units, and losing the selected unit cancels capture instead of silently switching to another. Use separate profile files when same-model units need different settings. Drafts are model-scoped and are retained when changing the selected controller. A different/reconnected session can be chosen with Next; gameplay's existing neutral-on-connect gate remains active.

For offline editing and validation:

```sh
cargo run --locked -p rr-input -- calibration init local/controller-calibration.json
cargo run --locked -p rr-input -- calibration set local/controller-calibration.json default dead_zone 0.10
cargo run --locked -p rr-input -- calibration set local/controller-calibration.json default response cubic
cargo run --locked -p rr-input -- calibration validate local/controller-calibration.json
cargo run --locked -p rr-input -- calibration curve local/controller-calibration.json default
cargo run --locked -p rr-input -- calibration reset local/controller-calibration.json default
```

`init` refuses an existing file. `show` prints the complete file. `set FILE MODEL_KEY FIELD VALUE` edits a model override; supported fields are `minimum`, `center`, `maximum`, `dead_zone`, `saturation`, `response`, `invert` and `trigger_threshold`. `reset FILE MODEL_KEY` removes that override and restores inheritance; `reset FILE all` restores the entire file. Resetting only the default preserves model overrides. A failed edit leaves the previous file intact. Parent directories must exist. `curve` prints 201 points produced by the gameplay transform, rather than a second UI implementation.

## Verification and limits

[09.03 evidence](evidence/09.03.md) records **137 Rust tests**, **10 Python tests**, 10 native calibration checks, 14 native device-lifecycle regressions, eight headless replay runs and four actual Bevy replay windows. Pure tests cover drift, full lock, symmetric/monotonic Linear/Square/Cubic curves, independent known-answer powers, asymmetric travel, invalid input, capture rejection, long sweeps and configuration rollback. Bevy tests cover saved profile loading into real tick commands, sub-1% axes surviving the transport, configured trigger thresholds, actual mouse hit testing/save, scoped drafts and separate identical-model sessions.

The native check creates a temporary Linux kernel controller and verifies saved cubic steering/trigger calibration through real evdev/Gilrs. Its actual Bevy preview reports the **same Q15 output as the gameplay adapter** for the same live input. Drift is suppressed and both physical test endpoints reach exactly ±32767. [Native receipt](evidence/calibration-native.json), [CLI receipt](evidence/calibration-cli.json), [replay checks](evidence/calibration-replays.json).

```sh
cargo build --locked --release -p rr-game --bins --example input_probe
cargo build --locked --release -p rr-input
gcc -O2 -Wall -Wextra -Werror tools/input/virtual_pad.c -o target/rr-virtual-pad
python3 tools/input/check_calibration.py --input target/release/rr-input \
  --probe target/release/examples/input_probe --preview target/release/rr-calibrate \
  --helper target/rr-virtual-pad --output local/calibration-native-check
```

Use a fresh output directory; the test refuses to overwrite its authored profile fixture. Existing `/dev/uinput` write access and event-device read access are required; no system permissions are changed. Only controller events are emitted. Cleanup closes descriptors, removes the temporary device and stops the diagnostic processes. Screenshots/reports stay local. Native preview smoke flags (`--smoke-frames`, `--capture`, `--report`, `--device-key`) support bounded acceptance runs, not a separate input algorithm.

Physical USB/Bluetooth controller models, other operating systems and original-game handling fidelity remain unverified. The software/kernel virtual-device checks do not replace that hardware acceptance. The next open task is **09.04: acceleration and gearing**.
