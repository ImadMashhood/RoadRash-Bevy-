# Downloaded RC5 pre-hour checks

These checks use source `24d05dcf399035fe6c6e70bb19fe1b12488c1cec`, downloaded
archive SHA256 `6656faaed67e79fb2d54898513806ce4a522efd48898028851781edc9c5cd910`,
and actual Ubuntu-built application SHA256
`4520d5a27a77ba5cbfadf2f2a7045dc206557dbf73be378eb47e67daea499f21`.
They are separate from the earlier local-build development measurements.

The actual 60-second allocator run covers two tours through all five courses.
Displayed heap peak is 485.06 MB, conservatively below 485.07 MB after rounding,
against the unchanged 768 MiB budget. Sampled Massif peak is 482,433,892 bytes.
Profiler RSS is 1.15 GB; retained allocations at exit are 1.03 MB, including shared
library/framework caches and persistent raster workers. This is not a zero-leak
claim. No competing acceptance app, build or profiler ran during the trace; its
profiled frame times are excluded from performance acceptance.
[Allocator receipt](native-heap-rc5.json).

All ten fresh-process startup checks pass: five courses, cold and warm, with first
frame 747–938 ms against 5,000 ms. Cold requests advisory eviction only on private
file copies after syncing; it does not claim a cold physical boot. No shader cache
files appeared in the requested private directories, so cache hits and individual
shader compilation/upload attribution remain unmeasured.
[Startup receipt](native-cache-rc5.json).

Ten scripted native-factory/countdown/quit/Main cycles pass with at most 14 CPU
image handles and 15 GPU image handles against the fixed limit of 16. Allocated
entity index storage stays at 512 after warmup. This typed lifecycle fixture is
distinct from the ordinary keyboard package walkthrough.
[Lifetime receipt](native-lifecycle-rc5.json).

Audio routing calibration uses the actual downloaded game with audio and effects
enabled. A process-local, space-free ALSA configuration and `PIPEWIRE_NODE` select
our own null sink by name. PipeWire client process identity links to output nodes
and active links; Pulse stream `client.id` independently identifies the same owned
output. Twenty-one observations verify routing to the owned sink, and normal exit
is zero. Global default sink/source and existing stream routes remain unchanged.
The first whitespace-path and numeric-target probes were rejected and their owned
modules cleaned up; quiet output was never substituted for routing proof.
[Routing receipt](native-audio-routing-rc5.json).

Only the owned null-sink module remains temporarily for the separate official
hour. That hour uses continuous native play, original graphics, the unchanged
default predictive policy, live opponents/traffic, audio routed to this sink,
and two laps through all five courses. Its predeclared full second lap supplies
the required warm timing series. No full-hour or five-course performance pass is
implied by these preflight checks; its own complete receipt must pass validation.
Commercial setups, traces, captures and raw audio/device metadata remain private.
