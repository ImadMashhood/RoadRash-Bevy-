# LIVE changing-terrain recovery

**216,718 native state observations match across 400 distinct controlled episodes.**
The family combines 240 center/shoulder episodes (127,361 observations) and 160
outside-boundary episodes (89,357). The first state of every episode is included;
these are whole compared state records, not a sum of their individual fields.
All350 non-wrecked episodes remount and continue for30 ticks. Fifty initially
wrecked-bike episodes reach their native standing-loop decision threshold. No
non-wrecked episode ends at the configured tick limit.
[Counts, trace hashes and preserved failures](source-recovery-terrain.json).

## Compared transactions

The producer calls actual pinned `Race::crash_reason` with controlled starting
poses/impulses, then actual LIVE half and quiet-bike callbacks with `dt=1`, refreshes
the sorted native list and executes `collide_riders`. Rust uses ordinary
`Simulation::step` after the equivalent controlled initial entry. Shared MSVC RNG
is anchored once, then free-runs for the entire episode. No recovery assist,
teleport, result injection, periodic RNG correction or disabled contact is used.

Fixtures cover building0, mountain1, tunnel3, hill4 and open255 terrain, both sides,
center/shoulder/outside positions, nonzero road offsets and fixed or changing
bend/rise/side heights. Initial cases include reasons1/2/3/4/5/6, stopped/forward/
backward throws, a high descending pose and a condition-zero bike. These initial
reason/velocity choices establish their subsequent recovery behavior; they do
not prove the mounted event that chooses a crash reason. The separate entry
kernel owns actual skid, depletion, contact and landing decisions. Voluntary
dismount is a repo control adaptation, compared with native reason4 semantics;
no original native dismount input flag is claimed.

Every state compares independent riderless-bike and body position, lateral,
height, speed, vertical/contact/ground/gravity/pitch state; quiet-bike gear, drive,
lean and condition; stamina maximum/current; active phase clock; animation code,
frame, queue, loop, deadline and retained descriptor; shared RNG; and the body's
physical heading (`+0x128`) separately from display yaw (`+0x5c`). Native inactive
crash timers and uninitialized animation sentinels are exported by their logical
meaning, rather than claiming byte-for-byte C++ object identity.

## Corrections exposed

The LIVE movement now runs each object's horizontal motion, boundary response,
terrain-ground query and vertical law before moving the next object. The body
runs before the quiet bike, followed by the native speed-eighth coupling. The
old NONLIVE recovery1 composition is retained for historical recordings.

The riderless bike also shifts gears during a crash. A valid16-episode subset
first showed1,048 differences, all in the retained gear. The same native trace
then matches9,206 complete states. Those observations are a subset of the final
matrix and are not added again.

Generic body motion retains source curvature accumulation. `BodyLaw.source_angles`
is optional and omitted when empty; current source factories explicitly load
the original257-byte atan table. Per-cross-sample integer weighting updates the
physical heading, while display yaw remains latched until native walking or wall
logic changes it. Without this feature, the final240-episode native trace has
62,396 differing records. It has zero after correction.

A contact half's randomized longitudinal nudge calls native `move_along` with
sweeping enabled. It therefore changes body physical heading on a curve. Ordinary
separation pushes use the non-sweeping form and retain that heading. The contact
adapter now preserves this distinction. A private before/after diagnostic at the
historical case37/tick3 showed position13063→13078 and heading−30240→−33600 with
zero pending impulse and flags; the initial pending-impulse hypothesis was
rejected by those measurements.

Body walls choose display direction from captured forward velocity (`+0x108`),
which can be negative while a walking body's scalar speed is positive. The
outside-boundary matrix first exposed1,552 display-yaw differences with every
physical field already matching. The source2 adapter now uses captured forward
sign; recovery1 retains its former policy. The same89,357-state native trace
then matches completely.

Source2 also retains the dormant body animation selection and quiet-bike queue
through remount and a subsequent new crash, and initializes a fresh body's
previous height from the pre-entry bike. Separate remount/contact evidence
records that correction. Four public regressions cover physical/display heading
separation, quiet-bike gears, new-crash frame/queue retention and ordinary riding
control quarantine through the remount frame. The paired quarantine run compares
neutral versus held throttle/steer/punch with no separate `walk` command for the
whole unassisted episode; they
stay physically identical until remount, and throttle resumes on the next frame.
Actual native held-control moving episodes have separate contact evidence. The
app intentionally maps on-foot arrow/axis input to a separate `Walk` command, so
this quarantine check does not claim those manual walking commands are ignored.

## Riderless-bike cache follow-up

The actual entry/quiet-callback family separately verifies the quiet bike's
retained display yaw and slip byte. Source2 clears the previous slip byte before
motion, refreshes cached yaw after grounded lateral-memory capture and before
walls, and uses the category4 gentle-ground vertical behavior. The native
576-case entry family exposes24 differences before this correction and none
afterward, using unchanged trace
`f65d4b952a52578f33411884c7f01b02b0795c13ff979ee7e86db3742c1acfa8`.
These48 quiet-callback cases belong to that entry family, not an extra terrain
observation count. The seventh public recovery regression preserves the old
source1 behavior while requiring source2 yaw refresh and clear/gentle slip
transitions. [Entry evidence](source-recovery-entry.md).

## Provenance and limits

Oracle revision `7aecf6044f2dd773b925b325823f30d648d61c89` and library hash
`f530ae9ce4ce031cea19ce022fde138f2a3fd77a52d01fc82cdce148e472fb51` are unchanged.
Pre-existing reference HUD edits were preserved and are outside these compared
functions. Original Windows runtime comparison remains owner-waived. Native
variable cadence and complete TrackRing graph scheduling are not claimed.

The first320-episode draft duplicated80 center cases because side± had no effect
at lateral0. It and its173,581 observations are superseded by the distinct240-case
matrix. Its larger diagnostic failure counts are not used for final accounting.
Early compile, callback-clock, source-cache, inactive-field and omitted-contact
harness attempts remain private and are excluded from engine failure counts.

Initially wrecked episodes stop at the native standing-loop decision threshold;
full game-over/session cleanup is separately verified. Initial display yaw is
zero in this controlled family, so actual steering-at-entry copy needs the entry
kernel. Three-or-more-owner moving recovery, actual trigger selection, final
native-art inspection, career and release acceptance belong to their respective
evidence and cannot be inferred from this family.

```sh
source local/reference-tools/env.sh
python3 tools/reference/check_recovery_terrain.py --data local/cdrom/ROADRASH \
  --output local/NEW-recovery-terrain
RR_RECOVERY_OUTER_ONLY=1 python3 tools/reference/check_recovery_terrain.py \
  --data local/cdrom/ROADRASH --output local/NEW-recovery-outer
CARGO_TARGET_DIR=../target cargo test -p rr-simulation --test recovery_curvature
```
