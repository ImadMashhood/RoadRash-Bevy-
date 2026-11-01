# Opponent intelligence contract

Schema 5 adds population decisions to the validated schema-4 handling/combat/recovery foundation. The application starts a full **15-rider field** by default. `--opponents 0..14` selects an opponent count; `--practice-riders N` retains the earlier constant-drive schema-4 practice mode. Opponents use their assigned source bike specification, independent stable actor IDs, source palette identity and the same crash/recovery state machine as the player.

## Modules and authority

- `crates/simulation/src/population.rs`: validated configuration, identities, runtime records, transient diagnostics and siren ownership adapter.
- `population/source.rs`: startup-only executable/course/vehicle-table decoding.
- `population/primitives.rs`: independently testable scheduling, density, arrest and vehicle-choice rules.
- `population/opponents.rs`: stable decisions, corner-speed targets, gap reservations and combat commands.
- `population/road_users.rs`: bounded spawn/movement/contact/pursuit transactions.
- `apps/game/src/race_recording.rs`: live input capture and bounded replay-file output.

The fixed stage order is Input → Spawn → Decisions → Movement → Contacts → Combat → Progression → Snapshot. Decisions emit domain commands; they never drive Bevy entities. Presentation entities, interpolation, asset caches and siren voices cannot change actor order or consume gameplay RNG. The new `with_population` factory requires validated handling, encounter and population setups. Earlier replay factories and canonical hashes remain intact.

Setup is plain editable serialized data: traits by ActorId, source zones, actor requests, traffic enablement, police enablement/detection and vehicle identities. Every hidden brain deadline, target, lane reservation, pending contact, NPC phase and shared RNG state is canonical. Per-opponent handling configurations are derived once from the canonical specifications and assigned bike IDs; this cache is excluded from serialization. Debug traces, effect events and presentation loop ownership are also excluded, and tests verify that toggling them cannot change state.

## Recovered scheduling and intentional policy

Research is pinned to arkits/road-rash revision `7aecf6044f2dd773b925b325823f30d648d61c89`: [scheduler](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_schedule.cpp), [personalities and opponent setup](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race.cpp), and [field construction](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_setup.cpp).

The source initializes player cadence to 1 and opponent cadence to 4, with the first due tick equal to the build tick. Its dynamic interval is 30 when no road slice is present; otherwise distance from the ring camera determines 1–8 ticks, with an additional slow-opponent delay before tick 300. `live_interval` reproduces that scalar arithmetic and matches 480 native vectors at boundary positions, speeds, ticks and player flags.

This engine applies that interval to **decisions** and advances all physical riders at the common fixed 60 Hz step. It does not reproduce the upstream variable-dt bike/half scheduler, ring parking, pace rubber bands or every historical position-list tie. Due opponents resolve in descending route progress, then descending ActorId. This explicit stable policy is tested at spawn/due boundaries and under cosmetic insertion. It is an intended modern architecture decision, not a claim of complete native scheduler parity.

## Routes, speed and overtaking

Each brain records desired lateral lane, target speed, target actor, next decision tick, cooldown and the last emitted command. Source profiles supply per-level normal and alternate pace tables. The alternate table is used as a bend-speed ceiling in this implementation. Source pace units are inverted with a bounded integer binary search and capped by the assigned bike's gear limits. A twelve-sample lookahead detects bends; a known authored corner verifies braking versus acceleration on a straight. That mapping and lookahead policy are explicit adaptations.

Route following remains in source road coordinates. Steering approaches a preferred road-relative lane; an optional branch choice selects a real outgoing portal. Tests traverse both fork options. Non-selected-route physics retains the previously documented ground-profile limitation in [RECOVERY.md](RECOVERY.md); selected-route projection and topology membership remain distinct.

Overtaking scans nearby live bikes and road users, compares longitudinal gap and relative speed, and evaluates five preferred road-relative lateral corridors in the functional factory (legacy replays retain three). Earlier decisions reserve their gap for that tick. A blocked field yields and reduces target speed instead of forcing two riders into the same opening. Tests cover slow traffic, all lanes blocked, competing overtakes and remounting into a blocked road. This is a deterministic clearance policy; it is not the full upstream scan-list/overtake-index algorithm.

## Personalities, combat and recovery

Startup reads the ten 0x204-byte personality records at VA 0x464558, including normal/alternate pace, overtake index and kick/backhand rolls. The source field uses rows 1–8, then the unjittered row-0 fallback for slots 9–14. Grid tables are 0x464088 or 0x464108; per-level bike assignments come from 0x4641c8; stamina/equipment come from the rider rows at 0x463c78. All 15 source field slots validate across five courses, five levels and both uniform branch choices. Native comparison verifies 50 decoded personality/level records.

Identified values retain their source names and provenance. Unsupported aggression/relationship semantics remain `None`; the modern policy has a documented default attack gate and 90-tick cooldown. It does not invent named faithful personalities, career jitter, per-character weight effects or native aggression calibration. Normal/alternate pace differences affect behavior; other decoded fields remain available for later policy refinement.

AI attacks require a live mounted target on the same local path, within longitudinal and lateral reach. A due decision chooses attack kind and side through the shared gameplay RNG and respects cooldown, stun and an existing swing. Combat rechecks eligibility at contact, so a crashed/removed/out-of-range rival cannot be hit through an obsolete decision. Incoming crash/damage/terminal events survive AI command updates. Crashed opponents emit neutral driving commands and use the same linked body/get-up/walk/remount system as the player; after recovery they wait for safe clearance before re-entry.

## Diagnostics and recorded controls

`set_rng_tracing(true)` records tick, actor, category and value for population draws and weapon theft without adding draws or changing canonical state. Spawn-stream initialization remains separate from the gameplay stream. The private acceptance driver exports decision/deadline and motion observations every 300 ticks plus an ordered RNG log. `tools/compare_population_traces.py` reports the first unequal scheduler, RNG and motion observation independently. A motion mismatch is an arithmetic candidate, not proof of its root cause. Negative tests alter each channel separately.

```bash
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course CITY --record-replay local/my-race.json
cargo run --locked --release -p rr-simulation --bin rr-replay -- local/my-race.json irregular local/my-race-report.json
cargo run --locked --release -p rr-simulation --bin rr-population-check -- local/cdrom/ROADRASH local/population-recorded local/my-race.json
python3 tools/compare_population_traces.py local/population-acceptance local/population-recorded local/trace-difference.json
```

Recording starts at tick zero, records per-tick player commands, saves on application exit with a final expected hash, rejects existing/non-JSON/immutable-source destinations and respects the replay's 4 MiB limit. The benchmark can replay the complete captured setup and extend it to 180 seconds with the final held controls. Captured sessions and imported configuration stay private. The unattended live-window recording was replayed to its recorded final hash; a human driving calibration/playthrough is not claimed. Authored controls are explicitly labeled separately.

Four 180-second seeded source runs agree at every canonical checkpoint across 30/60/144/irregular schedules; another four runs use the captured setup/control stream. These are virtual presentation schedules, not hardware FPS measurements. [Part 13 evidence](evidence/part-13.md), [public authored replay](fixtures/population-replay.json), [road users](ROAD_USERS.md).

## Native functional candidate update

The opt-in functional factory fixes unused trailing zero gear slots by taking the greatest positive upshift threshold. It uses bounded deterministic integer steering prediction (nine candidates, eighteen handling ticks) and five passing corridors within road margins. No extra gameplay RNG is consumed. A 3,500-speed driver now genuinely finishes fifteenth instead of qualifying against capped 1,000-speed rivals. The final competitive career completes all 25 course/level combinations in 67 ordinary-control attempts, independently replayed 268 times. Earlier noncompetitive careers are rejected evidence. [Career receipt](evidence/native-career.json), [functional policy](NATIVE_ACCEPTANCE.md).
