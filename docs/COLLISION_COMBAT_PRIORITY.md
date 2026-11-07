# Active priority: collision, fighting and crashes

This is the next required gameplay gate, ahead of additional presentation work,
texture enhancement and the architecture/optimization report. The owner asked
for stronger emphasis on accurate collisions, fighting and crashing. A plausible
render or deterministic replay alone does not establish accurate interactions.

Reference: arkits/road-rash at `7aecf6044f2dd773b925b325823f30d648d61c89`,
particularly `race.cpp`, `crash.cpp`, `actor_animation.cpp` and their public APIs.
Original Windows-runtime comparison remains waived; comparisons below use the
locally built pinned C++ port with the supported original data. Source assets,
setups and detailed traces stay private.

## Completed corrections in this pass

- [x] Preserve a failing native comparison before changing the engine: the first
  fists timing mismatch occurs at age 13; initial crash entry also discards a
  source backward impulse when initial speed is zero.
- [x] Add per-tick native animation probes for both sides, normal/backhand tables
  and all four weapon slots: 458 active-hit timing observations.
- [x] Correct the source hit gate to use retained descriptor flags OR the frame
  counter after animation advancement. The old selected-frame fallback could
  start and finish the hit window one tick late.
- [x] Add 24 native first/repeated crash-entry observations covering reasons
  1/2/4/5, speeds 0/5,000/20,000, body/bike launch heights, signed velocities,
  selected crash animation and repeated-entry return values.
- [x] Preserve signed source entry impulses, including backward throws at zero
  initial speed, instead of clamping the rider body to forward motion.
- [x] Keep source crash entry idempotent while the linked crash is active: retain
  body identity, position, velocities, phase/elapsed time and incident number;
  emit no duplicate crash event from repeated entry.
- [x] Record `combat_version: 1` in new native races. Absent/version-0 setups
  retain historical attack timing and recovery-entry behavior; unknown versions
  fail validation. Old recordings must retain their canonical hashes.
- [x] Test actual simulation hits on the first active tick and misses before it,
  source/legacy window endpoints, repeated crash events and rule validation.

[Execution receipt](evidence/collision-combat-priority.json) records results and
scope. These corrections do not close the remaining items below.

## Current contact/recovery/fight follow-up

[Detailed report and aggregate receipt](evidence/source-contact-recovery.md):
**327,086 observations across 22 families match.** Broad multi-actor and full-game
acceptance below stays open unless its complete coverage passes.

- [x] Source classifier, original-width separation, masses, elasticity and ordered
  bike/bike, bike/car and bike/static responses: 11,840 observations.
- [x] Read and independently compare 1,663 physical RRAN frame-box records; use
  source frame-two scenery colliders independently of sprite/display dimensions.
- [x] Compare 20,736 bike and 25,920 detached type-0 building boundary cases,
  including selectors, emergence, speed/depth and street openings.
- [x] Use a separate detached mass/box/handler: 10,368 response cases and 1,728
  combined bike/body transactions, including linked own-quiet-bike behavior.
- [x] Preserve changed scalar speed/previous pose between callbacks while keeping
  captured impact velocity separate for later contacts on the same tick.
- [x] Compare 328 complete controlled crash episodes (197,877 ticks), including
  negative starts, slopes, airborne starts, lateral memory and RNG variants.
- [x] Compare recovery-state player/rival stamina regeneration: 18 observations.
- [x] Compare 53,760 ticks of two-rider fights: all supported attack/side/equipment
  variants, held/pulsed commands, targets entering/leaving/crossing reach and
  both outcomes of the native clip-chain RNG branch.
- [x] Correct regeneration order, retain queued animation/clock through chains,
  preserve the initial return queue and reject stale descriptors on fresh windup.
- [x] Regression-test source contact/recovery replays at 30/60/144/irregular
  schedules with effects enabled/disabled, remount identity and disconnected roads.

## Verification follow-up

- [x] Rerun the 625 existing native observations and the focused gameplay tests.
- [x] Compare 210 combined stamina/bike-condition damage cases, including nearly
  depleted stamina/condition and every supported reason code; all match.
- [x] Compare nine isolated zero-drag longitudinal recovery observations. Three
  positive-speed controls match, while six backward-motion observations diverge
  from the first tick. Preserve both the failure and the invalid initial damage
  probe as separate evidence; do not confuse a harness mistake with engine behavior.
- [x] Correct and version the isolated negative-speed integration failures; all
  nine counterexamples/controls and 1,680 surface/contact drag observations match.
- [x] Compare complete controlled flat/sloped crash-to-remount episodes;
  changing terrain and recovery in moving populations remain open below.

The [original failed verification](evidence/interaction-verification.md) remains
as history. The [new correction report](evidence/gameplay-completion-progress.md)
supersedes its six integration failures. The overall interaction gate stays open.

## Required collision coverage

- [x] Compare the complete native contact classifier, not just an overlap box:
  contained versus glancing contacts, relative longitudinal/lateral/vertical
  motion, object height/class, elasticity, mass and source impulse ordering.
- [x] Compare bike/bike rear-end, side-swipe and crossing contacts at equal speed,
  opposing motion and just below/at/above the native speed-gap boundary. Verify
  which rider is displaced or knocked off and whether bike/stamina damage applies.
- [x] Compare bike/car and bike/static contacts separately. Test stopped/moving
  vehicles, head-on contacts, signs/trees, building corners and street openings.
- [ ] Verify swept contacts at maximum supported speed and lateral motion;
  include a genuine diagonal near miss and crossing an entire narrow obstacle
  in one tick. Never count a swept bounding-box near miss as a hit.
- [ ] Verify jumping over a short obstacle, striking its top/side, airborne
  riders at different heights, hills, shoulders, walls and open road edges.
- [ ] Prove one continuous scrape produces the intended damage/event cadence
  across adjacent panels. Separation and a later re-impact must create a new
  incident; simultaneous unrelated contacts must not erase one another.
- [ ] Run dense traffic/rival cases and split/merge/disconnected-road cases.
  Resolve contact in physical road coordinates and preserve actor ownership.

## Required fighting coverage

- [ ] Compare complete C++ and Rust combat transactions tick by tick, using
  identical controlled positions, native clips, equipment, health and commands.
  The current probes establish timing; they do not establish whole-fight parity.
- [ ] Cover fists, club, chain, kick and backhand on both sides, at strict
  longitudinal/lateral/height boundaries, including a target entering/leaving
  reach during windup. Audit target acquisition at windup versus impact.
- [ ] Compare damage, stamina maximum loss/regeneration, bike-condition damage,
  lean/steer displacement and speed transfer against the native producer.
- [ ] Verify hit immunity and hit-reaction gates immediately before, at and
  after their deadlines; compare whether immunity consumes or permits a later
  hit within the same swing. Check repeated/held attacks and source chaining.
- [ ] Compare interrupted and simultaneous attacks, attacker/target removal,
  mounted versus detached targets, police interactions and differing bikes.
- [ ] Verify weapon theft deadlines, native transition clips, failed theft,
  competing thieves and ownership/drop behavior without duplicate equipment.
- [ ] Capture short native-art fighting sequences with visible contact, reaction,
  damage and knock-off. Correlate frame/tick evidence with the state trace.

## Required crash and recovery coverage

- [x] Compare complete controlled detached rider/bike trajectories: signed
  backward throws, drag, gravity, bounce, fixed slopes, lateral memory and stopping
  distance. Source recovery no longer uses the legacy positive-speed clamp.
- [ ] Extend those comparisons to changing curves/elevation, other boundary types
  and three-or-more-body contact walks throughout crash recovery.
- [ ] Compare low-speed falls, high-speed impacts, depleted stamina, a wrecked
  bike, skid loss and severe landing. Verify reason-specific animation changes.
- [ ] Verify repeated impacts during sliding, getting up, walking and mounting,
  including the final remount tick. Preserve the correct linked body/bike and
  distinguish ignored re-entry from a genuinely new post-remount crash.
- [ ] Measure crash-to-stop, get-up, walk and remount timing against native
  traces. Check bike reach/facing, inaccessible branches, missing/destroyed bikes,
  police arrest and held-control quarantine. Keep explicit assists disabled.
- [ ] Render the full crash-to-remount sequence on the native assets; inspect
  body/bike separation, facing, ground contact, occlusion and animation timing.

## Acceptance before moving on

- [x] Publish aggregate native expected/actual counts and failure hashes; retain
  raw first divergences privately and mark remaining approximations explicitly. Do not close this gate because
  authored tests, internal replay hashes or a screenshot pass.
- [ ] Verify old replay compatibility and new-rule replay agreement at
  30/60/144/irregular schedules, including simultaneous incidents.
- [ ] Repair/revalidate the unattended driver without weakening opponents,
  disabling contacts, injecting results or changing career requirements. Complete
  the full native career and replacement-release checks already reopened.
- [ ] Update Part 19's required defect register and the release gate only after
  these acceptance cases pass. Texture production remains locked behind G5.

## Additional verified corrections

- [x] Compare 252 actual first-hit observations in the native race, including
  both sides, all attack/equipment variants, level scaling and strict boundaries.
  Use combat profile 2 to account for the preceding animation state read by fight.
- [x] Compare 336 direct native strike transactions: immunity boundaries,
  reaction gate, condition/health damage, signed backhand transfer and kick lean.
- [x] Compare 64 native theft transactions, including failed rolls, deadlines,
  actual victim frame, unique ownership and both receiver/victim animation queues.
- [x] Keep new motion/combat/collision profiles explicit and retain legacy omitted
  fields and replay behavior.
- [x] Require active swing/side/reach for population-police melee, and choose
  automatic swing side from nearby road users when no rider target exists.
  This adapter does not establish native police stamina/knock-off parity.
- [x] Regression-test low-speed shallow building-corner lateral escape.
- [x] Compare complete two-rider native clip chains and moving-target reach.
- [ ] Extend to three-or-more-actor retargeting/simultaneous fights and police as
  full rider combatants. Current cop one-hit disable/kinematic policy is unaccepted.
