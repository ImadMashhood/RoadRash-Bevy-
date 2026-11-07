# Ordinary dismount: same-tick source body context

The [actual RC5 keyboard failure](native-ordinary-terminal-rc5.md) exposed a
missing independent road context when normal **V** creates a LIVE body during
movement. Per-tick road preparation had already run while the rider was mounted.
The new body moved immediately and indexed a cache that would only be created
on the following tick. Physical-impact entry normally occurs later, so its
existing comparisons did not exercise this boundary.

The correction initializes the new body's exact source route/configuration from
the already prepared actor context at that transition. It does not fall back to
base ground or alter contact physics, random state, legacy profiles or the
dismount eligibility guard.

Two authored regression tests cover both movement dispatch modes, a route context
distinct from base, and two complete dismount-to-remount episodes with cleanup
and recreation of the cache. The original missing-key panic is preserved; both
corrected tests pass. Full source validation passes **514 workspace tests**,
58 explicit private-data ignores, strict Clippy/formatting, 39 general Python and
16 career-auditor checks. These two authored tests are separate from the native
observation totals. Invalid preliminary fixtures remain excluded.

[Exact source blobs, validation logs and scopes](source-manual-dismount.json).
The actual local rebuilt game passes CITY level1 and confirmed CITY level5
ordinary voluntary recovery, including repeated episodes, resumed driving and
normal exit. Misnavigated/menu-only captures are excluded. This is a local source
preflight; the replacement delivered candidate remains required. No owner G5
acceptance is signed.

[Actual downloaded RC6 keyboard proof](native-voluntary-downloaded-rc6.md) now
passes repeated ordinary recovery at both City levels, eight independent
replays and normal exits. The separate [final RC6 hour](native-performance-rc6.md) now passes.
