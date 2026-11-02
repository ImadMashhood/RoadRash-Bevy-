# Building, collision, AI and interface corrections after RC4

The reported gaps were real. Current source corrects them, with targeted passing
checks. This does **not** renew full-game/release acceptance: the previous
unattended career driver fails against the changed collision/AI behavior. The
career and replacement-candidate gates are reopened. RC4's immutable download and
receipts are retained as historical evidence. No original-runtime comparison,
owner signature, texture enhancement or 3D work is claimed.

## Changes and evidence

| Reported problem | Correction | Verification |
| --- | --- | --- |
| Missing building side textures | Preserve the first CLGP cell independently of the facade. Render exposed ends, height transitions and source palette-253 roofs. Share source junction spans with collision. | Authored `building-step` pixel assertions and reviewed render; 25 native course/level entries and 75 exact serial/parallel image/depth/stat comparisons. |
| Collision-free scenery | Add native building boxes from shared profiles; use imported sprite scale and pivot for existing solid billboard bounds. Integer swept wall tests prevent tunneling; contacts stop/clip the bike and trigger recovery. Continuous adjacent panels do not restart the crash. | Low/medium/high-speed frontal contact, lateral tunneling, diagonal misses, open lane and adjacent-panel regressions. Native City factory imports 8,929 building volumes; isolated native wall test yields one crash, zero speed and a bike outside the wall. |
| Bare AI | New native setups select tactics version 1: look ahead for scenery, moving road users and reserved lanes; approach rivals within combat reach according to aggression while allowing bike clearance. | Static avoidance, passive/aggressive approach and unknown-version tests; existing passing, combat, traffic, police, recovery and pool tests. Legacy setups omit version 0 and preserve existing replay hashes. |
| Bare UI | Imported menu backgrounds, larger main-menu buttons, focus cards, screen-owned mouse hitboxes and cached art metadata. Bevy owns the race instrument panel; the obsolete raster speed digit is removed. | Real Main, City, bike, crash and minimum-size settings captures; ten targeted five-language layout captures. Camera-based click/hover tests at normal, high-DPI and letterboxed sizes; 135 printable glyphs covered. |

The building reference is
[`building_panels.cpp`](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/building_panels.cpp)
and [`course_scene.cpp`](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course_scene.cpp)
at the inspected local reference revision. End textures, roof depth/color and
transition flags are informed by that source. Collision boxes and fixed-tick
planning are explicit modern policies; per-sample boxes do not establish exact
original collision semantics or complete original renderer parity.

## Executed checks

[Machine-readable receipt](native-gameplay-fixes.json) pins the measured binary
and native contact hash. The containing Git commit identifies implementation.
Commercial source art, screenshots, setups, logs and replay inputs stay ignored.
Authored regression fixtures and aggregate hashes/counts are public.

- Workspace: **360 Rust tests passed**, zero failed/ignored; **33 Python tests passed**.
- Native contact: all 45 tick hashes agree across 30/60/144/irregular schedules;
  rendered replay has the same final hash. Controlled initial position and
  removal of other actors deliberately isolate the wall; this is not an ordinary
  career playthrough or a fabricated finish.
- Native raster: all 25 course/level entries pass, with three fixed cameras per
  entry and zero serial/parallel image, depth or draw-stat differences. This
  checks renderer consistency, while the authored pixel tests and reviewed
  screenshots establish the new face coverage.
- Current UI: ten targeted Main/settings language cases plus bike, City and wall
  crash captures reviewed. Minimum settings are 320×240 at 150% text. These are
  additional current checks; the old 100-case RC4 matrix is not relabeled.
- Font: 135 printable codepoints, including the new navigation arrows, have
  nonzero glyph indices in the actual embedded Fira Mono font.
- A 45-second live ten-segment course tour completes 43.8 simulation seconds with
  zero paused frames. Decoded CPU texture census now includes terrain/building
  materials and all mips; observed course totals are 136.25–163.98 MiB. Concurrent
  testing ran, so this is a smoke check, **not** renewed frame-time/soak/leak or
  minimum-hardware acceptance.

Reproduce the authored and native contact checks from the repository root:

```bash
cargo test --locked --workspace --target-dir ../target
cargo clippy --locked --workspace --all-targets --target-dir ../target -- -D warnings
python3 -m unittest discover -s tools -p 'test_*.py'
cargo build --locked --release --target-dir ../target -p rr-game -p rr-simulation -p rr-render
python3 tools/verification/gameplay_fixes.py \
  --game ../target/release/rr-game --replay ../target/release/rr-replay \
  --install local/game-install --catalog local/catalog \
  --output local/new-wall-probe
../target/release/rr-native-verify local/cdrom/ROADRASH local/catalog
../target/release/rr-render authored local/new-authored-captures
```

Use a fresh output directory. The native probe records a fresh factory setup,
then constructs its explicitly controlled wall case; it never publishes that
setup or the commercial screenshots. It checks all tick hashes, wall separation,
incident count, stopped speed, recovery and rendered/headless agreement.

## Failed checks and open gates

The unchanged simple career driver fails 25 consecutive level-one Peninsula
attempts (all arrests). Its existing optional predictive mode also fails a
three-attempt probe. These are recorded failures, not evidence that a human
cannot finish a race. They mean the former 67-attempt career pass cannot certify
current physics/AI. No opponent difficulty reduction, injected result, seeded
money or acceptance waiver was used to conceal this failure.

Part 19's complete-career/defect/candidate acceptance, Part 21's current full
matrix/performance/lifetime/soak acceptance, and Part 22's replacement-release
notes/tag/checksum/signoff remain open in the manifest and README. Existing
RC4 evidence pages are explicitly historical. G5 still requires a technically
verified replacement candidate and the owner's actual decision.
