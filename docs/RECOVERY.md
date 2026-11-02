# Crash and recovery contract

**Current native rule profile:** `combat_version: 1` preserves signed crash-entry impulses and ignores entry while a crash body is already active, matching the measured C++ entry API. It does not reset the incident, body, elapsed time or animation or emit another crash event. Historical version-0 recordings retain their repeated-impulse/reset policy described below. Full multi-tick trajectory parity remains an [open accuracy gate](COLLISION_COMBAT_PRIORITY.md).

`recovery.rs` owns linked rider/bike state; `topology.rs` owns validated bidirectional route navigation. Schema 4 serializes every hidden motion, phase, animation, ownership, assist and terminal field. Rendering consumes copies and cannot remove a recovery target. [Combat](COMBAT.md) explains impact producers and [Part 12 acceptance](evidence/part-12.md) records verification.

## Identity and state progression

Every owner has stable bike entity `owner*2` and rider-body entity `owner*2+1`. Mounted state has no active detached body. The first impact clones physical motion into a linked body, adds source launch height 19200, gives independent impulses and animation clocks to body and bike, and enters Sliding. Repeated impacts preserve body identity and position, reset the phase/animation to Sliding and add impulses. Explicit incoming impacts reserve the recovery transition before movement: a collision on the final get-up/remount tick cannot discard the body or emit a stale remount event.

The normal sequence is Mounted → Sliding → GettingUp → Standing → Walking → Mounting → Mounted. The source entry clips are 33 (normal), 29 (reason 2) and 35 (reasons 4/5); low body height selects tumble/slide 34/29. Stopped grounded bodies select get-up 36 queued 37. Standing waits more than one completed loop before walk 38. Arrival selects mount 41 queued ride 2. Mount finishes only when the animation has left 41. The riderless bike independently selects 46/44 or stopped 48. All clips advance through the same imported tick-driven player verified in Phase 10.

The detached body has its own speed, heading, height, gravity, drag and suspension. Source startup reads the half-object behavior table at VA 0x463c10 and normal dimensions at 0x4643e0; values are named fields after loading. Generic motion preserves source sine selection, multiply wrap and arithmetic shift. Vertical contact reuses the tested bike vertical primitive with the body law. No frame delta or GPU state enters movement. The native adapter checks 108 displacement combinations including negative speed, all quadrants and nonzero acceleration. Full recovery timing is regression-tested; exact original Windows timing is unmeasured.

The authored flat-road regression records entry speeds 1000/5000/10000, respective slide travel 99/1092/3257 Q8 units and get-up entry after 41/93/152 subsequent ticks. These figures are the authored engine regression baseline, not measurements from the original game. The private source showcase also records every phase's first observed tick.

## Navigation and geometry

Location is `(path, local progress Q8, lateral, height, heading)`. Every road node has reciprocal endpoint portals; split/merge control nodes resolve into actual road connections at startup. Shortest portal-count routing uses sorted endpoint/node ties. A bike behind is approached backwards; a neighboring branch requires its connecting portal. Mere spatial proximity never permits crossing disconnected roads. Selected-route global progress is a separate optional projection, not an alias for path-local progress.

Auto walking steers to the bike's source offset of -6000 lateral units, with approximate source atan and speed 1200. Optional per-tick Walk commands select signed longitudinal/lateral input; an explicit zero command stops walking. Every Q8 interval traversed by detached motion integrates its own path's grade, including reverse motion and exact portal endpoints. Grade tests cross a +12/-8 split boundary, and the neighboring-path walking scenario uses a positive grade. Bike association stays linked to its owner throughout.

The classic renderer still follows the selected course route. An off-route body without a selected-route projection is temporarily culled; its authoritative state and recovery navigation continue. It is visible again when it returns to the selected route. This is a declared presentation limit, not a fallback teleport. Detached off-route motion integrates its own path grade; surface/shoulder drag still uses the selected-route ground profile. Presentation aliases use a high-bit bike ID in copied snapshots, retain source frame/pivot/mirror/shadow behavior, and never allocate simulation entities. During mount the combined source clip replaces the separate bike proxy. Palette banks are preloaded for all requested/replayed rider colors.

## Remount, terminals and fallbacks

Remount requires a valid, available linked bike on the same path, longitudinal difference ≤9 and `abs(bike lateral - body lateral - 6000) <6001`. Facing must be within a quarter-turn of the target. The distance predicate comes from the pinned native `recover_crash_actor`; the orientation gate is an intentional added rule. Arrival is checked before walking motion to avoid moving past an already valid target. A source-style elapsed limit above 1200 ticks permits mounting only after topology connectivity has been established; it cannot rescue an inaccessible branch. A broken/missing bike ends as Wrecked. Missing/invalid/disconnected target detection is bounded and does not loop indefinitely.

Progression resolves same-tick outcomes in this order: Arrested > Finished > Wrecked. It clears obsolete cosmetic events and activates exactly one terminal event, with an immutable completion tick. Later requests cannot replace a committed outcome. An imported course uses its level-specific selected-route finish; a detached rider can cross it while the bike remains behind. Arrest is a domain event entry point; police pursuit, arrest detection and result menus remain later phases.

`fast_recovery` and `rescue_unreachable` default to false. Fast recovery skips the usual standing/get-up wait and records assistance. Rescue explicitly relocates an invalid/disconnected body to a valid linked bike; it never revives a missing bike. Both settings are part of the canonical setup and replay identity. CLI assist flags apply to live races; playback uses its recorded assist settings. The test suite verifies that fast recovery is faster, default recovery remains unassisted, rescue requires its setting and effects toggles do not change state.

## Play and reproduce

```bash
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course CITY --practice-riders 2
# Explicit optional assists:
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course CITY --practice-riders 2 --fast-recovery on --rescue-recovery on
cargo test --locked -p rr-simulation --test encounters
```

Z/X/C request punch/kick/backhand with automatic side selection. Arrow/drive directions walk while detached; release them for auto return. `--effects off` suppresses the cosmetic event feed. `--practice-riders 0..14` adds explicitly constant-drive practice riders with unique equipment, not Phase 13 intelligence. `--capture-frame N` chooses the review frame; captures and source-derived replays stay private. Full traffic, opponent behavior, police, career and original-runtime parity are not claimed.
