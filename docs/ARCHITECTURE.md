# Classic game architecture

Current goal: recreate the original game with modern Rust/Bevy engineering. 3D is deferred.

## Implemented boundaries

```text
rr-game (composition root, Bevy input and fixed-tick adapter)
  -> rr-simulation (commands, state transitions, immutable snapshots)
  -> rr-presentation (Bevy 2D plugin consuming snapshots)
       -> rr-simulation (snapshot schema only)
rr-formats (bounded source-byte readers; independent of Bevy)
```

The app samples held controls at fixed ticks, advances the simulation once per tick, then publishes a snapshot. Presentation reads that snapshot and owns cameras, sprites and visual updates. Snapshot access cannot mutate gameplay state. Keyboard sampling is currently a live input adapter; tick-indexed recording/replay, pause/focus handling and gamepad controls remain to be implemented.

The simulation's 60 Hz rate, acceleration, braking and road width are authored scenario constants. They establish a tested boundary, not original-game parity. Recover original units, tick timing, arithmetic and actor ordering before substituting real gameplay. Fixed ticks alone do not guarantee cross-platform determinism: future AI, RNG and collision resolution need explicit ordering and numerical semantics.

The format crate preserves palette indices. Palette/remap application belongs to content conversion/presentation. Validated source bytes must become typed content records before entering simulation. The Python disc tool is a read-only research verifier, not the production importer.

## Next boundaries when needed

- Content: stable asset IDs, palettes, frame pivots, source hashes and texture override resolution.
- Import: selected local data, full manifest validation, staged installation, atomic activation and typed errors.
- Persistence: versioned career/settings records, atomic writes and migrations.
- Race: explicit ordered movement, collision, combat, AI, progression and snapshot stages.

Do not introduce empty crates for these boundaries before a working consumer exists. Keep game rules out of rendering, screen coordinates out of collision, filesystem operations out of tick systems, and source binary offsets out of domain types.

## Technology decisions

Use the pinned Rust/Bevy baseline and lockfile. Bevy handles the application lifecycle, input, scheduling and 2D GPU rendering. Core simulation and decoders remain ordinary Rust and run in headless tests. Use authored synthetic fixtures in public CI and private-data corpus checks locally. Enable audio/UI/gamepad features when their first tested feature arrives. Upgrades require compiling and checking the affected interfaces; adding dependencies does not itself modernize gameplay architecture.
