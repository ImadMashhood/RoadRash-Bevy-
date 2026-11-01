# RoadRash-Rust — complete recreation roadmap

A Rust and Bevy recreation of the PC Road Rash experience, with a faithful classic presentation, individually reviewed AI-enhanced textures, and a modular design that leaves room for a later 3D version.

This repository currently contains a plan, not a playable Rust release. All implementation checkboxes below are open until their acceptance evidence exists. The separate local Bevy prototype is exploratory work and has not been imported or certified against this roadmap.

## 1. Scope and definition of complete

The first release covers local single-player racing, original courses and progression, handling, combat, opponents, traffic, pedestrians, police, menus, saves, audio/video, and an optional enhanced texture pack. Multiplayer, mobile/browser delivery, and a full 3D remake are separate projects. Preserve classic behavior first; record intentional changes as selectable enhancements. Completion means a clean install from user-supplied data, full campaign completion, all required content covered, reproducible validation, and no unresolved release-blocking defects.

## 2. Inspection receipt

Inspection date: October 31, 2026. The target GitHub repository was empty when inspected. The local workspace contains a Bevy 0.20.1 prototype with handling, combat, crashes, racers, NPCs, course and asset-generation modules. Its documentation describes reconstructed 3D models and City level 1 experiments. These are useful research inputs, not proof of original-game parity. Existing local files were preserved.

The supplied ZIP contains one 518,402,048-byte ISO. Its ISO9660 signature was verified and all 848 filesystem entries representing files were inventoried. All **341/341 required game files** matched the reference port's SHA-256 manifest. The archive was read as data; game executables were not run.

- ISO SHA-256: `cc07917cb06ef0b3827afd3610f580009c4f42102efd849cdb82657b454bf6ff`
- ROADRASH.EXE: 541,184 bytes; SHA-256 `73d7f9744b5b1a9482b3909950cdf3fe34119fe13d023b031e9c6739a60f18a2`.
- Reference revision: `7aecf6044f2dd773b925b325823f30d648d61c89`.

The reference is a C++20/SDL3 native port, not an automatic Rust decompiler or a complete authoritative specification. Its README reports substantial measured parity, while its scheduler explicitly describes remaining differences. Some historical reverse-engineering notes describe earlier incomplete states; resolve conflicts using current source, tests, and original binary evidence at the pinned revision. Upstream test claims have not been independently rerun in this inspection.

Source inspection covered repository structure, status documentation, format definitions, importer contracts, course representation, sprite definitions, enhancement hooks, and scheduler behavior. It was not a line-by-line audit of every upstream function or a complete playthrough.

## 3. Working rules

- [x] Identify the supplied executable and match the required disc corpus.
- [x] Pin the inspected reference revision.
- [ ] Create a behavior ledger linking every implemented feature to evidence and tests.
- [ ] Establish a redistributable synthetic test corpus.
- [ ] Record provenance and applicable permissions before incorporating upstream code or assets.

Keep source code, user-supplied game data, generated working assets, and approved enhancement packs separate. Owning a disc is not a project distribution policy; public packages should import local game data. No game images, audio, video, ISO, executable, or derived texture pack is included in this documentation commit. Upstream GitHub metadata did not identify a repository license at inspection; do not assume that public source is permission to copy it. Choose and document the project's own code license independently.

## 4. Architecture contract

Start with modules and split into crates when boundaries prove useful. Avoid a generic engine framework before the first playable slice.

```text
apps/game             Bevy application and composition
crates/formats        bounded binary readers, no rendering
crates/content        IDs, schemas, catalogs, mod resolution
crates/simulation     race rules, actors, commands, replay
crates/presentation   classic renderer, animations, HUD/audio
crates/platform       input, persistence, filesystem adapters
tools/import          local disc verification and conversion
tools/asset-review    export, candidate review, pack validation
```

These are proposed paths, not existing commands or packages. Simulation owns gameplay truth. Rendering consumes snapshots and never determines hit results. Use explicit Bevy system ordering for input consumption, spawning, AI, movement, collisions, combat, progression and snapshot extraction. Retain legacy integer/fixed-point arithmetic wherever necessary for parity. A fixed timestep alone does not establish determinism: update order, RNG calls, numeric behavior and content versions also matter.

Represent a position as route/branch identity plus longitudinal progress, lateral displacement and elevation. A classic projection adapter and a future world-space adapter can consume the same course data. Keep sprite pivots, UV rectangles and screen scaling out of physics. Do not replace legacy handling with a general rigid-body solver merely because one is available. The local prototype's Rapier/Avian experiments need separate evaluation before reuse.

Use typed IDs and errors, bounded parsers, data-driven configuration, narrow plugins and explicit ownership. Avoid global mutable state, direct pointer-offset translation, unrestricted unsafe code, and filesystem access inside gameplay systems. Store saves in versioned domain records rather than serializing an entire Bevy world.

## 5. Delivery order and gates

| Gate | Required result | Depends on |
| --- | --- | --- |
| G0 | Verified local import and asset viewer | Baseline, foundation, formats |
| G1 | One course with faithful player driving | G0, simulation, road, input |
| G2 | Complete race with combat and opponents | G1, actors, recovery, traffic |
| G3 | Complete career and audiovisual frontend | G2, menus, saves, media |
| G4 | Reviewed enhanced textures and mod overrides | G0, stable catalog, G2 rendering |
| G5 | Tested desktop release | G3, G4, verification, packaging |

G1 should use original textures and a single measured course before mass upscaling. Each gate needs an executable revision, repeatable test recipe, recorded results, and documented remaining differences. No calendar estimate is implied.

## 6. Detailed implementation checklists

Each item is a deliverable. A checked box requires code or an artifact, verification, and a linked evidence record. Reference links are research entry points; they do not certify that the upstream implementation is exact.

### Phase 01 — Project baseline and evidence

Research: [README.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/README.md), [docs/CURRENT_STATUS.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/CURRENT_STATUS.md), [docs/PROGRESS.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/PROGRESS.md).

- [ ] **01.01** Record scope and supported disc revisions.
- [ ] **01.02** Choose classic visual fidelity targets.
- [ ] **01.03** Inventory every menu and transition.
- [ ] **01.04** Inventory courses and difficulty variants.
- [ ] **01.05** Inventory bikes and playable characters.
- [ ] **01.06** Record original control mappings.
- [ ] **01.07** Record observed bugs separately from desired changes.
- [ ] **01.08** Create evidence ledger with confidence levels.
- [ ] **01.09** Choose initial desktop platform.
- [ ] **01.10** Define fidelity and enhanced modes.
- [ ] **01.11** Set milestone owners and dependencies.
- [ ] **01.12** Record release acceptance criteria.

**Exit evidence:** a reproducible demonstration of project baseline and evidence, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 02 — Repository and Rust foundation

Research: [CMakeLists.txt](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/CMakeLists.txt), [docs/TOOLING.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/TOOLING.md).

- [ ] **02.01** Create Cargo workspace and game binary.
- [ ] **02.02** Pin Rust toolchain and Bevy version.
- [ ] **02.03** Commit Cargo.lock for the application.
- [ ] **02.04** Choose explicit Bevy feature set.
- [ ] **02.05** Document OS libraries and graphics requirements.
- [ ] **02.06** Add rustfmt and Clippy checks.
- [ ] **02.07** Add headless core test job.
- [ ] **02.08** Add desktop build matrix.
- [ ] **02.09** Create synthetic redistributable fixture pack.
- [ ] **02.10** Configure ignored private asset directories.
- [ ] **02.11** Create architecture decision records.
- [ ] **02.12** Document contributor setup from clean clone.

**Exit evidence:** a reproducible demonstration of repository and rust foundation, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 03 — Disc import and provenance

Research: [src/importer.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/importer.cpp), [src/iso9660.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/iso9660.cpp), [src/reference_hashes.inc](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/reference_hashes.inc).

- [ ] **03.01** Implement bounded ISO9660 reader or vetted library adapter.
- [ ] **03.02** Accept extracted ROADRASH directory.
- [ ] **03.03** Verify supported executable hash.
- [ ] **03.04** Verify complete required file manifest.
- [ ] **03.05** Report missing and mismatched files individually.
- [ ] **03.06** Reject path traversal and escaping symlinks.
- [ ] **03.07** Bound file size and allocation.
- [ ] **03.08** Stage import before atomic activation.
- [ ] **03.09** Preserve existing installation on failed import.
- [ ] **03.10** Support cancellation and interrupted import cleanup.
- [ ] **03.11** Record source hashes and importer version.
- [ ] **03.12** Test Unicode paths and read-only sources.

**Exit evidence:** a reproducible demonstration of disc import and provenance, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 04 — Binary formats and asset decoding

Research: [src/resource.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/resource.cpp), [src/cel.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/cel.cpp), [src/rri.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/rri.cpp), [src/bob.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/bob.cpp), [src/exe_tables.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/exe_tables.cpp).

- [ ] **04.01** Decode resource directory with explicit little-endian reads.
- [ ] **04.02** Validate offsets and payload lengths.
- [ ] **04.03** Preserve unknown fields in inspection output.
- [ ] **04.04** Decode CEL textures and palettes.
- [ ] **04.05** Decode RRI frontend images.
- [ ] **04.06** Decode BOB rider and police frames.
- [ ] **04.07** Decode shadow assets and transparency.
- [ ] **04.08** Extract executable tables only for known binary.
- [ ] **04.09** Preserve indexed originals and rider remaps.
- [ ] **04.10** Export lossless RGBA previews.
- [ ] **04.11** Create corpus coverage report.
- [ ] **04.12** Fuzz truncated and malformed synthetic inputs.

**Exit evidence:** a reproducible demonstration of binary formats and asset decoding, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 05 — Content catalog and tooling

Research: [src/assets.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/assets.cpp), [src/family.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/family.cpp), [src/actor_descriptors.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/actor_descriptors.cpp).

- [ ] **05.01** Assign stable namespaced asset identifiers.
- [ ] **05.02** Record source file and resource index.
- [ ] **05.03** Record frame dimensions and pivot.
- [ ] **05.04** Classify road terrain scenery rider UI audio video assets.
- [ ] **05.05** Track shared pixels without conflating semantic identities.
- [ ] **05.06** Add catalog schema version.
- [ ] **05.07** Export searchable contact sheets.
- [ ] **05.08** Add asset inspector with palette switching.
- [ ] **05.09** Build dependency graph for generated assets.
- [ ] **05.10** Cache by source and recipe hash.
- [ ] **05.11** Report missing references before launch.
- [ ] **05.12** Document reproducible rebuild commands.

**Exit evidence:** a reproducible demonstration of content catalog and tooling, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 06 — Simulation architecture

Research: [src/race_state.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_state.hpp), [src/race_schedule.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_schedule.cpp), [src/det_math.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/det_math.cpp).

- [ ] **06.01** Define simulation tick and unit types.
- [ ] **06.02** Keep screen coordinates outside simulation.
- [ ] **06.03** Define stable actor identifiers.
- [ ] **06.04** Represent route branch section and progress explicitly.
- [ ] **06.05** Define command input consumed at tick boundaries.
- [ ] **06.06** Define immutable presentation snapshots.
- [ ] **06.07** Order simulation stages explicitly.
- [ ] **06.08** Use seeded and recorded random streams.
- [ ] **06.09** Preserve original arithmetic where parity demands it.
- [ ] **06.10** Implement headless race runner.
- [ ] **06.11** Record canonical state hashes.
- [ ] **06.12** Test identical replay at different render frame rates.

**Exit evidence:** a reproducible demonstration of simulation architecture, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 07 — Course reconstruction

Research: [src/course.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course.cpp), [src/course_scene.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course_scene.cpp), [src/track_ring.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/track_ring.cpp), [src/terrain.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/terrain.cpp).

- [ ] **07.01** Decode section graph and root.
- [ ] **07.02** Validate references and terminal nodes.
- [ ] **07.03** Reproduce split and merge traversal.
- [ ] **07.04** Decode bend and rise independently.
- [ ] **07.05** Preserve original lane and shoulder units.
- [ ] **07.06** Reproduce interval stepping at boundaries.
- [ ] **07.07** Decode elevations and side flags.
- [ ] **07.08** Decode event zones and finish markers.
- [ ] **07.09** Decode scenery and family placements.
- [ ] **07.10** Inspect all five courses in viewer.
- [ ] **07.11** Compare branch seams and roadside placement.
- [ ] **07.12** Test forward and reverse traversal.

**Exit evidence:** a reproducible demonstration of course reconstruction, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 08 — Classic road rendering

Research: [src/road.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/road.cpp), [src/road_draw.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/road_draw.cpp), [src/raster.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/raster.cpp), [src/framebuffer.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/framebuffer.cpp).

- [ ] **08.01** Create classic camera specification.
- [ ] **08.02** Implement road projection using sampled route.
- [ ] **08.03** Render horizon and background layers.
- [ ] **08.04** Render hills cliffs buildings and shoulders.
- [ ] **08.05** Match road markings and surface transitions.
- [ ] **08.06** Implement sprite depth ordering.
- [ ] **08.07** Match clipping at near plane and viewport edges.
- [ ] **08.08** Preserve sprite ground contact and scale.
- [ ] **08.09** Support original aspect presentation.
- [ ] **08.10** Add optional widescreen framing policy.
- [ ] **08.11** Compare fixed camera captures at selected checkpoints.
- [ ] **08.12** Separate render interpolation from simulation ticks.

**Exit evidence:** a reproducible demonstration of classic road rendering, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 09 — Input and player handling

Research: [src/input.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/input.cpp), [src/bike.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/bike.cpp), [src/race_actions.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_actions.cpp).

- [ ] **09.01** Implement remappable action map.
- [ ] **09.02** Implement keyboard and gamepad input.
- [ ] **09.03** Calibrate dead zones and analog steering.
- [ ] **09.04** Reproduce acceleration and gearing.
- [ ] **09.05** Reproduce braking and coasting.
- [ ] **09.06** Reproduce lean and lateral motion.
- [ ] **09.07** Reproduce hill and airborne behavior.
- [ ] **09.08** Reproduce surface-dependent handling.
- [ ] **09.09** Implement bike damage and failure.
- [ ] **09.10** Validate every bike specification.
- [ ] **09.11** Pause safely on focus loss and disconnect.
- [ ] **09.12** Compare acceleration braking and cornering traces.

**Exit evidence:** a reproducible demonstration of input and player handling, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 10 — Rider animation and palette fidelity

Research: [src/rider_sprites.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/rider_sprites.cpp), [src/actor_animation.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/actor_animation.cpp), [src/animation.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/animation.cpp).

- [ ] **10.01** Map animation states to frame sequences.
- [ ] **10.02** Preserve original per-frame timing.
- [ ] **10.03** Implement steering and lean poses.
- [ ] **10.04** Implement punch kick and backhand poses.
- [ ] **10.05** Implement weapon carrying poses.
- [ ] **10.06** Implement crash and slide frames.
- [ ] **10.07** Implement getting up and on-foot states.
- [ ] **10.08** Preserve palette-based rider identity.
- [ ] **10.09** Match shadow placement and transparency.
- [ ] **10.10** Test mirrored sprites for asymmetric details.
- [ ] **10.11** Provide animation contact sheet playback.
- [ ] **10.12** Validate transitions without frame popping.

**Exit evidence:** a reproducible demonstration of rider animation and palette fidelity, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 11 — Combat and contact

Research: [src/race_actions.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_actions.cpp), [src/crash.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/crash.cpp), [src/race_helpers.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_helpers.cpp).

- [ ] **11.01** Specify attack startup active and recovery windows.
- [ ] **11.02** Define left and right reach.
- [ ] **11.03** Resolve target selection deterministically.
- [ ] **11.04** Implement punch damage and reactions.
- [ ] **11.05** Implement kick displacement and reactions.
- [ ] **11.06** Implement weapon ownership and theft.
- [ ] **11.07** Implement stun and health depletion.
- [ ] **11.08** Resolve simultaneous attacks consistently.
- [ ] **11.09** Separate cosmetic effects from hit logic.
- [ ] **11.10** Validate bike-to-bike contact.
- [ ] **11.11** Record isolated combat scenarios.
- [ ] **11.12** Compare outcomes against original captures.

**Exit evidence:** a reproducible demonstration of combat and contact, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 12 — Crashes and recovery

Research: [src/crash.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/crash.cpp), [src/race_schedule.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_schedule.cpp).

- [ ] **12.01** Separate rider and bike crash states.
- [ ] **12.02** Implement slide and tumble behavior.
- [ ] **12.03** Implement off-road collision consequences.
- [ ] **12.04** Track detached bike position.
- [ ] **12.05** Implement rider get-up timing.
- [ ] **12.06** Implement on-foot movement and recovery target.
- [ ] **12.07** Implement remount conditions.
- [ ] **12.08** Handle repeated collision during recovery.
- [ ] **12.09** Handle finish and arrest during recovery.
- [ ] **12.10** Prevent stuck or unreachable recovery loops.
- [ ] **12.11** Test recovery on slopes and branches.
- [ ] **12.12** Document deliberate accessibility assists.

**Exit evidence:** a reproducible demonstration of crashes and recovery, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 13 — Opponent intelligence

Research: [src/race.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race.cpp), [src/race_schedule.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_schedule.cpp), [src/race_helpers.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_helpers.cpp).

- [ ] **13.01** Recover rider scheduling intervals.
- [ ] **13.02** Preserve meaningful actor update order.
- [ ] **13.03** Implement route following.
- [ ] **13.04** Implement speed and cornering decisions.
- [ ] **13.05** Implement overtaking and avoidance.
- [ ] **13.06** Implement aggression and target choice.
- [ ] **13.07** Implement combat decisions.
- [ ] **13.08** Implement crash recovery decisions.
- [ ] **13.09** Match rider personalities where evidenced.
- [ ] **13.10** Audit RNG consumption per behavior.
- [ ] **13.11** Test full original field size.
- [ ] **13.12** Compare long-run position and behavior traces.

**Exit evidence:** a reproducible demonstration of opponent intelligence, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 14 — Traffic pedestrians and police

Research: [src/traffic.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/traffic.cpp), [src/pedestrians.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/pedestrians.cpp), [src/police.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/police.cpp).

- [ ] **14.01** Decode traffic spawn and density events.
- [ ] **14.02** Implement lane following and crossing traffic.
- [ ] **14.03** Implement traffic collision responses.
- [ ] **14.04** Implement pedestrian spawning and crossing.
- [ ] **14.05** Implement pedestrian hit outcomes.
- [ ] **14.06** Implement police patrol and pursuit.
- [ ] **14.07** Implement siren and warning cues.
- [ ] **14.08** Implement arrest conditions.
- [ ] **14.09** Implement arrest while on foot.
- [ ] **14.10** Handle route splits for all actors.
- [ ] **14.11** Match spawn ordering and RNG use.
- [ ] **14.12** Test dense scenes and simultaneous incidents.

**Exit evidence:** a reproducible demonstration of traffic pedestrians and police, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 15 — Race rules and results

Research: [src/race.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race.cpp), [src/session.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/session.cpp), [src/race_screen.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_screen.cpp).

- [ ] **15.01** Implement countdown and race start.
- [ ] **15.02** Compute progress across route branches.
- [ ] **15.03** Implement placement and standings.
- [ ] **15.04** Implement finish detection.
- [ ] **15.05** Resolve simultaneous finish events.
- [ ] **15.06** Implement race failure conditions.
- [ ] **15.07** Implement pause resume restart quit.
- [ ] **15.08** Implement results presentation.
- [ ] **15.09** Record race rewards and penalties.
- [ ] **15.10** Verify race-to-menu cleanup.
- [ ] **15.11** Test repeat races without leaked state.
- [ ] **15.12** Compare each outcome against reference behavior.

**Exit evidence:** a reproducible demonstration of race rules and results, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 16 — Menus career and economy

Research: [src/frontend.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/frontend.cpp), [src/career.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/career.cpp), [src/native_screens.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/native_screens.cpp), [src/native_strings.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/native_strings.cpp).

- [ ] **16.01** Build explicit frontend state graph.
- [ ] **16.02** Implement Thrash flow.
- [ ] **16.03** Implement Big Game career flow.
- [ ] **16.04** Implement character selection.
- [ ] **16.05** Implement course and level selection.
- [ ] **16.06** Implement bike showroom and purchase.
- [ ] **16.07** Implement repair and money changes.
- [ ] **16.08** Implement Schmooze interactions.
- [ ] **16.09** Implement unlock and progression rules.
- [ ] **16.10** Implement results and return transitions.
- [ ] **16.11** Implement settings credits and jukebox.
- [ ] **16.12** Audit every menu item against evidence ledger.

**Exit evidence:** a reproducible demonstration of menus career and economy, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 17 — Saves settings and migration

Research: [src/career.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/career.cpp), [src/settings.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/settings.cpp), [src/atomic_file.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/atomic_file.cpp).

- [ ] **17.01** Define versioned save schema.
- [ ] **17.02** Separate settings from career progress.
- [ ] **17.03** Use atomic save writes.
- [ ] **17.04** Keep recoverable backup.
- [ ] **17.05** Validate loaded data before world mutation.
- [ ] **17.06** Test truncated and corrupt saves.
- [ ] **17.07** Implement explicit schema migrations.
- [ ] **17.08** Record content pack compatibility.
- [ ] **17.09** Persist input bindings and audio levels.
- [ ] **17.10** Test interrupted writes.
- [ ] **17.11** Decide original-save import scope.
- [ ] **17.12** Document save location and reset procedure.

**Exit evidence:** a reproducible demonstration of saves settings and migration, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 18 — Audio music and video

Research: [src/sound.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/sound.cpp), [src/mixer.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/mixer.cpp), [src/race_music.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_music.cpp), [src/avi.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/avi.cpp), [src/cinepak.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/cinepak.cpp), [src/ima_adpcm.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/ima_adpcm.cpp).

- [ ] **18.01** Inventory sound effects and cue mappings.
- [ ] **18.02** Implement engine pitch and throttle response.
- [ ] **18.03** Implement collision and combat cues.
- [ ] **18.04** Implement police siren lifecycle.
- [ ] **18.05** Implement music selection and looping.
- [ ] **18.06** Expose independent volume buses.
- [ ] **18.07** Pause and resume audio consistently.
- [ ] **18.08** Handle missing audio devices.
- [ ] **18.09** Decode supported video and audio tracks.
- [ ] **18.10** Synchronize video playback timing.
- [ ] **18.11** Implement skip and return flow.
- [ ] **18.12** Compare cues and audiovisual timing.

**Exit evidence:** a reproducible demonstration of audio music and video, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 19 — Texture-by-texture enhancement

Research: [src/upscale_import.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/upscale_import.cpp), [src/upscale_cache.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/upscale_cache.cpp), [src/mmpx.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/mmpx.cpp).

- [ ] **19.01** Export immutable native-resolution masters.
- [ ] **19.02** Prioritize high-visibility assets.
- [ ] **19.03** Create one review item per texture or animation family.
- [ ] **19.04** Record AI model version and generation settings.
- [ ] **19.05** Record prompt seed and input hash.
- [ ] **19.06** Preserve alpha edges and pivot metadata.
- [ ] **19.07** Preserve palette-remap masks separately.
- [ ] **19.08** Reject invented text and changed silhouettes.
- [ ] **19.09** Review tiled textures for seams.
- [ ] **19.10** Review animation sequences for flicker.
- [ ] **19.11** Approve or reject each candidate explicitly.
- [ ] **19.12** Package approved overrides with native fallback.

**Exit evidence:** a reproducible demonstration of texture-by-texture enhancement, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 20 — Mod support and content contracts

Research: [src/assets.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/assets.hpp), [src/upscale_cache.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/upscale_cache.hpp).

- [ ] **20.01** Define versioned mod manifest.
- [ ] **20.02** Define stable content IDs.
- [ ] **20.03** Implement deterministic load priority.
- [ ] **20.04** Report conflicts with winning source.
- [ ] **20.05** Validate dependency and engine version requirements.
- [ ] **20.06** Support texture replacement first.
- [ ] **20.07** Support bike and handling data overrides.
- [ ] **20.08** Support courses after schema stabilizes.
- [ ] **20.09** Validate all numeric ranges and references.
- [ ] **20.10** Provide authored example mod.
- [ ] **20.11** Include active mod hashes in replay metadata.
- [ ] **20.12** Document unsupported script and native-code loading.

**Exit evidence:** a reproducible demonstration of mod support and content contracts, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 21 — UI accessibility and localization

Research: [src/hud.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/hud.cpp), [src/text.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/text.cpp), [src/native_strings.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/native_strings.cpp).

- [ ] **21.01** Implement HUD speed health damage and rank.
- [ ] **21.02** Support scalable UI and safe areas.
- [ ] **21.03** Support keyboard navigation and controller focus.
- [ ] **21.04** Expose readable modern text option.
- [ ] **21.05** Audit color-dependent information.
- [ ] **21.06** Provide reduced flashes and camera motion.
- [ ] **21.07** Implement subtitle policy for dialogue.
- [ ] **21.08** Inventory five supplied language resources.
- [ ] **21.09** Separate text keys from layouts.
- [ ] **21.10** Test long translated strings and glyph coverage.
- [ ] **21.11** Persist accessibility settings.
- [ ] **21.12** Run complete flow without mouse.

**Exit evidence:** a reproducible demonstration of ui accessibility and localization, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 22 — Verification and performance

Research: [docs/VERIFICATION.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/VERIFICATION.md), [docs/PERFORMANCE.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/PERFORMANCE.md), [tests/road_test.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/tests/road_test.cpp).

- [ ] **22.01** Build original-game capture protocol.
- [ ] **22.02** Record executable course bike seed and inputs.
- [ ] **22.03** Compare first divergent simulation tick.
- [ ] **22.04** Compare fixed-camera image regions.
- [ ] **22.05** Set explicit numerical tolerances.
- [ ] **22.06** Run all courses and levels in coverage matrix.
- [ ] **22.07** Measure CPU and GPU frame times separately.
- [ ] **22.08** Measure memory and texture residency.
- [ ] **22.09** Test repeated import race and unload cycles.
- [ ] **22.10** Test loading stalls and shader warmup.
- [ ] **22.11** Run soak tests on declared hardware.
- [ ] **22.12** Publish results with skips and limitations.

**Exit evidence:** a reproducible demonstration of verification and performance, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 23 — Packaging and release

Research: [docs/USER_GUIDE.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/USER_GUIDE.md), [docs/CURRENT_STATUS.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/CURRENT_STATUS.md).

- [ ] **23.01** Package clean desktop application.
- [ ] **23.02** Run first-launch import on clean machine.
- [ ] **23.03** Verify package contains no private game files.
- [ ] **23.04** Include applicable third-party notices.
- [ ] **23.05** Verify save and settings paths.
- [ ] **23.06** Document minimum tested hardware.
- [ ] **23.07** Test upgrade and uninstall behavior.
- [ ] **23.08** Provide troubleshooting and log collection.
- [ ] **23.09** Create release notes and known issues.
- [ ] **23.10** Tag reproducible source revision.
- [ ] **23.11** Attach checksums to release artifacts.
- [ ] **23.12** Complete full career and all release acceptance checks.

**Exit evidence:** a reproducible demonstration of packaging and release, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 24 — Future 3D preparation only

Research: [src/course.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course.hpp), [src/actor_descriptors.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/actor_descriptors.hpp).

- [ ] **24.01** Document coordinate axes and handedness.
- [ ] **24.02** Preserve route elevation and branch topology.
- [ ] **24.03** Keep visual representation separate from collision semantics.
- [ ] **24.04** Store physical dimensions separately from pixel size.
- [ ] **24.05** Record sprite view direction and camera assumptions.
- [ ] **24.06** Preserve neutral texture masters and masks.
- [ ] **24.07** Record material meaning and baked lighting.
- [ ] **24.08** Reserve optional mesh and rig fields in content schema.
- [ ] **24.09** Specify glTF import conventions when needed.
- [ ] **24.10** Define attachment semantics for grips feet and weapons.
- [ ] **24.11** Document hidden surfaces as unknown reconstructions.
- [ ] **24.12** Defer mesh production 3D physics and full 3D renderer to separate scope.

**Exit evidence:** a reproducible demonstration of future 3d preparation only, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

## 7. One-texture-at-a-time production procedure

1. Select a stable asset ID from the catalog. Record its source file, resource/frame index, palette variant, native dimensions, transparency and pivot.
2. Export an immutable lossless master. Keep indexed pixels and palettes where applicable. Hash the master before generation.
3. Generate a candidate at a declared scale. Save model/version, prompt, seed when available, parameters and output hash. Generation may not be exactly reproducible across service revisions; retain the approved output itself.
4. Compare native and enhanced images at native display size and intended game size. Inspect identity, silhouette, edges, lettering, pattern scale, palette and shading.
5. For animation, review the whole family in motion even when individual frames are processed separately. Reject texture crawling, inconsistent clothing details, changing face shapes and shifting contact points.
6. For terrain and roads, inspect repeated tiling and mip levels. For UI, inspect text legibility. For rider remaps, preserve semantic color masks instead of baking one rider's colors into every variant.
7. Review alpha on light and dark backgrounds. Use edge dilation and suitable alpha handling to prevent halos; do not enlarge collision bounds when adding visual detail.
8. Approve, reject or request another candidate. Record reviewer, date and reason. Only approved outputs enter the pack.
9. Build runtime textures and atlases from masters. Include padding, sampler policy, color-space metadata and memory budget. Keep build outputs replaceable.
10. Test in an actual race, with native fallback enabled. Disabling the enhancement pack must preserve gameplay and asset identity.

Suggested record fields: `asset_id`, `source_sha256`, `resource_id`, `frame`, `palette_id`, `native_size`, `pivot`, `alpha_mode`, `candidate_sha256`, `scale`, `model`, `recipe`, `review_state`, `review_notes`, `schema_version`. Suggested review states: `unreviewed`, `candidate`, `needs_changes`, `approved`, `rejected`, `superseded`.

A 4× increase in each dimension uses roughly 16× the uncompressed pixel storage. Measure the whole animation set before choosing universal scaling. Prioritize rider/bike silhouettes, frequently visible road assets and frontend images according to actual screen coverage. Keep original, enhanced and future material sources as separate variants of the same semantic asset.

## 8. Future 3D boundary

Original sprite artwork does not supply hidden geometry, a rig, clean albedo, normal maps or physically measured proportions. AI upscaling cannot recover those facts. Preserve source views and note uncertainty. Separate baked shadows from material color where feasible, but treat de-lighted outputs as authored interpretations requiring review.

Store optional real-world scale estimates with confidence and provenance. Preserve course topology, elevation, semantic surface types, collision categories, actor state names and attachment concepts. These are useful to a later 3D renderer without committing this release to mesh generation or realistic physics. A future glTF pipeline should define axes, units, material conventions, skeleton naming, sockets and animation clips in a separate approved design.

## 9. Verification matrix and acceptance templates

| Area | Minimum evidence | Failure examples |
| --- | --- | --- |
| Import | supported corpus; corrupted/truncated input; cancellation | partial install activated, path escape |
| Formats | per-format fixtures; full private corpus report | out-of-bounds read, lost transparency |
| Handling | acceleration/braking/cornering traces | render-rate-dependent speed |
| Courses | all courses/levels, branch and elevation checkpoints | split discontinuity, wrong finish |
| Combat | isolated and simultaneous attack replays | inconsistent hit order, stolen weapon duplication |
| Recovery | road/off-road/slope/arrest cases | unreachable bike, stuck rider |
| AI | full field, seeded runs, long-duration traces | changed RNG order, branch confusion |
| Career | complete progression and failure paths | money duplication, invalid unlock |
| Saves | migration, corruption and interruption | lost career, partially overwritten save |
| Visuals | native and enhanced reference captures | changed pivot, flicker, palette loss |
| Mods | conflicting IDs, invalid data, fallback | silent load-order differences |
| Release | clean-machine install through full career | missing dependency, packaged private asset |

A replay manifest should include game build, simulation schema, source corpus hash, content pack hashes, settings, course/level/bike/character, initial seed, tick-indexed commands and expected state hashes. Compare first divergence rather than only final finishing order. Capture screenshot differences at fixed simulation ticks with controlled camera and viewport. Keep exact numerical tests separate from perceptual visual tolerances.

Suggested feature issue template:

```text
Feature / roadmap ID:
Observed original behavior:
Evidence source and pinned revision:
Confidence: verified / upstream-reported / inferred / unknown
Implementation boundary:
Dependencies:
Acceptance scenarios:
Test inputs and expected outputs:
Implementation revision:
Known deviations:
```

Do not report upstream's tests as this project's tests. Do not label the local prototype as a faithful implementation merely because similar systems exist. Preserve failed comparisons and update the known-differences ledger.

## 10. First implementation sprint

- [ ] Establish the workspace, pinned toolchain and CI using an authored placeholder scene.
- [ ] Implement local import validation for the confirmed 341-file corpus.
- [ ] Decode one RRI image, one rider animation family and one course section.
- [ ] Build a catalog/inspection tool that exposes IDs, palettes and pivots.
- [ ] Implement a headless command-driven player simulation.
- [ ] Render one measured course segment with original sprites.
- [ ] Record and compare one acceleration/braking run against the original.
- [ ] Export one texture, review one enhanced candidate and demonstrate fallback.
- [ ] Publish the evidence report before widening content coverage.

## 11. Authoritative technology references

Use documentation matching the pinned dependency version. Bevy 0.20.1 was visible in the local manifest and current API documentation at inspection; select a project baseline explicitly and handle upgrades as separate changes.

- [Bevy learning resources](https://bevy.org/learn/) — official starting points and examples.
- [Bevy 0.20.1 API reference](https://docs.rs/bevy/0.20.1/bevy/) — engine APIs for the observed baseline.
- [Bevy examples](https://github.com/bevyengine/bevy/tree/v0.20.1/examples) — inspect examples at the chosen tag before adopting API calls.
- [Bevy migration guides](https://bevy.org/learn/migration-guides/) — plan upgrades explicitly.
- [The Rust Book](https://doc.rust-lang.org/book/) — ownership, errors and modules.
- [Cargo reference](https://doc.rust-lang.org/cargo/reference/) — workspaces, features, profiles and lockfiles.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — public interface design.
- [Rust Clippy](https://doc.rust-lang.org/clippy/) — lint policy.
- [Rust Fuzz Book](https://rust-fuzz.github.io/book/) — parser fuzzing workflow.
- [Khronos glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) — future mesh/material interchange.

These references guide implementation; they do not replace original-game measurements. Review plugin compatibility against the selected Bevy version before adding physics, UI, input or asset-processing dependencies.

## 12. Maintenance policy

Keep this README as the master checklist. As work grows, add focused design documents and evidence reports and link them beside completed items. Keep each change small enough to review: one decoder, one behavior, one asset family or one architectural boundary. Record the reason for abstraction and test its consumer before expanding it. Revisit scope at each gate, and maintain the distinction between faithful behavior, intentional enhancement and future 3D research.
