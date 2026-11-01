# ADR 0002 — classic presentation and independent simulation

Status: accepted; supersedes ADR 0001's 3D presentation direction.

The user explicitly deferred 3D. The current app uses Bevy's 2D renderer only. Replace the prior 3D slab/cuboid/light scene with a classic-style, entirely authored road/actor scaffold.

Extract simulation commands, integer state and snapshots into rr-simulation without Bevy dependencies. Extract cameras/sprites into rr-presentation. Keep app-level adapters in rr-game. This creates independently testable behavior while retaining Bevy's rendering/input lifecycle. Existing format decoding remains independent.

The scaffold is deliberately not labeled original handling: constants require later recovery. Original capture gaps block fidelity certification, but do not block importer work, format parsing, synthetic tests or architecture implementation. Future 3D planning creates no current dependency.
