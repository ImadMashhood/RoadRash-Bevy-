# Raster allocation correction

The measured allocation trace identified repeated vector growth in polygon near
clipping and projection. The drawing path already accepts at most 16 vertices;
clipping one plane emits at most 32. These two temporary buffers now use that
bounded stack storage. Clipping arithmetic, emitted vertex order, projection,
pixel/depth tests, statistics and the public unbounded `clip_near` API are unchanged.

The corrected renderer matches the previous renderer exactly on all 25 native
course/level combinations at three fixed cameras each: **75 views**, with exact
RGBA, floating-point depth bytes and complete render-receipt hashes. Each view
also checks serial versus six-worker rendering. There are no masks or tolerances.
Existing release renderer regressions pass. This establishes output preservation;
new frame-time measurements remain separate and are not inferred from fewer
allocations.

The first comparison attempt reused the old renderer through a shared build
target. Matching executable identities exposed that invalid harness run; it is
excluded. The accepted comparison rebuilt the changed library explicitly and
verified distinct old/new executable hashes before comparing output. Both full
aggregate reports hash to
`9bc989fb2d7b5a860fa94ad09ec3083ee0c94349a43326cf6f3c15a06ce1349d`.

Private reproduction: `local/release-raster-stack/comparison.json`. Public hashes
and scope: [native-raster-stack-current.json](native-raster-stack-current.json).
