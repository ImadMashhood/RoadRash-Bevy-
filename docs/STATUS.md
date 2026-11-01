# Implementation status

Parts 01–07 are complete within the current declared scope and first-platform Linux support. Part 01 original-runtime comparisons were removed by the owner; their deletion is documented in [scope changes](SCOPE_CHANGES.md), not recorded as successful original comparisons.

Parts 02–03 supply the pinned foundation and verified transactional importer. Parts 04–06 add bounded decoders, a reproducible semantic content catalog and browser inspector, typed deterministic simulation, headless replay, canonical checkpoints, interpolated Bevy presentation and texture overrides. All 341 required source files are represented: 164 decoded sources and 177 retained for later subsystems, 2,975 semantic entries, no conversion errors. Fine unknown semantics remain explicitly unreviewed.

Current verification: 73 Rust tests, 10 Python tests, formatting, workspace Clippy, release build, bounded mutation fuzzing, real-disc conversion, byte-identical warm rebuild, actual graphical replay runs and browser inspector review. Checkpoint hashes match across four controlled presentation schedules and a live texture switch. See [Parts 04–06 evidence](evidence/parts-04-06.md). Historical clean-container receipts cover the earlier source hashes only; a fresh remote Actions run is not claimed.

The app is an authored road/handling scaffold that can display imported rider frames. Classic source-course rendering/integration, complete handling/animation/combat/AI, career, saves, audio/video playback and reviewed AI enhancement packs remain future work. 3D is deferred. The next phase is **08: classic road rendering**. No complete playable recreation or original-runtime parity is claimed.

README parents close only when their remaining children and linked evidence pass. The removed original-runtime children retain stable retired IDs; later phases remain open.

Part 07 reconstructs all five source course graphs: 109 sections, 141 nodes, 26 branch combinations, 130 level-specific finishes and 132 verified seam transitions. The browser explorer, deterministic placements, direct endpoint cursors and full coverage/reproducibility reports are complete. See [course contract](COURSES.md) and [Part 07 receipt](evidence/part-07.md). Procedural terrain meshes and source-course driving integration remain Phase 08 work; raw source resets and empty family slots are explicitly reported.
