# Part 10 — Rider animation and palette fidelity

Complete within the visual-state boundary described by the roadmap. Implementation revision is the commit containing this receipt, code and checked tasks. [Animation contract](../ANIMATION.md) and per-task receipts 10.01–10.12 define mappings, timing, view thresholds, equipment/crash/on-foot inputs, palettes, shadow masks, mirroring and transition policies. Combat targeting and detached-body/recovery decisions remain their later phases.

The [native comparison matrix](riding-native.json) matches 10,015 source playback rows in both banks, including selected frame, deadline, loops, descriptor, duration, control and contact flags. Nine yaw/roll pairs for every row give 90,135 matching view/correction cases. It also compares all 2,790 nonempty runtime identity/frame combinations and the nonempty source shadow bank by cropped indexed-pixel hash. Source mirroring uses flag 1, while 0x10 corrects roll; source logical-size flags are honored. The runtime bank has all 15 identities without altering the earlier ten-preview catalog contract.

Twenty-four [authored domain tests](../../crates/simulation/tests/riding.rs) include missing-state rejection, source timing/contact, fallback/failed-selection rollback, directional mirror flags, weapon replacement/drop, crash speed/height thresholds, queued standing/remount and interrupted pose transitions. Two new [rendered regressions](../../crates/render/tests/classic.rs) verify independent grounded shadow pixels, per-frame pivots/logical dimensions, unchanged zero-angle rendering and nonzero body rotation. Old authored image goldens still pass. Body/shadow source frames are loaded at startup; draw systems never read files.

The actual exporter produced 48 labeled sequence sheets, 186 nonempty body-frame exports, a JSON timing/pivot manifest and a standalone offline HTML inspector. Color 14 also exported successfully. Browser review selected source sequence 10, stepped a tick, changed playback to 0.5×, selected tick 15 and preview height 25. The visible report showed frame 83 with contact true, a changed mirror flag and a shadow retained at ground contact. Screenshot and all commercial pixels remain private; only their hashes/metadata are in [the visual receipt](riding-visual.json). Attack-to-crash/get-up-to-walk hard cuts retain the same world anchor; no interpolation or enhancement resolution changes a simulation deadline.

```sh
cargo build --locked --release -p rr-render --bin rr-riders
python3 tools/animation/preview.py --data local/cdrom/ROADRASH --output local/animation-preview --color 0 --binary target/release/rr-riders
# Open local/animation-preview/index.html (no server required).
target/release/rr-riders local/cdrom/ROADRASH local/animation-palette14 14
```

Pillow is only a sheet-generator dependency. Use the shared workspace's `../target/release` if built there. The generator keeps non-looping clips within their state boundary, preserves empty frame slots and reports exact timing/pivots. The source/output guard refuses overlap with the imported files before writing.

Actual schema-3 Bevy replay windows at 30/144 schedules retain identical checkpoints/final snapshots and show the imported rider animation/shadow on a source course. A separate live window verifies automatic imported handling selection. [Graphical/replay results](riding-replays.json). [Part 09](part-09.md) lists full build/test commands; [totals](riding-checks.json) record 163 Rust tests, 10 Python tests, warnings-denied Clippy, release build and guards. No physical controller model, full original frame parity, complete combat/recovery game or remote CI execution is claimed. [Public-file audit](public-file-audit-parts-09-10.json).
