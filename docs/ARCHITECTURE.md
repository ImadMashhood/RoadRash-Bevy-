# Classic Rust/Bevy architecture

3D is deferred. The workspace uses pinned Rust 1.99.1 and Bevy 0.20.1, ordinary safe Rust core libraries and narrow presentation adapters.

```text
rr-import -> verified immutable install generations
rr-formats -> checked resource/PE/palette/sprite/CEL/family/RRI records
rr-content -> versioned semantic catalog + content-addressed objects + inspector
rr-course -> validated source-unit graph + route cursors + placements + explorer
rr-simulation -> commands -> ordered integer tick -> canonical state + snapshots
rr-session -> countdown/standings -> immutable result -> career transaction + typed frontend
rr-render -> immutable course/snapshot views -> clipped textured RGBA frame
rr-presentation -> frame image + viewport policy -> Bevy 2D GPU presentation
rr-game -> CLI/configuration, input, fixed-tick/replay adapter, composition
```

Tick execution never loads files or owns visual resources; startup-only decoders construct validated typed configuration before simulation begins. Format and content crates have no Bevy dependency. The composition root validates catalog references and objects before launch, then injects visual selections. Renderer interpolation and native/override image choices cannot alter simulation hashes. Input, simulation and snapshot system sets are explicitly ordered; presentation prepares interpolation before drawing.

The source importer preserves atomic activation, bounded paths, provenance and immutable originals. Asset conversion records dependencies and recipes, so decoder or palette changes invalidate affected previews. Unknown data remains raw with confidence labels. Versioned catalogs, replay records and typed IDs provide modifiable boundaries without translating binary addresses into gameplay objects.

See [formats](FORMATS.md), [content](CONTENT.md), [simulation](SIMULATION.md) and [import](IMPORT.md) for contracts and tests. Persistence, full race/results UI, media and enhancement review remain later roadmap phases. Fixed ticks and modern dependencies are infrastructure, not evidence of full original behavior. The separate parent-directory prototype is preserved.

The [course model](COURSES.md) keeps raw source units and backward endpoints separate from the simulation's authored millimetre scenario. Its first tested consumer is the offline explorer. Phase 08 now supplies an explicit visual bridge with authored conversion constants; source Q8 samples are never silently reinterpreted as physical metres. Course truth remains a graph with branch-local coordinates; sampled routes are immutable read caches.

The [classic renderer](RENDERING.md) resolves source/catalog resources at startup, then draws from immutable Scene and snapshot values. Its source projection, software rasterizer and image-diff suite run without Bevy; the Bevy adapter uploads the same pixels. No rendering branch consumes gameplay RNG or writes simulation state. Imported races now use schema-5 source Q8 coordinates directly; legacy replays retain the original visual bridge. Named handling and animation modules keep binary offsets confined to startup decoding. [Handling](HANDLING.md) and [animation](ANIMATION.md) document the phase boundaries and measured contracts.

`rr-input` is a pure action/configuration layer. The game owns the Bevy physical-key adapter and sends the same domain commands as replay; input has no renderer dependency and cannot mutate simulation internals. Race/menu/paused contexts are separate maps. Source RRAN geometry supplies sprite size/hotspots independently of bitmap/enhancement resolution.

Gamepad discovery is Bevy/Gilrs at the composition boundary. `rr-input::devices` owns engine-independent quantization and bounded deterministic device arbitration; `LiveInput` carries one normalized drive sample into fixed tick commands. Controller identity never enters simulation hashes. Schema 2 adds proportional steering without altering schema-1 playback encoding or outcomes. Focus/active-disconnect protection runs after physical polling and before fixed simulation; recorded replay bypasses the live physical path.

`rr-input::calibration` owns validated model profiles, response curves and streaming capture statistics. `gamepad_calibration.rs` supplies one shared Bevy transport/read boundary to the game, native preview and diagnostics. The native editor owns unsaved drafts and live-session selection; identical model sessions cannot mix capture data, while persisted profiles use stable model keys. Calibration runs before Q15 commands and never participates in replay execution or simulation hashing. Shared JSON storage is bounded and atomically replaces complete validated settings files. The separate `rr-calibrate` binary uses Bevy text/default fonts, with the original game retained as package `default-run`.

Combat definitions, encounter transactions, recovery entities and topology navigation are separate `rr-simulation` modules. Validated setup holds editable rules, graph portals, actor seeds and initial inventory; runtime state holds unique equipment ownership, swing/stun/immunity, linked bodies and terminal latches. Movement → Contacts → Combat → Progression resolves ascending ActorId order. Renderer proxies and cosmetic result feeds cannot feed back into physics. The composition root preloads palette variants and translates live actions to schema-4 requests. Replay schemas 1–3 retain their old factories/bytes. [Combat](COMBAT.md) and [recovery](RECOVERY.md) detail these boundaries and deliberate behavior policies.

Schema 5 composes a separate population authority with encounter transactions. Source decoding, scalar primitives, opponent decisions and road-user behavior live in separate modules under `crates/simulation/src/population/`. Per-actor bike configurations are startup-derived caches. Transient RNG traces, warning cues, siren ownership and copied/interpolated road-user projections cannot feed back into physics. Typed, serialized setup supports authored/modded traits, zones, actor requests and vehicle masks without embedding binary offsets in update code. The application boundary records per-tick live controls and saves a bounded replay on exit. [Opponent](OPPONENTS.md) and [road-user](ROAD_USERS.md) contracts specify scheduling and known fidelity differences.


`rr-session` has no Bevy/entity/renderer dependency. It consumes simulation snapshots and owns frontend records and immutable results; domain progression never queries a live race entity. The game shell is split into coordinator, menu view, physical menu input, snapshot storage and adapter tests under `apps/game/src/frontend/`. A persistent presentation camera/surface is reused across source scene replacements; framing changes resize the texture and projection together. Race exit drops source scenes/physics ownership, publishes empty snapshots and stops owned cue loops. Derived player-condition specification 16 is captured in existing schema-5 configuration while rivals retain original stock specs 1–15. [Race lifecycle](RACE_RULES.md), [frontend](FRONTEND.md), [saves](SAVES.md) and [audio/video](AUDIO.md) define the domain/adapter contracts and remaining Part 19 coverage.

`rr-media` depends on immutable simulation snapshots/effects, not Bevy. The game owns Rodio/CPAL output and bounded FFmpeg workers. `rr-session::persistence` serializes validated domain records separately from live worlds. Settings save in a Last-stage observer; cue capture follows every fixed snapshot and every replay tick. One shared RGBA upload helper replaces GPU image handles on dimension changes and reuses them for same-size frames. Neither media nor persistence changes canonical replay state.
