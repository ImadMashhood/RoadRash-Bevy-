# ADR 0001 — evidence-driven 3D foundation

Status: accepted implementation direction; behavior parity unverified.

Context: GitHub initially contains a roadmap, while a separate local Bevy 3D experiment contains substantial coupled research code. The user asks to begin sequential implementation toward Phase 23 and a future-capable 3D game.

Decision: establish an independently buildable workspace with bounded format readers separate from Bevy. Use Rust 1.99.1 / Bevy 0.20.1, matching the inspected local baseline; lock all dependencies. Start with Linux x86_64/X11. Commit authored geometry only. Keep stable original content IDs and route topology independent of graphics. Retain source integer/fixed-point semantics once measured; choose no general-purpose physics engine at foundation stage.

Alternatives: importing the entire experimental app would inherit unvalidated coupling and private assets; writing an abstract general engine would delay the measured playable slice. Select small modules with clear consumers and evaluate prototype modules individually later.

Consequences: initial scene is intentionally simple. A classic comparison adapter remains possible. Current 3D mesh production and camera work need their own measured acceptance. Dependency updates occur in separate reviewed changes. Audio, gamepad, UI and additional platform features are enabled when first required and checked.
