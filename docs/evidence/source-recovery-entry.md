# Actual mounted recovery-entry decisions

**576 native callback transactions pass, with zero initial-state or output
mismatches.** This family complements the complete changing-terrain recovery
trajectories: those select an initial reason/impulse, while this family invokes
actual decision callbacks. [Counts, hashes and preserved failures](source-recovery-entry.json).

Each of twelve groups has 48 controlled cases across speed 0/300/5000/12000,
signed lateral motion and four stamina/condition/drive setups. The groups invoke
`Race::knock_off` for unarmed/weapon depletion, the native car-entry wrapper,
actual traction/mounted movement, actual building/rail callbacks, hard-landing
mounted movement, and previous mounted movement followed by car/wall/rail entry.
The previous-move groups retain nonzero display yaw from the preceding real move.
A final 48-case group continues through the actual quiet-bike animation/motion
callback. Twelve of those install a controlled gentle-landing pose immediately
before that callback: height=suspension+1 and vertical speed−1.
The final record includes 35 integers for each initial and resulting linked
bike/body state, stamina, crash presence, quiet animation clock and quiet yaw.
Initial-state checks are prerequisites, not extra accepted observations.

The actual skid callback establishes the strict 25-tick threshold and stationary
exceptions. A skid fall uses reason 0 with no added impulses, before the remainder
of that bike's movement. The measured hard-landing callbacks do **not** crash;
the implementation does not invent an automatic severe-landing fall. Car rows
call the actual car-entry wrapper, while car overlap/contact selection has its
own evidence. Likewise, depletion uses the real `knock_off` decision helper;
combat acquisition and damage derivation are covered by the combat families.

## Corrections

- Body 2 now separates mounted traction from the remaining movement. When
  traction starts a fall, the body copies the pre-movement bike state, reason 0
  and zero impulses. The quiet bike then completes native speed/gear/movement,
  boundary and vertical work, including its changed contact drag 95. Fresh skid
  bodies do not cause that first quiet-bike boundary callback to be skipped.
- Source entry preserves the quiet bike's current drive. Weapon depletion also
  preserves it when condition becomes zero, before the entry helper runs.
  Historical generic damage/body 0/1 behavior remains available unchanged.
- Optional Body 2 source yaw and motion flags retain the actual mounted display
  yaw independently of the detached body's physical heading. A new body copies
  that yaw; its physical heading starts at zero. Nonangled wall 6 resets yaw
  before entry. Rail 3 enters before the yaw reset. The quiet yaw reset also
  remains visible to a later incident.
- Quiet-bike callbacks clear the prior slip byte, refresh their own display yaw
  during movement, and use the native gentle-landing slip 1 branch. These changes
  apply only to Body 2; old 0/1 remain unchanged.
- New wall/rail falls use the LIVE quiet animation initial state. They do not
  preinstall historical clip 46/queue 44. Default unselected native code −1 and the
  exact default Rust player are compared canonically; retained clocks are not
  normalized away.

The valid 384-case skid baseline has 27 differences and its correction has zero,
with native hash `089feb86a5997867e702b618061d97be28bd31b6c5a9cd3961f84723744993de`.
The richer 432-case drive/yaw baseline has 222 differences, then zero with the same
`73421bc8751b26866d200ae22af4beb5f58d1f12eab722145cfa96470b1b544a` oracle.
The reduced 528-case warm-wall record has 16 yaw differences, then zero against
its unchanged34-column native trace `2102b96db9232fce31a78da082f3130ae52c03151c1cc4a3ead1ac720f2f4370`.
The final 576-case, 35-column trace is
`f65d4b952a52578f33411884c7f01b02b0795c13ff979ee7e86db3742c1acfa8`.
Its valid baseline has 24 differences:12 prior-slip cases must clear the old slip
byte (eight also refresh stale quiet yaw), and 12 gentle landings must set slip 1.
After those corrections, all 576 records pass against the unchanged expected
trace in `local/recovery-entry-quiet-final/comparison.json`.
**Count the final 576 observations once**, rather than adding earlier matrices or
individual compared columns.

## Voluntary action policy and limits

The pinned source has no voluntary-dismount input flag or input wrapper. The
repo's independent action remains a functional extension: 48 rows invoke the
chosen native reason 4 entry primitive with zero vectors, without claiming an
original keyboard trigger. Body 2 now routes that action through the same LIVE
entry state. A public low-speed action regression verifies its first quiet-bike
animation consumes one native coin, rather than preselecting clip 46.

Four public regressions cover skid body-copy timing/zero impulses, condition
zero with retained drive, independent body yaw and the voluntary-action clock.
The published historical replay hash and strict simulation Clippy pass.
Profiles body 0/1 and published combat 0–3 do not enable the new optional yaw/flag
state. Intermediate Body 2 development recordings remain historical; final
current-profile recordings are regenerated by the parent integration gate.

All comparisons use the pinned C++ port and owned original data recorded in the
JSON receipt; the original Windows runtime comparison is owner-waived. These
controlled callback records do not prove whole-race scheduling, free RNG
trajectories or whole-game acceptance. Reference files/library were not changed.
Commercial traces remain in ignored `local/`.

Reproduce after sourcing `local/reference-tools/env.sh`:

```sh
python3 tools/reference/check_recovery_entry.py --data local/cdrom/ROADRASH \
  --output local/NEW-recovery-entry
CARGO_TARGET_DIR=../target cargo test -p rr-simulation --lib native_recovery_entry
```
