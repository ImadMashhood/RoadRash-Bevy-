# Native functional candidate: Parts 19–22

The acceptance scope is the modern **offline** Thrash and Big Game game on
Fedora Linux, using the owner's verified original disc and original graphics.
The [48-task manifest](functional-manifest.json) and
[defect register](functional-defects.json) are the machine-readable checklist.
Task receipts under `evidence/19.*` through `22.*` record passed checks and any
open gate. The original Windows runtime comparison remains owner-waived;
[scope decisions](SCOPE_CHANGES.md) preserve that decision. This is not a claim
of original scheduler, traffic trajectory, RNG stream or pixel-perfect menu parity.

The reference is the locally built C++ port at
[`7aecf6044f2dd773b925b325823f30d648d61c89`](https://github.com/arkits/road-rash/tree/7aecf6044f2dd773b925b325823f30d648d61c89).
The supported EXE SHA-256 is
`73d7f9744b5b1a9482b3909950cdf3fe34119fe13d023b031e9c6739a60f18a2`;
the supported ISO SHA-256 is
`cc07917cb06ef0b3827afd3610f580009c4f42102efd849cdb82657b454bf6ff`.
The importer validates 341 required files before an active install is usable.
Private originals, converted images, source configurations, input recordings,
screenshots and decoded audio remain outside Git. Public receipts contain
hashes/counts and authored fixtures, never the commercial payload.

## Required scenarios

| Area | Scenario and expected outcome | Implementation / proof |
| --- | --- | --- |
| Actions | Press/hold/release, nitro charge depletion/recharge, incompatible bikes, brake cancellation, airborne activation, stationary edge-triggered dismount, crash/walk/remount, held-input quarantine | `actions.rs`, `actions`/`riding` tests, Bevy keyboard transport and device tests |
| Population | 14 competitive rivals; vehicle yielding and passing; crossings; pedestrian interruption; police hit/recovery/arrest; disconnected paths and simultaneous incidents | `population` tests; genuine career; source actor census |
| Art | All five courses, 25 course/level combinations, rider/police banks, seven pedestrian state sequences with their source view flags | 75 exact serial/parallel image/depth/stat comparisons; 66,325 pedestrian pose cases; seven verified absent native queries excluded from collisions |
| Racing | Native countdown, full routes/branches, standings, qualifying and nonqualifying finishes, arrest, physical wreck, quit confirmation, return/cleanup | command-driven full career, damaged-bike scenario, frontend/session adapter tests, ordinary packaged keyboard run |
| Career | Fresh rider → real prizes → paid upgrades/repairs → five levels → finale; reopen after every race and settle only once | `--functional-check`, independent four-schedule replay and money audit |
| Content roster | All 15 bike specifications and eight source character profiles; all five save slots | earlier native primitive matrices, source factory tests, profile-path checker |
| Frontend | Main, character/course/level, street, club/dialogue, showroom/specifications/buy, repair, restroom/settings/input/name, jukebox/video/credits, pause/restart/quit/results/standings/level-up/finale | modern menu audit, actual Bevy adapter tests and packaged keyboard walkthrough |
| Text | Five DLLs read as bounded PE data, 6,400 dialogue selections, complete original credit tables, relationship reactions and persisted scores | `rr-text-check`; `strings`, `interface`, career and frontend tests |
| Media | Intro, six start clips, all result/progression variants, finale, all 15 showroom clips and other catalog clips; 14 PCM music tracks; contextual effects and voices | 59 complete video/audio decodes, zero orphan decoders; media timing/ownership/error tests |
| Profiles | Create/save/load/Save As, backup recovery, migration rejection/acceptance, compatibility, exact-once settlement, bindings/accessibility persistence | session disk/interface tests; five slots under each of two distinct Linux UIDs; packaged reopen |
| UI | Minimum 320×240, normal, ultrawide and high DPI; five languages at 150%; bounded pages/focus; actual embedded glyphs | 100 rendered layout checks, 133 printable characters, no missing glyphs |
| Regression | Same source setup/input/camera; exact per-tick state, scheduler, RNG and motion; unmasked repeated image regions | `tools/verification/regression.py`, `compare.py`; mismatched initial conditions reject comparison |
| Performance | Native-only two-lap five-course real-time run; CPU raster, wall and GPU diagnostics, residency, RSS and ownership counts | `performance.rs`; declared hardware, warmup exclusions and measured budgets in performance receipt |
| Package | Read-only app outside checkout; own-disc first-launch import; no Cargo/development data paths required; separate user data/config, upgrade/uninstall and notices | `tools/release`, player guide and package receipt |

## Explicit modern rules

Schema 6 enables the new player actions. Existing schema 1–5 bytes and hashes
remain supported. The source factory opts into `functional_traffic`; old replay
setups keep their earlier behavior. Nitro reads six words (83–88) from each
source bike SPEC. Braking cancels boost; intentional dismount is allowed near
standstill and does not grant health. Decision cadence is fixed-tick and bounded.

Native gear arrays have unused trailing zero slots. The functional opponent cap
uses the greatest positive gear threshold, rather than the last array element.
Opponents choose five road-relative passing corridors and evaluate bounded
integer steering predictions without consuming extra RNG. Traffic retains its
cruise speed through a red-light/obstacle stop and resumes after clearance;
crossing vehicles retain their lateral motion. Both bike and detached body are
obstacles. Pedestrians wait for approaching traffic and use complete CANS
sequences. Mounted attacks/contacts can disable police, stop the siren and permit
recovery. Full native variable-dt interleaving remains outside the parity claim.

Dialogue selects source attitudes/relationships and local strings, rather than
invented advice. Combat reactions accumulate once per race and settle into the
career once. Talking does not grant cash. Credits page every supplied line;
modern contributor/font/license credits are separate from original credits.

Films use original filename families with presentation-only deterministic variant
selection. Quitting requests no result film. Level/finale films take precedence
over the ordinary winning clip. The release soundtrack is the fourteen supplied
PCM RRA tracks; legacy MIDI/SBK synthesis is not implemented. This is an explicit
alternative soundtrack policy, not a claim that MIDI was ported. Character IDs
3/5/6 select the source F cheer/grunt set; the others use the M set. Pedestrian
speech slots come from family resources 2/3, big-endian word +8 minus `0x44`;
absent/invalid references are silent. Cue timing, bounded cooldown and priority
are modern policies. No media cue draws gameplay RNG.

Sound captions label swings, impacts, crashes, sirens, cheers, rider pain and
pedestrian voices. **They do not transcribe film speech or original spoken
pedestrian lines.** The 44 clips with audio are inventoried as untranscribed;
15 showroom clips are silent. Original Schmooze dialogue is readable text. This
coverage is displayed accurately in Settings and the player guide. Reduced
flashes suppress automatic films; motion reduction removes camera motion while
preserving authority and required HUD state. Color has text equivalents.

## Evidence interpretation and known differences

An early completed career was rejected after the unused-gear bug made rivals
noncompetitive. It is not final acceptance evidence. Failed/aborted driver and
soak probes remain private research history. Final career verification checks
ordinary inputs, rejects injected damage/terminal/impact/weapon/pose commands,
reopens each save, independently recomputes source prizes/trade-ins/repairs and
compares canonical checkpoints at four schedules. A separately labeled 1%-condition
fixture proves physical wreck; it is not represented as a fresh-character career.

The clean-profile/read-only/package/UID checks run on this Fedora host, including
rootless distinct Linux UIDs. They do not claim a second physical machine, Windows,
macOS, a certified physical controller, a human playability review or every GPU.
Source menus are modern text panels with original previews, not original layouts.
Some diagnostics/result prose remain English. Original binary save import,
network play, full spatial/Doppler/random voice parity and 3D are deferred.
The owner has not selected a project-wide license; dependency/font notices do
not grant rights to the original game data or change the pinned reference's
licensing. The archive is a review candidate until release acceptance is recorded.

The implementing agent owns reproducible scenario execution and evidence; the
project owner reviews the delivered candidate. Never sign on the owner's behalf.
G5 remains open for a required failing scenario or missing release signoff.
Part 24 enhancement production stays locked behind G5. No upscaled asset is needed
for these checks, and no upscaling batch is part of Parts 19–22.

[Player guide](PLAYER_GUIDE.md) · [Native interface results](evidence/native-interface.json)
· [Video corpus results](evidence/native-videos.json)
· [Native render matrix](evidence/native-render-matrix.json)
