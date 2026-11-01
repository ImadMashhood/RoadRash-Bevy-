# Part 17 — Saves, settings and migration

Complete within [SAVES.md](../SAVES.md). Implementation revision is the commit containing this receipt and the checked roadmap. The next open phase is **19: complete the native-asset offline game**, followed by UI/localization, verification and native packaging. Texture enhancement is now Part 24 and cannot begin before accepted G5.

Five validated slots, schema-2 integrity/content contracts, explicit v1 JSON migration, atomic saves, previous-generation backups, preserved corrupt primaries, independent schema-1 preferences and validated input bindings are implemented. Actual cold restart resumes selected slot 1 and reloads master/music levels. Original binary-save import is deliberately deferred; Linux is the tested platform.

Local verification: **319 Rust tests** (285 headless + 34 Bevy/application), **15 Python tests**, formatting, workspace Clippy with warnings denied, locked release workspace build, negative workflow guards and a fresh read-only **341/341** source-disc audit. Source factory acceptance explicitly receives its private corpus/catalog. No remote CI run is claimed. Native source-media checks pass **502 cases**: 86 PCM sources, 384 RPM cases, 28 pitch cases and four AVI comparisons. Original-runtime comparisons remain waived.

Reproduction from the repository root (use new private output directories/filenames for repeated window runs):

```sh
cargo fmt --all --check
cargo clippy --locked --target-dir ../target --workspace --all-targets -- -D warnings
cargo test --locked --target-dir ../target -p rr-formats -p rr-simulation -p rr-import -p rr-content -p rr-course -p rr-render -p rr-input -p rr-session -p rr-media
cargo test --locked --release --target-dir ../target -p rr-game -p rr-presentation
cargo build --locked --release --target-dir ../target --workspace
python -m unittest discover -s tools -p 'test_*.py' -v
python tools/check_workflow_guards.py
# Explicit private checks; native reference setup documented in NATIVE_REFERENCE.md:
RR_SESSION_DATA="$PWD/local/cdrom/ROADRASH" RR_SESSION_CATALOG="$PWD/local/catalog" RR_SESSION_METRICS="$PWD/local/new-source-lifecycle.json" cargo test --locked --release --target-dir ../target -p rr-game source_factory_repeated
source local/reference-tools/env.sh
python tools/reference/check_media.py --data local/cdrom/ROADRASH --output local/new-media-native
RR_TEST_ISO=local/source.iso python tools/private_corpus.py --require
../target/release/rr-game --install-root local/game-install --catalog local/catalog --profile-root local/new-profile --frontend on --ui-script docs/fixtures/persistence-media-tour.json --smoke-frames 580 --capture local/new-movie.png --capture-frame 280 --media-report local/new-media.json
```

The output transport check used a dedicated virtual PipeWire sink and its monitor, with explicit ALSA routing; no microphone/system-output capture or physical default change was used. A separate controlled unavailable output ran the same source replay. All tick hashes and three cue timestamps match connected and headless runs: final state `5b6472fbfc82818d6e5034a1bd817ec48ccdb63b8428b3005bb71d180f64412e`. Frame-independent synthetic cue tests cover 30/60/144/irregular schedules. Media-clock synthetic checks cover ten minutes, stalls and pause separately from sample/transport checks.

[Checks](persistence-media-checks.json), [native primitives/AVI samples](media-native.json), [actual graphical and digital-output review](persistence-media-visual.json). Private imports, PCM, captures, source-derived configurations and traces are excluded. Original RGB bit identity, subjective listening calibration, physical hotplug and other OS execution are not claimed.

| Task | Receipt |
| --- | --- |
| 17.01 | [Define versioned save schema](17.01.md) |
| 17.02 | [Separate settings from career progress](17.02.md) |
| 17.03 | [Use atomic save writes](17.03.md) |
| 17.04 | [Keep recoverable backup](17.04.md) |
| 17.05 | [Validate loaded data before world mutation](17.05.md) |
| 17.06 | [Test truncated and corrupt saves](17.06.md) |
| 17.07 | [Implement explicit schema migrations](17.07.md) |
| 17.08 | [Record content pack compatibility](17.08.md) |
| 17.09 | [Persist input bindings and audio levels](17.09.md) |
| 17.10 | [Test interrupted writes](17.10.md) |
| 17.11 | [Decide original-save import scope](17.11.md) |
| 17.12 | [Document save location and reset procedure](17.12.md) |
