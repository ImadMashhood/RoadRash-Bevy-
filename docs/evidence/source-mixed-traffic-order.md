# Mixed static, rider and traffic callback ordering

**400 controlled native transactions pass, including 400 independently checked
precontact mover snapshots and 40 explicit seed-1 stream checks.** Effects enabled
and muted produce identical state outputs and canonical hashes. The
[aggregate receipt](source-mixed-traffic-order.json) retains the failures and
scope. This is one 400-case family; the earlier 80/360 matrices and muted/legacy
reruns are not additional unique observations.

Three integration defects were measured before their corrections:

1. The host ran population contacts before the source static/rider scan. The
   first 80-case matrix failed 20 combined static/rider/car cases; deliberately
   reversing the actual native scans matched the old Rust output in all 80.
2. After correcting that phase order, a 360-case expansion found 20 failures
   when a first car crashed a bike. Native `collide_traffic` immediately returns
   from that bike's car pass; the host's car-outer loop continued that newly
   crashed bike against subsequent cars. In the first counterexample, this
   incorrectly added another vertical bump (1,360 versus 1,024), changed lean
   (-52 versus 0) and pushed a second car that native left alone.
3. Forty elevated-car controls then exercised the fresh detached body's actual
   car callback. Twenty body positions disagreed because missing human contact
   sound, horn and grunt variant draws advanced the shared stream differently.
   Native body progress 2,779 versus Rust 2,758 came from the same actual nudge
   law using different draws. Native's five draws ended at stream word
   3,403,800,452; the incomplete host used its first draw, ending at 2,745,024.
   The corrected host consumes the actual upstream draws before the nudge,
   including when presentation effects are muted.

The source-4 contact phase now runs the world static/rider walk before traffic.
Traffic jobs run one owner through all bike/car candidates, stop that bike pass
when a hit leaves it crashed, then resolve all detached-body/car candidates from
current state. A body created during the bike pass therefore participates in
its subsequent body pass. The existing physical-route handlers are reused.
Human impact sound draws read unscaled impulses; a car's human horn draw occurs
before or after the rider callback according to the original pair order. The
half handler's human grunt variant precedes its randomized longitudinal nudge.
These are authoritative stream draws, independent of audio delivery.

## Actual producers and bounded inputs

The producer uses the pinned C++ library without editing its sources. Original
physics, animation, crash and City vehicle resources initialize actual `Race`
and `TrafficSystem` objects. The native live scheduler's due flags and dt=1 are
explicitly initialized. Actual `step_rider` calls and `update_halves` produce the
precontact images; actual `collide_riders`, `collide_traffic` and
`collide_traffic_half` execute the transactions. The diagnostic alternatives
change call order and active-car selection while still invoking those handlers.
No native overlap, separation or response predicate is reimplemented in the
producer.

The Rust adapter invokes complete `Simulation.advance` ticks. A second,
contact-free host tick independently verifies each native mover's current and
previous poses, captured impact velocities, scalar speed, drive, lean, condition
and stamina before any mixed response is compared. Vehicle poses are held
constant by the controlled scenario. The observations include every bike's
postresponse fields, retained car positions and fresh detached-body motion.

| Rows | Cases | Coverage |
| --- | ---: | --- |
| 0–359 | 360 | Classes 0/1/2/3/16; speeds 0/5,000/20,000; both sides; static-only, car-only, rider-only and combined presence; 2 riders/1 car, 2 riders/2 cars, 3 riders/3 cars |
| 360–399 | 40 | Speed-20,000 frontal arrivals with 2 or 3 cars; the later cars retain authored height 4,096 so a newly detached body reaches a real car callback |

All 400 signatures `(class, speed, side, presence, riders, cars, elevated)` are
unique. Fifty cases create a fresh body; twenty execute an elevated-car body
response. The high-speed arrival uses an ordinary measured dt=1 displacement of
78 Q8 units, not a teleport across the contact window. Elevated car poses are
controlled retained-contact states, not a claim about ordinary flat-road vehicle
motion. The native driving/controller and car-pool reuse policies are outside
this transaction family. The native branch-ring walker limitation is unchanged.

The 360-case prefix of the final native trace is byte-identical to its earlier
frozen trace (`32aa591c…d91095a`). The complete final expected trace is
`a5893847ea0b1f149e04f50daa652a5eee29178d66f204e3add3e200b9453722`.
The original 80-case axes remain covered; an incidental inactive-rider distance
changed in the expansion, so those development observations are conservatively
not added again to the unique total.

## Failure preservation and legacy compatibility

The original 20-failure receipts were retained before each corresponding fix.
A private isolated source snapshot also reverses only the three contact-order/
stream corrections, reconstructing their prior behavior against the frozen
400-case input. It produces **150 physical-output mismatches and 40 stream
mismatches, with zero precontact mismatches**. The corrected run has zero of
all three. Its exact narrow source-file hashes are in the receipt's restoration
manifest; the shared working tree was never toggled, reset or stashed for this
check. This wider reconstruction supplements the original incremental failures.

All changes are gated on `combat_version >= 4`. Source-3 compatibility is checked
against an isolated restoration of the prior car-outer walk: all 400 pre/post
canonical hashes and outputs match the corrected tree. This is a Rust before/
after compatibility check, not a false native parity claim. A direct native
comparison retains 70 historical source-3 differences in the first 360 cases
and 110 across all 400, including its postcontact health regeneration and missing
contact-stream behavior. Native values were never normalized to conceal them.

Early compiler/due-flag/spec-condition adapter failures are invalid harness
results. The first elevated-car adapter also omitted the synthetic control's
retained car height; that precondition failure is invalid evidence. Fixing the
adapter yielded a valid 400-case before-stream receipt with zero precondition
mismatches and the twenty real jitter failures described above. Private traces
and invalid development runs remain under ignored `local/` for audit.

## Reproduce

```sh
source local/reference-tools/env.sh
python3 tools/reference/check_mixed_traffic_order.py \
  --data local/cdrom/ROADRASH --output local/NEW-mixed-fixed
RR_MIXED_MUTED=1 python3 tools/reference/check_mixed_traffic_order.py \
  --data local/cdrom/ROADRASH --output local/NEW-mixed-muted
# With a separately captured legacy Rust baseline from the prior contact walk:
RR_MIXED_LEGACY=1 RR_MIXED_LEGACY_EXPECTED=/absolute/path/mixed-traffic-rust.json \
  python3 tools/reference/check_mixed_traffic_order.py \
  --data local/cdrom/ROADRASH --output local/NEW-mixed-legacy-compat
```

Seventy-four targeted public collision/population/scenery regressions passed,
and all 638 prior native scenery observations passed with their unchanged trace.
Strict simulation all-target Clippy passed before unrelated Body2 API integration;
the parent integration gate owns the final workspace freeze check. The producer compiles with
`-Wall -Wextra -Werror`. This receipt does not establish full native traffic
controller behavior, a complete race/career, or original Windows-runtime parity.
