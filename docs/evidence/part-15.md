# Part 15 — Race rules and results

Complete within the declared modern offline scope in [race rules](../RACE_RULES.md) and [frontend/economy](../FRONTEND.md). Implementation revision is the commit containing this receipt and the checked roadmap. The next phase is **17: saves, settings and migration**.

Local verification: **287 Rust tests** (262 headless, including 27 session scenarios; 25 Bevy/application tests), **12 Python tests**, formatting, workspace Clippy with warnings denied, locked release workspace build, negative workflow guards, guarded source-overlap rejection and a fresh read-only **341/341** disc audit. The private source factory test was explicitly supplied its corpus/catalog and wrote source lifetime/hash/RSS observations; public CI can run without commercial data. No remote CI success is asserted.

Native career comparison passes **1,613 cases**: 8 character/default-bike rows, 15 prices, 15 prizes, 900 purchases and 675 settlements. Reference `7aecf6044f2dd773b925b325823f30d648d61c89`; supported executable SHA-256 `73d7f9744b5b1a9482b3909950cdf3fe34119fe13d023b031e9c6739a60f18a2`. These are API/data matches, not original-runtime or complete frontend parity.

The independently authored [race-rule fixture](../fixtures/race-rules-replay.json) crosses the finish after two simulation ticks following 180 countdown ticks. Four frame schedules and effects on/off preserve the immutable result, state and nonempty final checkpoint. The final result hash is `ff27c8e2ad11d5fe2f0f0987da655a8b0fdf9889a60dbbbf9b520aba393093f6`. Older schema-4 and schema-5 fixtures retain their expected hashes; an actual schema-5 Bevy window is checked against headless checkpoints.

Private graphics review covers the character screen/portrait, source bike preview/specifications, frozen countdown, quit results/15-rider standings and final menu cleanup. The committed [UI tour](../fixtures/menu-tour.json) exercises Big Game/club/showroom/settings/jukebox, wide framing, a City race, a level-five Sierra Nevada quick race, a restart and clean return. The tour uses real source assets and normal UI actions; it does not inject fabricated win events. Real finish/wreck/arrest adapters and money/progression boundaries are exercised separately in tests. Captures remain private.

The [inventory audit](frontend-menu-audit.json) covers **36 screens and 175 items** and explicitly lists modern approximations, unavailable original screens, independent nitro/dismount input gaps, source-exact dialogue gaps and later-phase save/media dependencies. A one-file career snapshot bridge supports current resume; full save-slot/migration/settings persistence is Part 17, and audible RRA/music/video/credits playback is Part 18. A prorated manual repair and normalized branch ranking are declared modern policies.

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p rr-formats -p rr-simulation -p rr-import -p rr-content -p rr-course -p rr-render -p rr-input -p rr-session
cargo test --locked --release -p rr-game -p rr-presentation
cargo build --locked --release --workspace
python -m unittest discover -s tools -p 'test_*.py' -v
python tools/check_workflow_guards.py
cargo run --locked --release -p rr-session --bin rr-race-check -- docs/fixtures/race-rules-replay.json irregular /tmp/new-race-report.json
# Explicit private source acceptance:
RR_SESSION_DATA="$PWD/local/cdrom/ROADRASH" RR_SESSION_CATALOG="$PWD/local/catalog" RR_SESSION_METRICS="$PWD/local/session-source-lifecycle.json" cargo test --locked --release -p rr-game source_factory_repeated
cargo build --locked --release --target-dir ../target --workspace
source local/reference-tools/env.sh
python tools/reference/check_economy.py --data local/cdrom/ROADRASH --output local/session-native
RR_TEST_ISO=local/source.iso python tools/private_corpus.py --require
```

For the graphical tour: `rr-game --install-root local/game-install --catalog local/catalog --frontend on --ui-script docs/fixtures/menu-tour.json --smoke-frames 520 --career-file local/tour-career.json`. Capture a chosen frame with `--capture local/tour.png --capture-frame 515`. The character and bike captures use the same typed actions: begin BigGame, choose character 1, then open Showroom/select bike 1. The quit-result capture begins Thrash, selects character/course 1, opens Paused/QuitConfirm, and confirms quit.

[Checks](session-checks.json), [native comparison](session-native.json), [frame schedules and legacy replay](session-replays.json), [source lifetime/preview evidence](session-source.json), [graphical review](session-visual.json).

| Task | Receipt |
| --- | --- |
| 15.01 | [Implement countdown and race start](15.01.md) |
| 15.02 | [Compute progress across route branches](15.02.md) |
| 15.03 | [Implement placement and standings](15.03.md) |
| 15.04 | [Implement finish detection](15.04.md) |
| 15.05 | [Resolve simultaneous finish events](15.05.md) |
| 15.06 | [Implement race failure conditions](15.06.md) |
| 15.07 | [Implement pause resume restart quit](15.07.md) |
| 15.08 | [Implement results presentation](15.08.md) |
| 15.09 | [Record race rewards and penalties](15.09.md) |
| 15.10 | [Verify race-to-menu cleanup](15.10.md) |
| 15.11 | [Test repeat races without leaked state](15.11.md) |
| 15.12 | [Verify intended outcome and frontend state transitions](15.12.md) |
