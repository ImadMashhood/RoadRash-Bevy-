# Render sizing calibration and start of Part 9

Complete for the requested sizing investigation/native Fedora reference setup and the first Part 9 task. Implementation revision is the commit containing this receipt, corrected renderer, reference tooling and 09.01 checkbox update. Part 9 as a whole remains open.

## Findings and corrections

The C++ port was built from pinned revision `7aecf6044f2dd773b925b325823f30d648d61c89` using the existing verified import. Two missing `<climits>` includes were the only upstream source edits. The native binary is usable through `tools/reference/run.sh` and the local application-menu entry; the build/probe can be reproduced with `tools/reference/build.sh`. [Build receipt](native-linux-build.json), [Fedora commands](../NATIVE_REFERENCE.md).

The previous renderer used the high-resolution 58×128 BOB storage dimensions, an authored bottom-center pivot, size factor 0.5 and foreground depth 420. Those were declared approximations; the actual native comparison showed they were insufficient. The new source registration is RRAN 29×64, hotspot (19,52), actor size factor 2 and projection depth 512. General non-bottom pivots had their vertical sign reversed. Road depth could then clip legitimate pixels below the hotspot; ground-anchor occlusion now retains the complete billboard while respecting a nearer hill. Canyon wall materials also incorrectly used the tunnel category; they now resolve category-1 landscape faces/caps. Synthetic tests distinguish the landscape art from deliberately conflicting tunnel art.

The source-port City sample-50 rider rectangle is exactly (287,198)–(345,326) in both renderers. Fifteen matched poses across all five courses pass: **zero rider-rectangle error**, and all **495 road projection vectors** agree within **1/1024 pixel**. The largest observed road value discrepancy is 0.0005 pixel, from six-significant-digit native diagnostic output. [Native/Rust measurement metadata](render-native-sizing.json). Full-frame equality is not asserted: terrain RNG interleaving, texture mapping/subdivision/filtering, groups, animation/shadows and HUD differ. Source geometry facts are measured against the port, not certified by running the original EXE.

The existing-output guard also received a regression fix: canonicalizing an already existing capture path must not append an empty suffix/trailing slash and turn file replacement into EISDIR. The CLI can now reproduce a capture into the same explicit output path.

## Verification

```sh
cargo test --locked -p rr-formats -p rr-import -p rr-content -p rr-course -p rr-simulation -p rr-render -p rr-input
cargo test --locked --release -p rr-game -p rr-presentation --target-dir ../target
cargo clippy --locked --workspace --all-targets --target-dir ../target -- -D warnings
cargo build --locked --release --workspace --bins --examples --target-dir ../target -j4
cargo fmt --all --check
python3 -m unittest discover -s tools -p 'test_*.py' -v
python3 tools/check_workflow_guards.py
```

Passed: **108 Rust tests** (103 engine/input tests plus two game adapter and three presentation tests), **10 Python tests**, formatting, workspace Clippy, linked release build and negative workflow guards. New tests cover RRAN dimensions/hotspot and truncated records, non-bottom image extents over road/nearer-hill occlusion, landscape/tunnel art separation, repeated output-file replacement, five input-config cases and two actual Bevy input/lifecycle cases. CI and the clean-build script include `rr-input`. Remote Actions/fresh-container execution is not claimed by these local checks.

All **620 private source-camera captures** were exported twice with byte-identical PNGs and reports; every whole-frame repeated comparison has zero differences and no masks. All 620 lower image regions contain no unpainted clear-color gaps. [Current checkpoint metadata](render-checkpoints.json), [aggregate reproducibility](render-reproducibility.json). Seven authored goldens were visually reviewed after the source-backed depth/size/pivot changes and deliberately updated; this is a documented baseline correction, not an unexplained golden replacement. Native/Rust paired course views, Canyon's corrected cliff texture and the actual updated Bevy classic/wide outputs were visually reviewed locally.

All **14 actual Bevy cases** pass: four controlled virtual presentation schedules, wide, integer, portrait, native-size, authored enhancement, live texture switch and the other four courses. Each preserves all headless canonical replay checkpoints and final state SHA-256 `f70c202279f360fc89e95d503443a91a523c816788d9e62102a6d3572fc7f143`. Bars outside expected viewports remain black; selection changes at tick 60. [GPU metadata](render-gpu-matrix.json). These schedules remain determinism checks, not hardware FPS measurements.

The native C++ test gate registered **134 tests: 119 passed, 15 private-fixture skips, zero failures**. Three native OpenGL full-field City timing runs produced 306.26 / 299.82 / 300.77 mean fps with vsync off, 4,000 warmup and 3,000 measured frames. The 640×480 original-mode settings and full timing distributions are in [benchmark metadata](native-linux-benchmark.json). This is C++ port throughput on Fedora/Ryzen 5 3600, not a Rust speed comparison or original-executable parity claim. Audio/controller hardware acceptance and long thermal/pacing soak tests are not claimed.

## Next-step implementation

[09.01](09.01.md) is complete: typed actions, race/menu/paused contexts, persisted keyboard bindings, conflict reporting, scoped/default reset and the Bevy action adapter. The actual CLI and game adapters are tested. Keyboard focus/neutral/explicit resume protection is preparatory work for 09.11; its controller-disconnect requirement remains open. Gamepads/analog handling, bike physics/source calibration and all remaining Part 9 parents remain unchecked.

All original-derived captures, PPMs, decoded pixels, native sources/builds, RPMs and preferences remain ignored/local. Public changes contain Rust/C++ tool source, a two-include portability patch, authored config/goldens, docs and metadata/hashes. [Public-file audit](public-file-audit-calibration.json) inventories the exact reviewed paths (excluding its own self-reference).
