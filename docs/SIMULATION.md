# Deterministic simulation contract

`rr-simulation` contains no Bevy, filesystem access in tick execution, GPU types, camera coordinates or texture IDs. Tick, ActorId, RouteId, BranchId, SectionId, Millimetres and SpeedMmPerTick are domain types. The data-free legacy scenario uses authored 60 Hz constants; imported races use the [schema-3 source-unit handling model](HANDLING.md).

Actors live in stable-ID ordered storage. IDs increase monotonically and are not reused after despawn. Route position stores route, branch, section and section-local progress alongside lateral position. Route graphs validate root, reachable sections, positive lengths, dangling edges, cycles and depth; fixtures exercise both sides of a split and a merge. This is the schema for later course interpretation, not imported original courses.

Commands target ticks and actors, ordered by tick/actor/sequence. Duplicate sequence, past tick, invalid controls and unknown actor fail without mutating authoritative state. Held drive controls persist until changed; schema-3 pose/equipment/damage events are consumed once. A frame can consume zero, one or multiple fixed ticks. The Bevy composition root orders input, simulation and snapshot publication explicitly. The domain stage order is Input, Spawn, Decisions, Movement, Contacts, Combat, Progression, Snapshot; Spawn/Decisions reserve extension points for later opponent behavior. Schema 4 implements contact, combat and recovery progression; complete opponent AI and career remain later work. Registration-order tests reverse/rotate stage registration and compare deterministic outcomes.

Presentation snapshots are owned values containing actor IDs and domain positions, never mutable references into the simulation. Presentation keeps previous/current snapshots and interpolation alpha; interpolation writes sprites only. Tests mutate copied snapshots without changing the world and exercise actual presentation-system positions at 30/144 schedules, checking bounded interpolation, smaller high-rate steps and identical final state. A disjoint-query test guards against the Bevy mutable Sprite query conflict found during graphical validation.

Spawn and gameplay RNG streams use explicit MSVC-style LCG arithmetic, recorded seeds and deterministic consumers. Spawn allocation draws only from the spawn stream; gameplay random draws are explicit domain calls. Presentation randomness stays outside both streams and outside canonical hashes. Known-answer seed-1 values are 41, 18467, 6334, 26500, 19169. Future gameplay consumers must document their draw order rather than relying on thread scheduling.

Arithmetic helpers declare wrapping add/multiply, Q16 multiplication, arithmetic floor shifts versus truncation toward zero, checked signed division and invalid shift/division behavior. Boundary tests cover negative values, overflow, minimum integer, division by zero and out-of-range shifts. These primitives support later source-faithful rules; the current handling model remains authored.

Canonical state encoding uses fixed-width little-endian fields, stable actor/route ordering, RNG and pending commands. It excludes addresses, padding, render resources and unordered iteration. SHA-256 checkpoints are recorded every 30 ticks and at the final tick. Schema-1 and schema-2 JSON replays reject unknown fields, unsupported versions, excessive duration/commands and invalid input; files are bounded to 4 MiB. An optional expected hash turns mismatches into a failed CLI result.

```sh
cargo run --locked --release -p rr-simulation --bin rr-replay -- docs/fixtures/replay.json irregular local/replay.json
cargo run --locked --release -p rr-game -- --catalog local/catalog --replay docs/fixtures/replay.json --render-hz 144 --report local/rendered.json
```

30, 60, 144 and irregular are controlled virtual presentation schedules. The graphical app renders those snapshots at available display cadence; these runs are not hardware FPS benchmarks. Headless and actual GPU runs agree at ticks 30/60/90/120. Native/override selection and a live tick-60 texture switch preserve every checkpoint. The checked-in fixture is a 120-tick architecture slice, not a finished race. See [matrix](evidence/replay-matrix.json).

## Schema 2 proportional steering and legacy replay compatibility

The data-free fallback uses schema 2; imported live races now use schema 5. `Command.steer_q15` is optional signed Q15 steering in -32767..32767. The legacy `steer` field still accepts only -1/0/+1; combining nonzero digital steering with an analog value is rejected. The symmetric range excludes -32768. Keyboard and full-stick inputs normalize to the same schema-2 command, independent of physical device identity. The current authored lateral model scales the existing full-lock displacement by Q15 using integer division truncating toward zero; this does not claim original physics calibration.

`Simulation::with_schema` selects an explicit supported encoding. Schema-1 replays run in the unchanged schema-1 mode and reject analog fields, including explicit zero analog values. Their four-byte command encoding remains throttle byte, brake byte, digital steer byte and fork byte (255 means absent). Schema-2 canonical command records append a presence byte and little-endian i16 axis payload (zero when absent). The canonical header records the selected schema; queued commands and held actor commands both include the axis data. Report versions reflect the simulation being played, rather than the current library's newest version. Unsupported schemas and invalid commands fail without mutating authoritative state.

The existing schema-1 fixture retains final SHA-256 `f70c202279f360fc89e95d503443a91a523c816788d9e62102a6d3572fc7f143`. The authored [schema-2 analog fixture](fixtures/replay-analog.json) expects `87faf52e185aa21028d6be58fb01403d1f94558d6960aea7eadbedc2544f675e`, distance 51,150 mm, speed 300 mm/tick and lateral offset 7 mm at tick 120. Each fixture preserves all checkpoints across 30/60/144/irregular schedules. Full-scale analog steering matches digital full-lock movement, partial steering is proportional and left/right traces are symmetric. These are authored engine regression scenarios, not original-game measured handling traces.

```sh
cargo run --locked --release -p rr-simulation --bin rr-replay -- docs/fixtures/replay-analog.json irregular local/analog-replay.json
```

[Device/replay acceptance](evidence/09.02.md) records the native backend, command transport and graphical/headless checks. Physical input polling and pause sampling are bypassed during recorded playback. Device arbitration/keyboard codes/controller IDs stay outside authoritative state and RNG streams.

## Schema 3 source handling and animation

`Simulation::with_handling` validates named bike specs/tables, ground profiles and animation banks before creating a source-unit race. `Replay.handling` is required for schema 3 and prohibited for older schemas. Source-derived replays contain private imported configuration; keep them local. Pose/equipment/damage commands are versioned additions. Pose/equipment/damage events are consumed once; held drive controls persist. Canonical state includes length-prefixed deterministic integer configuration and all hidden handling/animation fields, alongside existing ordered IDs and queue fields. Camera/pixel assets remain excluded. [Detailed handling contract](HANDLING.md), [animation contract](ANIMATION.md), [acceptance](evidence/part-09.md).

## Schema 4 encounters and recovery

`with_encounters` requires validated handling plus an `encounter::Setup`. Replay requires `handling` for schemas 3/4, `encounter` only for 4 and rejects mixed factories. Commands add optional one-shot attack, walking, impact and terminal fields; absent fields preserve schema-3 JSON canonical bytes. The entire ordered World, including setup, inventory, active contact pairs, swings, linked body animation/motion, assist settings and terminal completion ticks, is included in schema-4 canonical encoding. Effects and copied drawing proxies are excluded. One-shot commands clear at Snapshot; drive controls persist. Public fixture [encounter-replay.json](fixtures/encounter-replay.json) is independently authored and bounded to the usual 4 MiB replay limit.

[COMBAT.md](COMBAT.md) specifies target/tick ordering, damage and theft. [RECOVERY.md](RECOVERY.md) specifies body identity, graph navigation, remount thresholds, terminal priority, default timings and optional assists. The source setup uses the selected level's actual finish location; detached rider progress can trigger it. Police detection remains outside this phase. The source-derived showcase runs to tick 1800 with identical checkpoints at 30/60/144/irregular and effects on/off; current receipts are under [Part 11](evidence/part-11.md) and [Part 12](evidence/part-12.md).

## Schema 5 population

`Simulation::with_population` requires handling, encounter and population setup. `Replay.population` is required only for schema 5; schemas 4/5 require encounter setup. The existing schemas retain their constructors and byte encodings. The population world contains ordered brains, road users, source masks/zones, consumed request IDs, controller timers, contacts and hidden phase/deadline state. All are canonical. Per-actor config caches are derivable from the canonical specification assignments; diagnostics/cues/siren voices and graphics are excluded.

Spawn and Decisions now implement tagged population scheduling before physical Movement. Traffic contacts use the same impact transaction as combat/recovery; police eligibility runs before the existing terminal-priority transaction. Presentation uses optional selected-route projections across section boundaries, with interpolation on copies. `recording_setup(seed)` is allowed only at tick zero and captures the complete setup; the app records commands and final hash. [Public schema-5 fixture](fixtures/population-replay.json), [opponent decisions](OPPONENTS.md), [road users](ROAD_USERS.md), [acceptance](evidence/part-13.md). Source-backed replays remain private.


Part 15 adds an engine-independent `rr-session` wrapper rather than changing simulation schemas 1–5. Countdown/pause gate all simulation advances, including RNG. Session standings normalize alternate-path progress between selected-route anchors, and integer finish-tick/ActorId order seals immutable results. Session playback checks the final checkpoint even for a two-tick race. Career transactions consume validated result data exactly once; they do not modify live actors. See [RACE_RULES.md](RACE_RULES.md).
