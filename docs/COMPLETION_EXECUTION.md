# Remaining native-game completion work

Owner instruction, 2026-11-06: complete all remaining required work, using
sequential chat subtasks. Start the next requirement after the current one is
implemented and verified. Keep original assets; enhancement and 3D remain gated.

Owner clarification, 2026-11-07: finish every remaining base-game checkbox
through release acceptance, then stop before mod support, upscaling or other
additional improvements. Parts23–25 remain outside this execution; no production
changes for them are authorized by this completion pass. The owner-specific G5
decision must still be recorded honestly for the concrete delivered candidate.

The current working tree already contains verified driving-bounce, billboard
query/response and native contact-detection corrections. Their evidence remains
valid for its recorded scope and must be preserved during subsequent changes.
No full-game or release acceptance is implied by those component checks.

| Order | Work | State | Required exit evidence |
| --- | --- | --- | --- |
| 1 | Remaining source scenery: BORR, ordering, markings and loaded slices | Verified; evidence/source-scenery.md | Actual native detection/transactions; preserved failures; old replays; all source course/level imports |
| 2 | Collision height, jumps, continuous scrapes and dense/topology interactions | Verified; evidence/source-collision-topology.md | Native boundary/incident matrices and deterministic multi-actor replay; visual contact checks |
| 3 | Simultaneous combat, weapon ownership and complete police behavior | Verified; evidence/source-combat-police.md | Native whole transactions, police stamina/crash/recovery/arrest and contested equipment |
| 4 | Changing-terrain and repeated-impact recovery | Verified; original native comparisons plus corrected ordinary dismount context | Complete trajectories/timing, remount-edge cases and inspected native-art sequences |
| 5 | Genuine current-source career and ordinary player walkthrough | Verified; evidence/native-career-current.md and source-career-ui.md | All 25 course/level cases, ledger/save audit, independently replayed real commands and clean-profile route |
| 6 | Replacement release verification and packaging | Verified; actual RC6 career/package/recovery/finale/preflights/hour | Current immutable build, performance/memory/soak, import/install/download checks and reproducible package receipts |
| 7 | Project-owner release decision | Accepted from subsequent owner chat instruction | Concrete candidate and evidence for owner G5 acceptance; never self-signed |

The detailed acceptance source remains [the interaction checklist](COLLISION_COMBAT_PRIORITY.md),
[the feature manifest](functional-manifest.json), [the defect register](functional-defects.json)
and Parts 19–22 of the README. Completing a bounded subtask does not close its
parent while required cases remain untested or failing. Preserve older failed
and historical release receipts rather than relabeling them as current passes.

The scenery, collision/topology, combat/police, recovery and career subtasks are
verified. The active release subtask uses `release_performance` and
`release_package` in this chat. The parent integrates changes and owns the task ledger, defect register,
overall acceptance and final release validation. The next subtask is created only
after the preceding requirement is verified.

Implementation audit notes for the collision/topology subtask: mounted source
movement currently passes population navigation into `Topology::advance`, whose
fallback prefers the preselected route. Verify actual player steering at split
boundaries against the native producer, including ground/configuration and
camera/cache ownership after changing paths. A graph API or a launch-time branch
mask alone is not evidence that ordinary driving chooses the correct branch.


Scenery verification correction: the initial 290/311-case producer/adapter pair
mixed throttle drive with native steering speed in one column. Those receipts
are retained privately as invalid harness history, not counted as engine
failures or accepted parity. Fresh corrected before/fixed runs now cover 638 observations, including multi-tick
marking state and camera/cache behavior: 527 baseline mismatches and zero after correction.

The collision/topology subtask must also compare the complete mixed object walk:
the native outer object meets its scenery before later sorted rider/body pairs,
whereas the present Rust contact pass resolves all rider pairs before scenery.
Scenery-internal BORR/billboard ordering alone does not prove that mixed sequence.
Measure sorted-list early exits, callback-updated poses and captured velocities,
and preserve historical recordings when correcting the ordering.

Verify coordinate conventions at nonzero dual-road offsets in that same subtask:
source handling ground edges include `RoutePoint.offset`, while the presentation
view treats lateral values relative to a road center. Compare actual native/Rust
rider and traffic poses at branch offsets; do not infer correctness from straight
road tests where the offset is zero. Correct adapters without changing legacy
replay authority merely to align a picture.

When integrating police as full rider combatants, inherit the chase owner's
native road half at spawn, including shared sections after a merge. A shared
road-mask value alone does not reconstruct the owner's retained half or its
ring-cache flags. Keep bike/body identities and cache cleanup bounded through
spawn, crashes, recovery, arrest and despawn.


Scenery confirmation: the corrected 638-case actual-native trace passes, against
527 failures under preserved version-0 behavior. Together with the extended
existing native suite this is 329,902 observations across 26 families. The
independent 130-case identity inventory reports no conflicting identities and
explicit current-cache exclusions. The final 25-case factory/replay confirmation
completed with exit code 0; an earlier background shell returned SIGTERM after
its test printed pass and is retained separately, not used as the command receipt.
Global mixed callbacks, steered topology and full moving-body camera episodes
remain assigned to the subsequent collision/recovery subtasks.

Collision/topology integration passed on the current build: 444 workspace tests,
36 Python tests, strict workspace Clippy and formatting; all 25 source factories
at four schedules; ten ordinary steered approaches and independent release
replays; two inspected actual Bevy branch captures; and the corrected native
continuous-facade brush at four schedules plus graphical/headless agreement.
The original brush fixture hits a building corner on tick42 in both native and
Rust; it remains preserved as failed fixture history. No physics was weakened.
Private final integration receipt: `local/collision-topology-integration/integration.json`.
Current aggregate: 568,606 matching native observations across 38 families.
Full police/fights, complete detached recovery, career and release remain open.


Combat/police integration is verified: 517,560 additional deduplicated native
observations across 15 families, giving **1,086,166 across 53 families** overall.
The simultaneous kernel contributes64,512, police integration360,056, cop AI92,502,
cop assets90 and mixed traffic400. LIVE/NONLIVE shared constructor rows and the
coupled recovery prefix count once; actor-ticks are not added to field ticks.
All486 workspace tests pass (47 explicit private-data skips), along with36 Python
tests, strict workspace Clippy, formatting and diff checks. All25 source factories
pass four replay schedules with combat4/recovery2. Fourteen source-art attacks
and a natural900-tick cop crash/get-up/walk/mount/remount replay pass four rates;
actual release game-window replays agree on the final hashes. Cop foot art uses
an explicit original HIBOB palette6 adaptation because LOCOP has no upright foot
sprites; no native uniform/raster identity is claimed. Private integration receipt:
`local/combat-police-integration.json`. Changing-terrain/repeated-impact breadth,
full career and replacement release remain open. Stage4 starts after these checks.


Stage5 carried-condition audit is resolved: the frontend session explicitly uses
the original bike specification for condition capacity, including when the player
uses derived specification16. Fifteen actual-data course/bike combinations retain
50% carried condition and report 50% in the real quit summary. No normalization
defect was found. Private receipt: `local/career-carried-condition-audit.json`.


Recovery integration is verified. Stage4 contributes **371,904 deduplicated native
observations across 8 families**: terrain 216,718 (400uniqueepisodes), camera 69,120,
rail window 2,520, recovery contacts 79,610 (three families), entry 576 and manual
walk 3,360. Zero-walk controls, duplicate centers, actor-ticks and superseded pilots
are excluded. Overall component total: **1,458,070 across 61 families**.
Published legacy replay checks pass. The full workspace passes 506 tests/56 explicit
skips, with an additional new linked-bike safety regression passing separately
(507 total current tests). Strict workspace Clippy, targeted final test/helper
Clippy, formatting, 36 Python tests and diff checks pass. All 25 current factories
pass four schedules. Original-art player recovery passes all five courses, 14 attacks
and a full cop cycle remain green, and a genuine parked-car wreck completes terminal
pullback; all four schedules and four actual game-window replays agree. Palette
harness fallback was corrected without changing authority. Private integration:
`local/recovery-integration.json`. Stage5 begins after these verified gates.


Stage5 is verified: 31 genuine qualifying finishes cover all 25 course/level cases,
with 124 independent four-schedule replays checking every checkpoint, financial
transition and saved checksum. Final career: completed, $12,797, bike 8, condition
98%; five purchases, three paid repairs and six recovered crashes. The unchanged
final save also opens through ordinary keyboard navigation. The actual UI route
covers real driving/quit/results, neutral resume, quoted/denied purchase, save/load
and preference persistence. New source checks pass 508 tests/57 explicit skips,
36 root Python tests, 16 verifier tests, strict workspace Clippy, formatting and
diff checks. Evidence: `evidence/native-career-current.md` and
`evidence/source-career-ui.md`. Replacement release stage6 now begins; no old RC4
receipt or owner signoff is relabeled as current.


Stage6 preflight found and corrected orphaned font atlas retention: the actual
ten-cycle UI peaks at15 images against the unchanged16 limit, with exact Main
pixels and stable512 index slots. A full320-second City baseline genuinely missed
the wall-frame target; the preserved failure is not relabeled. Userspace profiling
identified polygon and texture-coordinate work plus repeated planner forecasts.
Bounded clipping buffers and exact texel indexing retain all75 native view
RGBA/depth/receipt hashes; at least2.2million original-formula index comparisons
pass. Driver caching preserves every one of536,874 recorded commands and31 final
hashes. The combined default-policy30-second probe meets its short-scope target
(p95 wall15.677ms), while proper long-run and replacement artifact gates remain
open. All timings identify their own binaries; no short probe substitutes for the
required five-minute or hour checks.


RC5 final ordinary-control gate found a real blocker after the completed hour:
CITY level5 stationary brake→V exits101 at encounter.rs1358 because the LIVE body
is created after per-tick road preparation but uses its independent body cache
in that same movement pass. Physical impact bodies normally start later, so the
existing collision traces did not cover this input-timing boundary. The narrow
cache-initialization correction and regression are assigned to
`fix_manual_dismount`. Earlier 31-win career, actual hour and package results
remain valid for their scoped RC5 bytes; they do not accept the broken dismount.
A new candidate and renewed release checks follow the fix. G5 stays unsigned.


The narrow source fix now passes two meaningful authored regressions: both
movement dispatch modes prepare the independent route body context, and two
complete schema6 dismount/remount episodes release and recreate it correctly.
All514 workspace tests pass (58 explicit ignores), strict Clippy/formatting and
39+16 Python checks pass. The actual local rebuilt game also visibly completes
V→fallen rider/bike→get-up→remount→driving at level1; repeated and level5 ordinary
checks are in progress before freezing the replacement. No physical law, RNG,
V eligibility or legacy profile changed. [Fix receipt](evidence/source-manual-dismount.md).


RC6 is published from frozen source d9dac51433d279c5f6a92a266f2db9fe4a665d42.
All six exact-source/request/candidate CI jobs pass. The actual downloaded a26b
application passes repeated ordinary CITYL1/L5 voluntary recovery and eight new
replay checks. Its fresh 31-win/25-case career matches the newly audited source
bytes; 124 actual source replay executions are reused by strict byte/dependency/
341-file identity, with zero additional executions claimed. Actual full graphical
finale hash matches. All712 package entries,369 notices, source-hidden first
import and profile/data retention pass. Local clean rebuild/all5binary equality
and normalized duplicate archives pass. Final delivered heap/startup/lifecycle/
audio/hour checks follow in serialized slots; owner G5 remains unsigned.
[Current RC6 review packet](NATIVE_RC6_REVIEW.md).


Final RC6 technical release verification is complete. Its actual downloaded game
exits zero after3,600.329 real seconds/3,598.517 simulated seconds, zero paused
frames, all five complete warm-course timing gates, bounded memory/images and
701 strict owned-audio observations. Both laps, temperatures/power, original
failed probes and rejected RC5 remain retained. Only owned null-sink/display
resources were removed; audio defaults match the original selections after
policy settling without restoration commands. All47 technical manifest parents
and144 children pass. Only owner G5 remains pending, never self-signed.
[Final RC6 receipt](evidence/native-performance-rc6.md). This pass stops before
mod support, upscaling and all Part23–25 enhancements.


Final owner acceptance is recorded in [owner-acceptance-rc6.md](evidence/owner-acceptance-rc6.md).
The owner instructed completion of the last base-game checkbox after RC6/G5 were
identified. All48 base-game tasks and144children are complete. No separate human
playtest is asserted. Stop before mod support, upscaling and Parts23–25 improvements.
Earlier pending/unsigned entries above are chronological history.
