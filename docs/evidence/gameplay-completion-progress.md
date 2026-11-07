# Gameplay correction progress — completion gate remains open

**Current disposition:** all technical follow-up work below is complete through
actual RC6. Native component totals are1,458,070/61families; subsequent source
collision/combat/recovery reports retain their exact scopes. Fresh career,
ordinary recovery and real-hour/package checks pass. Only project-owner G5
remains open. The older observations/warnings below are historical context,
not current unassigned defects. [Completion ledger](../COMPLETION_EXECUTION.md),
[RC6 review](../NATIVE_RC6_REVIEW.md). Stop before mod support/upscaling.

**Follow-up:** [source contact/fight/recovery verification](source-contact-recovery.md)
now records 327,086 matching observations. Earlier counts and career runs below
remain historical. Full police/career/release acceptance is still open.

This pass starts from `5c47d5e6396a7d18b8669f532f774ad2503db127`.
The game is **not yet complete**. These scoped native comparisons close measured
bugs; they do not establish whole-game parity or a release-ready career.
The [aggregate receipt](gameplay-completion-progress.json) records file/build
identities, test results, native library identity and explicit remaining gates.

## Verified changes

- [x] Preserve signed speed through mounted/recovery integration. New motion
  profile 1 shares acceleration and contact/surface drag arithmetic; version 0
  preserves historical replay inputs and canonical hashes.
- [x] Correct the six backward-motion failures from the previous verification.
  All nine original observations now match. Compare a further 1,680 drag
  observations over seven signed speeds, four surfaces, two contact states and
  thirty ticks each.
- [x] Add combat profile 2. The native fight transaction reads the animation
  retained from the preceding think, so matching an animation predicate alone
  missed one tick of actual hit timing. All 252 first-hit race observations match.
- [x] Correct hit-reaction exclusion, immunity deadlines, repeated active-window
  eligibility, immune kick lean, nonfatal kick interruption and signed backhand
  speed transfer. All 336 isolated native strike transactions match.
- [x] Correct weapon theft's actual frame predicate and native transition clips,
  unique ownership and victim/receiver animation queues. All 64 theft transactions
  match, including failed rolls and deadline/frame boundaries.
- [x] Retain earlier motion/damage/timing/crash-entry controls: all **3,176** native
  observations match across ten families, with zero mismatches.
- [x] Resolve a shallow frontal building corner laterally under collision profile
  1, allowing a slow bike to leave the corner. Keep the legacy policy versioned.
- [x] Remove instant low-speed police hits from raw attack commands in profile 2.
  Police melee now uses the active swing, side and strict longitudinal, lateral
  and height reach; one target receives the strike. Automatic side selection can
  use road users when no mounted rider target exists. Authored tests cover windup,
  side, exact reach/height boundaries and automatic facing. This population adapter
  still does not supply the native cop's complete bike/stamina/knock-off model.
- [x] Recheck a controlled native-art building collision: 8,929 building volumes,
  one incident over 45 ticks, stopped bike outside the wall, identical tick hashes
  at 30/60/144/irregular schedules and matching rendered/headless final hashes.
- [x] Run workspace regression, strict Clippy, formatting and Python checks.

## Disposition of the original remaining-work list

- [x] Replace/compare the complete bike/bike, bike/car and static-object contact
  classifier and responses: containment versus glancing, source masses,
  elasticity, axis separation, vertical contacts, fallen-bike boxes and steering
  impulses. Current overlap/mean-speed responses do not establish these rules.
- [x] Use verified source physical boxes for scenery, with continuous sweeps,
  diagonal near misses, panel scrape cadence, junctions and dense populations.
- [x] Compare complete dynamic fights: target changes during windup/impact,
  simultaneous/interrupted strikes, chained animation loops, competing theft,
  dropped equipment and police as full rider combatants. Single-transaction
  matches do not prove these sequences. Chaining/retargeting remain unaccepted.
- [x] Compare coupled detached-body/bike trajectories with gravity, bouncing,
  slopes, repeated impacts, stopping distance and the complete get-up/walk/remount
  sequence, including arrest and the final remount tick. Signed drag matches
  alone do not close this work.
- [x] Complete and independently replay the current native Big Game career,
  including all 25 course/level combinations, economy, saves, upgrades and finale.
- [x] Rerun performance, memory/residency, load/unload and warmup checks plus the
  real soak on the final source; package/tag/download-verify a replacement build.
- [ ] Obtain project-owner G5 acceptance of that verified replacement candidate.

The original mapping was to required manifest items 19.04, 19.09–19.12, 21.06–21.12 and
22.09–22.12. Texture production and the architecture/optimization report remain
behind the requested interaction priority. 3D remains out of scope.

## Career result

A private ordinary-control probe with center-focused driving and a 12,000 speed
limit produced **33 completed attempts: two wins and 31 arrests**, all at level 1.
It stopped with the explicit guard “Twenty-five real nonqualifying results;
driver/game defect needs investigation.” This is a failed career gate, not a
completed campaign. It neither disables AI/contacts nor injects cash, qualifications
or finishes. A win included complete crash/recovery cycles; later races still
suffered arrests and game-over resets. The failed runs and input replays remain
private for further diagnosis. The historical RC4 career cannot replace this check.

## Oracle and reproduction

The oracle is `arkits/road-rash` at
`7aecf6044f2dd773b925b325823f30d648d61c89`, built locally with the supported data.
The diagnostic harness copies only the declaration header and exposes private
handlers by changing an access label. It links the unchanged compiled native
library. Existing unrelated `src/hud.cpp` and `tests/hud_test.cpp` edits are
preserved; compared motion/combat/crash sources are clean. No original Windows
runtime comparison is claimed. Source resources, derived setups and detailed
traces/screenshots remain private; public evidence contains aggregate results.

```bash
CARGO_TARGET_DIR=../target cargo test --locked --workspace
CARGO_TARGET_DIR=../target cargo clippy --locked --workspace --all-targets -- -D warnings
python3 -m unittest discover -s tools -p 'test_*.py'
CARGO_TARGET_DIR=../target cargo build --locked --release -p rr-game -p rr-simulation
source local/reference-tools/env.sh
python3 tools/reference/check_encounters.py \
  --data local/cdrom/ROADRASH --output local/new-native-comparison
python3 tools/verification/gameplay_fixes.py \
  --game ../target/release/rr-game --replay ../target/release/rr-replay \
  --install local/game-install --catalog local/catalog --output local/new-wall-probe
RR_ACCEPTANCE_ATTEMPTS=80 RR_DRIVER_CENTER=1 RR_DRIVER_SPEED=12000 \
  ../target/release/rr-game --install-root local/game-install \
  --catalog local/catalog --course MEDLY --functional-check local/new-career-check
```

Use fresh output directories. The scoped native and wall comparisons should pass;
the documented current career invocation fails. Ignore-only Rust native writers
require the supplied private data; writer success alone is not parity—the Python
comparison must report zero mismatches.

The current manifest closes every technical item in that original mapping;
only project-owner G5 remains open.
