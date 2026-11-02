# Collision/combat priority: measured corrections

The [active interaction gate](../COLLISION_COMBAT_PRIORITY.md) is now the gameplay
priority. [Machine-readable results](collision-combat-priority.json) distinguish
passing narrow comparisons from the larger open mechanics requirements.

The first new C++ comparison failed: native fists could hit at attack age 13
while Rust remained inactive. New `combat_version: 1` definitions generate their
hit windows from retained source flags and the counter after advancement.
All **458** per-tick observations now match the pinned C++ animation API over
both sides, both attack tables and every weapon slot. Authored engine tests
also exercise the actual first-hit tick, strict window endpoints and the
source-versus-legacy counter boundary.

The crash-entry comparison also failed: a zero-speed body lost the source's
negative longitudinal launch. The corrected source entry preserves signed
impulses and rejects re-entry while the linked crash is active. All **24**
initial/repeated entry observations now match, including heights, velocities,
reason-specific animation and entry return values. A separate engine regression
checks that repeated input does not reset the incident/body or emit another
crash. Historical version-0 behavior remains available for old replay hashes.

Existing 108 generic-motion and 35 reason-scaled stamina observations still
match. This totals **625** exact native observations. The 50 source setup checks
and four-schedule, six-phase recovery scenario also pass. The current source
native wall probe again passes all 45 tick hashes across four schedules, one
wall incident, no penetration, stopped speed and rendered/headless agreement.
The receipt pins its actual executable, rule version and final state hash.

The workspace passes **364 Rust tests**, **33 Python tests**, formatting and
strict Clippy. Original commercial assets, the failing and passing detailed
traces, screenshots and source configurations remain ignored under `local/`.
The native reference is `7aecf6044f2dd773b925b325823f30d648d61c89`; the comparison
harness refuses another source revision.

Reproduce with the supported private import and existing pinned native build:

```bash
cargo test --locked --workspace --target-dir ../target
cargo build --locked --release --target-dir ../target -p rr-simulation -p rr-game
source local/reference-tools/env.sh
python3 tools/reference/check_encounters.py \
  --data local/cdrom/ROADRASH --output local/new-combat-probe
python3 tools/verification/gameplay_fixes.py \
  --game ../target/release/rr-game --replay ../target/release/rr-replay \
  --install local/game-install --catalog local/catalog \
  --output local/new-contact-probe
```

These probes compare authored starting states and native data/API behavior;
they do not compare against the Windows executable running. Entry equality
also does not establish multi-tick crash-body trajectory parity. The subsequent
recovery integrator still has modern policies, including speed clamping, and
complete native contact classification, fight transactions, trajectory traces,
visual crash-to-remount review and full-career acceptance remain **open**.
