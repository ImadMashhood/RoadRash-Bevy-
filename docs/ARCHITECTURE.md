# Classic Rust/Bevy architecture

3D is deferred. The workspace uses pinned Rust 1.99.1 and Bevy 0.20.1, ordinary safe Rust core libraries and narrow presentation adapters.

```text
rr-import -> verified immutable install generations
rr-formats -> checked resource/PE/palette/sprite/CEL/family/RRI records
rr-content -> versioned semantic catalog + content-addressed objects + inspector
rr-simulation -> commands -> ordered integer tick -> canonical state + snapshots
rr-presentation -> snapshots + catalog-selected visuals -> Bevy 2D sprites
rr-game -> CLI/configuration, input, fixed-tick/replay adapter, composition
```

Core simulation never loads files or owns visual resources. Format and content crates have no Bevy dependency. The composition root validates catalog references and objects before launch, then injects visual selections. Renderer interpolation and native/override image choices cannot alter simulation hashes. Input, simulation and snapshot system sets are explicitly ordered; presentation prepares interpolation before drawing.

The source importer preserves atomic activation, bounded paths, provenance and immutable originals. Asset conversion records dependencies and recipes, so decoder or palette changes invalidate affected previews. Unknown data remains raw with confidence labels. Versioned catalogs, replay records and typed IDs provide modifiable boundaries without translating binary addresses into gameplay objects.

See [formats](FORMATS.md), [content](CONTENT.md), [simulation](SIMULATION.md) and [import](IMPORT.md) for contracts and tests. Persistence, complete gameplay, source course reconstruction, UI, media and enhancement review remain later roadmap phases. Fixed ticks and modern dependencies are infrastructure, not evidence of full original behavior. The separate parent-directory prototype is preserved.
