# Rider animation, identity and review

`crates/simulation/src/animation.rs` decodes the supported EXE descriptors, two GLOBAL.RSC CANS banks and fallback/attack tables at startup. The binary relocation schema contains addresses/counts only; source durations, controls, flags and frame references are read from the verified local files. Runtime state is a named animation player, not an emulated memory object. `riding.rs` maps domain riding, attack, crash and on-foot poses into this controller. No pixel size or render frame count controls a deadline or hit window.

## Timing, states and transitions

The source CANS delay is converted by integer division by 20 to simulation ticks. A player returns the frame selected **before** advancing. It performs at most one transition per call, accumulates deadlines from the previous deadline, honors explicit loop targets and queued states, and preserves a valid player when an unavailable selection fails. One source fallback is attempted. Bank/frame/count/loop/duration/view bounds and required semantic states are validated before a race starts.

Normal riding chooses idle, acceleration, normal or tucked states from source speed/force/lateral gates. Yaw/roll view selection follows source descriptor thresholds and clamping; the presentation applies source roll and the yaw clamp. Small threshold changes produce the source's discrete view changes, not a smoothed replacement. Two banks (rider/police), all available sequences and nine yaw/roll pairs are compared with the native APIs.

Punch, kick and backhand use the imported side/kind/equipped-weapon table. Contact is the descriptor's `0x600000` window, exported in the authoritative snapshot. Phase 11 will use that window when resolving opponents/hits; this animation gate does not claim a completed combat system. Equipped fists/club/chain are typed state. Equipment changes reselect the descriptor, so drop/theft inputs cannot retain the previous weapon's attack image. Weapon geometry remains in its registered source frames; no floating independently placed weapon is invented.

Crash and slide poses consume authoritative bike state. Airborne crash, moving slide and low-speed recovery use explicit speed/height gates. Get-up queues standing, walking is an explicit pose, and remount queues riding. Pose changes can interrupt these visual sequences. Detached body impulses, walking-to-bike decisions, arrest and actual recovery gameplay remain Phase 12/14. Tests cover those presentation inputs independently of future producers.

Transitions use **hard cuts at one world ground anchor**. Every frame uses its own logical dimensions/hotspot. There is no cross-fade that delays contact or changes gameplay timing. Attack-to-crash and get-up-to-walk tests preserve the anchor while resetting the new sequence's timing. Non-looping preview clips stop at their state boundary instead of displaying an unrelated fallback frame in the contact sheet.

## Pixels, pivots, palettes and shadows

`crates/render/src/rider.rs` loads the local high-resolution BOB and shadow banks into immutable assets. There are 326 source slots, with empty slots preserved as explicit no-body draws; this disc has 186 nonempty rider frames. Optional sequence 43 references five empty body slots; they remain no-draw frames rather than showing an unrelated fallback pose. Its contact sheet labels those slots explicitly. RRAN metadata defines logical size and hotspot for both body and shadow. Bitmap resolution is independent of those dimensions. All runtime frames are resolved before drawing; no disk reads happen in the render loop.

Runtime rider identities use all **15** imported remaps. The earlier catalog's ten-preview contract stays stable, so existing catalog hashes are not invalidated by widening the runtime bank. Palette remapping precedes RGBA conversion/compositing and preserves transparent index zero. All 2,790 nonempty color/frame combinations are compared with the native `rider_texture` API by cropped indexed-pixel SHA-256. The last identity (14) is also exercised through the actual renderer exporter.

Mirroring is driven by source graphics flag **1**, not by steering sign. Flag `0x10` instead belongs to roll correction. Therefore source left/right exceptions, weapon asymmetry and distinct clothing/logos retain their authored view policy; the engine never guesses that a left pose should be a mirror of a right pose. Tests and the imported view audit cover both directions. Source view size flags `0x10000`/`0x20000` multiply logical size. Mirrored frames reflect the horizontal hotspot as well as the UV direction, so asymmetric pivots do not shift the rider or weapon anchor. Body roll rotates about the registered world anchor; shadow roll stays zero.

The shadow is drawn first at terrain contact, using its own pivot, source transparent-index-zero mask (nonzero indexed texels are opaque). Rider height only moves the body. Source terrain/road depth still occludes actors; shadows are not clipped by the body's ground row. An authored rendered test checks that airborne height cannot move the shadow and that different frame sizes/pivots keep the same anchor. The general single-image enhancement override remains a legacy preview path; complete per-frame reviewed AI enhancement packs belong to Phase 24.

## Offline inspector and contact sheets

```sh
cargo build --locked --release -p rr-render --bin rr-riders
python3 tools/animation/preview.py --data local/cdrom/ROADRASH \
  --output local/animation-preview --color 0 --binary target/release/rr-riders
```

Use `../target/release/rr-riders` for the shared workspace build. Pillow is needed only by the sheet generator. Open the generated `local/animation-preview/index.html` in a browser; it is standalone and works offline. All commercial images stay local.

The inspector offers sequence selection, previous/next simulation tick, a tick slider, play/pause, 0.25×/0.5×/1×/2× preview speeds, a height-only preview and labeled contact sheets for all 48 available rider sequences. It reports source frame, timing, logical size, pivot, contact flag, mirror flag and identity. The height control demonstrates the independent ground shadow. Changing sequence immediately demonstrates the fixed-anchor hard-cut policy. These controls never write to a game simulation.

The export includes a JSON manifest and one labeled PNG sheet per available sequence. Reproduction and [Part 10 acceptance](evidence/part-10.md) include native comparisons, browser interaction review, graphical replays and authored tests. This is source-port comparison, not a claim to running the original Windows executable.

## Part 19 complete pedestrian bank

The source factory now imports all seven pedestrian CANS sequences and view flags. Crossing, waiting, hit and recovery choose their authoritative state's source timing; first-drawable previews are superseded. Five courses resolve 168 complete appearances; seven confirmed absent queries remain collision-gated. The 25-case matrix checks 66,325 poses and 75 exact raster/depth pairs. [Matrix](evidence/native-render-matrix.json), [functional contract](NATIVE_ACCEPTANCE.md).
