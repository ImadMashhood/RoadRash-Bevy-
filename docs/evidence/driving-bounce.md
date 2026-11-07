# Driving bounce correction

The player visibly bobbed while riding, including on flat road. The renderer
interpolated the ground anchor using reciprocal depth, while native near-camera
projection uses a nonlinear scale lookup. A fixed-height actor therefore moved
vertically each time the camera crossed a source road sample.

`contact_anchor` now interpolates projected endpoints using the actual lookup
scales, with a finite fallback for equal scales/coincident endpoints. Suspension,
gravity, collisions, simulation rules and replay authority are unchanged.

Two public renderer regressions cover all 17 positions from the start through
the wrap of a road sample, visible wheel/foot pixels without ground clipping,
and equal-scale/coincident samples. The static-actor nonlinear authored fixture
failed before the fix with 85.8603 pixels of vertical movement; it now stays
within one pixel. It requires no original game assets. A separate opt-in
regression repeats the
same rendered-frame and visible-feet assertions with the hash-verified native
projection table; that test also passes with the private supported executable.

Private native captures at City samples 50 and 50.5 moved the player's foot from
326 to 342.9936 pixels before the fix; after it, the positions are 326 and
326.0580. This real City segment contains rise/bend and is not claimed as a
flat-road full-course test. The before captures use the pre-existing release
renderer, while the initial failing public regression was compiled against the
current base source before the fix. These captures are illustrative paired
positions, not a complete before/after binary equivalence audit.
Before/after PNGs, receipts and the separate initial failure observation are retained in `local/driving-bounce-2026-11-06/`.
A fresh release build ran an eight-second native-art City pilot and its actual
Bevy screenshot was visually inspected. This is a gameplay/rendering smoke,
not performance, full-career or release acceptance.

Validation: 33 asset-free renderer integration tests plus three renderer unit
tests pass, including existing reviewed authored goldens, hills, shadows,
airborne transitions and serial/parallel pixel equality. The opt-in native-table
regression also passes. Final combined verification after the collision-query
follow-up passes 401 workspace tests (17 private-data tests ignored in that
ordinary run), 36 Python tests, strict workspace Clippy, formatting and diff checks.
The private native comparison and original-table tests were run separately with
their required data. A fresh rebuilt game passed the 12-second native-art City
smoke and controlled boundary/replay checks described in the
[source-query report](source-static-queries.md). The actual race and wall captures
were inspected. The source change is based on `a5eff4c` plus this working tree,
verified 2026-11-06.

Reproduce the public checks:

```sh
cargo test --locked --target-dir ../target -p rr-render
cargo clippy --locked --target-dir ../target -p rr-render --all-targets -- -D warnings
cargo fmt --all -- --check
RR_NATIVE_DATA="$PWD/local/cdrom/ROADRASH" cargo test --locked --target-dir ../target \
  -p rr-render --test classic native_projection_keeps -- --ignored
```

The other required interaction/career/release items remain open.
