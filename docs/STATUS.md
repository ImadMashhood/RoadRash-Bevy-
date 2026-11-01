# Implementation status

The target is sequential implementation through Phase 23. This receipt describes actual completed work, not completion of the game.

## Current gate

Phase 01 is in progress: scope, platform choice, provenance, partial content inventories, confidence labels and acceptance criteria are written. Original menu transitions, original bindings, camera measurements and full behavior capture remain open. Phases 02–04 contain enabling foundation work; they are not certified complete. Phases 05–23 have not started.

## Implemented and checked locally

- Standalone Rust workspace and locked dependencies; Rust 1.99.1 and Bevy 0.20.1.
- Authored Bevy 3D foundation scene: road slab, actor placeholder, camera and light. `cargo check --locked -p rr-game` passed. Runtime window/GPU behavior has not been verified.
- Independent BOB reader with bounded frame count and byte slices. Three Rust tests passed, including all frame truncations and invalid-dimension/trailing-data cases. Original corpus sprite decoding remains unverified.
- Read-only ISO9660 verifier with bounded directory traversal, extent validation and streaming hashes. Six authored Python tests passed. Supported private ISO audit matched 341/341 required files, with zero failures.
- Formatting and format-crate Clippy passed. CI workflow is authored but no remote CI run is claimed.
- Source-derived inventories: 15 bike slots, eight character asset stems, five original course mappings × five level selections, and a partial screen index. These are upstream-reported metadata, not original gameplay captures.

## Next items, in order

1. Complete original-game capture setup and Phase 01 menu/input/camera evidence. Use baseline/RESEARCH_GAPS.md; do not silently promote source-derived inventories to verified observations.
2. Verify actual 3D app launch and establish a hardware/build CI receipt for Phase 02.
3. Turn the Phase 03 read-only audit into a complete production importer: extracted-folder support, staging, cancellation, atomic activation and recovery. These are not implemented yet.
4. Complete format coverage and original-corpus validation before interpreting model/texture extraction as complete.
5. Continue the phase gates in README through packaged release (Phase 23).

## Reproduction

See GETTING_STARTED.md for commands. The disc receipt is evidence/disc-corpus.json. Public code and manifests contain no original payload. The parent local prototype has not been modified.
