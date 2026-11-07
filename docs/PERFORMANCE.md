# Native performance protocol

The current candidate is RC6, source
`d9dac51433d279c5f6a92a266f2db9fe4a665d42`, actual downloaded game
`a26b1530633b3f53c6a399d7b401bb17b19358223dc576e6b8433becc21cffea`,
archive `08d7de03ca59e50a4f1cab4d6a0c44d10653771831fed6e5ba3885ad9896d1e6`.
Its [downloaded-byte preflights](evidence/native-preflight-rc6.md) pass unchanged
budgets: allocator peak 485.08 MB (conservative upper bound 485.09 MB) against
768 MiB, ten first frames at 776–942 ms against 5,000 ms, and ten UI cycles at
15 CPU / 16 GPU image handles against 16, with stable warmed 512 entity-index
slots. The 12-second owned-audio calibration passes 21 strict routing
observations, zero exit, unchanged global defaults and preserved preexisting
stream routes. Requested private shader cache directories remain empty; cache
hits and individual compilation/upload costs remain unmeasured.

**RC6 actual hour passed.** It completed 3,600.329 real seconds and 3,598.517
simulated seconds with zero paused frames. All five complete warm-course timing
gates pass (wall p95 14.151–16.265 ms); peak RSS is 1,126,232,064 bytes with
11 CPU/GPU image handles. Both laps and prior failures remain published.
[Final performance receipt](evidence/native-performance-rc6.md). No other
acceptance app/build/profiler overlapped. Only the owned audio/display resources
were stopped afterward; original audio defaults are verified after automatic
policy settling, without restoration commands. Owner G5 remains pending; stop
before mod support/upscaling.

RC5's following hour remains valid only for its exact bytes and tested scope.
Subsequent ordinary V dismount/recovery exposed a panic and rejected RC5 as the
current candidate. The [actual downloaded RC6 recovery proof](evidence/native-voluntary-downloaded-rc6.md)
passes repeated ordinary V recovery at levels 1 and 5 with eight passive replay
checks; RC5's passing soak does not erase that later correctness failure.

The historical [downloaded RC5 hour](evidence/native-performance-rc5.md) passes the
unchanged budgets on the declared platform. Source
`24d05dcf399035fe6c6e70bb19fe1b12488c1cec` and actual downloaded game
`4520d5a27a77ba5cbfadf2f2a7045dc206557dbf73be378eb47e67daea499f21`
completed 3600.308 real seconds and 3598.367 simulation seconds with zero paused
frames. All five predeclared warm second-lap timing series are complete; no
supplement is needed. Wall p95 is 16.340 ms City, 14.867 Highway, 15.585 Sierra
Nevada (MEDLY), 14.139 Napa and 16.228 Canyon; every wall p99 is below 50 ms.

The [full receipt](evidence/native-performance-rc5.json) retains both laps,
CPU/GPU stages, thermal/power ranges and exact artifact identities. Maximum
observed RSS is 1,128,783,872 bytes, with at most 11 CPU and 11 GPU image handles.
All 701 owned-audio routing observations pass; global default audio selections
are preserved and only the owned null-sink module was removed after the hour.
No other acceptance app, build, compression or profiler overlapped. The
[downloaded-byte preflights](evidence/native-preflight-rc5.md) separately pass:
60-second/two-tour allocator peak below 485.07 MB against 768 MiB, ten cold/warm
first frames at 747–938 ms, and ten UI cycles within the 16-image limit.
Profiler frame times are excluded from frame acceptance. These results do not
sign project-owner G5 review or establish performance on other hardware.

The measured platform is Fedora Linux x86_64 on a Ryzen 5 3600 (6 cores/12
threads), RTX 3080 and NVIDIA 615.78.08, with a 640×480 classic software drawing
surface fitted to a 960×720 window. Native textures, level 3, bike 8, fourteen
opponents and normal road users are enabled. No enhancement pack is active.
This is the lowest **tested** machine, not an inference about slower hardware.

The source game runs at 60 simulation ticks/second independently of presentation.
The existing release target is p95 wall-frame time ≤16.7 ms (60 Hz) and p99
≤50 ms outside documented loading pauses, measured for five minutes per course
([frozen release criteria](baseline/release-criteria.json)). A 30 Hz result does
not satisfy that target. The live desktop
measurement includes scheduling and presentation waits. CPU-stage and GPU-pass
percentiles are separate measurements and must not be added as if their maxima
occurred on the same frame.

| Native review budget | Limit | Scope |
| --- | ---: | --- |
| Simulation step p95 | 5 ms | Release-mode production core, exact recorded inputs |
| Owned snapshot extraction p95 | 1 ms | Simulation → owned presentation snapshot; excludes Bevy GPU extraction |
| Classic CPU raster p95 | 16.67 ms | Six workers maximum, exact serial reference retained |
| GPU pass p95 | 5 ms each | Hardware timestamp diagnostics, excluding compositor waits |
| Wall-frame p95 / p99 | 16.7 / 50 ms | Existing release target; five minutes per course |
| Decoded RGBA texture pixels | 192 MiB/course | Deduplicated Arc image identity; excludes structs/driver buffers |
| Profiled allocator heap peak | 768 MiB | Startup/course-switch peak; profiler RSS is separate |
| Instrumented process RSS peak | 1,536 MiB | Includes driver/runtime/allocator and bounded observation storage |
| GPU image descriptors | 16 MiB | Extents/mips/format; excludes swapchains and opaque driver allocations |
| Application image handles | 16 | CPU and GPU registries checked separately |
| Warmed entity-index storage | Stable after warmup | Bevy `Entities.len` counts allocated index slots, not live entities |

These limits are frozen against the native corpus; they are not an enhanced-pack
budget. Source palettes/rider/shadow images are the largest persistent decoded
bank (123,419,460 bytes). Loading a new course overlaps old/new resources briefly.
The heap profiler identifies `Palette.rgba` during `RiderAssets.load` as the
largest allocation site. The original texture set fits this budget; Part 24 must
measure enhancement residency independently.

`apps/game/src/performance.rs` records startup/first frame, synchronous course
loads, CPU raster elapsed milliseconds, Bevy render CPU/GPU diagnostics, window
and DPI, image counts/bytes, RSS, simulation liveness, pauses and restarts.
Sixty initial frames per segment are excluded from steady-state distributions;
loading stalls remain separately reported. `render/upscaling` is Bevy fitting the
completed frame to the window, not AI enhancement of game textures.

The hour-long soak uses two laps through all five courses, six minutes per course
per lap. Its pilot ignores physical controls/focus, uses continuous background
updates, normal controls and real terminal restarts. It is a stability/load test.
The historical RC5 hour supplies five complete warm second-lap six-minute timing series.
Window focus is not instrumented, and ordinary user desktop applications are
left untouched. The historical RC4 hour instead reached its global timing cap
and required a separate Canyon supplement, as documented below.

The earlier RC3 source-hour run overlapped builds/package probes: its first lap
missed the target on City, Highway and Sierra Nevada, while its warm lap passed.
The separate downloaded RC3 ten-minute probe was mixed, and the subsequent standalone Highway probes were later rejected because the result coordinator paused racing after about 91 seconds while their wall timer continued. Those probes are not accepted race-performance evidence. Those observations led to the persistent
worker change and remain explicit historical failures. They are not attributed
causally to other desktop work, and the short probe cannot replace the required
five-minute course coverage. Failure or insufficient sampling remains visible.

Live-pilot commands depend on observed update cadence. They are observational
soak inputs, not an exact replay oracle. Deterministic comparisons use the
current 31-race career covering all 25 course/level cases, 124 independent
replays, and the strict fixed-camera regression fixture. The Driver-cache
correction preserves all 536,874 recorded snapshot commands, including recovery
states ([comparison receipt](evidence/native-driver-cache-current.md)). The
earlier 67-race corpus remains historical evidence. Actual commands, seeds,
complete setups and expected hashes are preserved privately.
`rr-profile-replay` measures `Simulation.advance` and owned snapshot extraction
separately, verifies the final replay hash and emits only timing/count/hash data.
It includes thread descheduling; it does not measure input parsing or Bevy GPU
extraction. The live application's render diagnostics cover the render stages.

Current telemetry retains up to 500,000 timing samples independently per tour
segment, plus a separate aggregate prefix capped at 500,000, and 7,200
once-per-second memory observations. Explicit truncation flags and strict wall
and CPU sample-count checks prevent a capped segment from passing silently. The
aggregate prefix does not replace complete per-course series. Historical RC4
receipts used the older global cap and require their declared supplement. RSS maxima are observed samples and can miss brief transients; the allocator trace measures allocation peaks separately. Its vectors can grow during a run and are
included in RSS. The plugin is disabled in ordinary play unless explicitly
requested. RSS growth alone is therefore not an asset leak verdict. Repeated
source-scene weak-owner checks, warmed entity-index capacity, CPU/GPU image
counts, pixel residency and allocator profiling are evaluated separately.

The first lifecycle verifier assumed `Entities.len` meant active entities and
incorrectly rejected the allocator's 256→512 slot growth. The pinned Bevy source
defines it as allocated index storage and expands to capacity; the corrected
verifier requires a stable warmed plateau. Actual source scenes expire their weak
owners after all 15 course/bike setup pairs and eight character checks. The historical RC4
rendered ten-cycle fixture holds image counts at eight with brief nine-image
replacement overlap. RC5
[downloaded UI cycles](evidence/native-lifecycle-rc5.json) peak at 14 CPU and
15 GPU image handles, with stable warmed 512 entity-index slots. No growing count is hidden by an image mask or reset.

Allocator profiling uses a signed Fedora Heaptrack 1.5.0 package extracted into
a private tool directory; no system package/security settings are changed.
The historical RC4 two-lap five-course trace reports 476.79M displayed heap peak
(474,242,442 bytes at sampled Massif snapshots) and 1.03M unreleased at process
exit, including framework/global caches. Persistent raster workers intentionally
live until exit; scene lifetime is checked independently. The earlier RC3
trace (460.46M/962.47K) is retained separately. These allocations are reported, not called zero leaks.
The first profiler attempt failed because its preload path contained spaces;
that attempt is excluded and the tool was rerun from a space-free temporary path.
See the [upstream profiling method](https://github.com/KDE/heaptrack).

Cold/warm startup checks use fresh processes and private shader/cache directories;
where file eviction is requested it uses `POSIX_FADV_DONTNEED` on a private disk
copy after sync. That is advisory eviction, not a claim of a cold physical boot.
The driver wrote no files into the requested private cache directories in these
probes. Cold/warm therefore describes requested private-file eviction and repeated
process creation; shader-cache residency/hit rate was not independently observed.
Live GPU pass distributions and first-frame latency include first-use work, but
individual compilation/upload costs are not separately attributed.
No global cache drop, CPU governor, clock, fan or GPU power-limit change is made.
Observed power policy/limits and thermal extrema accompany the final receipt.

Reproduce with a private verified import:

```sh
cargo build --locked --release --workspace --target-dir ../target
../target/release/rr-profile-replay PRIVATE_REPLAY NEW_STAGE_REPORT.json
python3 tools/verification/regression.py --replay PRIVATE_REPLAY --output local/repeat \
  --target ../target/release --data local/cdrom/ROADRASH --install local/game-install --catalog local/catalog
python3 tools/verification/lifecycle.py --binary ../target/release/rr-game \
  --install local/game-install --catalog local/catalog --output local/lifetime
../target/release/rr-game --install-root local/game-install --catalog local/catalog \
  --course CITY --level 3 --bike 8 --audio on --videos off --benchmark-pilot on \
  --frontend off --smoke-frames 1000000 --benchmark-tour on \
  --benchmark-seconds 3600 --performance-report local/NEW-hour.json
```

Use the default predictive pilot without `RR_DRIVER_*` overrides. Reproducing
the audio scope also requires a process-local configuration targeting a verified
owned sink; the routing observer checks owned process/client/node/link identity
and unchanged global defaults. The historical
[RC5 performance receipt](evidence/native-performance-rc5.json) contains the
complete first- and second-lap results. Validate the declared
long-run criteria with `python3 tools/verification/performance.py --soak PRIVATE_HOUR
--frames PRIVATE_HOUR --output NEW_RECEIPT.json`; using the same file is explicit:
the first operation tests full-hour liveness/residency, the second checks the
predeclared warm second-lap percentiles. A shorter report is rejected.

Keep full replay setups, source frames, sound/video, profiler traces and raw logs
private. Publish aggregate values, hashes, explicit failures/skips and the exact
candidate identity. [Native acceptance](NATIVE_ACCEPTANCE.md),
[current CPU stages](evidence/native-cpu-stages-current.json),
[RC6 heap](evidence/native-heap-rc6.json),
[RC6 lifecycle](evidence/native-lifecycle-rc6.json).

The historical RC4 [downloaded-build short cross-check](evidence/native-delivered-short-profile.json) has mixed results: City/Highway warm segments and Canyon segments miss the wall target. It is retained as an exploratory failure, not silently replaced or labeled a pass. Its one-minute course segments do not satisfy the five-minute sampling requirement; the follow-up uses separate 320-second downloaded-build course runs. Observed competing desktop work is declared without claiming it caused the difference.

The RC4 renderer reuses at most six process-lifetime Bevy task workers and joins every borrowed stripe job before returning. Result stripes are sorted by column before assembly; scheduler completion order never becomes pixel order. It uses the already pinned `bevy_tasks` package, with no new third-party package. The native serial/parallel image/depth/receipt matrix and the pre-change actual Bevy capture agree exactly. [Pool regression and local reproducibility](evidence/native-pool-regression.json). The earlier standalone Highway timing comparisons were later rejected for paused simulation; no racing-performance improvement is claimed from them. The historical unpaused RC4 hour is valid for its exact artifact; RC5 is measured separately above.

[Rejected paused probes](evidence/native-excluded-perf-probes.json) are retained explicitly. Standalone performance reproduction adds `--frontend off --smoke-frames 1000000`: this suppresses result-session ownership, retains the native HUD/raster, and allows the benchmark pilot to restart actual terminal races. The 320-second wall timer ends before the large frame guard. The accepted supplemental report must show zero paused frames and at least 95% live simulation. The production game's ordinary results pause is intentional; it is not treated as a gameplay defect.

Historical RC4 result: the checksum-verified downloaded executable completed 3600.408 real seconds, 3594.617 simulation seconds, zero paused frames and 19 race/course restarts. Its accepted course samples have wall p95 9.79–13.75 ms; all five p99 values satisfy the original 50 ms limit. Full first/second-lap distributions, descriptor/RSS/decoded-pixel peaks and whole-hour CPU/GPU thermal extrema are published, including earlier failures. The final hour overlaps no other native acceptance app, local build or profiler. Ordinary user desktop applications remain present and focus is uninstrumented. This is a declared-platform measurement, not a guarantee under arbitrary competing workloads.

The RC4 hour rendered 514,442 frames and reached the bounded 500,000 timing-sample cap. Stability/liveness/memory observations cover the full hour, but the final Canyon timing series is incomplete. A separate full 320-second run of the same downloaded executable/settings supplies Canyon acceptance. The validator refuses incomplete coverage or replacement of a complete failed segment. [Exact samples and coverage](evidence/native-performance.json).

For that exact historical RC4 capped hour, validation requires `--supplemental-course-report PRIVATE_CANYON_320.json`. The valid supplement uses `--frontend off --smoke-frames 1000000 --benchmark-pilot on --benchmark-seconds 320`; the huge frame guard suppresses only normal result-session ownership and never ends this run. Native raster/HUD, audio, physics, traffic and real terminal restarts remain active. The verifier rejects any pause, insufficient live time, changed executable/window/level/bike or attempted replacement of a complete failed segment. The accepted Canyon report has zero pauses and a complete live timing series.
