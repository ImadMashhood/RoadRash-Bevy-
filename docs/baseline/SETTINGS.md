# Settings and fidelity contract

Presentation-only: texture selection, nearest/filtered sampling, output resolution, UI scale, volume, camera shake reduction and decorative effects. They must never enter simulation commands, RNG decisions, collision geometry or canonical state hashing.

Simulation-affecting: assists, modified handling, AI difficulty changes, damage multipliers and content/rules mods. Record them in replay configuration and reject mismatched comparisons. Presentation and simulation settings must use separate typed structures when implemented.

Classic mode selects original indexed art and original camera/layout policy. Enhanced mode substitutes only explicitly approved images through the same logical IDs and pivots. A missing enhanced image falls back to original. The catalog-backed sprite path supports native and supplied override images, with a live switch that preserves replay checkpoints (01.10.b). Full approved enhancement-pack review remains Phase 19. Without a catalog, the scaffold uses authored placeholder art.

## Current engine update

The owner removed original-runtime comparison gates. Native/override image selection and tick-60 live switching are now implemented and checkpoint-tested; earlier statements that these were unimplemented are historical. 640×480 logical resolution, 4:3 aspect, nearest native sampling and authored keyboard controls remain the active policy. See [current acceptance](../evidence/parts-04-06.md). This is not an approved AI enhancement pack.
