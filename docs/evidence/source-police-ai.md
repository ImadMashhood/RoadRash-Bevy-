# Native cop AI decisions and their host bridge

**92,502 observations match across three bounded native families:** 43,740 cop
brain transactions, 14,580 dense object/gate transactions and 34,182 actual
source raw-lane records. These are decision and geometry checks, not by
themselves complete police lifecycle or full-race acceptance. The coupled
AI/fight/motion, recovery and arrest evidence belongs to
[source-combat-police.md](source-combat-police.md). Aggregate receipts and frozen
first divergences are in [source-police-ai.json](source-police-ai.json).

`population/police_ai.rs` ports the actual pinned `Race::ai_think` dispatch and
its called routines. It keeps native states, per-state clocks, 60-tick replan
banks, rating/target arrays, easing, cop pursuit-speed decay/reset, neighbour
selection and tie rules, target steps, attack eligibility, strict hit-immunity
and random gates, curve/straight/follow/fight/shadow targets, overtake latches,
speed matching and downed/on-foot target caps. The old experimental
`tick % 30` punch scaffold is removed. A cop consumes the native kick draw even
though its cop gate prevents selecting a kick, then consumes the backhand draw
when starting its carried-weapon attack. Current attack animation recognition
uses all four variants of the latched weapon. Successful weapon theft updates
both participants' AI weapon latches at the inventory transfer callsite.

The control path calls the source low-level drive/steer primitives directly.
State 3 calls follow controls before the shared strike and its second steer
after that strike. Clocks, replanning and mounted regeneration follow the
strike as well, so same-tick reactions and weapon theft are visible. The pure
decision probe wraps both phases around its inactive strike fixture. Passing one proportional player
command would lose both the exact signed angle/sine quantization and those
repeated force updates. Cop specifications therefore bypass subsequent player
control application while still running traction/integration. The persisted
`Bike.ai_roll` holds the actual orientation value used by the airborne steer
gate; a selected display-sprite roll is a different value. The source 24-bit
angle wrap maps the exact behind-target angle onto the negative steering side.
The valid old-wrap baseline has 480 mismatches against the same native trace
as the zero-mismatch corrected run.

Mounted cop stamina regeneration belongs to the AI postdispatch path, with one
pass while mounted. Source recovery owns two passes in get-up/walk/mount and
none while sliding. The shared fight bookkeeping marker remains present for
cop thinkers so attack preparation and bank advancement still run; that marker
is not permission to add a second stamina pass. Coupled transaction ordering
must be verified separately from these one-think decisions.

Authoritative `Unit.police_ai` stores the bounded native words and typed actor/
object references. `Unit.police_relations` retains the counterpart personality
ratings/index/reaction words needed by immediate source callbacks. Attack-start
reactions and landed-hit reactions update that authority at their callsites.
A source attack retains its original target for those notifications even when
end-of-think replanning selects another rider: seven dense-case mismatches were
preserved before this correction. The AI state is initialized at actual cop
birth, after its source lateral snap, without controls, regeneration or random
draws; deferring birth replanning to the first think changes strict scan-edge
cases after other riders move.

## Native scopes and inputs

`tools/reference/police_ai.cpp` links the unchanged native library and calls the
actual `Race::ai_think`. The main 43,740 cases vary all five levels, six mounted
states, straight/left/right curves, signed gaps and lateral positions, speeds
0/4,001/8,001, replan ages 0/59 and target crash words 0/1/2. Clocks 90/100
expose the scaffold's invented periodic attack rule. The controlled road fields
are consistent: 500-unit widths and 512-Q8 lane words. Output comparison includes
RNG state, targets/speeds, neighbours/ratings, overtaking, low-level force/lean/
speed, latched weapon and effective native bank-1 animation, state clocks,
relationships and regeneration. These are controlled state inputs, not claims
that every state is reached by ordinary throttle.

The 14,580 dense transactions build four native riders plus actual traffic and
pedestrian pool objects. They vary target ownership, detached half poses,
creation-order ties, presence gates, signed object velocities, scan windows and
lateral stopping rules. They also exercise current attack clips, carried versus
latched weapons, self crash words 0–3, signed skid bytes including 128/255,
airborne contact, roll thresholds 13/14, immunity ticks 99/100/101 at clock 100,
and places 0/1/3/4/16. Those cases include 190 transactions consuming random
draws, 18 animation changes, and native retargeting among the nonplayer riders.
Raw crash words here test the decision dispatch and regeneration gates; they do
not stand in for complete crash/recovery trajectories. Active attack descriptors
in this decision matrix are not hit frames; complete strikes belong to the
coupled/source-fight evidence.

Raw `lanes_q8` is optional source-road metadata because world-space widths lose
fractional lane information used by native curve targeting. `Roads::from_course`
imports it directly. All 34,182 physical node/sample records across the five
source courses match actual pinned native `PathSample` lane words. The EXE atan
table is decoded independently for the native curvature calculation. Lane maps
must cover every physical ground node with matching sample counts and bounded
values. Absent metadata remains omitted in old setups, and old setups without
native police continue to use their historical authority.

The frozen unaccepted scaffold is tested only on fields it actually computed:
RNG, target lateral/speed, the resulting single player-control application and
selected attack clip. It has 42,384 mismatches against the identical final native
trace. No invented historical brain state is included in that baseline. Its
private adapter switch is `RR_POLICE_AI_LEGACY=1`; the valid wrap-only baseline
uses `RR_POLICE_AI_BEFORE_WRAP=1` and a corrected animation fallback adapter. An
initial 481-case residual mixed 480 real wrap failures with one invalid adapter
comparison of requested clip 18 versus native effective fallback clip 16. That
receipt remains historical and is not reported as 481 engine failures.

## Physical graph scope

The host aligns rider and object poses into the cop's own physical route axis,
including selected nodes beyond one adjacent portal and shared tails after
unequal branch lengths. It does not compare another rider's accumulated-distance
axis directly or insert zero as an offbranch coordinate. Objects outside owned/
visible physical context are excluded. Typed bike/body routes and retained
native half/cache history remain independent.

The source `TrackRing` does not support whole split/merge walking. Graph pursuit,
spawn, independent chased-body ownership and cache loading are functional Rust
adaptations rather than claimed full native Ring parity. Public regressions
cover multisection scan ownership, disconnected exclusion, optional/malformed
lane metadata, immediate relationship callbacks and authoritative serialization.
Eight public `police_roads` integration tests now pass (private command log
`local/police-graph-final.log`):

- A body takes the left arm while its quiet bike takes the right arm; cop birth
  inherits the chased body's route and retained half, and its decision view
  projects that body independently of the unowned bike.
- A cop survives a valid interior section entry with zero local progress.
- The cop traverses the short arm while the player takes the arm twenty samples
  longer. Both meet on the shared tail: their physical gap is below one sample,
  their accumulated-distance difference retains exactly twenty extra samples,
  and native follow selects the player and matches nearby speed.
- A far cop on the same connected road leaves the bounded physical window and
  releases its unit, linked agent, navigation, inventory and cached half.
- Disconnected spawns are rejected, and a disconnected rider is never selected
  by a valid cop.
- A cop crashes on the selected right arm, retains the independent body route
  and half at the next contact walk, recovers, and removes its dormant body half.
- A cop on the player's selected arm completes an arrest.
- A replay with independently routed player body/bike and a cop matches state
  hashes and checkpoints at 30/60/144 Hz and irregular frame schedules.

Before-fix logs retain the real wrong-body-route spawn, missing body-proxy
projection, and incorrect interior-zero unload failures:
`local/police-graph-body-route-before.log`,
`local/police-graph-body-proxy-before.log`, and
`local/police-graph-multinode-before.log`. The generalized graph regression
also exposed invalid authored expectations: schema 6 without actions, checking
a newly detached half before its first eligible preparation, and a short
terminal tail plus an arbitrary 128-Q8 catch condition. Corrected fixtures use
schema 5, the next contact walk, and a long tail with a one-sample physical
reunion check; the exact twenty-sample axis assertion is retained.

The lifecycle window uses bounded portal traversal in both directions, retaining
only actors within the configured physical disengagement range (64 samples by
default). Global graph connectivity does not imply residency, and interior node
boundaries do not imply unloading. This is the explicit functional graph policy
in place of unavailable whole native Ring residency. Public graph cases are
not additional native observation families.
The approved fixed-60-Hz decision architecture and isolated source geometry
streams are retained; this does not port every general opponent brain or claim
race-wide original RNG interleaving.

## Reproduction and provenance

Source revision is `7aecf6044f2dd773b925b325823f30d648d61c89`; the only reference
working-tree edits remain the approved HUD source/test paths. The native library
and original executable hashes are recorded in the JSON. Private copied headers
expose declarations without modifying the linked implementation or reference
checkout. Source assets and raw output remain private. Original Windows runtime
comparison remains owner-waived.

After sourcing `local/reference-tools/env.sh`, run
`tools/reference/check_police_ai.py --data local/cdrom/ROADRASH --output NEW_PATH`
for main decisions, add `--objects` for dense object/gate transactions or
`--geometry` for actual lane import. Each run refuses to reuse its output
directory. Valid before/fixed receipts retain matching native hashes and first
divergences. Counts in this report include only the final three families; older
matrices and adapter diagnostics are not added again.
