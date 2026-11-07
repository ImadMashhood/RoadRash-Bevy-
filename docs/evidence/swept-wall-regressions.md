# Swept authored-wall regression coverage

This follow-up covers the authored `ObstacleKind::Building` path through normal
simulation ticks. It adds five public regressions to
[`crates/simulation/tests/collision.rs`](../../crates/simulation/tests/collision.rs).
It does **not** close the native swept-contact acceptance item.

## Cases and assertions

- Complete longitudinal crossings of a one-unit-half-length wall at initial
  speeds 30,000 and 1,000,000. The latter is the authored setup ceiling, not the
  native speed ceiling. Small authored rider extents make both expanded endpoint
  boxes miss. The world broadphase still finds the wall, stops the bike one unit
  outside the expanded wall and emits exactly one contact event.
- Eight genuine diagonal near misses: both turn directions, both initial speeds
  and both small authored and normal `[7680, 64, 8960]` rider dimensions. Each
  fixture independently checks that longitudinal exit precedes lateral entry
  while the swept bounding rectangle overlaps the wall. The complete bike state
  equals an obstacle-free control and no contact event is emitted.
- Complete lateral crossings in both directions with normal rider dimensions.
  Both endpoint boxes miss laterally. The bike is displaced to the entry side,
  retains forward progress and stays mounted, with one contact event.
- An otherwise intersecting wall on a disconnected path. The complete bike state
  equals the obstacle-free control and no contact event is emitted.
- Three recordings (one longitudinal impact and two diagonal near misses), each
  played for 120 ticks at 30/60/144/irregular frame schedules with effects both
  enabled and disabled. Final state hashes and canonical checkpoints agree
  across all eight schedules/settings for each recording (24 runs total).

Synthetic extreme bends deliberately exercise lateral crossings; these fixtures
are not measurements of attainable native riding trajectories. All data is
public authored content, with no commercial assets required.

## Reproduce

```sh
cargo test --locked --target-dir ../target -p rr-simulation --test collision
cargo test --locked --target-dir ../target -p rr-simulation
cargo clippy --locked --target-dir ../target -p rr-simulation --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Verification result

Verified on 2026-11-06 against base revision `a5eff4c` plus this working-tree
change. The collision target passes all 12 tests, including the five additions.
The full simulation suite passes 166 tests with zero failures; 14 private native
comparison tests are explicitly ignored by that ordinary test invocation. Strict
Clippy for all simulation targets, formatting and `git diff --check` pass.
No native producer or graphical acceptance run is claimed for this follow-up.

## Limits

The existing sweep calculation needed no behavior change for these cases. Native
source-box scenery and type-0 source building boundaries take different paths;
these tests do not establish their swept-contact parity. The native checklist
stays open for those paths, height/top contacts, coupled boundary ordering, dense
populations and simultaneous incidents. The 327,086 native-observation count is
unchanged. Career and release acceptance remain open.
