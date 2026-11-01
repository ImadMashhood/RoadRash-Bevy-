# Deterministic simulation contract

`rr-simulation` contains no Bevy, filesystem access in tick execution, GPU types, camera coordinates or texture IDs. Tick, ActorId, RouteId, BranchId, SectionId, Millimetres and SpeedMmPerTick are domain types. The current scenario uses authored 60 Hz handling constants; these are not a claim to recovered original physics.

Actors live in stable-ID ordered storage. IDs increase monotonically and are not reused after despawn. Route position stores route, branch, section and section-local progress alongside lateral position. Route graphs validate root, reachable sections, positive lengths, dangling edges, cycles and depth; fixtures exercise both sides of a split and a merge. This is the schema for later course interpretation, not imported original courses.

Commands target ticks and actors, ordered by tick/actor/sequence. Duplicate sequence, past tick, invalid controls and unknown actor fail without mutating authoritative state. Held controls persist until changed. A frame can consume zero, one or multiple fixed ticks. The Bevy composition root orders input, simulation and snapshot publication explicitly. The domain stage order is Input, Spawn, Decisions, Movement, Contacts, Combat, Progression, Snapshot; Spawn/Decisions/Combat/Progression reserve extension points for later gameplay. No complete opponent AI, combat or career is implemented by declaring these slots. Registration-order tests reverse/rotate stage registration and compare deterministic outcomes.

Presentation snapshots are owned values containing actor IDs and domain positions, never mutable references into the simulation. Presentation keeps previous/current snapshots and interpolation alpha; interpolation writes sprites only. Tests mutate copied snapshots without changing the world and exercise actual presentation-system positions at 30/144 schedules, checking bounded interpolation, smaller high-rate steps and identical final state. A disjoint-query test guards against the Bevy mutable Sprite query conflict found during graphical validation.

Spawn and gameplay RNG streams use explicit MSVC-style LCG arithmetic, recorded seeds and deterministic consumers. Spawn allocation draws only from the spawn stream; gameplay random draws are explicit domain calls. Presentation randomness stays outside both streams and outside canonical hashes. Known-answer seed-1 values are 41, 18467, 6334, 26500, 19169. Future gameplay consumers must document their draw order rather than relying on thread scheduling.

Arithmetic helpers declare wrapping add/multiply, Q16 multiplication, arithmetic floor shifts versus truncation toward zero, checked signed division and invalid shift/division behavior. Boundary tests cover negative values, overflow, minimum integer, division by zero and out-of-range shifts. These primitives support later source-faithful rules; the current handling model remains authored.

Canonical state encoding uses fixed-width little-endian fields, stable actor/route ordering, RNG and pending commands. It excludes addresses, padding, render resources and unordered iteration. SHA-256 checkpoints are recorded every 30 ticks and at the final tick. Schema-1 JSON replays reject unknown fields, unsupported versions, excessive duration/commands and invalid input; files are bounded to 4 MiB. An optional expected hash turns mismatches into a failed CLI result.

```sh
cargo run --locked --release -p rr-simulation --bin rr-replay -- docs/fixtures/replay.json irregular local/replay.json
cargo run --locked --release -p rr-game -- --catalog local/catalog --replay docs/fixtures/replay.json --render-hz 144 --report local/rendered.json
```

30, 60, 144 and irregular are controlled virtual presentation schedules. The graphical app renders those snapshots at available display cadence; these runs are not hardware FPS benchmarks. Headless and actual GPU runs agree at ticks 30/60/90/120. Native/override selection and a live tick-60 texture switch preserve every checkpoint. The checked-in fixture is a 120-tick architecture slice, not a finished race. See [matrix](evidence/replay-matrix.json).
