# Part 09 — Input and player handling

Complete within the roadmap's listed input/handling boundary. Implementation revision is the commit containing this receipt, code and checked tasks. Previously accepted 09.01/09.02/09.03/09.11 evidence remains linked from the README. This change closes 09.04–09.10 and 09.12. Full combat/recovery/race decisions remain later phases; no original Windows runtime parity or physical controller certification is claimed.

Implementation: [source handling](../HANDLING.md), [safe Rust handling](../../crates/simulation/src/handling.rs), [state composition](../../crates/simulation/src/riding.rs), [schema/replay](../../crates/simulation/src/lib.rs), [Bevy entry point](../../apps/game/src/main.rs). Startup binary offsets become named fields; ticks do not read files or render time. The existing EXE SHA-256 guard authorizes source tables. All 15 SPEC IDs pass the same schema. Invalid configuration fails before activating the new simulation. Imported live races use schema 3; legacy/default scenarios retain schema 2 and schema-1/2 fixture hashes.

[Native matrix](riding-native.json) matches every compared field: 252,000 drive/gear/brake/coast/steering/surface rows, 8,100 full Race vertical-impulse rows and 27,000 full Race combined curved-road/crest rows across all 15 bikes. The combined check caught the negative bend sign and the source's retained previous-sample rise; the implementation now reproduces them. JSON values match exactly; raw JSON-file hashes differ because native/Rust whitespace differs. The metadata includes a shared canonical-value SHA-256.

[Authored tests](../../crates/simulation/tests/riding.rs) supply 24 independent behavioral checks. They include three speeds/three steering durations, strict surface boundaries, stopping distance/no reverse, drift/grade/flight/landing, damage equality/repetition/one-shot delivery, schema validation and configuration/hidden-state hashing. [Hand-calculated fixture](../fixtures/riding-regression.json) has first-five-tick acceleration and braking answers, including the source's negative brake-rate equality. Four actual SVG plots are generated locally from the measured native traces and integrated Rust CSV; neither chart data nor source-derived gameplay configuration is published.

[Replay/graphical receipt](riding-replays.json) records source schema-3 comparisons across 30/60/144/irregular schedules, two actual Bevy 30/144 replay windows and a 120-frame live imported-race window. Corresponding checkpoints and final snapshots agree. Schema-1 remains `f70c202279f360fc89e95d503443a91a523c816788d9e62102a6d3572fc7f143`; schema-2 remains `87faf52e185aa21028d6be58fb01403d1f94558d6960aea7eadbedc2544f675e`. These virtual schedules measure determinism, not hardware FPS.

Reproduce from the repository with the verified local import and built native reference:

```sh
cargo build --locked --release --workspace --bins --examples
source local/reference-tools/env.sh
python3 tools/reference/check_riding.py --data local/cdrom/ROADRASH --output local/riding-native --rust target/release
target/release/rr-riding-check local/cdrom/ROADRASH local/riding-acceptance
python3 tools/animation/plot_handling.py --native-trace local/riding-native/handling-native.json --riding-trace local/riding-acceptance/riding-trace.csv --output local/riding-plots
cargo test --locked -p rr-formats -p rr-import -p rr-content -p rr-course -p rr-render -p rr-input -p rr-simulation
cargo test --locked --release -p rr-game -p rr-presentation
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
python3 -m unittest discover -s tools -p 'test_*.py' -v
python3 tools/check_workflow_guards.py
```

This workspace uses `--target-dir ../target`; substitute `../target/release` for its binaries. The separately authored C++ adapters call the pinned API and compile with warnings denied. No native gameplay logic was patched. [Native reference provenance/build](../NATIVE_REFERENCE.md).

[Verification totals](riding-checks.json): 163 Rust tests (149 data-free/headless plus 14 Bevy tests), 10 Python tests, all-target Clippy with warnings denied, formatting, release binaries/examples and negative workflow guards pass. Existing authored visual goldens remain unchanged. Local source/output-overlap checks protect the immutable import. Fresh remote Actions/container execution is not claimed. Only source, authored fixtures, documentation and metadata/hashes are published; [public-file audit](public-file-audit-parts-09-10.json).
