# Traffic, pedestrians and police contract

Schema 5 owns a separate road-user population beside the 15-rider field. Stable IDs, route locations, motion, phase, spawn consumption, collision debounce and pursuit are authoritative. Source textures and sound voices stay outside that state. Pools are bounded to **16 vehicles, 15 pedestrians and one police actor**; at most 32 road users coexist. The stress scenario combines all pools with 15 racers, attacks, impacts and arrest.

## Research and scope

Pinned native reference `7aecf6044f2dd773b925b325823f30d648d61c89`: [traffic](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/traffic.cpp), [pedestrian pool](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/pedestrians.cpp), [pedestrian behavior](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_pedestrians.cpp), [police predicate](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/police.cpp), [RRAN aliasing](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/animation.cpp).

Native API probes verify density configuration, arrest thresholds, vehicle selection and personality/cadence decoding. Complete traffic physics, native scan-list interleaving, pedestrian animation tables and full police behavior are not asserted as ported byte-for-byte. The policies below are explicit engine behavior. Original Windows runtime comparisons remain waived by the owner.

## Events, traffic and intersections

Startup converts raw event type to runtime type (`raw + 1`) and converts bounded course events into path-local zone records with source type, lane span (`parameter[0]`), signed density/speed (`parameter[1]`) and flags (`parameter[2]`). Signed negative density sets cruise to `-density` and rate to `-density*100`; zero selects density 6; positive density reloads timers with `rate*density*60/256`. Native comparisons cover six successive configurations, including negative-rate changes retaining reloads. Oncoming/behind/ahead timers advance once per simulation tick using the player's source-distance delta and the source rate term.

Zone entry applies configuration once and optionally creates a type-1/2/3 intersection controller. Exit removes that controller; no duplicate controller is created while resident. There are at most ten controllers. They think every 30 ticks and use the documented 600/180/780 cycle, with bounded crossing spawn opportunities. This implementation uses player-centered zone residency and preferred road-relative corridors, rather than the entire original ring-window setup, signal phase and lane-change machinery. Lane parameters remain available in setup; the current corridor policy is explicitly simplified.

Longitudinal vehicles advance through real topology portals, with separate forward/oncoming directions. Crossing vehicles move laterally at a fixed path anchor and despawn beyond the crossing bounds. Source wrapper masks, inherited CANS pairing and sorted sequential ANIM identities determine accepted vehicle classes. The original chooser excludes the last vehicle and filters masks with 0x1c0000 by kind; 640 native cases across all five vehicle libraries match. Authored configurations without masks retain an explicit generic vehicle choice.

A spawn expiry consumes a tagged lane and identity draw in the documented engine order; caps prevent allocation when full. Exact native full-pool/jitter/RNG call interleaving is not claimed. Requests, counters and IDs are canonical, and IDs do not recycle into stale contacts. Vehicle speed/orientation affects collision severity: relative forward speed for rear/head-on contact and combined crossing speed for lateral impacts. A gap above 6000 can detach the rider using the existing source impulse pair; lesser contacts slow/damage without repeated overlap damage. Continuous contact pairs debounce consequences until separation.

## Pedestrians and source appearances

Source DZHR actor requests retain family, selector, subtype, path and source position. Crossing types 4–7 wait for proximity, then move across the road; other subtypes remain standing. Requests are consumed once even if a pool is full. Traffic can interrupt a crossing without a racer nearby. One hit per pedestrian lifetime records reaction state and the pedestrian-hit counter; this is a scoring hook, not invented career money. Hit/recovery waits are explicit 180/90-tick policy values. Reactions and collision counts are independent of rendering.

The classic renderer resolves category-2 family graphics at startup. If the usual static view is empty it finds a drawable source frame; no file reads occur during updates. The five-course level-1 census has 719 pedestrian requests and 712 drawable appearances after fallback. All seven unavailable static lookups also return no appearance in the pinned native API. They remain counted and are excluded from live collisions, preventing invisible hazards; this is confirmed source-resource absence at those queried locations. Native pedestrian contacts also require a loaded appearance; complete runtime residency/animation equivalence is unmeasured. The source preview is used for walking, with mirroring and a knocked-down rotation; the complete per-pedestrian CANS pose/timing library is a declared fidelity limit.

Vehicle previews retain source view-specific hotspots while sharing aliased pixels. The RRAN three-view fallback now handles resources whose pixels exist only in the third view. All 54 vehicle models and 162 cached view cases across five courses have nontransparent pixels. Police use the 45-slot LOCOP/ANIM-2 bank (33 nonempty slots), with the imported bank-1 animation player. Commercial pixels and decoded animation/configuration payloads stay private.

## Patrol, pursuit, warning and arrest

Police transition Patrol → Chase → Disengaged, with explicit Hit/Recovering states. Detection uses same-path proximity; an existing pursuit can follow connecting portals to another path, never a spatial shortcut across disconnected roads. Pursuit speed and an adjacent lateral corridor are intended policies, capped at 18000. Traffic can disable a cop; a crashed cop cannot arrest or retain a siren. Periodic spawn opportunities use the 4096-tick cadence after startup, in addition to source roadside requests, and the one-cop cap remains enforced.

The source local arrest predicate requires a found, non-crashed cop; speed below 300 for mounted cases or absolute speed below 1800 for crash-state 2; strict longitudinal difference below 512 and relative speed below 5000. Caller eligibility also requires the same path, lateral proximity, the local player and a chasing cop. This engine maps detached recovery phases to the on-foot threshold and measures their body motion; that phase mapping and additional route/lateral gates are explicit policy. Native probes check 2,240 predicate combinations. Tests cover exact boundaries, a disabled cop and arrest during get-up, standing, walking and mounting.

Arrest enters the existing ordered terminal transaction: Arrested > Finished > Wrecked, exactly once. Recovery movement/swing is stopped and obsolete effects are cleared. Simultaneous attacks, vehicle hits and pursuit cannot open multiple result flows. Full arrest menus/career penalties are Phase 15/17 work.

Siren start/stop and warning cues follow pursuit state. A presentation-side `SirenBus` reconciles owned loop IDs on pause/resume, effects disable, despawn, restart and terminal completion. Actual Bevy adapter tests exercise that lifecycle without changing simulation. The HUD displays pursuit warnings. Phase 16 supplies sound-asset playback; this phase implements cue and loop ownership, not an audible audio engine.

## Routes, visibility and verification

Every road user retains its path and local progress. Movement crosses only real reciprocal portals; disconnected paths cannot collide, pursue or arrest by proximity alone. Selected-route projection is optional. Rendering uses that projection across section boundaries, while off-selected-route agents can be culled without removal from authority. Position interpolation operates only on copies. Non-selected-route visual/ground-profile limits remain documented in [RECOVERY.md](RECOVERY.md).

```bash
cargo test --locked -p rr-simulation --test population
cargo run --locked --release -p rr-render --bin rr-population-art -- local/cdrom/ROADRASH local/catalog
cargo build --locked --release --target-dir ../target --workspace
source local/reference-tools/env.sh
python3 tools/reference/check_population.py --data local/cdrom/ROADRASH --output local/population-acceptance
python3 tools/reference/check_population_art.py --data local/cdrom/ROADRASH --catalog local/catalog --output local/population-art-acceptance
```

[Part 14 evidence](evidence/part-14.md), [opponents](OPPONENTS.md), [authored replay](fixtures/population-replay.json). Checkpoints, source counts, native primitive receipts and private graphical reviews are distinct evidence categories. No full native trajectory/RNG parity or original-runtime calibration is claimed.
