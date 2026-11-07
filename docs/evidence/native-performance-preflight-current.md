# Current-source performance preflight

These are bounded development measurements of source
`9ff9c95eb9174823e1847150ae63a22c5b05faf5`, application SHA256
`6efe6d66d1267faf38970e929445a28fa36f3228b8c2023a7a9e0ff2772fe1b5`.
They do not certify a later candidate or replace its required five-course hour.

Five complete level-three career recordings pass the production simulation and
owned snapshot budgets, with exact expected final hashes. Simulation p95 ranges
from 0.787 to 0.822 ms against 5 ms; owned snapshots range from 0.0020 to 0.0023 ms
against 1 ms. [CPU measurements](native-cpu-stages-current.json).

An actual 60-second Heaptrack run covers two tours through all five courses.
Displayed peak heap is 484.97 MB, with sampled Massif peak 484,921,973 bytes,
against the unchanged 768 MiB budget. Exit retention is 1.01 MB, including shared
library/framework caches and persistent workers; this is not a zero-leak claim.
The clean rebuild overlapped this allocation trace, so its frame times are
excluded from timing acceptance. [Heap receipt](native-heap-current.json).

Ten fresh-process startup checks cover all five courses, cold and warm. First
frame is 783–955 ms against the existing 5,000 ms budget. Cold means advisory
eviction of private file copies after syncing them, with private cache directories;
it does not mean a cold physical boot. No cache files were produced in the requested
directories, so shader-cache hits and individual compilation/upload costs remain
unmeasured. [Startup receipt](native-cache-current.json).

Three short eight-second probes miss the wall-frame p95 target. The XWayland
probe measures 19.17 ms; the virtual X11 probes measure 23.12 and 23.46 ms. These
are preserved exploratory failures, not accepted five-minute measurements. All
use the actual NVIDIA RTX 3080 Vulkan adapter. Switching to the simpler legal
pilot does not improve that virtual-display short scope.
[Short probes](native-performance-probes-current.json).

The subsequent uncontended 320-second City baseline also fails: wall p95 is
24.313 ms against 16.7 ms, with p99 27.071 ms. Raster p95 is 15.577 ms. All
16,864 settled wall/CPU samples are present; 320.170 real seconds contain
319.067 simulation seconds, zero paused frames and one real restart. The real
RTX 3080 Vulkan adapter is verified. No other acceptance application, local build
or profiler ran during this baseline; ordinary user desktop applications remained
untouched. This confirms the failure requires a correction before an official
hour, rather than extrapolating from the shorter probes.
[Complete diagnostic receipt](native-city-baseline-current.json).

A subsequent 30-second development probe includes the exact-pixel stack clipping
and source-font ownership corrections plus pilot instrumentation. Its different
executable and duration are identified separately: wall p95 still misses at
24.723 ms, raster p95 is 15.162 ms. Whole FixedUpdate pilot p95 is 2.907 ms,
including nested command planning at 2.848 ms; all 1,745 callbacks are sampled.
The default predictive policy remains enabled. These percentiles describe
different scopes and cannot be added or subtracted as matching-frame values.
Further attribution is required; no full-duration speedup is claimed from this
short probe. [Pilot diagnostic](native-pilot-diagnostic-current.json).

Another uncontended 30-second diagnostic separates course preparation and direct
surface updates. Wall p95 is 23.987 ms, raster 14.940 ms, course scene/camera/
snapshot preparation 0.111 ms, and direct main-world `upload_rgba` 0.0021 ms.
All 1,518 settled samples are present. The last timer excludes Bevy render-asset
extraction and GPU transfers. These small preparation/update values do not
explain the remaining wall target miss; CPU callstack profiling is the next
attribution step. [Stage diagnostic](native-stage-diagnostic-current.json).

The ten-cycle frontend lifetime probe also fails the fixed 16-image-handle budget:
CPU/GPU registries peak at 20 and warm at 19, while entity index storage remains
stable at 512. Its private report is retained under
`local/release-perf-current/lifecycle/`. The source-font atlas ownership correction
and replacement performance measurements are separate follow-up work; these
failures are not converted into passes by increasing budgets.

The benchmark now caches its route planner per race and resets on real restarts
and course changes. Each course segment has its own bounded timing storage, so
a full global summary prefix cannot prevent later Canyon sampling. Reports mark
aggregate and segment truncation explicitly. Validation requires every settled
wall and CPU sample for accepted course series, rejects truncated independent
runs, and preserves the old RC4 complete-supplement path. Thirty-nine public
Python tests pass normally and with optimization enabled; the actual historical
RC4 hour plus its required Canyon supplement still validates.

Commercial recordings, original images, allocator traces and raw logs remain
private. No power, clock, governor or driver policy was changed.
