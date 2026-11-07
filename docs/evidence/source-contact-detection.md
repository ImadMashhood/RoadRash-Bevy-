# Native contact detection at high speed and endpoint boundaries

**144 additional observations match; the complete comparison now passes 327,230
observations across 23 families.** This does not close swept-contact acceptance.
The [aggregate receipt](source-contact-detection.json) retains every family,
reference/library/data identities and trace hashes. The earlier 327,086-row
[source-contact/recovery receipt](source-contact-recovery.md) remains historical.

## What the probe executes

The native producer calls the actual `Race::collide_riders` position-sorted scan,
including its broadphase, endpoint tests and conditional callbacks. It does not
assume an overlap and call the separator unconditionally. A private declaration
access shim exposes that method without changing the pinned source/library.
The Rust counterpart calls `World::contacts` over identical controlled poses and
reports whether a contact event is emitted. The producer observes whether either
native bike's contact flags changed from a sentinel; strict-edge positive and
negative controls establish that the scan really ran.

Both normal and fallen physical extents are covered, with velocities 0 and
30,000, positive/negative sides and three neighboring endpoint margins:

| Scenario | Observations | Contacts | Misses |
| --- | ---: | ---: | ---: |
| Longitudinal endpoint edge | 24 | 16 | 8 |
| Lateral endpoint edge | 24 | 16 | 8 |
| Height endpoint edge | 24 | 16 | 8 |
| Complete longitudinal crossing | 24 | 0 | 24 |
| Complete lateral crossing | 24 | 0 | 24 |
| Genuine diagonal near miss | 24 | 0 | 24 |

Endpoint boundaries are inclusive on all axes. The diagonal path has overlapping
swept bounding rectangles but clears the expanded longitudinal slab before it
enters the lateral slab, so its absence of contact is correct.

## Measured limitation: this source scan is not continuous collision detection

Both the native and Rust scans test current endpoints. Controlled complete
crossings therefore miss even when their path intersects the target in between.
This is a measured agreement about discrete detection, not evidence that narrow
obstacles cannot be tunneled through. No production physics was changed to hide
this distinction.

At speed 30,000, zero acceleration and dt=1, native longitudinal integration moves
117 Q8 units per tick. The injected two-bike longitudinal crossings span
259/260/261 Q8 units; these must not be described as ordinary one-bike native
motion. Likewise, injected full lateral crossings are detector inputs, not a
proof that the motion controller produces those trajectories. Scenery/source-box
queries, reachable changing-curve trajectories and complete continuous-contact
acceptance remain open in the priority checklist.

Two public authored tests retain the inclusive endpoint policy for mounted/fallen
boxes and explicitly demonstrate a missed between-endpoints crossing. They do
not replace the native world-scan comparison.

## Reproduce and results

```sh
source local/reference-tools/env.sh
python3 tools/reference/check_encounters.py --data local/cdrom/ROADRASH \
  --output local/NEW-contact-detection
cargo test --locked --target-dir ../target -p rr-simulation --test contact_detection
cargo clippy --locked --target-dir ../target -p rr-simulation --all-targets -- -D warnings
cargo fmt --all -- --check
```

Verified 2026-11-06 on base `a5eff4c` plus this working tree: all 15 opt-in private
native tests ran, every compared family passed, the two public regressions passed,
and strict simulation Clippy, formatting and diff checks passed. Final private
traces and build logs remain under `local/contact-detection-2026-11-06-final/`
and its sibling log. The first 144-row run is preserved separately; the final
producer varies crossing endpoint margins instead of repeating those poses.
No original Windows execution, full police model, dense multi-actor sequence,
career completion or replacement release acceptance is claimed.
