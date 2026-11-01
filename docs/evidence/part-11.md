# Part 11 — Combat and contact

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
| [11.01](11.01.md) | Data-driven startup, active and recovery windows | imported_clip_windows_use_first_and_last_selected_tick |
| [11.02](11.02.md) | Side and reach boundaries | reach_strict_boundaries_side_height_and_relative_position |
| [11.03](11.03.md) | Stable target selection | nearest_behind_wins_ties_then_stable_actor_id |
| [11.04](11.04.md) | Punch damage and reactions | punch_lands_once_and_overlap_does_not_repeat |
| [11.05](11.05.md) | Kick displacement and reactions | kick_impulse_and_condition_damage_are_symmetric |
| [11.06](11.06.md) | Unique equipment and theft | inventory_transfer_has_one_owner_even_with_competing_requests |
| [11.07](11.07.md) | Independent health, stun and knockoff | damage_formula_reason_scaling_and_health_zero |
| [11.08](11.08.md) | Simultaneous resolution and removal | simultaneous_attacks_are_repeatable_and_low_id_ordered |
| [11.09](11.09.md) | Cosmetic event boundary | effects_are_observers_and_do_not_consume_gameplay_rng |
| [11.10](11.10.md) | Bike and scenery contact | source_contact_speed_threshold_and_separation |
| [11.11](11.11.md) | Isolated scenarios and replay | all_attack_weapon_side_scenarios_have_tick_range_expectations |
