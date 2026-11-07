# Shared rider combat and police integration

The native combat/police simulation gates below pass. Final stage closure also requires the factory graphics and population integration checks in the completion ledger. The source profile uses combat version 4; profiles 0–3 retain their previous behavior. Police configuration is optional and omitted from legacy serialization. A source police actor is a full shared rider, with its own selected motorcycle, stamina, equipment, combat, contact, crash and recovery state. The population entry mirrors that actor and does not add a second colliding or rendered cop.

## Reference and scope

All native comparisons use revision `7aecf6044f2dd773b925b325823f30d648d61c89`, library SHA256 `f530ae9ce4ce031cea19ce022fde138f2a3fd77a52d01fc82cdce148e472fb51`, and supported commercial EXE SHA256 `73d7f9744b5b1a9482b3909950cdf3fe34119fe13d023b031e9c6739a60f18a2`. The pre-existing HUD edits are outside the compared functions. Commercial traces remain under ignored `local/`. Original Windows runtime comparison is waived and was not performed.

Fixed 60 Hz host decisions and explicit `dt=1` actor composition are retained policies. The coupled oracle uses actual LIVE half/bike callbacks; it does not establish parity with the native variable cadence scheduler. Multi-node split/merge routing uses the functional physical graph bridge, not a complete native TrackRing reproduction.

## Verified evidence

| Family | Observations | Receipt / scope |
| --- | ---: | --- |
| Raw police configuration | 1 | All decoded constructor, personality, steering and level tables |
| Pursuit personality setup | 800 | Actual source constructor formula across levels, ranks and personalities |
| Police rider construction | 135 | Levels, lanes, positions and elevations |
| Mounted cop controls and motion | 13,500 | Levels, steering, speeds, throttle, braking, brake-to-throttle and coasting |
| Complete detached cop episodes, NONLIVE branch | 76,865 | 70 controlled episodes, including wreck repair and distant walking; historical combined callback scope |
| Complete detached cop episodes, LIVE branch | 92,535 | 70 actual half/bike callback episodes; every episode remounts, including wreck repair and distant walking |
| Cop scan and arrest predicates | 95,040 | Actual `find_cop` and `police_think`, crash phases, strict boundaries, flags, half residency and intervening riders |
| Coupled LIVE AI, combat, animation, motion and contact | 81,040 | 224 episodes; eight continue through cop depletion, recovery, remount and 30 further ticks; all retain a free shared RNG |
| Active player LIVE arrest timing | 140 | Mounted staged frame-2 backhand, sliding, walking, mounting, entry-to-mount and remount speed/range boundaries |

The first five rows share `local/police-riders-control-transitions-fixed/comparison.json`: **91,301 observations, zero differences**, native trace SHA256 `ccc342ca898da86b0ba6c1bb5f36bf7cf50711b202e32deb8f79d8ca93c8d452`. This supersedes the smaller 84,551-row constructor/motion/NONLIVE recovery receipt; do not add both. The additional control transition matrix first exposed 714 differences with this same native trace. The correction scales the raw braking steering term before shifting it, as the source does, in the new cop specification path.

The scan/arrest receipt is `local/police-arrest-final/comparison.json`, native trace SHA256 `5b154a86883a0bf509d9a9209f27fb6a2e128021d5d2dfb8d9148cae5a6eac45`.

The coupled LIVE receipt is `local/police-pipeline-live-final/comparison.json`, native trace SHA256 `31634c3616d46007895b325c73ced8d55f905df68b3c6088f9e27e77d5d1081e`. Six episodes end with an NPC crash and eight with a cop crash. Sixteen episodes set an NPC hit deadline, 46 set a cop hit deadline, and four perform a theft. These are episode counts, not additional native observations. The trace compares both riders' complete controlled motion, stamina, equipment, hit/theft deadlines, retained animation clocks, cop target/state/replan fields and the unanchored shared RNG.

The full coupled recovery receipt is `local/police-pipeline-live-recovery-final/comparison.json`: 3,472 ticks over eight naturally depleted cops, native trace SHA256 `2c1b159e3dea82bf0790800c2ba3a1e736b44f112b15a9fc86a5f7e47e01e9e9`. NPC held attacks stop after the first cop crash; ongoing source clips continue normally. The cop runs its actual AI throughout. All eight episodes recover, remount and run 30 additional ticks with zero differences. These eight replace 1,490 first-crash-prefix ticks in the 79,058-row receipt, giving **81,040 deduplicated coupled observations**, not 82,530. The initial same-trace recovery baseline had 1,738 differences. Correcting cop recovery regeneration ownership reduced that to 120; retaining the source throttle accumulator across remount removed the remaining differences. The native remount resets drive and steering but preserves the underlying force accumulator. Old recovery version 1 remains unchanged.

The LIVE recovery receipt is `local/police-riders-live-final/comparison.json`, native trace SHA256 `ddbf1ce435dde1bcb18149f9e50559266f2cc94fd31fa70370fd7788753fd64f`. Its 106,971 total rows include the same 14,436 constructor/control rows as the NONLIVE receipt; count those common components once. All 70 LIVE episodes reach remount; none terminate at the configured tick limit.

The active player receipt is `local/police-active-arrest-final/comparison.json`, native trace SHA256 `b92dbafd4a4cf4d6c5680484382ee74943d352a7d7ccc9f8ae542be64b252aaa`. It supersedes the earlier 40 mounted controls. The matrix includes negative speed supplied after validated actor construction, speeds 0/299/300, cop gaps −512/−511/0/511/512, passive or active backhand, and five detached transition setups. Walking-body motion can cross the arrest boundary after the scan; entering mount changes the speed gate before the scan. These validate prepare-recovery → arrest → crash-phases/body-motion ordering. The first two mounted-only attempts failed in test setup because negative speeds were placed in a prohibited spawn seed; they are invalid harness attempts, not engine mismatches.

## Corrections and preserved counterexamples

Source profile 4 runs each rider's fight and controls before that rider's animation and motion. Cop attack-state continuation runs after the shared hit transaction, so its second steering update, replan and regeneration observe the hit. Theft updates the police brain's retained attack weapon together with carried weapon ownership. Cop birth initializes its brain before earlier actors move; source lane placement is computed after the native lane draw. Its body inherits the chased player's actual body road context when detached.

Earlier coupled experiments used the NONLIVE native combined callback. Their immediate/deferred reaction differences remain distinct branch counterexamples and are not relabeled fixed engine defects. The first LIVE experiment also omitted level-4 immunity and theft-code inputs in the Rust fixture. The corrected inputs and identical native hash must be used for outcome claims. After immunity matching, 914 differences remained; after splitting AI continuation, 198 remained; the correctly configured theft state exposed and verified the retained police attack-weapon correction. Missing fixture tables are recorded as harness issues.

The public stale-mounted-reaction regression confirms that a landed detached body uses its own clip for the immunity gate even when the quiet mounted animation still says reaction 9. Its initial fixture attempts incorrectly selected frame 2 from a two-frame authored clip and then left the newly fallen body at height 19,200, correctly outside melee reach. The corrected fixture selects the flagged descriptor at index 1 and explicitly represents the body after landing. These failures were harness assumptions; no extra runtime correction was made for them.

An independent brake-force correction applies the source throttle ramp when a negative braking force transitions to throttle. Legacy wrappers preserve their existing behavior, including the approved brake-priority policy when both input buttons are pressed. Public actions, encounters and riding tests pass (84 tests).

## Related evidence and pending closure

See [simultaneous combat](source-simultaneous-combat.md), [police AI](source-police-ai.md), [police presentation](source-police-presentation.md), and [mixed traffic ordering](source-mixed-traffic-order.md) for independently bounded families. Their observation totals must be aggregated once by the completion ledger.

LIVE detached recovery uses a different callback order from the historical NONLIVE episode family. The full coupled AI/fight/recovery trajectory now passes. Final integrated population/graphics checks are recorded by the parent completion ledger before closing this stage. Changing-terrain and repeated-impact recovery breadth remains stage 4. The graph cache uses an explicit bounded physical road window rather than claiming native cache flag parity; its multi-node lifecycle regressions are documented with the police AI evidence.

## Deduplicated observation accounting

This report contributes **360,056 observations across nine families**: configuration 1, pursuit 800, construction 135, mounted motion 13,500, NONLIVE recovery 76,865, LIVE recovery 92,535, scan/arrest 95,040, coupled LIVE transactions 81,040, and active player arrest 140. NONLIVE recovery remains an explicitly separate compatibility branch. The 14,436 configuration/construction/motion observations shared by the two rider receipts count once. The 40-case mounted arrest pilot and smaller constructor/motion matrices are superseded. Related combat, AI, mixed-contact and raster evidence has separate family accounting.
