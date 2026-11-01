# Settings and fidelity contract

Presentation-only: texture selection, nearest/filtered sampling, output resolution, UI scale, volume, camera shake reduction and decorative effects. They must never enter simulation commands, RNG decisions, collision geometry or canonical state hashing.

Simulation-affecting: assists, modified handling, AI difficulty changes, damage multipliers and content/rules mods. Record them in replay configuration and reject mismatched comparisons. Presentation and simulation settings must use separate typed structures when implemented.

Classic mode selects original indexed art and original camera/layout policy. Enhanced mode substitutes only explicitly approved images through the same logical IDs and pivots. A missing enhanced image falls back to original. Neither mode is currently implemented as a complete texture pipeline; live texture-switch/replay-hash acceptance remains open (01.10.b). The scaffold's red rectangle is authored placeholder art.
