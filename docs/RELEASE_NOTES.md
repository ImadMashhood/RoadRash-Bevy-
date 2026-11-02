# Native functional Linux candidate RC4

[Download RC4](https://github.com/ImadMashhood/RoadRash-Bevy-/releases/tag/native-functional-2026-11-01-rc4).
This is a review candidate for the offline Thrash and Big Game recreation,
using the player's supported original disc. It is not an accepted G5 release.
There are no AI-enhanced assets in the application or acceptance corpus.

Source: `234337749774a5a5a6d3fc9cafa88b9c1d55f3dd`.
Archive SHA-256: `02821ad359c978cebf82350e62d79bafddc159e945177ef47e694d56162d3f11`.
The adjacent `SHA256SUMS` and all 712 packaged manifest entries were verified
after downloading. RC4 adds persistent raster workers after the earlier live-tour timing failures.
The importer bytes match RC3; all 6,140 freshly converted payload files and
semantic catalog entries match despite the new build fingerprint. Exact
serial/parallel and actual old/new Bevy image comparisons agree. See [download receipt](evidence/native-download-rc4.json) and
[build/reproduction procedure](RELEASE_PIPELINE.md).

Run the extracted `RoadRash` launcher and select the supported ISO or ZIP when
asked. Python 3, Zenity, FFmpeg and the documented graphics/audio system libraries
are prerequisites. The archive contains the application, launcher, guide and
notices; the player supplies game data. Saves and settings live separately from
the application. [Installation, controls, backup recovery and removal](PLAYER_GUIDE.md).

## What this candidate adds

- Complete native actions, including independent lean, charge-based nitro,
  stationary dismount, combat, crash recovery and remount.
- Competitive rivals with corrected gear limits; road users, pedestrian
  animation/voices, police pursuit/recovery and source courses/roster.
- An offline career with paid purchases/repairs, progression, finale, atomic
  save slots, settings and explicit backup recovery.
- Five source languages, source dialogue/credits, scalable modern menus, text
  scaling, contrast, reduced motion/flashes and accurately labeled sound captions.
- Original PCM music, contextual effects and source film selection; safe media
  ownership and bounded decoder cleanup.
- Strict deterministic replay/visual regression tools, native raster performance
  improvements, telemetry and data-free packaging with dependency/font notices.

## Executed acceptance

The actual downloaded package completes the same 67 ordinary-control career
attempts as the audited reference: 44 wins, 22 arrests, one nonqualifying loss,
all 25 course/level combinations, five purchases and nine repairs. Final cash
is 25,567. Every recorded replay file is byte-identical to the reference, whose
268 independent runs preserve all checkpoints across four schedules. A separate
physical wreck fixture, ordinary keyboard/import/reopen/recovery walkthrough and
graphical full-finale replay cover integration paths.
[Packaged career](evidence/native-packaged-career.json),
[reference audit](evidence/native-career.json),
[keyboard run](evidence/native-keyboard-walkthrough.json),
[delivered graphical replay](evidence/native-delivered-runtime.json).

Other executed checks include 100 rendered language/layout combinations, 133
printable glyphs, all 59 complete film decodes, exact native serial/parallel
raster comparisons, distinct Linux UID isolation, a source-hidden fresh import,
read-only installation, upgrade preservation and removal preserving user data.
The actual downloaded RC4 executable also completed a real unpaused hour with
live simulation/audio and repeated courses/results. CPU/GPU/memory/loading evidence
is collected separately; performance targets and observed failures remain
visible in [the performance protocol](PERFORMANCE.md). An elapsed soak alone
does not establish 60 Hz presentation.

## Known issues and coverage limits

These records are material limitations, not successful tests of unavailable
platforms/features. The IDs below are stable evidence records for issue reports.

| ID | Limitation / effect | Evidence / disposition |
| --- | --- | --- |
| LIMIT-01 | Only Fedora Linux x86_64 on the stated Ryzen 5 3600/RTX 3080 was physically tested. No lower hardware, other OS or certified physical controller claim. | [Performance platform](PERFORMANCE.md), [input scope](INPUT.md); document additional configurations before expanding support. |
| LIMIT-02 | Fixed-tick modern actor/traffic/RNG and passing rules differ from the original variable-dt implementation; menus use modern panels. Windows runtime comparison was owner-waived. | [Modern policies and parity scope](NATIVE_ACCEPTANCE.md), [scope decisions](SCOPE_CHANGES.md). |
| LIMIT-03 | The fourteen original PCM tracks are supported; original MIDI/SBK synthesis is not implemented. | [Audio contract](AUDIO.md), [native media receipt](evidence/19.07.md); explicit soundtrack alternative. |
| LIMIT-04 | Sound captions label events; they do not transcribe film speech or pedestrian recordings. Some diagnostics/result prose remains English. | [Accessibility and text scope](NATIVE_ACCEPTANCE.md), [rendered interface](evidence/native-interface.json), [film inventory](evidence/native-videos.json). |
| LIMIT-05 | Seven native pedestrian appearance queries have no drawable result and are excluded from live collisions. | [Native actor matrix](evidence/native-render-matrix.json), [road-user contract](ROAD_USERS.md). |
| LIMIT-06 | Original binary saves, networking, complete spatial/Doppler/random-voice parity and 3D are outside this offline release scope. | [Saves](SAVES.md), [audio](AUDIO.md), [scope](NATIVE_ACCEPTANCE.md). |
| LIMIT-07 | Project-wide source license remains unselected; the pinned reference supplies no license. Dependency/font notices do not license commercial data. | [Licensing provenance](RELEASE_PIPELINE.md), [packaged notice receipt](evidence/22.04.md). |
| LIMIT-08 | Heaptrack reports nonzero exit retention including reflection/global/driver caches; this is not a zero-leak claim. Telemetry retains bounded observation vectors. | [Heap receipt](evidence/native-heap.json), [lifecycle](evidence/native-lifecycle.json), [measurement scope](PERFORMANCE.md). |
| LIMIT-09 | Requested cold/warm file-cache probes do not prove physical cold boot or observed shader-cache hits. Other user desktop activity is left untouched. | [Loading receipt](evidence/native-loading.json), [measurement protocol](PERFORMANCE.md). |
| LIMIT-10 | Napa first-lap p95 is 20.90 ms; the accepted warm sample is 12.29 ms. Standalone probes that paused at results are rejected. The final hour reached its timing cap on Canyon and requires the live 320-second supplement. | [Full distributions and coverage](evidence/native-performance.json), [rejected probes](evidence/native-excluded-perf-probes.json); no fixed frame rate under every scene/load is promised. |
| G5-PENDING | The required warm performance pass is recorded; project-owner acceptance remains pending before promoting this candidate or starting texture production. | [Release criteria](baseline/release-criteria.json), [manifest](functional-manifest.json), [acceptance owner](baseline/milestones.json). |

When reporting a problem, include the RC4 identity, exact controls/menu steps,
course/level/bike, timing and redacted application log. Keep original media,
source replay setups, profiles and private captures out of public reports.
Follow [log collection and privacy instructions](PLAYER_GUIDE.md).

The [downloaded-build short cross-check](evidence/native-delivered-short-profile.json) has mixed results: City/Highway warm segments and Canyon segments miss the wall target. It is retained as an exploratory failure, not silently replaced or labeled a pass. Its one-minute course segments do not satisfy the five-minute sampling requirement; the follow-up uses separate 320-second downloaded-build course runs. Observed competing desktop work is declared without claiming it caused the difference.

Current RC4 accepted-course wall p95 is 9.79–13.75 ms across four complete six-minute warm samples and an independent 320-second Canyon sample; every p99 meets 50 ms. [Exact hour and both laps](evidence/native-performance.json). The older short-profile failures remain RC3 evidence; their retest led to the documented pool fix. [Native pool regression](evidence/native-pool-regression.json).

The final hour rendered 514,442 frames and reached the bounded 500,000 timing-sample cap. Stability/liveness/memory observations cover the full hour, but the final Canyon timing series is incomplete. A separate full 320-second run of the same downloaded executable/settings supplies Canyon acceptance. The validator refuses incomplete coverage or replacement of a complete failed segment. [Exact samples and coverage](evidence/native-performance.json).

The [actual downloaded/local Bevy image comparison](evidence/native-delivered-image-rc4.json) is exact. Rejected paused probes are retained separately and make no racing-performance pass claim.
