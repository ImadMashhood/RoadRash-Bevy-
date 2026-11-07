# Source static queries: original boxes before contact-response lowering

**1,980 new static-query observations match. The complete native comparison now
passes 329,210 observations across 24 families.** The wider swept-contact gate
remains open. [Aggregate receipt and preserved first failure](source-static-queries.json).

## A real detection mismatch, retained before the correction

The first three-type matrix failed 16 observations. Adding twelve raised-frame
controls produced a valid 1,980-case baseline with **22 mismatches**, including
six false contacts with grounded riders beneath elevated frame boxes. The native
producer and Rust writer completed normally; the Python comparison correctly
returned failure and retained the first divergence.

The source tests a billboard's original frame box before dispatching its type.
For types 1 and 2, only the contact-response box then lowers its bottom to
`top - 8192` or `top - 16383`. The Rust importer lowered the stored box before
both detection and response. A grounded rider could therefore collide with an
elevated frame which the native query correctly rejected.

Newly imported type-1/2 source boxes carry optional `query_bounds` containing the
original anchored frame rectangle. Detection for both bikes and detached bodies
uses this rectangle; response keeps the independently lowered `bounds`.
Both rectangles have validated order, coordinate range and dimension budgets.
The bounded native detector family here covers bikes with mounted/fallen extents;
it does not claim full detached static-transaction parity.

## Old recordings retain their behavior and canonical hashes

Missing `query_bounds` falls back to the historical stored bounds and is omitted
from serialization. A public 180-tick legacy recording froze its encoded bytes
and final state hash before the field was added; both still match at
30/60/144/irregular schedules. Explicit new query geometry changes replay identity
and malformed query boxes fail validation. Existing old replay checkpoints stay
separate from the newly imported geometry.

## The native query and what was measured

Authored frame boxes are attached through native `set_static_frames` and
`ring_statics`. The actual sorted `collide_riders` scan then applies the strict
longitudinal midpoint gate and dispatches `sweep_statics`. The Rust writer calls
its full world contact path with the importer's sample-midpoint obstacle layout.
It does not call the separator unconditionally or copy the native query predicate.
The private access shim exposes declarations without editing the pinned native
sources or library.

| Scenario | Observations | Contacts | Misses |
| --- | ---: | ---: | ---: |
| Strict longitudinal midpoint edges | 72 | 24 | 48 |
| Inclusive lateral frame-box edges | 72 | 48 | 24 |
| Asymmetric height edges | 72 | 20 | 52 |
| Complete controlled longitudinal crossings | 72 | 0 | 72 |
| Genuine diagonal near misses | 72 | 0 | 72 |
| Complete controlled lateral crossings | 72 | 0 | 72 |
| All sample fractions at speed 30,000 | 1,536 | 762 | 774 |
| Grounded rider beneath a raised frame | 12 | 0 | 12 |

Every scenario covers normal/fallen dimensions and all three billboard types.
For each type/dimension combination, exactly 127 of the 256 sample fractions
contact: the midpoint interval is strict. The phase matrix uses a 117-Q8-unit
previous-to-current displacement, consistent with speed 30,000, dt=1 and zero
acceleration. Two new public tests also drive ordinary Rust ticks across every
phase and both lateral edges.

As in the [rider contact-scan follow-up](source-contact-detection.md), injected
complete crossings miss between endpoints in both engines. Their larger pose
deltas must not be called ordinary max-speed native trajectories. Native agreement
here establishes discrete query policy, not continuous collision detection.

## Reproduce and verification

```sh
source local/reference-tools/env.sh
python3 tools/reference/check_encounters.py --data local/cdrom/ROADRASH \
  --output local/NEW-static-queries
cargo test --locked --target-dir ../target -p rr-simulation --test contact_detection
cargo clippy --locked --target-dir ../target -p rr-simulation -p rr-game --all-targets -- -D warnings
cargo fmt --all -- --check
```

Verified on base `a5eff4c` plus this working tree on 2026-11-06: all 16 opt-in
native tests ran, all 24 compared families passed and seven public contact-query
regressions passed. Strict simulation/game Clippy, formatting and diff checks
passed. Before/fixed native expected traces are byte-identical; the
Rust outputs changed from 22 mismatches to zero. Raw observations, source-data
identities, exact first failures and captures remain private under
`local/static-query-bounds-before/` and `local/static-query-bounds-fixed/`.
The earlier 16-failure matrix is retained separately. A separate explicit-data
frontend factory check loaded all five source courses at level 1, built each
with bikes 1/8/15, checked the City character cases and released scene owners;
it passed with the original query rectangles enabled. Its log is retained in
`local/driving-bounce-2026-11-06/source-factory-final.log`.

Final combined validation: 401 workspace tests passed (17 private-data tests
ignored by the ordinary run), 36 Python tests passed, strict workspace Clippy,
formatting and diff checks passed. All 16 private native comparison writers and
the separate original-table renderer test also ran successfully with the required
data. A fresh release binary passed the 12-second full-field City smoke and
45-tick native boundary brush at 30/60/144/irregular schedules, including an
actual Bevy run with identical headless/rendered state hashes. All six City
source boxes in the captured setup explicitly retain original query rectangles.
Actual race/wall screenshots were inspected. This does not renew performance,
career, package or owner acceptance. Runtime receipts are retained in
`local/final-gameplay-2026-11-06/` and the aggregate JSON above.

Still open: BORR-specific box expansion, live slice loading/visibility policy,
remaining static classes, complete detached static transactions, changing terrain
and three-or-more-actor interactions. This bounded correction does not certify
full career, performance, packaging or project-owner release acceptance.
