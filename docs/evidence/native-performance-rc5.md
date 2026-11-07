# Downloaded RC5 native performance

**Status: passed for the declared platform and predeclared warm-course scope.**

Source `24d05dcf399035fe6c6e70bb19fe1b12488c1cec`, actual game `4520d5a27a77ba5cbfadf2f2a7045dc206557dbf73be378eb47e67daea499f21`, archive `6656faaed67e79fb2d54898513806ce4a522efd48898028851781edc9c5cd910`.

Real elapsed 3600.308s; simulation 3598.367s; paused frames 0; actual terminal/course restarts 19.

Both laps remain in the JSON. The complete warm second lap was selected before the hour, with unchanged thresholds.

| Warm course | Samples | Wall p95 ms | Wall p99 ms | Raster p95 ms |
| --- | ---: | ---: | ---: | ---: |
| CITY | 27249 | 16.340 | 17.599 | 11.336 |
| HIWAY | 29744 | 14.867 | 16.227 | 10.023 |
| MEDLY | 28775 | 15.585 | 17.250 | 10.661 |
| NAPA | 31267 | 14.139 | 15.398 | 9.287 |
| CANYN | 28446 | 16.228 | 18.103 | 11.689 |

Hardware GPU timestamps, full settled sample coverage, liveness and native residency are checked by the unchanged strict validator. Allocator/startup/UI-lifetime checks are in [the preflight record](native-preflight-rc5.md).

Owned native audio routing has 701 verified observations. Global default audio selections are equal before and after; raw device names/serials and other stream metadata remain private.

CPU/GPU thermal and power ranges, complete first/second-lap distributions, image/residency counts, exact hashes, explicit failures and limitations are in [the JSON receipt](native-performance-rc5.json). Ordinary user desktop applications were left untouched; no other acceptance app, build, compression or profiler overlapped. This is not a performance guarantee for untested machines or arbitrary workloads.

Earlier failed development probes remain preserved and were not substituted as current passes. Project-owner G5 review is a separate decision.
