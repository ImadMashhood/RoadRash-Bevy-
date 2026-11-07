# Downloaded RC6 native performance

**Status: passed for the declared platform and predeclared warm-course scope.**

Source `d9dac51433d279c5f6a92a266f2db9fe4a665d42`, actual game `a26b1530633b3f53c6a399d7b401bb17b19358223dc576e6b8433becc21cffea`, archive `08d7de03ca59e50a4f1cab4d6a0c44d10653771831fed6e5ba3885ad9896d1e6`.

Real elapsed 3600.329s; simulation 3598.517s; paused frames 0; actual terminal/course restarts 19.

Both laps remain in the JSON. The complete warm second lap was selected before the hour, with unchanged thresholds.

| Warm course | Samples | Wall p95 ms | Wall p99 ms | Raster p95 ms |
| --- | ---: | ---: | ---: | ---: |
| CITY | 27325 | 16.265 | 17.659 | 11.260 |
| HIWAY | 29708 | 14.981 | 16.344 | 10.078 |
| MEDLY | 28995 | 15.576 | 17.359 | 10.631 |
| NAPA | 31089 | 14.151 | 15.306 | 9.288 |
| CANYN | 28433 | 16.245 | 18.272 | 11.658 |

Hardware GPU timestamps, full settled sample coverage, liveness and native residency are checked by the unchanged strict validator. Allocator/startup/UI-lifetime checks are in [the preflight record](native-preflight-rc6.md).

Owned native audio routing has 701 verified observations. Global default audio selections are equal before and after; raw device names/serials and other stream metadata remain private.

CPU/GPU thermal and power ranges, complete first/second-lap distributions, image/residency counts, exact hashes, explicit failures and limitations are in [the JSON receipt](native-performance-rc6.json). Ordinary user desktop applications were left untouched; no other acceptance app, build, compression or profiler overlapped. This is not a performance guarantee for untested machines or arbitrary workloads.

Earlier failed development probes remain preserved and were not substituted as current passes. Project-owner G5 review is a separate decision.
