# Action bindings

Part 9 is complete. `rr-input` owns versioned, renderer-independent action maps and configuration I/O. The game translates Bevy key codes at its composition boundary; simulation commands contain actions rather than physical keys. Keyboard and gamepad input are active. [Native controller calibration/preview](CALIBRATION.md) is complete; [source-unit handling](HANDLING.md) is active for imported races. Z/X/C request punch/kick/backhand through remappable actions in schemas 4–6, with automatic target-side selection. Arrow/drive directions walk while recovering; releasing them resumes automatic return to the linked bike. Schema-3 replays retain their visual-only inputs.

```sh
cargo run --locked -p rr-input -- init local/bindings.json
cargo run --locked -p rr-input -- set local/bindings.json race accelerate Space
cargo run --locked -p rr-input -- validate local/bindings.json
cargo run --locked -p rr-game -- --bindings local/bindings.json \
  --install-root local/game-install --catalog local/catalog --course CITY
cargo run --locked -p rr-input -- reset local/bindings.json race
```

`set` replaces that action's alternatives; separate alternatives with commas, such as `ArrowUp,KeyW`. Unknown physical key/action/context names are rejected. The [default file](fixtures/default-bindings.json) lists supported actions in `race`, `menu`, and `paused` contexts. Supported keys are arrows, A–Z (`KeyA` etc.), digits (`Digit0` etc.), Space, Escape, Enter, Tab, Backspace, left/right Shift/Control/Alt and F1–F12. External file edits apply on the next launch. Restroom → Configure Input edits and persists active bindings in-game.

All required actions must remain bound. Each action accepts one to four distinct alternatives. Duplicate/conflicting physical keys are rejected **within a context**, with both action names in the error; different contexts may reuse a key. For example, W accelerates in the race map and navigates up in the menu map. Menus consume their own map; the modern frontend is implemented. Resetting one context preserves customizations in the others; `reset FILE all` restores the entire map.

Reads are capped at 64 KiB. Unknown fields/schema versions are rejected. Edits validate before changing state or touching disk. Writes use a same-directory exclusive temporary file, file sync and rename on the supported Linux platform. Invalid edits preserve the previous complete file; interrupted temporary files are never read as active configuration. `init` refuses to overwrite an existing file. Parent directories must already exist.

Default driving controls remain Up/W, Down/S, Left/A and Right/D. Simultaneous left/right cancels; braking precedence stays in the simulation. Escape/P pauses. Enter/P resumes after movement controls return to neutral. Losing window focus also pauses and clears the pending command; regaining focus alone never resumes. The window title shows the configured Resume alternatives. The active controller disconnecting also pauses; other controllers disconnecting do not interrupt play. Start pauses/resumes with the same neutral/explicit-action protection.

Recorded replays bypass physical keyboard/pause sampling and retain their existing canonical hashes. The data-free fallback uses schema 2 proportional steering; imported live races use schema 6. Legacy schema-1–5 recordings preserve their own rules/encoding/hashes. Source handling is measured independently; the fallback remains authored. The binding layer’s original five pure tests, two adapter tests and [CLI acceptance](evidence/input-bindings-cli.json) cover defaults, context reuse, conflict reporting, rejected-edit rollback, schema/size limits, durable complete-file replacement, context-specific reset, actual remapped tick commands, opposed steering and focus/neutral/explicit resume. Physical gamepad hardware has not been tested. Two Linux kernel virtual controllers have exercised the real backend as described below.

## Gamepad controls and device ownership

| Input | Action |
| --- | --- |
| Left stick X | Proportional steering |
| D-pad left/right | Full steering; overrides the stick; opposed directions cancel |
| South face button or right trigger | Accelerate |
| East face button or left trigger | Brake; simulation braking takes precedence over throttle |
| Start | Pause; press again to explicitly resume once controls are neutral |

These are the fixed standard controller preset through Gilrs mappings. The existing binding-file editor configures keyboard alternatives; a controller layout editor is not claimed here. The shared bridge configures Bevy's axis settings to pass values through, then applies the selected saved calibration exactly once. Factory settings use a 15% stick dead zone, 98% full-lock threshold, Linear response and symmetric Q15 quantization. Non-finite axes become neutral, finite out-of-range values clamp, and triggers use a 0.5 threshold. Trigger pressure is currently a digital driving gate, not proportional source acceleration. Model calibration, response curves and a live native Bevy preview are implemented in [09.03](CALIBRATION.md).

Keyboard activity wins simultaneous input. Otherwise the current controller keeps ownership until another armed pad changes meaningfully. A 1.5% cumulative stick change avoids tiny held-stick jitter stealing another controller, while gradual deliberate movement can still switch devices. Simultaneous candidates use stable session IDs as the tie-break; raw IDs never enter simulation hashes. At most 16 controllers are considered, with selection independent of backend enumeration order. Newly connected/reconnected pads must first report neutral. Holding a button on connection cannot take over; a neutral active pad never inherits another pad's old held throttle.

Start/P cannot pause and instantly resume from the same press. Current driving controls must also be neutral at the actual resume edge, even if they were released earlier during pause. Focus regain and reconnection alone never resume. A held reconnected controller remains disarmed through keyboard resume until it reports neutral and receives fresh input.

## Reproduce backend acceptance on Linux

The normal tests use authored Bevy connection/button/axis messages. The additional explicit diagnostic uses the real Linux backend and two temporary kernel controllers:

```sh
cargo test --locked -p rr-input -p rr-simulation
cargo test --locked --release -p rr-game -p rr-presentation
cargo build --locked --release -p rr-game --example input_probe
gcc -O2 -Wall -Wextra -Werror tools/input/virtual_pad.c -o target/rr-virtual-pad
python3 tools/input/check_linux.py --probe target/release/examples/input_probe \
  --helper target/rr-virtual-pad --output local/input-backend
```

This requires existing write access to `/dev/uinput` and read access to its resulting event devices; it never changes system permissions. The [kernel uinput API](https://www.kernel.org/doc/html/latest/input/uinput.html) supplies the temporary controllers. Only controller axes/buttons are emitted, never keyboard or mouse input. File descriptors are closed and devices destroyed on success or failure. The native helper uses Linux's generic Gilrs trigger codes (`ABS_HAT2X/Y`) rather than guessing that an unmapped joystick Z axis is a trigger. Named physical devices are not required or certified by this test.

Fourteen native checks passed on Fedora. The report includes a separately counted pre-existing neutral backend device. The 16-bit kernel endpoint can differ by one Q15 count; the diagnostic permits two counts, without changing the transport curve. A reconnect may retain cached axis state until a fresh report, so the test explicitly reports neutral before reusing that controller.

[09.02 evidence](evidence/09.02.md), [09.11 evidence](evidence/09.11.md), [native backend trace](evidence/input-device-backend.json) and [replay matrix](evidence/input-device-replays.json) distinguish pure/device-event, real native backend and actual game-window validation. Linux builds require libudev development metadata (`libudev-dev` on the Ubuntu CI image; `systemd-devel` on Fedora), in addition to the earlier desktop dependencies. CI and clean-build recipes include that dependency. Windows/macOS/physical USB/Bluetooth acceptance remains open.

The game auto-loads `local/controller-calibration.json` when present; `--calibration FILE` overrides it. The native tool saves only on request, and a running game must be restarted after saving. Model IDs are stable; identical models share profiles while the preview keeps live sessions separate. [Calibration commands, algorithms and evidence](CALIBRATION.md).

`--record-replay PATH` captures the live command stream at the fixed-tick boundary, including repeated one-shot actions, and saves a new bounded replay on app exit. Recorded playback bypasses physical polling. Pausing stops simulation ticks; the presentation siren adapter also stops/resumes owned loop cues and respects effects disable. Actual Bevy adapter/recorded-file tests and the unattended source-window roundtrip are documented in [Part 13 evidence](evidence/part-13.md). Human driving calibration is not asserted by the unattended capture.


Frontend menus use the separate Menu action map (keyboard Up/Down, Enter/Escape by default) or gamepad D-pad/South/East. Menu input stays sampled while paused, so releasing held driving controls permits neutral-gated resume. Countdown consumes 180 session ticks with no simulation commands, then held driving input can unlock. In Configure Input, select a race action and press a supported key; Escape cancels. The player-name editor accepts up to 19 ASCII characters. These menu settings/bindings are currently in-memory; durable input/settings persistence remains Part 17. A result summary owns the finished run independently from current actors, and recording flush/rebase occurs before world replacement.

## Part 19 actions and current defaults

Z/X/C attack, Left Shift requests source nitro, V requests a stationary edge-triggered dismount, and Q/E provide independent lean. Nitro requires a compatible bike and remaining charge; throttle release, braking, depletion or a crash cancels it. These controls are remappable optional actions, so old custom bindings are preserved before adding new defaults. A conflicting optional default is omitted with a warning rather than overwriting a user's key.

Controller West/North/left bumper attack; right bumper boosts, Select dismounts, and D-pad Up/Down leans. Analog triggers/stick retain driving control. Buttons participate in device arbitration, neutral gating and held-on-connect quarantine. See [player guide](PLAYER_GUIDE.md), [action tests](../crates/simulation/tests/actions.rs) and [native candidate](NATIVE_ACCEPTANCE.md).
