# Part 12 — Crashes and recovery

Complete within the current declared scope. Implementation revision is the commit containing this receipt, the checked roadmap and linked code. The [combat contract](../COMBAT.md) and [recovery contract](../RECOVERY.md) identify researched constants, intended rules and remaining presentation limits. Task 11.12 remains retired by the owner; it is not counted as a passed comparison.

Verification: **208 Rust tests** (194 headless, 14 Bevy adapter tests), including 44 new encounter scenarios and a new detached-render-proxy test; 10 Python tests; formatting; workspace Clippy with warnings denied; release workspace build; both negative workflow guards; immutable-source output guards; 341/341 disc hashes. No remote CI run is asserted by this receipt.

The native reference is pinned at `7aecf6044f2dd773b925b325823f30d648d61c89`. Separately authored API adapters match 108 generic displacement vectors and 35 reason-scaled damage vectors. This narrow comparison is not full native race or original Windows parity. Source startup validates 50 setups (five courses × five levels × all-left/all-right branch selections), including all road-node connections and 15 practice riders per setup. Four schedules and effects on/off preserve every checkpoint; actual Bevy 30/144-window runs match headless state. Source-mode practice riders use constant drive; Phase 13 AI remains open.

The public authored replay expects SHA-256 `34cc853ad681b7ba53035e2aa6367d195807365c81d54f06ae9337ba4e4abccd`. Source-derived replay/configuration stays private. Legacy schema-3 showcase remains `b475c015f69bcd555262de0c8c721d969e2c6b362546deb5d0dea9d626f1abff`; existing schema-1/2 tests preserve their pinned hashes.

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p rr-formats -p rr-simulation -p rr-import -p rr-content -p rr-course -p rr-render -p rr-input
cargo test --locked --release -p rr-game -p rr-presentation
cargo build --locked --release --workspace
python3 -m unittest discover -s tools -p 'test_*.py' -v
python3 tools/check_workflow_guards.py
cargo run --locked -p rr-simulation --bin rr-replay -- docs/fixtures/encounter-replay.json irregular /tmp/encounter-report.json
# Private data/native checks (see NATIVE_REFERENCE.md for bootstrap):
cargo build --locked --release --target-dir ../target -p rr-simulation --bins
source local/reference-tools/env.sh
python3 tools/reference/check_encounters.py --data local/cdrom/ROADRASH --output local/encounter-acceptance
RR_TEST_ISO=local/source.iso python3 tools/private_corpus.py --require
```

[Checks](encounter-checks.json), [native primitive receipt](encounter-native.json), [source setup/replay receipt](encounter-replays.json), [visual review](encounter-visual.json), [isolated scenarios](../fixtures/combat-scenarios.json). Private graphical runs use `--replay local/encounter-acceptance/showcase-replay.json --render-hz 30` or `144`, with the verified install/catalog/course arguments. Capture-frame 40 at 30 records Sliding; frame 720 at 144 records Walking. Effects are disabled in the 144 run. Controlled presentation schedules are not hardware FPS benchmarks.

| Task | Deliverable | Representative passing test |
| --- | --- | --- |
| [12.01](12.01.md) | Linked rider and bike entities | rider_and_bike_move_independently_after_separation |
| [12.02](12.02.md) | Slide, tumble and decay | slide_travel_and_stop_timing_increase_with_entry_speed |
| [12.03](12.03.md) | Obstacle and terrain consequences | collision_and_obstacle_policies_distinguish_decorative_and_solid |
| [12.04](12.04.md) | Persistent bike target | culling_or_modifying_copied_snapshot_cannot_delete_recovery_target |
| [12.05](12.05.md) | Tick-driven get-up | full_recovery_visits_every_phase_without_teleporting |
| [12.06](12.06.md) | Walking to a linked bike | walking_routes_backwards_and_to_neighboring_branch_without_timeout |
| [12.07](12.07.md) | Remount gates and concurrent contact | remount_distance_and_orientation_edges_are_explicit |
| [12.08](12.08.md) | Repeated recovery impacts | repeated_impact_does_not_replace_body_identity_or_snap_position |
| [12.09](12.09.md) | Finish, arrest and wreck ordering | terminal_outcomes_prioritize_arrest_and_latch_once |
| [12.10](12.10.md) | Bounded unreachable-target fallback | removing_bike_produces_explicit_terminal_fallback |
| [12.11](12.11.md) | Slopes and split/merge navigation | grade_integration_crosses_exact_portal_endpoint |
| [12.12](12.12.md) | Explicit recovery assists | fast_recovery_is_faster_and_default_retains_unassisted_timeline |
