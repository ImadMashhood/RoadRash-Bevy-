# Source scenery queries, markings and loading

**638 native scenery observations match, with 527 mismatches preserved in the
version-0 baseline against the identical native trace.** This closes the bounded
scenery import/query/loading follow-up. Global rider/static callback interleaving,
steered branch selection and full-race acceptance remain subsequent work.
Aggregate results are retained in [the receipt](source-scenery.json).

New source races use explicit `scenery_version: 1` metadata. They import physical
BORR slots and billboard classes 0–15 independently of display-sprite availability.
Source frame-two boxes and indexed texture flags are retained independently of
pixel aliases. Actual native resource reads match all 1,663 frame records; an
additional 54 actual native BORR placement observations match on the five source
courses with the approved isolated seed-1 placement stream. This does not claim
race-wide native random-stream interleaving.

BORR queries precede billboard queries and stop at the first overlap. A one-box
BORR slot at absolute lateral distance at least 1,024 source units widens its
frame-relative lateral bounds threefold unless its active native road-half cache
word contains `0x4000`. Both detection and response use the widened rectangle.
The cache has two 128-entry arrays indexed by global sample; texture absence
preserves the previous word. Road-half history survives shared sections; a fresh
detached body inherits its current bike's road half, and remounting removes the
old body's history. These fields are authoritative serialized state.

Billboard classes 3–15 run marking callbacks without a solid contact event.
Speed 1,000/1,001, classes with and without timer refresh, repeated same-sample
queries, fallen bikes, clock ages 0–16, sample departure and bike-contact clearing
are compared with the actual native handlers. Flash expiry retains the last
sample, including the native display-copy lag. Grip uses the native decaying
timer. The probe explicitly distinguishes native `0xf0` lateral speed,
`0x114` drive and `0x10c` captured impact velocity: marking clears lateral speed,
not drive.

Loading applies the preceding frame's preparation. Mounted follow position is
captured after movement and before contacts; detached follow position uses the
postcontact body. New game factories explicitly seed completed countdown
visibility in their replay setup. Native comparison covers countdown, front/back
thresholds, modulo-128 cache aliasing across sections, shared-road cache history,
12 actual moving `Race.step` ticks with an attached straight `TrackRing`, and
241 controlled camera frames travelling from sample 50 to 170 and back to 50.
The reverse sequence proves that re-entering slices lose readiness when rebuilt
and become queryable only after preparation is applied. Physical keys are indexed
once and loading searches nearby graph ranges rather than scanning all obstacles
on every tick.

The pinned native `TrackRing` declares split/merge walkers unsupported. The live
ring comparisons therefore use an authored straight course; they do not claim
arbitrary branch-ring parity. Rust loading follows physical reciprocal topology
portals and has a disconnected-road regression. The exhaustive content inventory
found zero identity conflicts in 130 course/level/branch-mask combinations. In
120 combinations some physical instances are outside the current selected/Dual
cache (maximum 133); these are explicit current-cache exclusions. Every source
node appears in at least one branch-mask inventory. Dynamic steered path selection
is a separate open gate. The 25 real game factories retain all imported boxes,
load all five levels, take three ordinary ticks and replay at four frame rates;
these are factory/replay checks, not complete races. Observed imported classes
are 0–11 and 16; the controlled native producer also exercises classes 12–15.

Historical version-0 setups omit the new fields exactly. The existing frozen
legacy recording bytes and state hash still pass at all four replay schedules.
Version/metadata/cache budgets reject malformed configurations. Eight public
scenery tests cover ordering, loading, relative expansion, marking thresholds,
clock/latch behavior, disconnected ownership, serialization and replay. A private
module regression verifies detached-cache cleanup and fresh-body inheritance.

## Valid and invalid baselines

The valid 638-case baseline uses the frozen version-0 behavior and old factory
omissions against the same native producer as the corrected run. It fails 527
observations; the corrected run has zero mismatches. The legacy byte/hash fixture
provides an independent compatibility check. This is a newly generated baseline,
not a renamed earlier probe.

Earlier 290/311-row development receipts and the early 345-row lifecycle variant
using drive as native field `0xf0` are **invalid adapter evidence** and must not be
counted as engine failures. The separate corrected-fields 345-case run is valid:
it preserved 288 baseline mismatches and passed after correction, with native trace
`7cb8231144cc1e79a902efabbfe2a8c08de44494f7ddd2aa45bcdf7b12785a68`.
The final 638-case receipt supersedes that smaller valid matrix. Compiler failures
and the initial overly wide PDAT-tail bounds check are harness failures too. They
remain private for audit. The corrected field mapping was used for the valid
before/fixed traces recorded here.

## Reproduce

```sh
source local/reference-tools/env.sh
RR_SCENERY_LEGACY=1 python3 tools/reference/check_scenery.py \
  --data local/cdrom/ROADRASH --output local/NEW-scenery-before
python3 tools/reference/check_scenery.py \
  --data local/cdrom/ROADRASH --output local/NEW-scenery-fixed
python3 tools/reference/check_encounters.py \
  --data local/cdrom/ROADRASH --output local/NEW-encounters
cargo test --locked --target-dir ../target -p rr-simulation
cargo clippy --locked --target-dir ../target -p rr-simulation -p rr-game \
  -p rr-formats -p rr-render --all-targets -- -D warnings
cargo fmt --all -- --check
```

Raw original-data traces and local source identities stay under ignored `local/`.
Verification passed 182 simulation tests (18 private tests ignored in the ordinary
run), 59 formats/render tests (one private test ignored), strict affected-package
Clippy, formatting and diff checks. No original Windows-runtime comparison was performed. No complete contact-walk,
full-career, police or release gate is closed by this scenery receipt.

Parent integration passed the complete workspace: 412 tests, zero failures,
21 private tests ignored, strict workspace Clippy and formatting. A clean
25-factory rerun completed with exit code zero (98.01 seconds); 130 branch/level
identity inventories contain no conflicts or duplicates. The rebuilt source
game also passed a twelve-second field smoke and a 45-tick boundary episode
at four schedules, including rendered/headless hash equality. Receipts are in
`local/source-scenery-completion` and `local/source-scenery-gameplay-final`.
These are component checks; later collision/topology changes require their own validation.
