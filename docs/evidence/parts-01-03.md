# Historical Parts 01–03 receipt

This receipt describes the earlier commit. Original-runtime blockers below were subsequently retired by the [owner scope amendment](../SCOPE_CHANGES.md); current Part 01 closure and texture-switch evidence are in [Parts 04–06](parts-04-06.md). Earlier test totals and unimplemented-feature statements are historical.

# Parts 01–03 implementation and acceptance receipt

Scope: classic Rust/Bevy recreation. 3D remains deferred. The app is still an authored architecture scaffold, not a complete recreation.

## Completed implementation

- Rust `rr-import` library/CLI: bounded ISO9660 and extracted-directory readers, case normalization, manifest/hash/size verification, per-file errors, cancellation, serialized imports, staging, immutable generations, atomic pointer activation, interrupted staging recovery and versioned provenance.
- Supported manifest now records verified file sizes as well as SHA-256 for 341 required files. No original payload is committed.
- Game launch accepts `--install-root`, resolves the validated generation, and supports bounded smoke frames and screenshots. Rendering preserves a 640×480 logical viewport with 4:3 pillarboxing.
- Build/toolchain/dependency gates, authored fixture pack, private-corpus skip/require reporting, negative formatting/lockfile checks, platform dependencies, architecture decisions, milestone graph and release acceptance records.
- Binary metadata inventories cover 36 original screen tables (175 base rows), 13 popup tables, three course-menu extra rows, 15 selectable bikes, eight characters, original binding words and course distance labels. Runtime handler semantics are explicitly unverified.

## Verification

- `cargo test --locked -p rr-formats -p rr-simulation -p rr-import`: 20 Rust tests passed (3 formats, 2 simulation, 15 import).
- `python3 -m unittest discover -s tools -p 'test_*.py' -v`: 10 tests passed on current source.
- `cargo fmt --all --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- Release build and 180-frame actual desktop smoke launch: passed. Screenshot visually inspected for correct authored scene and pillarboxing. See [platform-smoke.json](platform-smoke.json).
- Process acceptance against real supplied media: 341-file ISO and read-only folder imports passed, cancellation/forced termination retained active pointer and launch, abandoned stages recovered, modified original executable rejected. See [private-import.json](private-import.json).
- Negative CI gates reject deliberately unformatted source and a changed dependency requiring lockfile updates. See [workflow-guards.json](workflow-guards.json).
- Public source and dry-run package audit: see [public-file-audit.json](public-file-audit.json).
- Clean official Rust container: receipt recorded separately in [clean-build.json](clean-build.json); [source hashes](clean-build-source-hashes.json) identify the Rust source actually checked. Remote GitHub Actions execution is not claimed from a local container run.

An initial root-container permission test correctly exposed that root bypasses chmod restrictions. The clean recipe now builds/tests as an unprivileged user. An early development receipt lacked the later build-hash field; schema-1 legacy receipts are accepted with an explicit `legacy-unrecorded` build identifier. Newly imported receipts carry the build hash.

## Part 01 remaining acceptance

These items are still open, even though source metadata has advanced:

- 01.02: original screenshot/camera comparisons at matching course positions.
- 01.03: complete runtime-dependent menu/handler/entry/cancel transitions and walkthrough.
- 01.04: all 25 original course/level selections, unlock conditions and actual finish-event evidence.
- 01.06: original keyboard/joystick held/pressed semantics and simultaneous input captures.
- 01.10.b: actual native/enhanced texture switching during a replay with equal simulation hashes; the full texture pipeline is not implemented yet.

Original executable launch was attempted using an isolated Proton 11.0 prefix. It exited 53 before gameplay. Static imports require AWEMAN32.dll, absent from the supplied 341-file game corpus; original live capture is therefore not certified. The original executable was not patched. A working original installation/runtime is needed to finish those captures. Source inspection is not substituted for a successful runtime comparison.

No complete Part 01 or whole-game parity claim is made. Checkbox evidence tracks completed requirements individually.

The project owner confirmed that no working original installation is available. Original-runtime acceptance remains open; independent development proceeds using verified source data and explicitly labeled research evidence.
