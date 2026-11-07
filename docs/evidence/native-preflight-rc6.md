# Downloaded RC6 pre-hour checks

These measurements use immutable source `d9dac51433d279c5f6a92a266f2db9fe4a665d42`, release
`native-functional-2026-11-07-rc6`, actual downloaded game `a26b1530633b3f53c6a399d7b401bb17b19358223dc576e6b8433becc21cffea` and archive
`08d7de03ca59e50a4f1cab4d6a0c44d10653771831fed6e5ba3885ad9896d1e6`. RC5 measurements remain historical, exact-byte evidence;
they do not replace these RC6 checks after the ordinary V recovery correction.

The actual allocator run completed 60.184s and two tours through
all five courses. Displayed heap peak is 485.08M; conservative
rounding upper bound 485,090,000 bytes
is below the unchanged 768 MiB budget. Sampled Massif peak is
481,723,098 bytes. Profiler RSS is
1.16G; process-exit retention is
1.04M, including framework/library caches
and process-lifetime workers. This is not a zero-leak claim. Profiled frame times
are excluded. [Allocator receipt](native-heap-rc6.json).

All ten fresh-process startup checks (five courses, cold/warm) pass with first
frame 776–942 ms against 5,000 ms. Cold requests advisory
eviction only on private copies after syncing; it is not a cold physical boot.
Cache hit rates and individual shader compilation/upload costs remain unmeasured.
[Startup receipt](native-cache-rc6.json).

Ten typed native-factory/countdown/quit/Main cycles pass with at most
15 CPU and 16 GPU image handles against
16, and warmed entity-index storage at 512. This
fixture is distinct from the ordinary keyboard V recovery walkthrough.
[Lifetime receipt](native-lifecycle-rc6.json).

Audio/effects-on calibration verifies 21 observations
through the owned PipeWire client/output node/active links into only our own null
sink; the process exits zero. Per-process space-free ALSA configuration overrides
both default and pipewire PCM; PIPEWIRE_NODE/PULSE_SINK use owned node name.
Global default selections and preexisting stream routes are preserved. Private
raw default names, serials and other-stream metadata are not published.
[Routing receipt](native-audio-routing-rc6.json).

Only the owned null-sink module remains for the separate ROOT-reviewed official
hour, then only that module will be removed with defaults verified. No official
hour or full-course timing pass is implied by these preflights. The hour retains
both laps through all five courses and requires complete predeclared warm second
lap series at unchanged budgets, live traffic/opponents, native graphics and
verified owned audio. Root coordinates all acceptance apps/builds/profilers;
these checks ran sequentially without competing acceptance work. No global
power/clock/governor/cache policy was changed; ordinary desktop applications
remain untouched. Commercial setups, traces, captures and raw logs stay private.
