# Content catalog and asset review

`rr-content` builds schema-1 catalogs from a verified immutable import. It rechecks every manifest hash and size before conversion. Source and output directories cannot overlap. Source masters, indexed masters, metadata and PNG previews are content-addressed objects; stable semantic IDs are separate from storage identity.

An ID such as `rr:data/bikers/hibob_dat/frame/000` derives from the canonical source identity and frame, never a generated filename or packing position. Reordering the manifest or renaming a generated object does not change that identity. Renaming an original canonical source requires an explicit logical-ID migration: arbitrary source renaming is not automatically inferred. Two semantic frames with identical bytes share storage while allowing independent overrides.

Every entry records source/hash, resource tag and ID or family locator, frame, decoder recipe, indexed object, previews and dependencies. Native rectangle has origin (0,0) and native_size; trim rectangle, logical size and pivot are independent. `pivot_twice` stores half-pixel precision. Bottom-center contact pivots are an authored policy, not recovered animation physics. Tests compare actual trimmed/untrimmed pixel placement at a shared anchor.

Kinds distinguish rider, police, shadow, scenery/course textures, UI, backgrounds, audio, video, bike specs, courses, palettes, executable and text. Subcategories include road, terrain, building, prop, actor, menu/character/bike art, HUD, horizon and cloud. Confidence is SourceDerived, Inferred or Unreviewed. Filename-based scenery labels are explicitly inferred. Unclassified fine semantics are counted in the report and remain Phase 07 research; source-domain coverage is complete, not every fine semantic label. Typed references reject, for example, a UI entry used as a BikeSpec. Resolution applies semantic overrides and reports the pack, referrer and missing/wrong-kind ID before launch.

Catalog schema mismatch produces a schema-specific rejection; there is no silent migration. Reads are bounded to 64 MiB; blob paths, IDs, sizes and trim arithmetic are validated. Recipe identity includes converter version (`rr-content-3`), decoder source/lockfile fingerprint and conversion settings. Legacy missing converter fields load as legacy-unrecorded and invalidate cache. Cache keys include source and palette/remap dependencies. Only changed sources and their dependents rebuild; missing/corrupt generated objects rebuild from immutable masters. Changing the recipe invalidates stale outputs. JSON ordering is stable and timestamps are excluded.

```sh
cargo run --locked --release -p rr-content -- build /absolute/private-install /absolute/private/catalog
cargo run --locked --release -p rr-content -- inspect /absolute/private/catalog
cargo run --locked --release -p rr-content -- find /absolute/private/catalog hibob
python3 -m http.server 8789 --bind 127.0.0.1 --directory /absolute/private/catalog
```

Open http://127.0.0.1:8789/inspector.html. The generated inspector embeds the catalog and reads relative preview objects: search by ID/source/category, select a contact-sheet tile, step/play frames, choose palette/remap, nearest/linear sampling, alpha background and zoom. IDs and provenance remain visible. Pivot crosshairs use logical dimensions; empty frame slots remain selectable. The inspector never writes source files.

For reproducibility, use pinned Rust 1.99.1 and Cargo.lock. The fixture test builds two independent directories, reverses manifest order, compares catalogs/objects and exercises a warm cache. The full private warm build reused 341/341 sources with byte-identical catalog output. See [receipt](evidence/parts-04-06.md). AI upscaling, reviewed enhancement packs and a complete mod distribution contract remain later phases; the current override example is authored test art.
