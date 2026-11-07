# Crowded recovery contacts and the remount boundary

The native callback comparisons pass **79,610 observations across three families**:
2,400 complete crowded bike/body callback walks, 768 ordered remount/static
transactions, and 76,442 full moving crowded-recovery ticks. Initial poses are checked separately, not added to the total.
The camera endpoint is an additional field of the crowded records, not an additional
family or extra observations. [Machine-readable accounting](source-recovery-contacts.json).

## Crowded bodies and stable list history

The 600 controlled cases contain three or four owners, with one, three or four
detached bodies. Initial source clips 35/36/37/38/41 represent sliding, getting up,
standing, walking and mounting. Both sides and two crowded layouts are exercised,
with speeds 0/1699/1700/1701/8000/20000. The native `collide_riders` walk runs four
times per case, refreshing the native sorted schedule between host transactions.
The shared RNG is anchored once per case, then free-runs. These contact fixtures
deliberately do not schedule the intervening motion callbacks.

All 600 initial kinematic/clock fields agree. Contact walk version 1 diverges on
62 of the 2,400 records, involving 22 cases. The first divergence is case 98, second
walk: the fourth body reaches its own bike's position on the first walk. Native
stable sorting retains body-before-bike; the old creation-key tie-break reverses
them. The third bike's retained previous lateral becomes 3040 instead of 3120,
and later cases also diverge in body nudges, animation, impulses and shared RNG.

Explicit contact walk version 2 preserves list history, including the source's
post-contact sort. Dormant halves retain their pre-contact mirror key until the
next move. New actors are inserted after existing equal-position objects;
despawn removes both bike/half keys. The history is serialized for version 2 and
remains absent under version 1. Version1's output was independently rerun after
the fix and remains byte-for-value identical to its preserved failing receipt.
Its recorded behavior is not silently changed.

The accepted callback/motion fields match the same native matrix as the 62-failure
baseline. The final trace also records `Race::camera_position` after every walk.
It equals the actual World `scenery_prepare(true)` pending-camera endpoint,
including contact-driven moves of the player's detached body. No longitudinal
camera lag was reproduced for these bodies. This does not replace the separate
camera height, lateral, terminal-pullback or terrain-cache comparisons.

## Contact on the last mount tick

The second family uses 384 controlled poses and compares two actual native
operations in both orders: `crash_recovery` and `hit_static` (separation followed
by the source static response). A still-active mount clip 41 is distinguished
from completed mount clip 2 with its retained selected descriptor 41. Other
sliding/get-up/standing/walking poses are controls. Speeds 5000/11000/11001/20000,
both crossing directions, condition 1/150 and short/tall objects are covered.

The initial 384 poses match. The valid baseline has 14 differing transactions.
After a completed mount, a tall-object contact can create a new incident. Native
preserves the quiet-bike queue 0 and the previous body's selected mount descriptor;
the new body's previous height remains the pre-entry height. Rust had discarded
those retained fields. Six of the 14 cases also exposed a forced-contact path
calling historical `begin_source`, selecting quiet-bike clip 46 instead of using
the LIVE entry policy. recovery version 2 now retains the clocks and previous pose; forced
contacts select LIVE entry. All 768 transactions match the identical native trace.

Public regressions reproduce the converged-body ordering difference, bound the
despawned history, and distinguish ignored crash re-entry during mount from a
new crash after completed remount while retaining owner/incident identity and
native clock selections. The same correction is used by the independently
measured full recovery trajectories; these callback kernels alone do not prove
whole-trajectory terrain or native variable-cadence scheduler parity.

## Full moving crowded recovery

The 144 actual LIVE episodes contain three or four initially fallen riders. They
cross longitudinal spacings 0/40/128, lateral spacings 2000/8000, speeds 0/5000,
normal impacts and low-speed falls. Player commands are neutral, held throttle,
or held throttle+left+punch; no invented walk-control bits are supplied. Each
actor runs its actual half and bike callbacks at `dt=1`, followed by the native
stable sort, mixed contact walk, post-contact sort and camera update. Rust uses
ordinary `Simulation.step` with the same commands and an initially seeded shared
RNG that then free-runs. Flat wide ground isolates inter-body recovery contacts;
the separate full-terrain family covers changing ground and boundary types.

All 144 episodes reach every owner's first remount and continue 30 more ticks.
None stops at the 3000-tick limit. All 76,442 records agree, including signed
motion, retained previous poses, stamina/condition, quiet/body clocks, the visible
mounted rider clock after remount, and shared RNG. Owner links remain valid.
This directly covers contacts during sliding, get-up, walking and mounting as
multiple bodies move, rather than reusing the component-only four-walk fixture.

A 16-episode held-input subset exposed 915 physical differences when an early
remounted player steered beyond the native LIVE lateral bound while other
riders still walked. Native half/bike callbacks clamp live objects to±819200;
contact walk version 2 now applies those callback-end clamps without changing velocities.
The identical subset trace passes afterward. Expanding to 144 episodes then
exposed 36 first-tick previous-height differences: crash entry lifts the quiet
bike by its suspension, but its physical Location had retained the pre-lift
height. recovery version 2 now updates that current Location after retaining the body's
pre-entry previous pose. The identical physical-field trace passes afterward.

The final native trace extends those same records with the effective visible
rider clock. The native driver now advances its actual previous-camera endpoint
on every host tick; a diagnostic which omitted that driver step produced stale
NPC distance clips and is invalid for clock acceptance. The corrected final clock
trace passes all 76,442 records. Its physical columns are byte-for-value identical
to the two valid physical-field counterexample matrices. Neither the 8-episode
nor 16-episode pilots are added to the 144-episode total.

## Additional integration correction

The changing-terrain producer found that a detached contact's randomized nudge
calls native `move_along` with curvature enabled. Geometric separation uses the
position-only variant. The recovery version 2 contact adapter now applies the measured curve
sweep to that randomized nudge only. It does not add curvature to separation. Wall-facing selection additionally
uses the captured longitudinal velocity sign, matching the native wall callback
when a walking body faces opposite its scalar speed.
The full trajectory producer owns acceptance/counting for this cross-component
correction; no additional observation count is claimed here.

## Provenance and preserved harness failures

Reference revision is `7aecf6044f2dd773b925b325823f30d648d61c89`; the original
Windows runtime remains owner-waived. No reference source or library was edited.
The pre-existing HUD edits remain recorded and outside these compared callbacks.
Commercial traces stay under ignored `local/`.

The first crowded harness omitted native schedule refresh between passes and
compared the native uninitialized animation sentinel−1 to Rust's default 0.
Both represent an unselected constructor descriptor. The corrected producer
canonicalizes only that sentinel and refreshes the actual native sort; its first
600 poses then match. The invalid run remains `local/recovery-contacts-first`.
It is not an engine-failure baseline. An early remount producer failed C++
warnings-as-errors before running; this is also harness history, not a mismatch.

Reproduce from the repository root:

```sh
source local/reference-tools/env.sh
python3 tools/reference/check_recovery_contacts.py --data local/cdrom/ROADRASH --output local/NEW-recovery-contacts
python3 tools/reference/check_remount_contacts.py --data local/cdrom/ROADRASH --output local/NEW-remount-contacts
RR_RECOVERY_MULTIWALK_FULL=1 python3 tools/reference/check_recovery_multiwalk.py --data local/cdrom/ROADRASH --output local/NEW-recovery-multiwalk
CARGO_TARGET_DIR=../target cargo test -p rr-simulation --lib source_contact_history
CARGO_TARGET_DIR=../target cargo test -p rr-simulation --lib source_remount_contact_distinguishes
```

Accepted receipts are `local/recovery-contacts-final/comparison.json` and
`local/remount-contacts-final/comparison.json`, plus
`local/recovery-multiwalk-camera-aligned/comparison.json`; exact identities and before/fixed
hashes are retained in the adjacent public JSON. Strict simulation Clippy passes.
The parent owns final workspace checks and complete recovery-art inspection.
