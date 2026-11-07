# Native manual walking and pickup decisions

**All4,200 comparisons pass** against3,780 actual native `Race::crash_recovery`
transactions. The additive family contributes **3,360 nonzero directional-input
observations**. Native0 controls form420 regression-control rows; each compares
Rust `None` and `Some(Walk::default())`, giving840 additional comparisons that are
excluded from the observation aggregate. [Receipt and hashes](source-recovery-walk.json).

The same frozen native trace first exposed1,560 failed comparisons:1,248
non-player manual overrides and312 explicit-zero walk inputs that incorrectly
stopped the body. Source2 now applies a manual override only for the human
player with a nonzero directional axis. `None` and explicit zero both map to
native0 and continue automatic return at the source's1200 pre-motion speed.
Source0/1 retains the historical explicit-zero hold behavior.

The producer uses the actual native race wrapper rather than copying its
algorithm into an expected-value table. It covers forward/back/left/right and
all four diagonals, human versus non-player eligibility, condition150 versus0,
longitudinal gaps−1000/−10/−9/0/9/10/1000, lateral reach residuals
−6001/−6000/0/6000/6001, and initial facing0/90/180 degrees. Pickup requires
absolute forward gap≤9 and absolute lateral residual<6001; it does not add a
facing gate. A usable bike enters mount clip41 and queue2; a condition-zero bike
enters standing clip37. Manual movement updates physical/display heading and
resets the recovery clock; ignored NPC and zero controls preserve that clock.

Command mapping is forward `0x800`, back `0x1000`, left `0x4000`, right `0x2000`.
The command API provides signed axes, not independent opposing raw key bits.
The app intentionally maps on-foot arrow/axis input to this separate `Walk`
command. Tests of raw riding-control quarantine with `walk=None` therefore do
not claim that intentional manual walking is ignored.

Two public regressions check zero-input automatic return while preserving
recovery1 hold, and player-only diagonal facing versus NPC automatic return.
All six tests in `recovery_curvature.rs` pass. The new rule changes no neutral
`None` path in the400-episode terrain family; complete recovery trajectories,
moving multi-body contacts and final visuals have separate evidence.

Native revision/library/EXE identities and pre-existing untouched HUD changes
are recorded in the receipt. The native trace remains
`cf6d2ea5e6537499a1d8bb8896c12befe35a7ad3335ca78f808c2419d352ebda`
before and after the correction. This is a controlled recovery-decision kernel,
not a claim about native variable scheduler cadence, attract-demo input or
post-arrest walking. Original Windows runtime comparison remains owner-waived.
Commercial traces stay private.

```sh
source local/reference-tools/env.sh
python3 tools/reference/check_recovery_walk.py --data local/cdrom/ROADRASH \
  --output local/NEW-recovery-walk
CARGO_TARGET_DIR=../target cargo test -p rr-simulation --test recovery_curvature
```
