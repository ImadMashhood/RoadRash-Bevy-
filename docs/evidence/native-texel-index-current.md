# Exact texture indexing optimization

A userspace CPU profile of the actual application identified polygon drawing
at 58.72% of sampled cycles and remainder/floor routines at another 15.44%.
The profiled run is diagnostic evidence, excluded from frame-time acceptance.

Texture repetition now removes the integral part instead of invoking general
floating-point remainder for divisor one. Coordinates already in [0,1) take a
direct path. A saturating integer conversion supplies the existing nonnegative
floor/lower clamp, followed by the same upper texture bound. No interpolation,
depth, geometry, filtering or texture quality changed.

The original formula remains an independent test oracle. More than **2.2 million**
comparisons cover deterministic floating-point bit patterns at 11 texture widths,
both repeat and clamp, texel boundaries and their immediate neighbors, signed
zeros, subnormal/large values, infinities and NaNs. Every selected texel matches.

All **75 native views across 25 course/level cases** also retain exact RGBA,
depth bytes and full render-receipt hashes, including serial/parallel agreement.
Both reports hash to
`9bc989fb2d7b5a860fa94ad09ec3083ee0c94349a43326cf6f3c15a06ce1349d`.
Strict renderer Clippy passes. These checks establish compatibility; the combined
application still requires fresh timing measurements.

[Aggregate receipt](native-texel-index-current.json). Private reproduction:
`local/release-raster-stack/texel-comparison.json`.
