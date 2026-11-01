# Classic Rust/Bevy architecture

3D is deferred. The workspace uses pinned Rust 1.99.1 and Bevy 0.20.1, ordinary safe Rust core libraries and narrow presentation adapters.

```text
rr-import -> verified immutable install generations
rr-formats -> checked resource/PE/palette/sprite/CEL/family/RRI records
rr-content -> versioned semantic catalog + content-addressed objects + inspector
rr-course -> validated source-unit graph + route cursors + placements + explorer
rr-simulation -> commands -> ordered integer tick -> canonical state + snapshots
rr-render -> immutable course/snapshot views -> clipped textured RGBA frame
rr-presentation -> frame image + viewport policy -> Bevy 2D GPU presentation
rr-game -> CLI/configuration, input, fixed-tick/replay adapter, composition
```

Core simulation never loads files or owns visual resources. Format and content crates have no Bevy dependency. The composition root validates catalog references and objects before launch, then injects visual selections. Renderer interpolation and native/override image choices cannot alter simulation hashes. Input, simulation and snapshot system sets are explicitly ordered; presentation prepares interpolation before drawing.

The source importer preserves atomic activation, bounded paths, provenance and immutable originals. Asset conversion records dependencies and recipes, so decoder or palette changes invalidate affected previews. Unknown data remains raw with confidence labels. Versioned catalogs, replay records and typed IDs provide modifiable boundaries without translating binary addresses into gameplay objects.

See [formats](FORMATS.md), [content](CONTENT.md), [simulation](SIMULATION.md) and [import](IMPORT.md) for contracts and tests. Persistence, complete gameplay, original handling, UI, media and enhancement review remain later roadmap phases. Fixed ticks and modern dependencies are infrastructure, not evidence of full original behavior. The separate parent-directory prototype is preserved.

The [course model](COURSES.md) keeps raw source units and backward endpoints separate from the simulation's authored millimetre scenario. Its first tested consumer is the offline explorer. Phase 08 now supplies an explicit visual bridge with authored conversion constants; source Q8 samples are never silently reinterpreted as physical metres. Course truth remains a graph with branch-local coordinates; sampled routes are immutable read caches.

The [classic renderer](RENDERING.md) resolves source/catalog resources at startup, then draws from immutable Scene and snapshot values. Its source projection, software rasterizer and image-diff suite run without Bevy; the Bevy adapter uploads the same pixels. No rendering branch consumes gameplay RNG or writes simulation state. Phase 09 must recover handling/controller units before replacing the authored visual bridge with the real race coordinate contract.

`rr-input` is a pure action/configuration layer. The game owns the Bevy physical-key adapter and sends the same domain commands as replay; input has no renderer dependency and cannot mutate simulation internals. Race/menu/paused contexts are separate maps. Source RRAN geometry supplies sprite size/hotspots independently of bitmap/enhancement resolution.
