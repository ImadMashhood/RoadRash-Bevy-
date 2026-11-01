# Course reconstruction contract

`rr-course` is the bounded source-unit course model and offline explorer. It consumes `rr-formats` resource/PE readers and verified `rr-import` generations; it has no Bevy dependency. It does not replace the existing authored driving scenario yet. Phase 08 supplies the classic road renderer and its integration adapter. Source positions use Q8 samples (256 units per sample), not guessed metres; source lane/shoulder/elevation units remain explicit.

## Source records and graph validation

Facts were inspected at [reference revision 7aecf60](https://github.com/arkits/road-rash/tree/7aecf6044f2dd773b925b325823f30d648d61c89/src), especially course.cpp/course.hpp, road.cpp, terrain.cpp scenery.cpp and race_pedestrians.cpp. The Rust implementation was written from format/behavior facts; no upstream C++ implementation was copied. The supported binary/corpus hashes are the existing importer contract. Original executable runtime comparison remains waived.

A course container supplies resource `SGS ` ID 1 (`SGSR`) and `NOD ` ID 1 (`DONR`). Section rows are 52 bytes and retain all 13 original words. They link HTPR path bytes, NALR lane intervals, DLSR side intervals, terrain, CESR events, MSRR family requests, BORR roadside profiles and DZHR scenery records. Unknown words, flags and terrain bytes remain preserved with source file offsets.

Nodes retain source-relative and file offsets, kind, forward/backward/link fields, length, primary byte and original words. Kinds are road, split, merge, backward terminal and forward terminal. All referenced records must be in bounds, aligned, nonoverlapping and counted by the header. Road nodes require valid section IDs and positive lengths within their source samples. Forward travel must reach an end without cycles. Backward references are intentional and are not treated as forward cycles. Validation reports offending node/file offsets; corrupt graphs cannot silently become flat paths.

Limits include a 16 MiB source file, 4,096 sections/nodes, one million aggregate source/route/alternate samples, 4,096 scenery records for pairwise ordering, eight split bits for offline enumeration and two million aggregate exported route samples. Source/output overlap and parent traversal are rejected before creating output under immutable source data. Bounded helpers validate offset, extent, count and allocation products before access. Public authored tests exercise malformed directories, node aliases, missing roots, dangling references, forward cycles, oversized counts, all container truncations and 3,000 deterministic nested-table mutations without panics.

## Traversal and branch identity

The authoritative representation is the graph plus section-local records. Enumerating a route creates a read-only cache of `(node, section, local, branch)` points, not a replacement flat polyline. Branch bits map to sorted split-node offsets. Bit 0 selects the node's first outgoing field; bit 1 selects its second. Merge records retain arrival half, opposing branch and outgoing link. Branch identity resets after a merge. All 26 supported combinations are enumerated: City 2, Sierra Nevada 4, Peninsula 8, Napa 8 and Pacific Highway 4.

Source lookahead exposes both halves after a split and in the 81-sample merge window. Lane-edge shifts use Q8 widths converted by `trunc(q8*250/256)`. Bend difference integrates first into separation delta, then into separation. The merge window reconstructs those values backward so separation closes at the merge. Dual samples retain the other node/section/local record, side, center and separation; they do not compare unrelated branch-local coordinates as if they were the same global Z.

A cursor stores bounded signed Q8 progress on a selected route, whose choices preserve reverse-travel history. Forward sample intervals are half-open; the terminal endpoint is valid but cannot be indexed as another sample. `endpoint` exposes the direct terminal backward initializer; `section_endpoint` exposes direct source endpoint values at internal node ends. These are separate from cached forward samples and from continuous inspection geometry. Advance/rewind validates the full new position before mutation and rejects overflow or stepping outside the sampled path. Tests exhaustively traverse every fractional position of the authored route in both branch choices; private inspection checks every whole sample forward/reverse and both fractional sides of every seam.

## Geometry and interval semantics

HTPR bend and rise are signed bytes. Rise changes elevation; bend changes curvature, rather than being an absolute lateral coordinate. Source section elevation begins at the signed header short and adds each passed rise. Backward endpoint height is the other header short; the two following source bytes are retained as the direct endpoint bend/rise when present. They are not fabricated by indexing beyond a vector.

Source headers reset between sections. `Sample.elevation` preserves this static per-section value. `RoutePoint.route_elevation` separately integrates rise continuously along the selected route for inspection. `Cursor.geometry` blends center, lane width, shoulders and side heights over fractional Q8 progress, and uses the continuous rise integral. This is an explicit inspection presentation policy, not a claim of recovered original physics or final terrain rendering. Raw resets and side-height steps remain visible in seam reports. The plan-view map uses a schematic heading scale of bend/512 and is not a measured world map.

Lane tables preserve byte endpoints and signed Q16 slopes; their Q16 values become Q8 lane units before world conversion. Missing lane tables default to two lanes per side (Q8 512, width 500). Side tables preserve shoulder width, height and uninterpreted side flags; their values are read back in source world units. The initial sample uses record zero's start bytes without a slope step. While `j < end`, one step adds slopes; `j == end` loads exact end bytes. Past an interval, the next record is entered only when `j == next.start`, carrying the existing value on that entering tick. Gaps retain values. Adjacent/equal boundary quirks are explicitly tested, rather than replaced by generic linear interpolation.

Terrain tags DLBR (building), NTMR/DWDR (wall) and LIHR (hill) become typed records with raw payloads. Wall parameter arrays and signed base heights keep physical order, even when boundaries are out of order. Hill records keep signed endpoints, including City's intentional `0..-6` first interval, and signed point heights. Selector arrays and direct endpoint selector bytes are retained. Random hill/wall geometry and full terrain rendering are Phase 08/gameplay integration, not implemented meshes in this phase.

## Events, finish and scenery

CESR intervals retain source order, signed start/end, raw type and three parameter bytes. Active runtime type is raw+1, with the first sample's special initial no-type behavior preserved. Inclusive event ends, zone-end markers, tunnel/intersection marking flags and parameter bytes have explicit boundary tests. The finish is the first raw-type-6 event start carrying the requested level tag (1–5), recovered separately for each route. The offline player grid start comes from supported EXE address 0x464218, seat zero: Q8 `0x3200` (12,800). Level is not mistakenly used as a network seat index.

MSRR load/unload requests preserve record order, duplicate positions, type, slot, family, raw words and forward bias. The inspector reconstructs resident selectors with a 33-sample lookahead plus that bias; requests of kinds 1/3 replace a slot, while kinds 2/4 retain residency until replacement. Family IDs are checked against FAMILIES.RSC and unresolved selections are counted.

BORR profiles decode spacing, repetition, side alternation, size, mirror and selector. DZHR records decode cumulative position/jitter, level/probability gate, signed 12-bit displacement, billboard/actor kind and selector. The source pairwise exchange ordering is preserved for equal positions; a stable sort would change it. Instance IDs namespace the course fingerprint, section, source record and ordinal. Identical/repeated source objects retain separate instances. `Route.instances` resolves fixed world positions once; camera movement never consumes placement RNG or regenerates identities.

Offline placement uses an isolated MSVC-style seed-1 stream. It does not claim full race terrain/scenery/random-call interleaving. Some source selectors resolve to empty family slot 0: five roadside instances across Pacific Highway's levels in this run. They remain in the data and coverage report; no artwork is silently invented. Actor requests keep raw words and level-gated source samples, with lateral position unset until gameplay spawning; police requests have no scenery-family binding. Billboard/family animation rendering and actor spawning are later phases.

## Build and inspect

```sh
cargo run --locked --release -p rr-course -- inspect /absolute/private-install-root /absolute/private-course-output
python3 -m http.server 8790 --bind 127.0.0.1 --directory /absolute/private-course-output
```

The input is the importer root containing `active.json`, not the extracted ROADRASH directory. The CLI validates the active import, generates five private course JSON files, coverage.json and index.html, and fails on source errors or pinned branch-census disagreement. Open http://127.0.0.1:8790/index.html. Select course, level and branch; jump to finish or step the sample cursor. The explorer shows source metadata, schematic route/elevation plots and fixed-camera before/at/after seam cross sections with source-local scenery dots and unknowns.

Run `cargo test --locked -p rr-course` for the 20 public authored tests. The CLI checks every one of the 130 branch/level finishes against [pinned numerical expectations](fixtures/course-expectations.json), every whole sample in both directions, all 132 seam transitions, closed merge separation, retained road edges and continuous inspection geometry. Two independent output roots produce byte-identical files. No private course payload or generated screenshot is included in the repository. See [acceptance receipt](evidence/part-07.md).

## Part 08 integration update

The [classic renderer](RENDERING.md) now consumes this graph, dual windows, terrain profiles, static placements and source materials in Bevy. The earlier statements assigning those capabilities to Phase 08 describe that dependency and are now fulfilled for the documented renderer contract. Handling/race units and actor behavior remain later gameplay work; source records are unchanged.
