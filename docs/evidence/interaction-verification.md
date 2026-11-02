# Interaction verification: overall accuracy check fails

Verification of source revision `5a9c53b8f6c702310b8a912165453be3ae70bf08`
confirms a remaining crash-motion defect. **838 of 844 native observations
match; six diverge.** The [machine-readable report](interaction-verification.json)
records results, the first counterexample, executable identity and scope.
No production engine behavior was changed during this verification pass.

| Verification family | Match | Diverge | Scope |
| --- | ---: | ---: | --- |
| Generic actor motion | 108 | 0 | Existing paired motion primitives |
| Reason-scaled stamina | 35 | 0 | Existing reason table |
| Native attack hit timing | 458 | 0 | Both sides, both tables, all weapon slots |
| Initial/repeated crash entry | 24 | 0 | Heights, velocities, animation and entry guard |
| Combined stamina/bike damage | 210 | 0 | Seven reasons, five amounts, three initial bike conditions, two health levels |
| Isolated recovery motion | 3 | 6 | Three ticks each at speeds -256, -80 and +5000; dt=1, heading=0, zero drag |

## Confirmed defect

The source crash-entry correction preserves a negative rider launch, but
`recovery::motion_step` immediately computes `motion.speed.max(0)` and clamps
again after acceleration. Starting at progress 1000 with speed -256, the native
actor integrator reaches progress **999**, retaining speed **-256**, on tick 1.
Rust stays at **1000** and sets speed to **0**. Both negative-speed cases diverge
on all three observations. All positive-speed controls match.

This isolates the signed horizontal integration boundary. It does not establish
complete source drag, gravity, bounce, terrain or recovery-state parity. The
required `reverse-recovery-motion` defect is open; complete trajectory and
crash-to-remount checks remain open in the
[interaction priority](../COLLISION_COMBAT_PRIORITY.md).

## Checks that still pass

The fresh native wall probe imports 8,929 building volumes, produces one contact
incident, stops the bike outside the wall, and agrees across all 45 tick hashes
at four presentation schedules. Its rendered/headless final hashes match.
This validates that controlled scenario, not the complete original collision
classifier. Focused actions/encounters/population tests pass **103/103**; Python
verification-tool tests pass **36/36**. Simulation Clippy and formatting pass.
The earlier 364-test workspace receipt is historical; it is not relabeled as a
new full-workspace run.

The comparison tool now writes all passing/failing families and first
divergences before exiting nonzero. Missing or extra trace rows also fail;
three Python tests cover the comparator contract. Detailed native/source
traces and screenshots remain private. Original Windows-runtime comparison
remains waived; the oracle is the pinned C++ port
`7aecf6044f2dd773b925b325823f30d648d61c89`. The reference working tree has existing local changes to `src/hud.cpp` and `tests/hud_test.cpp`; these were preserved. Compared collision/combat/animation sources are clean, and the receipt records the actual native library hash and dirty paths. The harness rejects changes to other reference production sources.

## Reproduce

```bash
cargo build --locked --release --target-dir ../target \
  -p rr-simulation --bin rr-encounter-check
source local/reference-tools/env.sh
python3 tools/reference/check_encounters.py \
  --data local/cdrom/ROADRASH --output local/new-expanded-verification
```

**Expected current result: exit 1**, with `comparison.json` reporting six
`recovery_motion` mismatches and the other five families passing. The test
fixture uses explicit named C++ stamina assignments: an earlier new probe used
an incorrect aggregate field order and was excluded, not reported as an engine
damage failure. Its raw result is retained privately for provenance.

Full contact classification, whole-fight behavior, complete recovery trajectories,
current career and release acceptance remain open. Passing internal regression
or replay checks cannot override the measured native discrepancy.
