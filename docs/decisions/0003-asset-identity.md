# ADR 0003 — stable asset identity

Status: accepted contract; catalog implementation follows in Phase 05.

Context: original palette-indexed frames, upscaled derivatives and repacked atlases must identify the same logical art without changing save or simulation references.

Decision: use a namespaced logical ID based on source family and original resource/frame identity. Preserve original hashes, dimensions, trim/pivot, palette/remap identity and conversion recipe outside packed pixels. Storage deduplication does not merge semantic IDs. Import receipts distinguish source hash, manifest hash, importer version and recipe version.

Alternatives: filesystem names alone change during packing; output hashes change with enhancement; Bevy handles are transient and unsuitable for saves.

Consequences: catalog build validates missing references and collisions. Presentation chooses a visual variant; simulation references stable logical content. Texture swaps cannot silently change physics dimensions or attack reach. See ARCHITECTURE.md for the renderer boundary and ADR 0002 for integer simulation placeholders; original arithmetic is adopted only after measurement.
