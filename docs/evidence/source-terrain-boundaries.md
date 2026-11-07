# Source terrain boundaries and ground height

The three supplementary families contain **67,902 matching native records**:

| Family | Records | Valid baseline mismatches | Corrected mismatches |
| --- | ---: | ---: | ---: |
| Boundary response and crash entry | 15,360 | 5,247 | 0 |
| Integer terrain-height queries | 18,360 | 8,064 | 0 |
| Every physical source road sample | 34,182 | 3,229 | 0 |

[Aggregate receipt](source-terrain-boundaries.json) records hashes and private
before/fixed receipts. The separate 67,142-tick mounted driving comparison is
included in [collision/topology evidence](source-collision-topology.md), not
counted twice here. Complete detached recovery remains the next recovery stage.

## Corrections

The old physical boundary adapter implemented building type 0. New factories
also retain native mountain/cliff type 1, tunnel type 3, hill type 4 and open
type 255. A mountain checks wall bases and cliff tops differently for human and
AI riders; a tunnel has separate entry-angle rules. Hills and open terrain do
not acquire invented solid shoulder walls. Type-5 event rails use interpolated
current/next edges, the previous approach side, native height/contact gates and
decoded resource heights when the source camera has drawn the strip.

An airborne mounted rider can enter a reason-3 rail crash. The detached body
copies the steering velocity before the quiet bike's later reflection. Native
reason 3 passes no additional impulses; the adapter supplies explicit zero
vectors instead of substituting generic knock-off defaults. Normal boundary
crashes retain reason 6 and the native rider/bike impulse pair. A detached body
clearing a low rail is ignored without an accidental lateral reposition.

Ground queries now retain native integer interpolation for sidewalks, building
openings, mountain ramps, cliffs, tunnel sidewalks and hill bands. Source ground
height had previously remained zero in the game factory. The zero-ground
baseline uses an absent surface profile, reproducing that factory behavior;
it is not a claim that an earlier implementation of `terrain_height` existed.

Factory grip labels were also wrong: buildings used hill type 4, and hills used
building type 0. New factories use building 0, mountain 1, tunnel 3 and hill 4.
Open type 255 uses the existing nonzero ground-class sentinel 2; the ten native
surface-class samples per physical record verify that classification equivalence.

The native order is movement, boundary response, then vertical physics. The new
source profile defers vertical integration until after the boundary callback,
then samples ground height and current grade while preserving the captured
movement/curve integrals. Legacy wrappers still execute their previous order.
The complete mounted episode comparison verifies this integration through the
first crash, rather than assuming isolated callback agreement proves it.

## Real content and compatibility

The per-sample comparison covers all 34,182 physical road samples across the
five courses, including both fork arms, 244 tunnel samples, decoded rail heights
and ten road/shoulder grip positions per sample. It compares the actual pinned
terrain iterator using the same documented isolated seed-1 terrain stream as
Rust. Native family resolution supplies building selectors and rail metadata.

It found 3,229 real import errors: building groups shorter than 17 native units
must change their physical selector to 255. The renderer already omitted these
groups, but collision retained the original selector. The corrected importer
matches every record, including all 341,820 surface-class checks.

`Boundary.surface` is optional and omitted when absent. Five public regressions
cover cliff tops versus wall bases, AI/human differences, rail clearance and
entry order, adjacent tunnel panels, open shoulders and exact legacy boundary
JSON bytes. Existing legacy recording/hash regressions remain required during
parent integration. No commercial geometry traces or pixels are committed.

An initial whole-`CourseScene` inventory was **invalid as a global comparison**:
that constructor consumes scenery randomness between terrain sections, while
Rust intentionally uses the documented private terrain stream. Its 24,931
mismatches are not engine failure counts. The corrected producer invokes the
actual terrain iterator with matching inputs; its fresh 3,229-failure baseline
and zero-mismatch correction use the identical frozen native file. Earlier
short-route and compile failures are harness history too. Original Windows
runtime comparison was not performed.

## Reproduce

```sh
source local/reference-tools/env.sh
RR_BOUNDARY_LEGACY=1 python3 tools/reference/check_terrain_boundaries.py \
  --data local/cdrom/ROADRASH --output local/NEW-boundary-before
python3 tools/reference/check_terrain_boundaries.py \
  --data local/cdrom/ROADRASH --output local/NEW-boundary-fixed
RR_GROUND_LEGACY=1 python3 tools/reference/check_terrain_ground.py \
  --data local/cdrom/ROADRASH --output local/NEW-ground-before
python3 tools/reference/check_terrain_ground.py \
  --data local/cdrom/ROADRASH --output local/NEW-ground-fixed
RR_SESSION_DATA="$PWD/local/cdrom/ROADRASH" \
RR_SESSION_CATALOG="$PWD/local/catalog" \
RR_NATIVE_GROUND="$PWD/local/NEW-ground-fixed/native.json" \
RR_GROUND_OUTPUT="$PWD/local/NEW-ground-fixed" \
  cargo test --locked --target-dir ../target -p rr-game --bin rr-game \
  source_native_terrain_inventory -- --ignored
cargo test --locked --target-dir ../target -p rr-simulation --test terrain_boundaries
```
