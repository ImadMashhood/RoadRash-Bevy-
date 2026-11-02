# Combat and contact contract

**Active accuracy gate:** [collision, fighting and crash priority](COLLISION_COMBAT_PRIORITY.md). New native races record `combat_version: 1`; omitted/version-0 recordings retain the historical rules. The complete contact classifier and full fight/recovery parity remain open.

Schema 4 adds ordered encounter systems to source-unit handling. `combat.rs` defines immutable attack windows, stamina, reach, target ordering, unique weapon ownership and cosmetic result events. `encounter.rs` owns the movement/contact/combat/progression transactions; `recovery.rs` owns detached motion. They contain no Bevy or asset-cache dependencies. Configuration is validated once before a race. Serializing an authored setup provides an editable, replayable rules boundary; imported source tables remain private.

## Evidence and source boundary

Research uses arkits/road-rash revision `7aecf6044f2dd773b925b325823f30d648d61c89`, particularly [fight](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race.cpp), [crash primitives](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/crash.cpp) and [animation](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/actor_animation.cpp). These are research references, not code copied into this project. The disc and executable hashes remain those in the README. The separately authored native adapter compares public APIs: 108 generic-motion vectors and 35 reason-scaled stamina cases. Existing Phase 10 checks establish the imported animation player's timing. The current expanded comparison also covers 458 native attack-timing observations and 24 first/repeated crash-entry observations. Broader combat decisions are verified by authored regression scenarios, not asserted as full native race parity.

## Tick phases and attacks

Input records a `Request { kind, side }`. Movement takes one eligible mounted target from the start-of-stage position set, starts the imported clip and freezes the selected target and weapon in a `Swing`. Contacts can cancel it before Combat. Combat processes ascending stable ActorId values. A successful earlier hit can stun/cancel a later attacker on the same tick. Removal invalidates a target safely; it never retargets halfway through a swing.

`Definition::from_library` reads each CANS descriptor's signed positive duration. The player selects the current frame before advancing: frame zero includes its deadline; subsequent frame starts are cumulative durations plus one. Version 1 generates active windows using the animation player: retained source flags `0x600000` OR the counter **after** advancement equal to 2. Version 0 retains the older selected-frame-index-2 fallback. A native 458-tick probe detected and verifies the one-tick boundary correction. Windows are half-open `[start,end)` in simulation ticks. Each swing can resolve one contact. A missed active tick may try later in its active window; recovery and stun prevent attacks. Repeated overlap cannot repeat damage. Holding a physical attack action requests another attack after the prior swing ends.

Target acquisition requires the same route and branch and longitudinal separation less than 480 source Q8 units. Nearest longitudinal distance wins; a target behind wins an equal-distance front/back tie, then lower ActorId wins. An explicit side filters candidates; Auto chooses the selected target's lateral side. Contact rechecks the target's live position and strict boundaries: longitudinal distance `<64`, height difference `<12288`, lateral distance `<[20480,28672,32768,20480]` for fists, club, chain and kick. Crossing the boundary misses regardless of visual overlap.

## Damage, stun and displacement

Stamina has distinct base, maximum, current health and strength. Damage scales strength by maximum/base in Q8, floors the intermediate result at half strength, then uses Q8 weapon multipliers `[256,320,384,64]`. Applying damage lowers maximum by `4*p` with a base/2 floor and health by `64*p` with a zero floor. Mounted living riders regenerate one point per tick up to maximum. Reason codes preserve source scaling: 0/6 no damage; 1/4/5 unchanged; 2 multiply by 7/4 with original rounding; 3 divide by 8. Bike condition is independent. Zero health or condition enters linked crash recovery.

A living hit selects source reaction 9 and queues ride 2. Stun expires at `tick >= stun_until`; hit immunity permits another damage event only at `tick > hit_until`. Difficulty supplies `(5-level)*6` immunity ticks for zero-based level 0–4. A new hit on the expiry tick cancels the newly started swing and renews stun. Punch/backhand lean is ±3000; kick lean is ±11000; attacker lean is opposite ±1000; all are bounded by 11776. Backhand transfers 300 speed units from victim to attacker. Kicks also damage bike condition by damage/4+1. These once-per-swing displacement rules deliberately avoid repeated overlap impulses.

## Equipment and theft

Inventory is a stable map from WeaponId to one `Item {kind, owner}`. Validation rejects duplicate owners, fists as item tokens, missing actors and invalid IDs. An atomic transfer requires the current owner and an empty recipient; a competing attempt cannot duplicate the token. Dropping or despawning leaves an unowned token and cancels the swing. A command cannot conjure equipment.

A fists, non-kick attacker may steal from an equipped opponent during its early attack frames (index <2), after the theft deadline, on gameplay RNG draw `&3 ==3`. The successful thief gets a 300-tick deadline; ownership and carried visuals update in the same transaction. Both obsolete swings are cancelled and return to ride. This immediate ride transition is a deliberate modern policy; the upstream theft transition clips are not reproduced. The competing-thief scenario checks one owner and the second attack's normal damage after the transfer.

## Contact policy and effects

Source startup reads normal dimensions and scenery placement. Bike contacts use strict road-relative AABBs, height, branch and relative speed. They separate lateral overlap once per continuous contact. A speed gap above 6000 knocks off the faster rider using source rider/bike vertical impulse pair 512/256; lower gaps transfer mean speed. The complete upstream contact classifier and traffic collision solver are not ported here; this deterministic separator is an intended model. Traffic/AI belongs to later phases.

Billboard class 0/1/2 with the actor flag clear is solid; other classes are decorative. Sweeps compare local path progress, lateral width and height, avoiding global/local coordinate confusion. Authored rail/cliff categories are available; road edges remain traversable shoulders, with surface drag handled by the existing bike model. A cliff forces a crash; a rail scales damage with reason 3. Continuous overlap records one incident until separation. Solid hazards and repeated recovery impacts cannot delete or replace the rider body.

Typed Swing/Hit/Stolen/Contact/Crash/GetUp/Remount/Rescue/terminal events report resolved gameplay. `set_effects_enabled(false)` suppresses that feed only. Events, renderer proxies and the effects setting are excluded from canonical state; tests compare all checkpoints and RNG with effects on/off. Sound assets, particles and full result screens are later phases. The small race HUD reads snapshots only.

## Reproduction

```bash
cargo test --locked -p rr-simulation --test encounters
cargo run --locked -p rr-simulation --bin rr-replay -- docs/fixtures/encounter-replay.json irregular /tmp/encounter-report.json
# Private verified data and an already built pinned native reference:
cargo build --locked --release --target-dir ../target -p rr-simulation --bins
source local/reference-tools/env.sh
python3 tools/reference/check_encounters.py --data local/cdrom/ROADRASH --output local/encounter-acceptance
```

[18 isolated tick-range scenarios](fixtures/combat-scenarios.json), [Part 11 acceptance](evidence/part-11.md), [recovery contract](RECOVERY.md). Public fixtures use independently authored descriptors and generated trigonometric tables; no imported animation/configuration/texture payload is published.
