# Current native UI image ownership

The actual ten-cycle start/quit/unload check now passes the unchanged limit of
16 CPU and GPU image assets: both peak at **15**, with stable entity index storage
of **512** slots across five courses and eight character selections. The final
uninstrumented application exits normally. This is resource ownership evidence,
separate from frame timing, full race completion and owner G5 acceptance.

The original failure remains preserved: images peaked at 20 and settled at 19.
An actual asset inventory identified 11 retained font atlas pages. Bevy cached
font sizes after the old menu text entities had despawned. The correction runs
after text layout, retaining every atlas page referenced by any live text layout,
including hidden HUD labels. It releases only orphan font pages and their image
assets. Source artwork, fonts, font sizes and the telemetry budget are unchanged.

A public regression checks a retained hidden glyph, an orphan page, preservation
of the shared default white and owned dashboard images, and release after the last
text entity despawns. It passes, as does strict rr-game Clippy.

Actual Bevy Main captures before and after the correction have **zero changed
channels**, with no masks, and were visually inspected. Both complete 960×720 PNGs
hash to `2704c9728a18dcfb26c7327348a3bd19648b7bca9887ab074b2149cd8773ec27`.
The initial capture attempt lacked its output directory; its failed log remains
private and is excluded from this comparison.

The final tested application SHA256 is
`fa8508b29ec6b2cba9db5604d3dcb1dd0c15fd0fa37b20288a80712b712615a6`.
It also includes the parent's raster stack-allocation correction. Original images,
temporary before/after font inventories and raw logs remain private. The temporary
inventory instrumentation was removed before the final uninstrumented check.

Private reproduction receipt: `local/release-ui-lifecycle-final/receipt.json`.
The [aggregate receipt](native-ui-image-residency-current.json) preserves both
binary identities, limits, counts, hashes and scope.
