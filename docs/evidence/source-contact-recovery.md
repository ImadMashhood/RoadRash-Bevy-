# Source contacts, fight chains and complete controlled recovery

**Current disposition:** all technical follow-up work below is complete through
actual RC6. Native component totals are1,458,070/61families; subsequent source
collision/combat/recovery reports retain their exact scopes. Fresh career,
ordinary recovery and real-hour/package checks pass. Only project-owner G5
remains open. The older observations/warnings below are historical context,
not current unassigned defects. [Completion ledger](../COMPLETION_EXECUTION.md),
[RC6 review](../NATIVE_RC6_REVIEW.md). Stop before mod support/upscaling.

**327,086 observations match the pinned native C++ library across 22 families.**
The game is still **not complete**. These measured corrections replace the older
overlap/mean-speed contacts and approximate recovery. Full police behavior,
dense/simultaneous interactions and renewed campaign/release acceptance remain open.
[Aggregate receipt](source-contact-recovery.json),
[checked and remaining work](../COLLISION_COMBAT_PRIORITY.md).

## What changed and why

- [x] Source contact profile 2 uses previous-axis separation, containment/glancing,
  mass, elasticity, original-width arithmetic and ordered callbacks. Bike/bike,
  bike/car and bike/static responses keep their distinct damage/knock-off rules.
- [x] Detached riders have their own source mass, dimensions and response handler.
  Existing body proxies join the contact scan; newly created crash bodies join on
  the next tick. Own quiet bikes avoid invented self-damage/contact events.
- [x] Preserve callback-updated scalar speed/previous position while retaining the
  captured impact velocity for later contacts. Body/body second-callback failures
  exposed why these are separate state fields.
- [x] Physical scenery boxes come from source RRAN frame-two metadata. Image size
  and display-building panel size no longer manufacture physical barriers.
- [x] Type-0 building walls use continuous road boundaries, selector emergence
  and junction flags. Side brushes preserve forward motion as the native handler
  does. Both bikes and detached bodies have independently compared responses.
- [x] Recovery profile 1 advances crash animation before motion and walking/pickup
  afterward. It preserves signed motion, separate bike/body integrators, airborne
  lateral memory, negative landing rebound, source phase timer, pickup offset and
  mounting animation. Explicit fast/rescue assists retain their separate policy.
- [x] Fix `-1` meaning no queued animation, rather than resolving it as bank zero.
  This previously reset crash loops under some RNG tapes.
- [x] Fight profile 3 keeps the animation clock and queued repeat through complete
  clip loops. It reads the retained hit gate before animation advancement, avoids
  stale-descriptor hits on a fresh windup, and regenerates stamina in actor order.
  Population melee consumes the same active gate, including chained loops.
- [x] Source get-up/walk/mount regenerates two stamina units per tick, plus one
  for the player wrapper; sliding regenerates none. Native comparisons include
  zero stamina and maximum clamping.
- [x] Arrest eligibility reads the linked bike's speed, maps native crash states,
  and removes an invented lateral-distance gate. This is a scoped correction;
  police remain population agents with incomplete native rider behavior.
- [x] Contact arithmetic, world adapters, fight transactions and source recovery
  live in separate modules. Rendering owns none of their outcomes. Motion 2,
  collision 2, combat 3 and recovery 1 are explicit in new imported races;
  absent legacy fields and old replay policies remain supported. Combat 0–2
  are frozen in a compatibility module. A public authored golden and a native-art
  full-field recording match a separately built published `dba66c6` executable.
- [x] The acceptance pilot uses physical scenery/boundaries for avoidance instead
  of obsolete display-panel boxes. It issues ordinary controls only.

## Native comparison coverage

| Family | Compared observations |
|---|---:|
| Motion, damage, timing, crash entry and damage transactions | 835 |
| Signed recovery motion and surface/contact drag | 1,689 |
| First fight impacts, isolated strikes and weapon theft | 652 |
| Two-rider dynamic fights and animation chains | 53,760 |
| Contact separation | 576 |
| Bike/bike, bike/car and bike/static transactions | 11,264 |
| Source scenery collision-box records | 1,663 |
| Bike type-0 building boundaries | 20,736 |
| Complete coupled recovery trajectories | 197,877 |
| Crash-state stamina regeneration | 18 |
| Detached response handlers | 10,368 |
| Combined bike/body and linked-own-bike transactions | 1,728 |
| Detached type-0 building boundaries | 25,920 |

Recovery coverage is **328 complete episodes**, with backward starts, three fixed
slope values, airborne starts, lateral memory and four controlled RNG tapes. Every
episode reaches remount. This does not compare arbitrary changing terrain or the
native global AI/audio RNG stream. Fight coverage is **336 sequences of 160 ticks**:
both sides, normal/backhand variants, all four weapon slots where supported,
single/held/pulsed commands, reach entry/exit/side crossing and chain/no-chain RNG.
These are controlled two-rider transactions, not a complete multi-actor race oracle.

The C++ oracle is `arkits/road-rash` at
`7aecf6044f2dd773b925b325823f30d648d61c89`. The harness links its unchanged native
library. A private declaration-header access shim exposes diagnostic handlers;
it does not patch implementation code or object layout. Existing unrelated
`src/hud.cpp` and `tests/hud_test.cpp` changes are preserved. Commercial files,
setups, raw traces and captures stay private. Original Windows-runtime comparison
remains owner-waived.

## Failure history retained

The aggregate receipt preserves hashes/counts from failed probes. In particular:

- Earlier coupled recovery failed before signed rebound, animation queue and
  phase-order fixes. One initial previous-rise mismatch was a fixture error;
  correcting that input is recorded separately from engine fixes.
- The first valid combined body/body probe had 40 mismatching observations because
  the second callback read stale scalar speed. Captured velocity must remain fixed.
- The first fight sequence used an incorrectly ordered C++ stats aggregate. That
  fixture failure is retained separately. The valid probe then found regeneration
  order and a queued-return discrepancy. Expanded chain RNG found clock drift and
  a stale hit descriptor on the first tick of a new swing. All are now compared.

## Runtime checks and limits

Public authored regressions verify source-profile recovery/remount identity, own
quiet bike behavior, chained animation continuity, disconnected-road exclusion,
legacy serialization and contact/recovery replay hashes at 30/60/144/irregular
schedules with effects enabled/disabled. The private native-art wall check also
requires rendered/headless hash equality and a moving, undamaged side brush.
A fresh City capture was visually inspected: building sides, riders and interface
are visible. That screenshot alone does not establish gameplay accuracy.

The earlier rebuilt binary's **100-attempt career probe failed**: 28 wins, 69
arrests, two losses and one wreck. It reached all five courses at level 1, ending
with four qualifications; it did not complete the campaign. Its exact binary
identity is retained. It cannot certify later changes or replace career acceptance.
The subsequent combat-profile-3 pilot completed level 1 but failed its **200-attempt**
budget: 72 wins, 101 arrests, 23 losses and four wrecks, ending at level 2 with one
qualification. Seven course/level cases were visited. That failed receipt is also
preserved; a completed and independently replayed career is still required.

Original follow-up list, updated with current disposition:

- [x] Complete source police bike/stamina/combat/recovery; remove the one-hit
  population disable approximation through a validated shared rider model.
- [x] Dense three-or-more-body contact walks, simultaneous fights/retargeting,
  competing theft and repeated impacts during recovery/remount.
- [x] Other boundary types, changing hills/curves, high-speed sweeps and diagonal
  near misses; coupled wall/vertical ordering remains unaccepted.
- [x] Render and inspect complete native-art fights and crash-to-remount sequences.
- [x] Finish all 25 career course/level cases and independently replay/audit the
  successful career, then rerun performance/memory/full soak/package verification.
- [ ] Project-owner G5 acceptance. Texture enhancement and 3D remain deferred.

## Reproduce with a private verified import

```sh
CARGO_TARGET_DIR=../target cargo test --locked --workspace
CARGO_TARGET_DIR=../target cargo clippy --locked --workspace --all-targets -- -D warnings
python3 -m unittest discover -s tools -p 'test_*.py'
CARGO_TARGET_DIR=../target cargo build --locked --release -p rr-game -p rr-simulation
source local/reference-tools/env.sh
python3 tools/reference/check_encounters.py --data local/cdrom/ROADRASH \
  --output local/NEW-contact-native
python3 tools/verification/gameplay_fixes.py --game ../target/release/rr-game \
  --replay ../target/release/rr-replay --install local/game-install \
  --catalog local/catalog --output local/NEW-contact-wall
RR_ACCEPTANCE_ATTEMPTS=200 RR_DRIVER_CENTER=1 RR_DRIVER_SPEED=22000 \
  ../target/release/rr-game --install-root local/game-install \
  --catalog local/catalog --course MEDLY --functional-check local/NEW-career
```

Use fresh output directories. An ignored writer passing is not parity: the Python
comparison must report zero mismatches. A bounded failed career run stays failed.

Implementation:
[collision arithmetic](../../crates/simulation/src/collision.rs),
[world contact adapters](../../crates/simulation/src/encounter/contact.rs),
[fight transactions](../../crates/simulation/src/encounter/fighting.rs),
[source recovery](../../crates/simulation/src/recovery_source.rs),
[building boundaries](../../crates/simulation/src/building_wall.rs),
[RRAN boxes](../../crates/formats/src/animation_boxes.rs),
[public regressions](../../crates/simulation/tests/collision.rs),
[native harness](../../tools/reference/check_encounters.py).
