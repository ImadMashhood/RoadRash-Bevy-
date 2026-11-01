# Audio, music and movies

Part 18 adds the engine-independent `rr-media` crate and the Bevy `MediaPlugin`. Snapshot and resolved-effect inputs feed a bounded presentation planner; media never draws gameplay RNG or writes race authority. Live fixed ticks and **every** tick crossed by a replay frame are observed, preserving cue timing at 30/60/144 Hz and irregular rendering.

## Runtime and controls

Install FFmpeg and ffprobe locally for AVI playback and tests (`sudo dnf install ffmpeg` on a Fedora installation with the appropriate enabled package repository). PCM WAV/RRA effects and soundtrack playback use the Rust reader plus Rodio 0.22.2/CPAL; the Bevy application owns the output device explicitly. Linux builds need ALSA development headers (`alsa-lib-devel`; CI uses `libasound2-dev`). There is no downloaded codec binary, proprietary payload or video embedded in this repository. Missing FFmpeg or media returns a diagnostic to the owning menu.

After import/conversion, run:

```sh
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --frontend on
```

Audio is on by default for imported data. `--audio off` provides explicit silence; `--videos off` disables automatic movies. Settings provide master, engine, effects and music levels, independent toggles and automatic-video policy. The jukebox exposes six artists and all fourteen source tracks. End-of-track advances through available tracks and wraps; absent files are skipped in one bounded traversal, and an entirely absent soundtrack stops. Leaving race/jukebox ownership stops music. The Videos menu lists local source AVIs in pages. Space pauses/resumes; Enter/Escape or controller South/East skips. The Enter that selects a video cannot also skip it in the same frame. Skip returns to its owning menu without settling a race or changing career state.

Interactive startup may play INTRO. Racing selects START1–6; results select the complete WIN1–6, LOSE1–10, BUSTED1–6 and WRECK1–6 families, with LEVEL1–6 and FINALWIN for progression/finale. Quitting requests no terminal film. Showroom bikes expose their original clips; the Videos catalog exposes all 59 supplied AVIs, including DUEL/JESSIE. Selection is presentation-only and never draws gameplay RNG. Ownership/skip returns to the requesting screen. Reduced flashes suppress automatic films; explicit playback is available. The native release soundtrack uses all fourteen supplied PCM RRA tracks. Legacy MIDI/SBK synthesis is intentionally unimplemented under this alternative soundtrack policy. Original spatial/Doppler/random voice interleaving is not claimed. G5 release acceptance remains separate from implemented media functionality.

Character IDs 3/5/6 select the source F cheer/grunt set; other selectable IDs use M. Hit/crash grunts use a deterministic presentation variant. Pedestrian families provide two speech references (resources 2/3, big-endian word +8 minus `0x44`); invalid/absent references remain silent. Proximity/hit cues use a bounded 180-tick cooldown, source filenames and the shared 128-event priority queue. The five-course census resolves 116 valid voice references (47/16/29/12/12). Actual final source playback records these contextual voices through the same audio device. Cue priorities/cooldowns are explicit modern policies rather than full native RNG parity.

Settings labels the option **SOUND CAPTIONS**: effects, cheers, rider pain and pedestrian voices have timed descriptive labels. Original Schmooze dialogue is readable text. Film speech and spoken pedestrian words have no transcript. The full corpus inventory records 44 films with audio and 15 silent showroom clips; this limitation is stated in the player guide and is not disguised as complete spoken subtitles. [59-video decode receipt](evidence/native-videos.json), [interface/coverage policy](NATIVE_ACCEPTANCE.md).

## Source inventory and cue policy

The pinned source declares 69 effect slots, with intentional duplicate ENGOTHER/ENGCAR11 filenames. Adding three engine variants yields **69 unique loaded effects** on the verified disc. The complete private audio census contains **72 WAV + 14 RRA = 86 RIFF/PCM files**, all PCM8 on this disc; PCM16 is supported using authored fixtures. RRA contains ordinary RIFF/WAVE data. Native comparison checks sample values, rate, channels and count for all 86 files. The three engine variants use bike groups 0–4/5–9/10–14. Native RPM arithmetic retains cached-speed updates, wrapped ratio multiplication, 8192 step limits, crash decrement and airborne increment; bounded pitch conversion matches the source primitive. This is measured scalar compatibility, not an original-runtime listening comparison.

| Resolved event | Effect | Priority |
| --- | --- | --- |
| Swing | SWISHGEN | 20 |
| Hit / Kick | PUNCHHEA / KICK_MW4 | 70 |
| Stolen / Dropped | SWISHCHA / CLUBIMPA | 45 / 40 |
| Crash / Contact / Land | CRASHHAR / CRASHLIT / BOUNCE | 100 / 60 / 50 |
| GetUp / Remount | FOOTSTEP / BMWLONG | 25 / 40 |
| Arrest / Finish / Wrecked | SIRENFST / M_YEAH / CRASHMED | 95 / 90 / 95 |
| Rescue | Explicitly silent assist | — |

Engine, skid and one stable police-ID siren loop are reconciled by ownership. Pursuit/race exit, disabled effects or device reset removes the corresponding loop. The one-siren budget is an intentional global mix policy, not complete native spatial mixing. Pending effects are capped at 128; dedupe retains 512 identities. The device permits three loops and sixteen priority-controlled one-shots, plus music/movie voices. Initial sources are paused and muted before append; aggregate gains are normalized to at most 0.85 to leave headroom. Engine, effects and music buses multiply master gain; video follows music gain. Engine remains independently adjustable when effects are muted. This global normalization can reduce all voices when more sources play. Software mixing checks simultaneous full-scale peaks and repeated pause position; subjective speaker/timbre calibration remains a release review.

Race pause preserves voice ownership and freezes players; resume reuses sources. Cue observation stays outside authority and is invariant across frame schedules. Device initialization failure leaves gameplay running. CPAL loss callbacks retire owned voices, then retry after two seconds. Reconnection restarts current music/loops and resumes movie audio at its clock offset; obsolete one-shots are discarded. Controlled unavailable-output and connected virtual-sink runs passed on Linux. Physical USB/hotplug and other operating systems are not certified.

## Bounded video and timing

The adapter resolves regular files under the local import, rejects URI/traversal/symlink escapes, limits input to 256 MiB and enables FFmpeg's `file,pipe` protocols only. Probe output is capped at 64 KiB/5 seconds. Supported streams are Cinepak/rawvideo and IMA/PCM audio: at most 1920×1080, 60 fps, ten minutes and 36,000 frames. Audio decode is capped at 128 MiB/20 seconds. Active RGBA decoding has a three-frame channel plus one pending future frame. No progress for ten seconds fails cleanly. Decode workers and child processes are killed/reaped when active playback is dropped.

One preparation job and one latest pending request are allowed. Skipping while preparation runs releases UI ownership immediately; that bounded preparation job drains before another starts (up to its decode timeout). During active playback the decoder is stopped immediately. One image/backdrop/label is reused; dimension changes replace the GPU image handle and remove the old asset, preventing stale black textures. This same surface upload helper fixes course framing changes.

A microsecond clock carries nanosecond remainders, includes long unpaused stalls and freezes on pause. While connected, movie audio position is the clock reference; otherwise wall time advances it. The newest decoded frame at or before the clock is displayed and obsolete frames drop; late decoding never slows gameplay authority. Ten-minute synthetic schedules, a five-second stall, frame starvation/drop, immediate/paused skip and actual source video rendering are checked independently from decoded audio equality. Four source AVIs match native header/frame counts and exact IMA audio samples. The Part 19 corpus audit additionally decodes all 59 supplied films (24,900 RGBA frames and every audio stream) through the production adapter and verifies child reaping. RGB decoder bit identity against the C++ reference is **not** claimed.

`--media-report local/new-report.json` writes bounded versioned cue timestamps, voice counts, device status and movie counters to a new, source-safe path. Capture metadata adds clock/frame/device/mix observations. Source cue traces, raw audio, configurations, screenshots and movies remain ignored local acceptance data. Public receipts contain aggregate counts, hashes and review outcomes only. The report samples ownership before application destruction, so a replay ending mid-race can legitimately report active voices; the menu-exit tour verifies zero owners after cleanup.

[Planner](../crates/media/src/cues.rs), [bounded video](../crates/media/src/video.rs), [device](../apps/game/src/media/device.rs), [tests](../crates/media/tests/media.rs), [Part 18 evidence](evidence/part-18.md).
