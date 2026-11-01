# Part 14 — Traffic, pedestrians and police

Complete within the declared modern engine scope. Implementation revision is the commit containing this receipt and the checked roadmap. [Opponent contract](../OPPONENTS.md) and [road-user contract](../ROAD_USERS.md) identify source-derived primitives, explicit policy choices and fidelity limits. Earlier schemas remain available; the next phase is 15.

Verification: **251 Rust tests** (235 headless and 16 Bevy/application tests), **11 Python tests**, formatting, workspace Clippy with warnings denied, release workspace build, source-overlap/output guards, negative workflow guards and the verified 341-file disc corpus. This includes 38 population integration scenarios, an additional portal-pursuit unit case, rendering/interpolation/alias regressions and live-recording/cue adapter tests. No remote CI outcome is asserted here.

The separately authored native adapters match **3416 scalar/decoder/vehicle-choice cases**: 480 cadence, 6 density configurations, 2240 arrest, 50 personality-level records and 640 vehicle choices. The source appearance census checks all 54 vehicle models/162 cached views and confirms all seven unavailable pedestrian queries against the native lookup. No original executable is run. Reference: `7aecf6044f2dd773b925b325823f30d648d61c89`; executable SHA-256 `73d7f9744b5b1a9482b3909950cdf3fe34119fe13d023b031e9c6739a60f18a2`.

Fifty source setups cover five courses × five levels × all-left/all-right branch choices, with 15 racers and 14 brains. Four 180-second source runs preserve every checkpoint at 30/60/144/irregular schedules. Another four runs use a complete captured live setup/control stream, extended with held final controls. The unattended live recording itself replays to its recorded final hash. This validates the recording/comparison workflow; it does not claim a human driving session or original-game calibration. Scheduler, RNG and motion observations have independent first-difference reporting.

Actual Bevy 30/144 runs of the private source-asset visual fixture match headless hashes/checkpoints, with effects disabled in the 144 case. The source field, traffic, pedestrian and police warning were reviewed in captures. Virtual presentation rates are not hardware FPS benchmarks. Native full-race trajectory/RNG parity is not claimed.

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p rr-formats -p rr-simulation -p rr-import -p rr-content -p rr-course -p rr-render -p rr-input
cargo test --locked --release -p rr-game -p rr-presentation
cargo build --locked --release --workspace
python3 -m unittest discover -s tools -p 'test_*.py' -v
python3 tools/check_workflow_guards.py
cargo run --locked --release -p rr-simulation --bin rr-replay -- docs/fixtures/population-replay.json irregular /tmp/population-report.json
# Private verified source data and pinned reference build:
cargo build --locked --release --target-dir ../target --workspace
source local/reference-tools/env.sh
python3 tools/reference/check_population.py --data local/cdrom/ROADRASH --output local/population-acceptance
python3 tools/reference/check_population_art.py --data local/cdrom/ROADRASH --catalog local/catalog --output local/population-art-acceptance
RR_TEST_ISO=local/source.iso python3 tools/private_corpus.py --require
```

[Checks](population-checks.json), [native primitives](population-native.json), [source setup/long-run metadata](population-replays.json), [recorded-input comparison](population-recorded.json), [appearance verification](population-art.json), [graphical review](population-visual.json), [public authored fixture](../fixtures/population-replay.json). Raw imports, source-derived replays, images and traces stay under ignored `local/` paths.

| Task | Deliverable | Representative passing case |
| --- | --- | --- |
| [14.01](14.01.md) | Source traffic zones | `density_timer_sign_and_reload_semantics` |
| [14.02](14.02.md) | Lane and crossing movement | `crossing_vehicle_moves_laterally_without_changing_path` |
| [14.03](14.03.md) | Vehicle impact responses | `vehicle_rear_headon_and_crossing_impacts_are_distinct` |
| [14.04](14.04.md) | Pedestrian requests and trajectories | `pedestrians_spawn_once_cross_and_hit_once_per_lifetime` |
| [14.05](14.05.md) | Pedestrian hit outcomes | `pedestrians_spawn_once_cross_and_hit_once_per_lifetime` |
| [14.06](14.06.md) | Patrol, pursuit and disengagement | `pursuit_enters_detection_and_disengages_with_terminal_cleanup` |
| [14.07](14.07.md) | Siren and warning lifecycle | `siren_pause_resume_remove_restart_reconcile` |
| [14.08](14.08.md) | Arrest eligibility | `source_arrest_thresholds_are_strict` |
| [14.09](14.09.md) | Arrest during recovery | `arrest_in_each_on_foot_phase_ends_recovery_once` |
| [14.10](14.10.md) | Branch ownership | `crossing_contacts_on_each_branch_keep_association` |
| [14.11](14.11.md) | Spawn order, identity and random draws | `trace_categories_identify_first_different_draw_without_rng_changes` |
| [14.12](14.12.md) | Dense simultaneous incidents | `full_field_with_full_road_user_pools_and_simultaneous_incidents_is_bounded` |
