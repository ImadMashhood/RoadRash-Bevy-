# Part 08 acceptance receipt

Scope: classic source-course renderer and Bevy presentation integration. All 12 parents and their **35 remaining implementation/acceptance/evidence children** are complete. Original-runtime gate 08.01.b remains removed by the owner; it is not marked as a passed comparison. 3D remains deferred. Implementation revision is the Git commit containing this receipt, code, tests and checkbox updates.

## Implemented result

`rr-render` supplies validated source camera/depth-table projection, dual roadways, native road materials/mips, horizon wrapping, seeded hills/walls/cliffs, building/shoulder surfaces, source static scenery pixels/hotspots, stable depth/class/ID ordering, near/screen clipping, logical sprite contact, aspect/integer/wide policies, authored goldens, headless captures and actual image diffs. Bevy displays this same kernel's image and consumes immutable interpolated snapshots. The game accepts course, level, branch, inspection camera, framing and viewport options.

Source-course drawing is integrated. Handling is still the declared authored scenario model through the explicit source-unit visual bridge; original handling/animation/combat/AI/career/media are later phases. Renderer completion is not a complete-game release or an original-runtime parity claim.

## Commands and local results

```sh
cargo test --locked -p rr-formats -p rr-import -p rr-content -p rr-course -p rr-simulation -p rr-render
cargo test --locked --release -p rr-presentation --target-dir ../target
cargo clippy --locked --workspace --all-targets --target-dir ../target -- -D warnings
cargo build --locked --release --workspace --bins --examples --target-dir ../target -j4
cargo fmt --all --check
python3 -m unittest discover -s tools -p 'test_*.py' -v
python3 tools/check_workflow_guards.py
../target/release/rr-render authored local/render-authored
../target/release/rr-render suite local/game-install local/catalog local/render-checkpoints-final
../target/release/rr-render suite local/game-install local/catalog local/render-checkpoints-repeat
```

Passed: **97 Rust tests** (23 new renderer tests, one additional Bevy adapter test, 73 existing tests), **10 Python tests**, formatting, workspace Clippy, linked release build and negative workflow guard checks. Rust remains pinned to 1.99.1 and Bevy to 0.20.1 with Cargo.lock. CI and clean-build recipes now include rr-render; a new remote Actions/clean-container execution is not claimed from these local checks.

Public authored acceptance includes camera specification/error behavior, known projection vectors, straight/bend/crest/valley depths, signed-safe horizon wrapping, independent terrain categories/seed reproducibility, source event-to-material endpoints, native minification mips, registration-independent equal-depth sprite ties, alpha/depth behavior, near/screen clipping, degenerate/behind-camera/nonfinite cases, several-distance contact, original-table nonlinear contact, aspect/integer/wide policies, immutable interpolation, overlap-safe outputs, real pixel diffs, seven reviewed image hashes and bounded RRAN pixel aliases/hotspots/truncations. The Bevy adapter test checks actual image-resource updates without a GPU while preserving authoritative snapshots and hashes.

## Native image coverage

[Checkpoint metadata](render-checkpoints.json) records **620 fixed-camera captures**: all five courses and 26 branches, 396 before/at/after seam views for 132 transitions, 130 finish views with the corresponding level placements, starts and terrain-category views. These are frozen source-camera inspection scenarios at simulation tick 0, not completed race outcomes. Every record identifies level, branch, source sample, camera, dimensions, statistics and RGBA/PNG hashes. All whole-frame repeated comparisons have **zero differing pixels**, with **no masks**.

A second independent export produces byte-identical PNGs and reports. [Reproducibility receipt](render-reproducibility.json) records the aggregate comparison. A scan of all 620 lower image regions found no unpainted clear-color gaps. Seven authored fixtures were visually inspected individually and their golden hashes committed; they contain no commercial pixels. Native views of City, Sierra Nevada, Peninsula, Napa and Pacific Highway, plus wide, integer and authored override examples, were visually reviewed locally.

## Actual Bevy/window/replay acceptance

For each virtual presentation rate in `30`, `60`, `144`, `irregular`:

```sh
../target/release/rr-game --install-root local/game-install --catalog local/catalog --course CITY --replay docs/fixtures/replay.json --render-hz RATE --framing classic --window-size 960x720 --report local/part-08-gpu/classic-RATE.json --capture local/part-08-gpu/classic-RATE.png
```

Additional runs use wide `1280x720`, integer `1280x960`, integer portrait `800x1000`, native `640x480`, an authored replacement image, a live tick-60 texture switch and all four other course names. Exact arguments, PNG hashes, capture sidecars, viewport rectangles and state/checkpoint hashes are in the [14-case GPU matrix](render-gpu-matrix.json). The authored override was generated by the existing `authored_override` example; it is not AI-generated or approved enhancement art.

All 14 cases match the headless runner at ticks 30/60/90/120. Final canonical SHA-256 remains `f70c202279f360fc89e95d503443a91a523c816788d9e62102a6d3572fc7f143`. The live switch log confirms tick 60. Exact aspect-fit/integer viewport rectangles were computed from screenshot dimensions and camera settings; all pixels outside those rectangles are black. Native/override captures visibly demonstrate the selected image while logical dimensions/pivot remain fixed.

Screenshots are requested after the Draw stage, with `.capture.json` sidecars recording simulation tick, accumulator alpha, source start, course/level/branch, camera and configured window size. These are actual engine captures, not original-game screenshots. Virtual rate schedules are controlled time inputs, **not measured hardware FPS benchmarks**. Performance budgeting remains the later profiling phase.

## Source facts, deliberate policies and limits

The source EXE depth table's near region is nonlinear. A foreground visibility bug was found in the first real-course capture and corrected by projecting contact against the actual road strip. A dedicated regression now covers it. The source road descriptor/marked-lane alias, horizon-name table, palette indices/mips, course flags and static RRAN view/hotspot aliases are retained. Source facts reference pinned [road.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/road.cpp), [road_materials.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/road_materials.cpp), [course_scene.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course_scene.cpp), [terrain.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/terrain.cpp), [animation.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/animation.cpp) and family/scenery records at the same revision. No upstream C++ implementation was copied.

The visual unit bridge, foreground proxy, stable sprite tie policy, deterministic terrain streams, simple roof geometry, projective rasterizer and highest-cell/native-mip sampling are declared policies. Full original random-call interleaving, every CCB shading/blend effect, exact group thresholds, rails/elaborate tunnel/roof details, rider animation/shadows and actual opponent/traffic systems are not certified by this phase. Optional unresolved/opaque leaves are listed before rendering in per-course source notes; native ground fallbacks or sprite omission remain explicit. Required road/horizon/rider bindings and unsupported binaries fail startup. See the [renderer contract](../RENDERING.md) for precise boundaries.

Private source, generated PNGs, decoded data and image diffs remain ignored. Public changes contain safe Rust, authored fixture definitions, documentation and metadata/hash receipts only. The next phase is **09: input and player handling**.
