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

## Required collision coverage

- [ ] Compare the complete native contact classifier, not just an overlap box:
  contained versus glancing contacts, relative longitudinal/lateral/vertical
  motion, object height/class, elasticity, mass and source impulse ordering.
- [ ] Compare bike/bike rear-end, side-swipe and crossing contacts at equal speed,
  opposing motion and just below/at/above the native speed-gap boundary. Verify
  which rider is displaced or knocked off and whether bike/stamina damage applies.
- [ ] Compare bike/car and bike/static contacts separately. Test stopped/moving
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

- [ ] Compare multi-tick detached rider and bike trajectories, not just entry:
  signed motion after a backward throw, surface drag, gravity, bounce, slopes,
  lateral impulses and stopping distance. The present recovery integrator still
  contains modern policies, including subsequent speed clamping.
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

- [ ] Publish native expected/actual cases and first divergences; retain failures
  and mark remaining approximations explicitly. Do not close this gate because
  authored tests, internal replay hashes or a screenshot pass.
- [ ] Verify old replay compatibility and new-rule replay agreement at
  30/60/144/irregular schedules, including simultaneous incidents.
- [ ] Repair/revalidate the unattended driver without weakening opponents,
  disabling contacts, injecting results or changing career requirements. Complete
  the full native career and replacement-release checks already reopened.
- [ ] Update Part 19's required defect register and the release gate only after
  these acceptance cases pass. Texture production remains locked behind G5.
