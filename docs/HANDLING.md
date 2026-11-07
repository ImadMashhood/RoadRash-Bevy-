# Source-unit player handling

Imported races use the safe Rust handling model in `crates/simulation/src/handling.rs`, composed with the animation controller in `riding.rs`. Bevy only samples commands and presents snapshots. There are no binary object offsets, filesystem reads, GPU types or render-frame delta times in the tick path. The startup decoder converts verified source SPEC/EXE records into named fields and rejects missing, duplicate, unsupported and invalid specifications before launch.

```sh
cargo run --locked --release -p rr-game -- \
  --install-root local/game-install --catalog local/catalog --course CITY --bike 1 --rider-color 0
```

Rust bike IDs are **1–15**, matching SPEC resource IDs; the native reference CLI uses zero-based bike indices. Rider color IDs are **0–14**. Up/W accelerates, Down/S brakes, Left/Right or A/D steers. Z punches, X kicks and C backhands with the default action map. These actions select the source animation/contact-window contract; opponent targeting, damage from attacks and weapon theft decisions belong to Phase 11. Older binding files retain their required five race actions; the three attack actions are optional additions in schema 1 and can be remapped normally. Pause/focus/controller safety and calibration continue to use [the input layer](INPUT.md).

## Coordinates and fixed-step order

The supported source scheduler is 60 Hz with `dt = 1`. Bike speed, force, lean and lateral/vertical velocity retain source integer units. Longitudinal source Q8 displacement is `trunc(((speed * 2 - acceleration) >> 1) / 256)` per tick; each road sample spans 256 source Q8 units. Lateral and above-ground height are Q8 world units. These are not a conversion to physical metres.

A tick applies drive/brake and steering; resolves traction; integrates speed and shifts gear; advances source position; accumulates signed bend/rise over every crossed sample; applies curve lateral push; then resolves contact, launch, gravity, landing/rebound and pitch. The source retains the previous sample's rise until another boundary crossing, including subsequent ticks inside the new sample. This subtle persistence is covered by combined native movement comparisons. Bend accumulation uses the source's negative sign.

Schema-3 snapshots carry the full typed bike and animation state. The classic renderer reads source position/lateral/height directly and interpolates copied positions only. The old `distance_mm`/`lateral_mm` fields are compatibility summaries; they do not define the new handling model. Physics never reads sprite dimensions, pivots, palette IDs or enhancement scale.

## Recovered behavior

- All 15 SPECs share one seven-slot gear schema, including the source's unused zero final slot where present. Torque/drive force, force ramp/release rates, automatic up/down thresholds, mass, steering rates and condition are imported rather than assigned by bike name.
- Brake takes priority over throttle. Negative source brake rates are preserved, including the `current > rate` equality behavior that can snap force to the limit. Coasting releases force and drag brings speed to zero; braking does not produce reverse movement.
- Lean follows the source sine table, speed, steering divisor, limits and response rates. Traction, damping, skid loss and curve coefficients depend on surface. Source sine quadrant indexing and signed shifts produce small measured left/right differences; they are retained, not averaged away.
- Road/shoulder/terrain resolution preserves strict edge comparisons, marking overrides and source classes 0–3. The game supplies terrain category from its decoded scene. Curves, grade integrals and crests use source sample boundaries, independently of rendering.
- Air contact removes drive/side force. Launch/gravity, landing thresholds, rebound, lean wobble and pitch use the source law. The fixed-step model is tested with upward/downward impulses and combined changing grades.
- Condition starts from SPEC maximum (`+0x2ec` in the binary decoder). Damage is a one-shot domain event, saturates at zero and latches failure at exact equality. Failed bikes cannot accelerate; repeated damage cannot underflow. Collision/combat producers of these events remain their later phases.

Player/rider mass input is explicitly 75 in this slice, matching every native comparison. Nitro, AI rubber-banding, collision impulses, wall impacts, detached-body movement, recovery decisions and race/career rules are separate later systems. The completed Part 9 gate covers its listed handling/input deliverables, not those later phases or original-executable parity.

## Versioned replay contract

Schemas 1 and 2 retain their previous command encoding, handling and hashes. `Simulation::default()` remains the data-free schema-2 regression scaffold. `Simulation::with_handling` creates schema 3 only after validating its full configuration. Imported live races select this automatically. Schema-3 replays require that configuration; old schemas reject pose, weapon and damage additions. Held drive controls persist; pose, equipment and damage events are consumed once.

The schema-3 canonical byte stream includes length-prefixed deterministic serialization of the integer configuration, held commands and every hidden bike/animation field, in stable actor order. Source tables, selected bike, ground profile, gear, force, airborne/contact state, queued animation/deadline, equipment and contact flags therefore affect hashes. Renderer state and physical device identity do not. Private source-derived configurations/replays stay under ignored `local/`; public tests use original authored data.

## Reproduce acceptance and plots

Build the Rust tools, and build the native reference once using [the Fedora instructions](NATIVE_REFERENCE.md). Then:

```sh
cargo build --locked --release --workspace --bins --examples
source local/reference-tools/env.sh
python3 tools/reference/check_riding.py --data local/cdrom/ROADRASH \
  --output local/riding-native --rust target/release

target/release/rr-riding-check local/cdrom/ROADRASH local/riding-acceptance
python3 tools/animation/plot_handling.py \
  --native-trace local/riding-native/handling-native.json \
  --riding-trace local/riding-acceptance/riding-trace.csv --output local/riding-plots
```

If using this workspace's shared target directory, substitute `../target/release`. The Python comparison builds separately authored C++ API adapters against the pinned local reference; it does not patch gameplay code. It compares decoded JSON values, so whitespace differences in trace-file hashes are harmless. The scripts stop on the first field mismatch and retain the exact failing rows locally.

The schedules include 1,800 ticks of throttle followed by 600 ticks of brake/coast, left/right steering, all four surfaces, airborne impulses, and combined curves/crests. Four plotted SVGs show representative bikes and integrated speed/lateral/height; the exact schedules live in the tools, not in manually edited graph data. [Authored independent answers](fixtures/riding-regression.json) guard acceleration, the brake equality case and contact timing without commercial files. [Part 9 evidence](evidence/part-09.md) links the measured acceptance metadata.

## Signed motion profile

New native races set handling `motion_version: 2` and body `motion_version: 1`.
Profile 2 additionally preserves airborne lateral memory and captures impact
velocity separately from callback-updated scalar speed. Profiles 0/1 retain their
historical arithmetic. Mounted and detached motion share the
signed acceleration/contact-drag integrator; backwards travel persists and
negative-speed braking approaches zero without immediately clipping motion.
Omitted/profile-0 configurations retain the old arithmetic and canonical replay
inputs. Unknown versions are rejected. See the
[scoped native comparison receipt](evidence/gameplay-completion-progress.md);
these tests do not establish complete collision or coupled-recovery parity.
