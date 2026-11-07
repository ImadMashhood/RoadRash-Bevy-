# RoadRash-Rust — complete recreation roadmap

**Active gameplay priority: [collision, fighting and crash accuracy](docs/COLLISION_COMBAT_PRIORITY.md).** This gate comes before additional presentation/enhancement work and the architecture report. It tracks native expected/actual comparisons and keeps unverified mechanics open.

**Latest verification: [327,086 native observations match](docs/evidence/source-contact-recovery.md).** Controlled contact classification, physical scenery/building boundaries, complete recovery episodes and two-rider fight chains pass. Full police behavior, dense/simultaneous interactions and current career/release acceptance remain open.

**Original interface:** [Native menus, fonts and all three dashboards](docs/evidence/original-ui.md) now replace the standard modern overlay. Original rider portraits, course postcards and the restroom/street layouts are restored; accessibility alternatives remain available.

> **Current implementation:** 3D is deferred. The active target is the classic game in Rust/Bevy using separate bounded format readers, a renderer-independent simulation, and a 2D presentation plugin. See [implementation status](docs/STATUS.md), [build instructions](docs/GETTING_STARTED.md), and [architecture](docs/ARCHITECTURE.md). The detailed roadmap below includes future preparation; those items do not expand the current scope.

A Rust and Bevy recreation of PC Road Rash with deterministic gameplay, bounded content readers, a separate presentation layer and a modern offline interface.

**Current source fixes the reported building sides, building/scenery collision, opponent planning and bare interface. Full career and replacement-release acceptance are open.** [Correction report and reproducible checks](docs/evidence/native-gameplay-fixes.md).

The [published Linux RC4](https://github.com/ImadMashhood/RoadRash-Bevy-/releases/tag/native-functional-2026-11-01-rc4) is historical and does not contain these fixes. Its earlier career/performance receipts apply to that immutable build. The old unattended career driver fails against the changed physics/AI; this failure is recorded rather than reusing its old pass. [Player guide](docs/PLAYER_GUIDE.md), [known limitations](docs/RELEASE_NOTES.md), [open release gate](docs/evidence/release-signoff.json).

Completed phases are collapsed individually below, with their detailed checked children and linked receipts preserved. Parts 19, 21 and 22 retain their reopened acceptance checkboxes; Part 22 needs a verified replacement candidate and owner acceptance. Enhancement production follows accepted G5 and the Part 23 mod contracts; all current acceptance uses native assets.

Run the current source-course view after local import/conversion:

```sh
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course CITY
```

[Classic renderer controls, camera policy and checkpoints](docs/RENDERING.md). Imported races now use schema-6 source handling, combat, linked recovery, 14 opponents and bounded traffic/pedestrian/police systems. Use `--opponents 0` for solo population testing or `--practice-riders 2` for the earlier constant-drive mode. Z/X/C attack; Left Shift boosts compatible bikes, V dismounts near standstill, Q/E lean; arrows walk during recovery. `--record-replay local/my-race.json` records a live session. The data-free fallback retains its authored model. Race results, career, save slots and audio/video playback are available; the native candidate, acceptance results and remaining release gates are recorded in [native acceptance](docs/NATIVE_ACCEPTANCE.md).

## 1. Scope and definition of complete

**Owner scope update:** Running the original executable and matching its runtime captures are no longer required. Source-disc hash verification, bounded parsing, static research and tests of this engine remain required. Original-runtime checklist gates 01.02.b, 01.03.b, 01.04.b, 01.06.b, 08.01.b, 15.12.b and task 11.12 are retired without renumbering other IDs. Part 01 now accepts documented static metadata and current-engine validation. See [scope decision](docs/SCOPE_CHANGES.md).

The first release covers local single-player racing, original courses and progression, handling, combat, opponents, traffic, pedestrians, police, menus, saves, audio/video, using native assets. An optional enhanced texture pack follows the accepted functional native release. Multiplayer, mobile/browser delivery, and a full 3D remake are separate projects. Preserve classic behavior first; record intentional changes as selectable enhancements. Completion means a clean install from user-supplied data, full campaign completion, all required content covered, reproducible validation, and no unresolved release-blocking defects.

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
- [x] Create a behavior ledger linking every implemented feature to evidence and tests.
- [x] Establish a redistributable synthetic test corpus.
- [x] Record provenance and applicable permissions before incorporating upstream code or assets.

Keep source code, user-supplied game data, generated working assets, and approved enhancement packs separate. Owning a disc is not a project distribution policy; public packages should import local game data. No game images, audio, video, ISO, executable, or derived texture pack is included in this documentation commit. Upstream GitHub metadata did not identify a repository license at inspection; do not assume that public source is permission to copy it. Choose and document the project's own code license independently.

## 4. Architecture contract

Start with modules and split into crates when boundaries prove useful. Avoid a generic engine framework before the first playable slice.

```text
apps/game             Bevy application and composition
crates/formats        bounded binary readers, no rendering
crates/content        IDs, schemas, catalogs, mod resolution
crates/course         source-unit course graph, traversal and inspector
crates/render         classic projection, surfaces, raster and visual checkpoints
crates/simulation     race rules, actors, commands, replay
crates/presentation   classic renderer, animations and HUD
crates/input          validated bindings and input state
crates/session        race lifecycle, career, menus and persistence
crates/media          bounded PCM/video readers, cue planning and media clock
tools/import          local disc verification and conversion
tools/asset-review    export, candidate review, pack validation
```

All listed runtime crates exist; enhancement production remains gated future work. The game shell composes platform input, storage and Rodio/CPAL playback adapters with the pure domain crates. Simulation owns gameplay truth. Rendering consumes snapshots and never determines hit results. Use explicit Bevy system ordering for input consumption, spawning, AI, movement, collisions, combat, progression and snapshot extraction. Retain legacy integer/fixed-point arithmetic wherever necessary for parity. A fixed timestep alone does not establish determinism: update order, RNG calls, numeric behavior and content versions also matter.

Represent a position as route/branch identity plus longitudinal progress, lateral displacement and elevation. A classic projection adapter and a future world-space adapter can consume the same course data. Keep sprite pivots, UV rectangles and screen scaling out of physics. Do not replace legacy handling with a general rigid-body solver merely because one is available. The local prototype's Rapier/Avian experiments need separate evaluation before reuse.

Use typed IDs and errors, bounded parsers, data-driven configuration, narrow plugins and explicit ownership. Avoid global mutable state, direct pointer-offset translation, unrestricted unsafe code, and filesystem access inside gameplay systems. Store saves in versioned domain records rather than serializing an entire Bevy world.

## 5. Delivery order and gates

| Gate | Required result | Depends on |
| --- | --- | --- |
| G0 | Verified local import and asset viewer | Baseline, foundation, formats |
| G1 | One course with faithful player driving | G0, simulation, road, input |
| G2 | Complete race with combat and opponents | G1, actors, recovery, traffic |
| G3 | Complete career and audiovisual frontend | G2, menus, saves, media |
| G3F | Complete native-asset functional candidate | G3, Part 19 required-feature/gap closure |
| G5 | Accepted native-asset desktop release | G3F, Part 20 UI/localization, Part 21 verification, Part 22 packaging |
| G4 | Reviewed enhanced textures and stable mod overrides | **G5 accepted first**, Part 23 mod contracts, Part 24 per-asset approval |
| G6 | Optional enhanced-art release | G4 and the unchanged native-only fallback |

Original textures remain the baseline through functional completion, UI/localization, verification and native release. Existing override/registration experiments are infrastructure tests, not authorization to start enhancement production. **No AI upscaling or enhanced candidate production begins until G5 has a recorded acceptance signoff.** G5 does not depend on G4. Each gate needs an executable revision, repeatable test recipe, recorded results and explicit remaining differences. No calendar estimate is implied.

## 6. Detailed implementation checklists

Each parent item is a deliverable with independently checkable implementation, acceptance and evidence steps; owner-retired runtime children have been removed. A checked box requires code or an artifact, verification, and a linked evidence record. Reference links are research entry points; they do not certify that the upstream implementation is exact.

### How to use the task hierarchy

A parent checkbox represents one complete deliverable. All remaining implementation, acceptance and evidence children must be complete before the parent is checked. Retired original-runtime children are removed, not marked as passed. A finished implementation with an unresolved reference mismatch remains open. Completed IDs 01–18 remain unchanged. This user-requested reorder creates roadmap revision 2 for future work; [the migration map](docs/ROADMAP_REORDER.md) preserves every previous future-ticket identity and its replacement. Record roadmap revision plus ticket ID when creating new issues.

The steps describe required results rather than asserting that unknown behavior has already been recovered. Constants and binary field meanings must be supported by static evidence or explicitly labeled design choices. Original-runtime measurement is not a completion gate. Phase prerequisites, inputs, outputs and invariants apply to every task in that phase. Within a phase, data/schema tasks generally precede their consumers; document any additional dependency when turning a task into an issue.

For every code task, the evidence record must identify affected interfaces, normal operation, the named edge cases, error behavior and any compatibility consequence. For document/design tasks, verification means an actual completeness review or exercised procedure, not writing a meaningless unit test. Mark a task blocked when required source behavior is unknown and link the research question. Do not silently treat an approximation as a pass.

### Navigation

- [Phase 01: Project baseline and evidence](#phase-01--project-baseline-and-evidence)
- [Phase 02: Repository and Rust foundation](#phase-02--repository-and-rust-foundation)
- [Phase 03: Disc import and provenance](#phase-03--disc-import-and-provenance)
- [Phase 04: Binary formats and asset decoding](#phase-04--binary-formats-and-asset-decoding)
- [Phase 05: Content catalog and tooling](#phase-05--content-catalog-and-tooling)
- [Phase 06: Simulation architecture](#phase-06--simulation-architecture)
- [Phase 07: Course reconstruction](#phase-07--course-reconstruction)
- [Phase 08: Classic road rendering](#phase-08--classic-road-rendering)
- [Phase 09: Input and player handling](#phase-09--input-and-player-handling)
- [Phase 10: Rider animation and palette fidelity](#phase-10--rider-animation-and-palette-fidelity)
- [Phase 11: Combat and contact](#phase-11--combat-and-contact)
- [Phase 12: Crashes and recovery](#phase-12--crashes-and-recovery)
- [Phase 13: Opponent intelligence](#phase-13--opponent-intelligence)
- [Phase 14: Traffic pedestrians and police](#phase-14--traffic-pedestrians-and-police)
- [Phase 15: Race rules and results](#phase-15--race-rules-and-results)
- [Phase 16: Menus career and economy](#phase-16--menus-career-and-economy)
- [Phase 17: Saves settings and migration](#phase-17--saves-settings-and-migration)
- [Phase 18: Audio music and video](#phase-18--audio-music-and-video)
- [Phase 19: Complete the native-asset offline game](#phase-19--complete-the-native-asset-offline-game)
- [Phase 20: UI accessibility and localization](#phase-20--ui-accessibility-and-localization)
- [Phase 21: Verification and performance](#phase-21--verification-and-performance)
- [Phase 22: Packaging and native-asset release](#phase-22--packaging-and-native-asset-release)
- [Phase 23: Mod support and content contracts](#phase-23--mod-support-and-content-contracts)
- [Phase 24: Texture-by-texture enhancement](#phase-24--texture-by-texture-enhancement)
- [Phase 25: Future 3D preparation only](#phase-25--future-3d-preparation-only)

### Phase 01 — Project baseline and evidence

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** None; this establishes the evidence baseline. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Original disc metadata, pinned upstream README/status and current-engine validation.

**Required outputs:** Scope document, content inventories, behavior ledger and release criteria. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Separate upstream-reported behavior from locally measured facts; record unknowns as unknown.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [README.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/README.md), [docs/CURRENT_STATUS.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/CURRENT_STATUS.md), [docs/PROGRESS.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/PROGRESS.md).

- [x] **01.01 — Record scope and supported disc revisions**

  Completed: [implementation and acceptance evidence](docs/evidence/01.01.md).

  Progress: [evidence and remaining work](docs/evidence/01.01.md).

  - [x] **01.01.a — Implement:** List supported disc fingerprints, single-player modes and exclusions in a scope document.
  - [x] **01.01.b — Verify:** Reject an unknown revision with an actionable message rather than silently decoding it.
  - [x] **01.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.02 — Define classic presentation targets**

  Completed: [implementation and acceptance evidence](docs/evidence/01.02.md).

  - [x] **01.02.a — Implement:** Specify the logical viewport, aspect policy, pixel sampling and current-engine smoke captures.

  - [x] **01.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.03 — Inventory static menu and navigation records**

  Completed: [implementation and acceptance evidence](docs/evidence/01.03.md).

  - [x] **01.03.a — Implement:** Inventory static screen, popup, item, handler and navigation records; label dynamic conditions for later frontend implementation.

  - [x] **01.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.04 — Inventory course and difficulty source data**

  Completed: [implementation and acceptance evidence](docs/evidence/01.04.md).

  - [x] **01.04.a — Implement:** Record five course source identities and 25 level selections with distance labels and source references; defer route interpretation to Phase 07.

  - [x] **01.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.05 — Inventory bikes and playable characters**

  Completed: [implementation and acceptance evidence](docs/evidence/01.05.md).

  Evidence: [implementation and verification](docs/evidence/01.05.md).

  - [x] **01.05.a — Implement:** Catalog all bike and character IDs with display names, source records and unlock rules.
  - [x] **01.05.b — Verify:** Verify every selectable ID resolves to a complete record without duplicate identities.
  - [x] **01.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.06 — Record source binding tables and current input policy**

  Completed: [implementation and acceptance evidence](docs/evidence/01.06.md).

  - [x] **01.06.a — Implement:** Record the supported executable binding words and documented actions, and state the current portable input policy without runtime-parity claims.

  - [x] **01.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.07 — Record observed bugs separately from desired changes**

  Completed: [implementation and acceptance evidence](docs/evidence/01.07.md).

  Evidence: [implementation and verification](docs/evidence/01.07.md).

  - [x] **01.07.a — Implement:** Assign each deviation an observed trigger, source evidence and proposed treatment.
  - [x] **01.07.b — Verify:** Show that fidelity fixes and optional improvements cannot be confused in the issue tracker.
  - [x] **01.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.08 — Create evidence ledger with confidence levels**

  Completed: [implementation and acceptance evidence](docs/evidence/01.08.md).

  Evidence: [implementation and verification](docs/evidence/01.08.md).

  - [x] **01.08.a — Implement:** Record binary hash, upstream revision, capture settings and confidence per claim.
  - [x] **01.08.b — Verify:** Ensure inferred claims remain visibly unverified until supported by a reproducible capture.
  - [x] **01.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.09 — Choose initial desktop platform**

  Completed: [implementation and acceptance evidence](docs/evidence/01.09.md).

  Evidence: [implementation and verification](docs/evidence/01.09.md).

  - [x] **01.09.a — Implement:** Choose a first operating system, architecture, graphics backend and test machine.
  - [x] **01.09.b — Verify:** Run the placeholder build on that machine and record drivers and launch requirements.
  - [x] **01.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.10 — Define fidelity and enhanced modes**

  Completed: [implementation and acceptance evidence](docs/evidence/01.10.md).

  - [x] **01.10.a — Implement:** Define which settings affect presentation only and which alter simulation.
  - [x] **01.10.b — Verify:** Switch texture modes during the same replay and verify simulation hashes remain equal.
  - [x] **01.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.11 — Set milestone owners and dependencies**

  Completed: [implementation and acceptance evidence](docs/evidence/01.11.md).

  Evidence: [implementation and verification](docs/evidence/01.11.md).

  - [x] **01.11.a — Implement:** Create a dependency graph with an owner or explicitly unassigned status per milestone.
  - [x] **01.11.b — Verify:** Identify blocked work and demonstrate there is no circular prerequisite chain.
  - [x] **01.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **01.12 — Record release acceptance criteria**

  Completed: [implementation and acceptance evidence](docs/evidence/01.12.md).

  Progress: [evidence and remaining work](docs/evidence/01.12.md).

  - [x] **01.12.a — Implement:** Write measurable launch, content, career, stability and performance release criteria.
  - [x] **01.12.b — Verify:** Ensure every criterion names a test procedure and an accountable sign-off record.
  - [x] **01.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/01.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of project baseline and evidence, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 02 — Repository and Rust foundation

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 01.01, 01.09 and 01.11. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Chosen platform matrix and architecture contract.

**Required outputs:** Buildable placeholder application, workspace manifests, lockfile and CI configuration. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Do not claim an asset-dependent launch works from a clean checkout until import exists.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [CMakeLists.txt](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/CMakeLists.txt), [docs/TOOLING.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/TOOLING.md).

- [x] **02.01 — Create Cargo workspace and game binary**

  Progress: [evidence and remaining work](docs/evidence/02.01.md).

  - [x] **02.01.a — Implement:** Create minimal game, format and simulation boundaries with one-way dependencies.
  - [x] **02.01.b — Verify:** Build the game and headless simulation without requiring private assets.
  - [x] **02.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.02 — Pin Rust toolchain and Bevy version**

  Progress: [evidence and remaining work](docs/evidence/02.02.md).

  - [x] **02.02.a — Implement:** Select exact engine/toolchain versions and record why they were chosen.
  - [x] **02.02.b — Verify:** Build on a clean runner using those versions rather than a developer's cached setup.
  - [x] **02.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.03 — Commit Cargo.lock for the application**

  Progress: [evidence and remaining work](docs/evidence/02.03.md).

  - [x] **02.03.a — Implement:** Generate the lockfile from the chosen dependency graph and commit it.
  - [x] **02.03.b — Verify:** Run a locked build and ensure CI fails if dependency resolution would change it.
  - [x] **02.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.04 — Choose explicit Bevy feature set**

  Progress: [evidence and remaining work](docs/evidence/02.04.md).

  - [x] **02.04.a — Implement:** Enable only the platform/render/audio features needed for the first slice.
  - [x] **02.04.b — Verify:** Document which feature enables each subsystem and verify a release build.
  - [x] **02.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.05 — Document OS libraries and graphics requirements**

  Evidence: [implementation and verification](docs/evidence/02.05.md).

  - [x] **02.05.a — Implement:** List distro packages, compiler tools, GPU API and audio/display requirements.
  - [x] **02.05.b — Verify:** Follow the setup on a clean environment and record any manual step.
  - [x] **02.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.06 — Add rustfmt and Clippy checks**

  Progress: [evidence and remaining work](docs/evidence/02.06.md).

  - [x] **02.06.a — Implement:** Configure formatting and lint gates with a justified warning policy.
  - [x] **02.06.b — Verify:** Introduce a temporary formatting violation locally and confirm the documented check detects it.
  - [x] **02.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.07 — Add headless core test job**

  Progress: [evidence and remaining work](docs/evidence/02.07.md).

  - [x] **02.07.a — Implement:** Run core tests without a window, GPU or original disc.
  - [x] **02.07.b — Verify:** Prove the job distinguishes skipped private-corpus tests from passing synthetic tests.
  - [x] **02.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.08 — Add desktop build matrix**

  Evidence: [implementation and verification](docs/evidence/02.08.md).

  - [x] **02.08.a — Implement:** Select supported OS/architecture jobs and separate compile checks from runtime tests.
  - [x] **02.08.b — Verify:** Publish a matrix that does not claim hardware testing from cross-compilation alone.
  - [x] **02.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.09 — Create synthetic redistributable fixture pack**

  Evidence: [implementation and verification](docs/evidence/02.09.md).

  - [x] **02.09.a — Implement:** Author tiny indexed images, resource containers and route graphs with known answers.
  - [x] **02.09.b — Verify:** Verify fixtures contain no copied game payload and each explains its expected decoding.
  - [x] **02.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.10 — Configure ignored private asset directories**

  Progress: [evidence and remaining work](docs/evidence/02.10.md).

  - [x] **02.10.a — Implement:** Ignore source discs, extracted media, generated private textures and local caches.
  - [x] **02.10.b — Verify:** Inspect tracked files and a dry-run package manifest for accidental private content.
  - [x] **02.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.11 — Create architecture decision records**

  Progress: [evidence and remaining work](docs/evidence/02.11.md).

  - [x] **02.11.a — Implement:** Add decision records containing context, options, decision and consequences.
  - [x] **02.11.b — Verify:** Link the first records for simulation arithmetic, renderer boundary and asset identity.
  - [x] **02.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **02.12 — Document contributor setup from clean clone**

  Progress: [evidence and remaining work](docs/evidence/02.12.md).

  - [x] **02.12.a — Implement:** Write clean-clone build and placeholder launch commands with expected output.
  - [x] **02.12.b — Verify:** Have a fresh directory complete the instructions with no undocumented local files.
  - [x] **02.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/02.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of repository and rust foundation, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 03 — Disc import and provenance

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 02.01, 02.09 and supported revision decision in 01.01. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** User-selected ISO or directory and pinned required-file hashes.

**Required outputs:** Importer library/CLI, validation manifest, staged installation receipt. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Never activate an incomplete corpus; preserve existing verified installation through failure.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/importer.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/importer.cpp), [src/iso9660.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/iso9660.cpp), [src/reference_hashes.inc](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/reference_hashes.inc).

- [x] **03.01 — Implement bounded ISO9660 reader or vetted library adapter**

  Progress: [evidence and remaining work](docs/evidence/03.01.md).

  - [x] **03.01.a — Implement:** Parse volume descriptors and directory extents with checked offset arithmetic.
  - [x] **03.01.b — Verify:** Reject an extent outside the image and directory recursion beyond a configured bound.
  - [x] **03.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.02 — Accept extracted ROADRASH directory**

  Evidence: [implementation and verification](docs/evidence/03.02.md).

  - [x] **03.02.a — Implement:** Normalize an explicitly selected game directory while retaining original path evidence.
  - [x] **03.02.b — Verify:** Accept supported case conventions and report ambiguous case-colliding files.
  - [x] **03.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.03 — Verify supported executable hash**

  Progress: [evidence and remaining work](docs/evidence/03.03.md).

  - [x] **03.03.a — Implement:** Hash executable bytes before using version-specific tables or offsets.
  - [x] **03.03.b — Verify:** Test correct bytes, a one-byte modification and a same-size unrelated executable.
  - [x] **03.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.04 — Verify complete required file manifest**

  Progress: [evidence and remaining work](docs/evidence/03.04.md).

  - [x] **03.04.a — Implement:** Check each required path against the pinned 341-file manifest.
  - [x] **03.04.b — Verify:** Produce a complete mismatch report and activate only a fully verified corpus.
  - [x] **03.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.05 — Report missing and mismatched files individually**

  Evidence: [implementation and verification](docs/evidence/03.05.md).

  - [x] **03.05.a — Implement:** Distinguish absent, unreadable, wrong-size and wrong-hash files in diagnostics.
  - [x] **03.05.b — Verify:** Test several simultaneous failures and preserve all paths in the report.
  - [x] **03.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.06 — Reject path traversal and escaping symlinks**

  Evidence: [implementation and verification](docs/evidence/03.06.md).

  - [x] **03.06.a — Implement:** Resolve destination paths under a dedicated root and validate archive member names.
  - [x] **03.06.b — Verify:** Exercise absolute paths, parent traversal and symlink escape attempts.
  - [x] **03.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.07 — Bound file size and allocation**

  Evidence: [implementation and verification](docs/evidence/03.07.md).

  - [x] **03.07.a — Implement:** Apply explicit limits to directory entries, file lengths and total imported bytes.
  - [x] **03.07.b — Verify:** Test arithmetic overflow and oversized claims without huge allocations.
  - [x] **03.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.08 — Stage import before atomic activation**

  Evidence: [implementation and verification](docs/evidence/03.08.md).

  - [x] **03.08.a — Implement:** Write to a uniquely named staging directory and commit a verified install atomically.
  - [x] **03.08.b — Verify:** Interrupt before activation and confirm no partial installation becomes active.
  - [x] **03.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.09 — Preserve existing installation on failed import**

  Evidence: [implementation and verification](docs/evidence/03.09.md).

  - [x] **03.09.a — Implement:** Treat an existing verified install as immutable until replacement is complete.
  - [x] **03.09.b — Verify:** Simulate failed replacement and verify the old installation still launches.
  - [x] **03.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.10 — Support cancellation and interrupted import cleanup**

  Evidence: [implementation and verification](docs/evidence/03.10.md).

  - [x] **03.10.a — Implement:** Check cancellation between bounded read/copy units and track staging ownership.
  - [x] **03.10.b — Verify:** Cancel at multiple stages and remove only this import's temporary files.
  - [x] **03.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.11 — Record source hashes and importer version**

  Evidence: [implementation and verification](docs/evidence/03.11.md).

  - [x] **03.11.a — Implement:** Write a provenance manifest with source hashes, importer build and schema version.
  - [x] **03.11.b — Verify:** Verify a later audit can distinguish source changes from conversion changes.
  - [x] **03.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **03.12 — Test Unicode paths and read-only sources**

  Evidence: [implementation and verification](docs/evidence/03.12.md).

  - [x] **03.12.a — Implement:** Handle non-ASCII filenames, spaces and read-only media without shell interpolation.
  - [x] **03.12.b — Verify:** Import from a path with Unicode and spaces and report unwritable output cleanly.
  - [x] **03.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/03.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of disc import and provenance, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 04 — Binary formats and asset decoding

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 03.03–03.04 for private corpus; 02.09 for public tests. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Verified files and authored malformed fixtures.

**Required outputs:** Typed decoders, inspection output, bounded error types and corpus report. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Do not convert offsets to pointers or trust counts before validating byte ranges.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/resource.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/resource.cpp), [src/cel.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/cel.cpp), [src/rri.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/rri.cpp), [src/bob.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/bob.cpp), [src/exe_tables.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/exe_tables.cpp).

- [x] **04.01 — Decode resource directory with explicit little-endian reads**

  Completed: [implementation and acceptance evidence](docs/evidence/04.01.md).

  - [x] **04.01.a — Implement:** Read container magic, table location, count and fixed-width entries explicitly.
  - [x] **04.01.b — Verify:** Compare a known directory listing and reject swapped-endian or invalid magic inputs.
  - [x] **04.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.02 — Validate offsets and payload lengths**

  Completed: [implementation and acceptance evidence](docs/evidence/04.02.md).

  - [x] **04.02.a — Implement:** Validate each payload range before exposing a borrowed slice to a decoder.
  - [x] **04.02.b — Verify:** Test offset-plus-length overflow, overlap policy and truncated payloads.
  - [x] **04.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.03 — Preserve unknown fields in inspection output**

  Completed: [implementation and acceptance evidence](docs/evidence/04.03.md).

  - [x] **04.03.a — Implement:** Expose raw unknown fields alongside interpreted values in inspection output.
  - [x] **04.03.b — Verify:** Round-trip inspection metadata without silently inventing semantics.
  - [x] **04.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.04 — Decode CEL textures and palettes**

  Completed: [implementation and acceptance evidence](docs/evidence/04.04.md).

  - [x] **04.04.a — Implement:** Decode CEL dimensions, palette relationships and pixel layout into typed records.
  - [x] **04.04.b — Verify:** Compare representative decoded pixels and reject inconsistent dimensions.
  - [x] **04.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.05 — Decode RRI frontend images**

  Completed: [implementation and acceptance evidence](docs/evidence/04.05.md).

  - [x] **04.05.a — Implement:** Decode RRI JPEG headers and RGB pixels, retaining source identity without inventing an indexed palette.
  - [x] **04.05.b — Verify:** Check frontend image boundaries and bounded RGB decoding; document the modern JPEG policy.
  - [x] **04.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.06 — Decode BOB rider and police frames**

  Completed: [implementation and acceptance evidence](docs/evidence/04.06.md).

  Progress: [evidence and remaining work](docs/evidence/04.06.md).

  - [x] **04.06.a — Implement:** Parse width and height per BOB frame and preserve exact frame order.
  - [x] **04.06.b — Verify:** Validate the reference 326 rider and 45 police frame expectations against supported inputs.
  - [x] **04.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.07 — Decode shadow assets and transparency**

  Completed: [implementation and acceptance evidence](docs/evidence/04.07.md).

  - [x] **04.07.a — Implement:** Decode shadow variants separately from opaque rider pixels.
  - [x] **04.07.b — Verify:** Compare their alpha/silhouette on contrasting backgrounds without black rectangles.
  - [x] **04.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.08 — Extract executable tables only for known binary**

  Completed: [implementation and acceptance evidence](docs/evidence/04.08.md).

  - [x] **04.08.a — Implement:** Translate known executable virtual addresses through verified PE sections.
  - [x] **04.08.b — Verify:** Reject unknown binaries and unmapped address ranges before reading tables.
  - [x] **04.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.09 — Preserve indexed originals and rider remaps**

  Completed: [implementation and acceptance evidence](docs/evidence/04.09.md).

  - [x] **04.09.a — Implement:** Keep original indices plus remap tables as reusable source data.
  - [x] **04.09.b — Verify:** Compare remap variants and ensure transparent index behavior remains correct.
  - [x] **04.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.10 — Export lossless RGBA previews**

  Completed: [implementation and acceptance evidence](docs/evidence/04.10.md).

  - [x] **04.10.a — Implement:** Export deterministic RGBA PNG previews with declared palette and alpha policy.
  - [x] **04.10.b — Verify:** Redecode exported PNGs and compare pixel bytes to the decoder output.
  - [x] **04.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.11 — Create corpus coverage report**

  Completed: [implementation and acceptance evidence](docs/evidence/04.11.md).

  - [x] **04.11.a — Implement:** Count parsed, unsupported and failed resources by source file and format.
  - [x] **04.11.b — Verify:** Ensure every required asset is accounted for rather than silently skipped.
  - [x] **04.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **04.12 — Fuzz truncated and malformed synthetic inputs**

  Completed: [implementation and acceptance evidence](docs/evidence/04.12.md).

  - [x] **04.12.a — Implement:** Build malformed cases for short headers, invalid counts and nested offsets.
  - [x] **04.12.b — Verify:** Run fuzz targets under bounded resources and retain minimized failures.
  - [x] **04.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/04.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of binary formats and asset decoding, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 05 — Content catalog and tooling

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 04 decoder outputs and 02.10 private-directory policy. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Decoded source records, source hashes and conversion recipes.

**Required outputs:** Versioned catalog, asset inspection tool, contact sheets and build graph. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Asset identity must survive repacking, enhancement and filename changes.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/assets.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/assets.cpp), [src/family.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/family.cpp), [src/actor_descriptors.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/actor_descriptors.cpp).

- [x] **05.01 — Assign stable namespaced asset identifiers**

  Completed: [implementation and acceptance evidence](docs/evidence/05.01.md).

  - [x] **05.01.a — Implement:** Define IDs from namespace, semantic family and stable source identity.
  - [x] **05.01.b — Verify:** Show IDs remain stable when packing order or generated filenames change; document canonical-source rename migrations.
  - [x] **05.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.02 — Record source file and resource index**

  Completed: [implementation and acceptance evidence](docs/evidence/05.02.md).

  - [x] **05.02.a — Implement:** Attach source hash, container path, resource ID and decoder version to each entry.
  - [x] **05.02.b — Verify:** Trace any preview pixel family back to its source record.
  - [x] **05.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.03 — Record frame dimensions and pivot**

  Completed: [implementation and acceptance evidence](docs/evidence/05.03.md).

  - [x] **05.03.a — Implement:** Store native rectangle, trim offset, logical size and pivot independently.
  - [x] **05.03.b — Verify:** Render differently trimmed frames at a common contact point without jumping.
  - [x] **05.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.04 — Classify road terrain scenery rider UI audio video assets**

  Completed: [implementation and acceptance evidence](docs/evidence/05.04.md).

  - [x] **05.04.a — Implement:** Assign typed categories and subcategories to all imported assets.
  - [x] **05.04.b — Verify:** Report uncategorized entries and prevent a UI asset from resolving as a bike specification.
  - [x] **05.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.05 — Track shared pixels without conflating semantic identities**

  Completed: [implementation and acceptance evidence](docs/evidence/05.05.md).

  - [x] **05.05.a — Implement:** Deduplicate storage by bytes while retaining distinct logical asset IDs.
  - [x] **05.05.b — Verify:** Replace one semantic asset without unexpectedly replacing unrelated identical pixels.
  - [x] **05.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.06 — Add catalog schema version**

  Completed: [implementation and acceptance evidence](docs/evidence/05.06.md).

  - [x] **05.06.a — Implement:** Version the catalog and define compatibility and migration behavior.
  - [x] **05.06.b — Verify:** Load an older fixture or reject it with a schema-specific diagnostic.
  - [x] **05.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.07 — Export searchable contact sheets**

  Completed: [implementation and acceptance evidence](docs/evidence/05.07.md).

  - [x] **05.07.a — Implement:** Generate contact sheets with readable stable IDs and palette labels.
  - [x] **05.07.b — Verify:** Select a sheet entry and resolve it to the exact catalog record.
  - [x] **05.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.08 — Add asset inspector with palette switching**

  Completed: [implementation and acceptance evidence](docs/evidence/05.08.md).

  - [x] **05.08.a — Implement:** Provide frame, palette, alpha and nearest/filtered preview controls.
  - [x] **05.08.b — Verify:** Inspect a full rider family without modifying source files.
  - [x] **05.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.09 — Build dependency graph for generated assets**

  Completed: [implementation and acceptance evidence](docs/evidence/05.09.md).

  - [x] **05.09.a — Implement:** Track generated outputs back to all source and recipe dependencies.
  - [x] **05.09.b — Verify:** Change one source and rebuild its dependents without rebuilding unrelated assets.
  - [x] **05.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.10 — Cache by source and recipe hash**

  Completed: [implementation and acceptance evidence](docs/evidence/05.10.md).

  - [x] **05.10.a — Implement:** Hash source bytes, converter version and settings into cache keys.
  - [x] **05.10.b — Verify:** Change a recipe parameter and verify stale output is never reused.
  - [x] **05.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.11 — Report missing references before launch**

  Completed: [implementation and acceptance evidence](docs/evidence/05.11.md).

  - [x] **05.11.a — Implement:** Validate cross-references after mod resolution and before race setup.
  - [x] **05.11.b — Verify:** Report missing IDs with the referring asset and selected content pack.
  - [x] **05.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **05.12 — Document reproducible rebuild commands**

  Completed: [implementation and acceptance evidence](docs/evidence/05.12.md).

  - [x] **05.12.a — Implement:** Specify input directory, output directory and tool versions for rebuilds.
  - [x] **05.12.b — Verify:** Rebuild a fixture twice and compare manifests and output hashes.
  - [x] **05.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/05.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of content catalog and tooling, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 06 — Simulation architecture

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 01 fidelity policy and 02 foundation; use synthetic content initially. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Validated domain data and tick-indexed input.

**Required outputs:** Headless simulation, command stream, snapshot boundary and replay hashes. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Simulation results cannot depend on render rate, camera visibility or cosmetic RNG.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/race_state.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_state.hpp), [src/race_schedule.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_schedule.cpp), [src/det_math.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/det_math.cpp).

- [x] **06.01 — Define simulation tick and unit types**

  Completed: [implementation and acceptance evidence](docs/evidence/06.01.md).

  - [x] **06.01.a — Implement:** Document tick duration and explicit speed, distance and fixed-point units.
  - [x] **06.01.b — Verify:** Test conversions at negative values, boundaries and maximum supported ranges.
  - [x] **06.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.02 — Keep screen coordinates outside simulation**

  Completed: [implementation and acceptance evidence](docs/evidence/06.02.md).

  Progress: [evidence and remaining work](docs/evidence/06.02.md).

  - [x] **06.02.a — Implement:** Restrict simulation interfaces to domain coordinates and dimensions.
  - [x] **06.02.b — Verify:** Run simulation with no camera or viewport resource installed.
  - [x] **06.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.03 — Define stable actor identifiers**

  Completed: [implementation and acceptance evidence](docs/evidence/06.03.md).

  - [x] **06.03.a — Implement:** Allocate stable actor IDs independently of transient Bevy entity handles.
  - [x] **06.03.b — Verify:** Despawn and respawn actors without replay ID reuse or ambiguous references.
  - [x] **06.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.04 — Represent route branch section and progress explicitly**

  Completed: [implementation and acceptance evidence](docs/evidence/06.04.md).

  - [x] **06.04.a — Implement:** Store branch identity, section index and local progress rather than one global Z.
  - [x] **06.04.b — Verify:** Follow a split and merge without comparing unrelated branch coordinates directly.
  - [x] **06.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.05 — Define command input consumed at tick boundaries**

  Completed: [implementation and acceptance evidence](docs/evidence/06.05.md).

  - [x] **06.05.a — Implement:** Queue commands with target tick and deterministic ordering.
  - [x] **06.05.b — Verify:** Test multiple input samples between ticks and a frame that executes several ticks.
  - [x] **06.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.06 — Define immutable presentation snapshots**

  Completed: [implementation and acceptance evidence](docs/evidence/06.06.md).

  Progress: [evidence and remaining work](docs/evidence/06.06.md).

  - [x] **06.06.a — Implement:** Build snapshots containing only presentation-required state and stable IDs.
  - [x] **06.06.b — Verify:** Read snapshots without mutating the authoritative world.
  - [x] **06.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.07 — Order simulation stages explicitly**

  Completed: [implementation and acceptance evidence](docs/evidence/06.07.md).

  - [x] **06.07.a — Implement:** Declare system sets and ordering for actors, collision, combat and results.
  - [x] **06.07.b — Verify:** Randomize unrelated system registration and confirm unchanged replay results.
  - [x] **06.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.08 — Use seeded and recorded random streams**

  Completed: [implementation and acceptance evidence](docs/evidence/06.08.md).

  - [x] **06.08.a — Implement:** Document RNG algorithm, seed initialization and each consumer's call order.
  - [x] **06.08.b — Verify:** Replay with visual effects toggled and confirm gameplay randomness stays identical.
  - [x] **06.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.09 — Preserve original arithmetic where parity demands it**

  Completed: [implementation and acceptance evidence](docs/evidence/06.09.md).

  - [x] **06.09.a — Implement:** Implement signed division, shifts, overflow and rounding rules explicitly.
  - [x] **06.09.b — Verify:** Compare boundary vectors against explicit integer semantics and pinned source arithmetic rules.
  - [x] **06.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.10 — Implement headless race runner**

  Completed: [implementation and acceptance evidence](docs/evidence/06.10.md).

  - [x] **06.10.a — Implement:** Expose a CLI scenario runner using commands and fixtures without renderer startup.
  - [x] **06.10.b — Verify:** Run a race slice in CI with deterministic exit status and summary.
  - [x] **06.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.11 — Record canonical state hashes**

  Completed: [implementation and acceptance evidence](docs/evidence/06.11.md).

  - [x] **06.11.a — Implement:** Serialize canonical fields in stable order before hashing.
  - [x] **06.11.b — Verify:** Exclude pointers, padding and unordered-map iteration from hash inputs.
  - [x] **06.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **06.12 — Test identical replay at different render frame rates**

  Completed: [implementation and acceptance evidence](docs/evidence/06.12.md).

  - [x] **06.12.a — Implement:** Render a fixed command sequence under low, high and irregular virtual presentation schedules, distinguished from hardware FPS benchmarks.
  - [x] **06.12.b — Verify:** Verify identical simulation checkpoints and separately assess interpolation smoothness.
  - [x] **06.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/06.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of simulation architecture, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 07 — Course reconstruction

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 04 resource readers and 06 domain unit/route definitions. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Five course containers and known executable tables.

**Required outputs:** Validated course graph, route cursor, event data and inspection report. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Preserve section-local endpoints and branch identity; a flat polyline is insufficient.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/course.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course.cpp), [src/course_scene.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course_scene.cpp), [src/track_ring.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/track_ring.cpp), [src/terrain.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/terrain.cpp).

- [x] **07.01 — Decode section graph and root**

  Completed: [implementation and acceptance evidence](docs/evidence/07.01.md).

  - [x] **07.01.a — Implement:** Parse section records and node links into validated typed graph structures.
  - [x] **07.01.b — Verify:** Reject dangling section references and report the source node offset.
  - [x] **07.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.02 — Validate references and terminal nodes**

  Completed: [implementation and acceptance evidence](docs/evidence/07.02.md).

  - [x] **07.02.a — Implement:** Verify the root exists and terminal nodes can be reached through valid links.
  - [x] **07.02.b — Verify:** Detect unintended cycles without rejecting intentional graph structures blindly.
  - [x] **07.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.03 — Reproduce split and merge traversal**

  Completed: [implementation and acceptance evidence](docs/evidence/07.03.md).

  - [x] **07.03.a — Implement:** Model branch entry, branch choice and merge state transitions explicitly.
  - [x] **07.03.b — Verify:** Test both branches and reverse traversal around each boundary.
  - [x] **07.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.04 — Decode bend and rise independently**

  Completed: [implementation and acceptance evidence](docs/evidence/07.04.md).

  - [x] **07.04.a — Implement:** Accumulate rise as elevation change and bend according to recovered curvature semantics.
  - [x] **07.04.b — Verify:** Compare signed bend/rise sequences without treating both as absolute positions.
  - [x] **07.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.05 — Preserve original lane and shoulder units**

  Completed: [implementation and acceptance evidence](docs/evidence/07.05.md).

  - [x] **07.05.a — Implement:** Retain raw lane and shoulder values and document conversion functions.
  - [x] **07.05.b — Verify:** Test narrow, wide and asymmetric road sections at exact endpoints.
  - [x] **07.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.06 — Reproduce interval stepping at boundaries**

  Completed: [implementation and acceptance evidence](docs/evidence/07.06.md).

  - [x] **07.06.a — Implement:** Reproduce stepwise interval update rules including equality at endpoints.
  - [x] **07.06.b — Verify:** Cover gaps, adjacent intervals and the reference's first-sample boundary cases.
  - [x] **07.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.07 — Decode elevations and side flags**

  Completed: [implementation and acceptance evidence](docs/evidence/07.07.md).

  - [x] **07.07.a — Implement:** Keep absolute elevation, shoulder heights and raw flags in course records.
  - [x] **07.07.b — Verify:** Compare hills/cliffs and preserve unsupported flags for later research.
  - [x] **07.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.08 — Decode event zones and finish markers**

  Completed: [implementation and acceptance evidence](docs/evidence/07.08.md).

  - [x] **07.08.a — Implement:** Translate event ranges and parameters without losing original record order.
  - [x] **07.08.b — Verify:** Test event start, interior, end and one tick beyond the boundary.
  - [x] **07.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.09 — Decode scenery and family placements**

  Completed: [implementation and acceptance evidence](docs/evidence/07.09.md).

  - [x] **07.09.a — Implement:** Decode scenery placement and family instances with stable instance IDs.
  - [x] **07.09.b — Verify:** Compare repeated objects and placement around section transitions.
  - [x] **07.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.10 — Inspect all five courses in viewer**

  Completed: [implementation and acceptance evidence](docs/evidence/07.10.md).

  - [x] **07.10.a — Implement:** Build a route inspector that can select every named course and level.
  - [x] **07.10.b — Verify:** Export a coverage report listing visited sections and unsupported content.
  - [x] **07.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.11 — Compare branch seams and roadside placement**

  Completed: [implementation and acceptance evidence](docs/evidence/07.11.md).

  - [x] **07.11.a — Implement:** Capture before, at and after split/merge seams using a fixed camera.
  - [x] **07.11.b — Verify:** Verify lane edges, terrain and object placement do not jump.
  - [x] **07.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **07.12 — Test forward and reverse traversal**

  Completed: [implementation and acceptance evidence](docs/evidence/07.12.md).

  - [x] **07.12.a — Implement:** Advance and rewind route cursors across section and branch transitions.
  - [x] **07.12.b — Verify:** Check endpoint semantics and avoid reading past the sampled path.
  - [x] **07.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/07.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of course reconstruction, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 08 — Classic road rendering

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 05 content catalog, 07 course data and 06 snapshots. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Course samples, original textures and actor presentation snapshots.

**Required outputs:** Classic renderer plugin, camera specification and visual checkpoint suite. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Projection differences must not be hidden by changing gameplay state.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/road.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/road.cpp), [src/road_draw.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/road_draw.cpp), [src/raster.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/raster.cpp), [src/framebuffer.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/framebuffer.cpp).

- [x] **08.01 — Create classic camera specification**

  Completed: [implementation and acceptance evidence](docs/evidence/08.01.md).

  - [x] **08.01.a — Implement:** Specify focal/projection parameters, horizon, viewport and camera follow rules.

  - [x] **08.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.02 — Implement road projection using sampled route**

  Completed: [implementation and acceptance evidence](docs/evidence/08.02.md).

  - [x] **08.02.a — Implement:** Project sampled road boundaries in a dedicated renderer adapter.
  - [x] **08.02.b — Verify:** Test straight, tight bend, crest and valley sections for monotonic depth handling.
  - [x] **08.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.03 — Render horizon and background layers**

  Completed: [implementation and acceptance evidence](docs/evidence/08.03.md).

  - [x] **08.03.a — Implement:** Layer sky/horizon imagery with correct wrapping and camera response.
  - [x] **08.03.b — Verify:** Test course transitions and extreme steering for visible seams.
  - [x] **08.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.04 — Render hills cliffs buildings and shoulders**

  Completed: [implementation and acceptance evidence](docs/evidence/08.04.md).

  - [x] **08.04.a — Implement:** Render terrain categories from decoded course semantics.
  - [x] **08.04.b — Verify:** Compare buildings, cliffs and hills independently before combining them.
  - [x] **08.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.05 — Match road markings and surface transitions**

  Completed: [implementation and acceptance evidence](docs/evidence/08.05.md).

  - [x] **08.05.a — Implement:** Apply lane markings and surface textures according to event and section data.
  - [x] **08.05.b — Verify:** Test transitions at exactly the source interval endpoints.
  - [x] **08.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.06 — Implement sprite depth ordering**

  Completed: [implementation and acceptance evidence](docs/evidence/08.06.md).

  - [x] **08.06.a — Implement:** Sort sprites using stable depth and documented tie-breaking.
  - [x] **08.06.b — Verify:** Test overlapping racers, roadside objects and traffic at equal depths.
  - [x] **08.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.07 — Match clipping at near plane and viewport edges**

  Completed: [implementation and acceptance evidence](docs/evidence/08.07.md).

  - [x] **08.07.a — Implement:** Clip polygons and sprites crossing near and screen boundaries safely.
  - [x] **08.07.b — Verify:** Test behind-camera objects and degenerate polygons without NaNs or streaks.
  - [x] **08.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.08 — Preserve sprite ground contact and scale**

  Completed: [implementation and acceptance evidence](docs/evidence/08.08.md).

  - [x] **08.08.a — Implement:** Anchor sprites using logical pivot and perspective scale.
  - [x] **08.08.b — Verify:** Compare wheel contact and rider size at several distances.
  - [x] **08.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.09 — Support original aspect presentation**

  Completed: [implementation and acceptance evidence](docs/evidence/08.09.md).

  - [x] **08.09.a — Implement:** Provide letterbox or pillarbox presentation with explicit integer scaling option.
  - [x] **08.09.b — Verify:** Test common aspect ratios without stretching source pixels unexpectedly.
  - [x] **08.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.10 — Add optional widescreen framing policy**

  Completed: [implementation and acceptance evidence](docs/evidence/08.10.md).

  - [x] **08.10.a — Implement:** Specify additional field of view and HUD anchoring for widescreen mode.
  - [x] **08.10.b — Verify:** Show that extra visible scenery does not alter spawn or simulation behavior.
  - [x] **08.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.11 — Compare fixed camera captures at selected checkpoints**

  Completed: [implementation and acceptance evidence](docs/evidence/08.11.md).

  - [x] **08.11.a — Implement:** Create checkpoint captures with known tick, resolution and camera parameters.
  - [x] **08.11.b — Verify:** Produce image diffs with masks only for documented nondeterministic regions.
  - [x] **08.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **08.12 — Separate render interpolation from simulation ticks**

  Completed: [implementation and acceptance evidence](docs/evidence/08.12.md).

  - [x] **08.12.a — Implement:** Interpolate previous/current visual transforms using accumulator fraction.
  - [x] **08.12.b — Verify:** Ensure interpolation never feeds back into collisions or race progress.
  - [x] **08.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/08.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of classic road rendering, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 09 — Input and player handling

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 06 command/tick model and 04 bike table decoding. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Action bindings, bike specs and route surfaces.

**Required outputs:** Input adapter, handling model and measured trace scenarios. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Calibrate against evidence; no arbitrary physics-engine substitution for legacy behavior.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/input.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/input.cpp), [src/bike.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/bike.cpp), [src/race_actions.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_actions.cpp).

- [x] **09.01 — Implement remappable action map**

  Completed: [implementation and acceptance evidence](docs/evidence/09.01.md).

  - [x] **09.01.a — Implement:** Define action IDs separate from physical keys and persist binding contexts.
  - [x] **09.01.b — Verify:** Detect duplicate/conflicting bindings and support restoring defaults.
  - [x] **09.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.02 — Implement keyboard and gamepad input**

  Completed: [implementation and acceptance evidence](docs/evidence/09.02.md).

  - [x] **09.02.a — Implement:** Map digital and analog controls into the same tick command structure.
  - [x] **09.02.b — Verify:** Test simultaneous keys, controller connection and device switching.
  - [x] **09.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.03 — Calibrate dead zones and analog steering**

  Completed: [implementation and acceptance evidence](docs/evidence/09.03.md).

  - [x] **09.03.a — Implement:** Apply documented dead-zone and response curves with calibration preview.
  - [x] **09.03.b — Verify:** Test center drift, full deflection and left/right symmetry.
  - [x] **09.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.04 — Reproduce acceleration and gearing**

  Completed: [implementation and acceptance evidence](docs/evidence/09.04.md).

  - [x] **09.04.a — Implement:** Recover torque/acceleration and gear-change behavior per bike.
  - [x] **09.04.b — Verify:** Compare speed-versus-tick traces from stationary start to top speed.
  - [x] **09.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.05 — Reproduce braking and coasting**

  Completed: [implementation and acceptance evidence](docs/evidence/09.05.md).

  - [x] **09.05.a — Implement:** Implement throttle release, brake force and low-speed stopping behavior.
  - [x] **09.05.b — Verify:** Measure stopping distance and ensure braking cannot produce unintended reverse speed.
  - [x] **09.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.06 — Reproduce lean and lateral motion**

  Completed: [implementation and acceptance evidence](docs/evidence/09.06.md).

  - [x] **09.06.a — Implement:** Model lean response and lateral displacement in domain units.
  - [x] **09.06.b — Verify:** Compare left/right maneuvers at several speeds and steering durations.
  - [x] **09.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.07 — Reproduce hill and airborne behavior**

  Completed: [implementation and acceptance evidence](docs/evidence/09.07.md).

  - [x] **09.07.a — Implement:** Implement grade influence, launch, airtime and landing transitions.
  - [x] **09.07.b — Verify:** Test crests and landings without relying on render-frame delta time.
  - [x] **09.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.08 — Reproduce surface-dependent handling**

  Completed: [implementation and acceptance evidence](docs/evidence/09.08.md).

  - [x] **09.08.a — Implement:** Resolve surface type from road/shoulder/terrain position.
  - [x] **09.08.b — Verify:** Compare traction and deceleration when crossing each boundary.
  - [x] **09.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.09 — Implement bike damage and failure**

  Completed: [implementation and acceptance evidence](docs/evidence/09.09.md).

  - [x] **09.09.a — Implement:** Track damage thresholds and resulting handling/failure transitions.
  - [x] **09.09.b — Verify:** Test exact threshold equality and repeated damage after failure.
  - [x] **09.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.10 — Validate every bike specification**

  Completed: [implementation and acceptance evidence](docs/evidence/09.10.md).

  - [x] **09.10.a — Implement:** Load all supported bike specifications through the same schema.
  - [x] **09.10.b — Verify:** Report missing fields and compare representative slow, fast and heavy bikes.
  - [x] **09.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.11 — Pause safely on focus loss and disconnect**

  Completed: [implementation and acceptance evidence](docs/evidence/09.11.md).

  - [x] **09.11.a — Implement:** Pause on focus loss or active controller disconnect with explicit resume.
  - [x] **09.11.b — Verify:** Verify held throttle does not remain stuck after reconnection.
  - [x] **09.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **09.12 — Compare acceleration braking and cornering traces**

  Completed: [implementation and acceptance evidence](docs/evidence/09.12.md).

  - [x] **09.12.a — Implement:** Create measured input scripts with known initial conditions.
  - [x] **09.12.b — Verify:** Plot speed, lateral position and state transitions against versioned engine regression fixtures.
  - [x] **09.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/09.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of input and player handling, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 10 — Rider animation and palette fidelity

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 04 BOB/remap readers, 05 frame metadata and 06 actor state model. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Source frame sequences and simulation states.

**Required outputs:** Animation tables, sprite presentation adapter and family previews. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Visual frame count or enhancement scale cannot determine gameplay timing.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/rider_sprites.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/rider_sprites.cpp), [src/actor_animation.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/actor_animation.cpp), [src/animation.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/animation.cpp).

- [x] **10.01 — Map animation states to frame sequences**

  Completed: [implementation and acceptance evidence](docs/evidence/10.01.md).

  - [x] **10.01.a — Implement:** Create explicit state-to-animation mappings with fallback policy.
  - [x] **10.01.b — Verify:** Report missing required states before a race starts.
  - [x] **10.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.02 — Preserve original per-frame timing**

  Completed: [implementation and acceptance evidence](docs/evidence/10.02.md).

  - [x] **10.02.a — Implement:** Convert source animation durations to simulation-driven timing.
  - [x] **10.02.b — Verify:** Compare full loops under varying render frame rates.
  - [x] **10.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.03 — Implement steering and lean poses**

  Completed: [implementation and acceptance evidence](docs/evidence/10.03.md).

  - [x] **10.03.a — Implement:** Select lean/steer views from simulation state and documented thresholds.
  - [x] **10.03.b — Verify:** Test threshold crossings for oscillation and abrupt view changes.
  - [x] **10.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.04 — Implement punch kick and backhand poses**

  Completed: [implementation and acceptance evidence](docs/evidence/10.04.md).

  - [x] **10.04.a — Implement:** Map each attack phase to its intended visual frames.
  - [x] **10.04.b — Verify:** Verify visible contact frames agree with gameplay hit timing.
  - [x] **10.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.05 — Implement weapon carrying poses**

  Completed: [implementation and acceptance evidence](docs/evidence/10.05.md).

  - [x] **10.05.a — Implement:** Represent equipped weapons as state or attachments with stable anchors.
  - [x] **10.05.b — Verify:** Test theft and drop transitions without stale weapon images.
  - [x] **10.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.06 — Implement crash and slide frames**

  Completed: [implementation and acceptance evidence](docs/evidence/10.06.md).

  - [x] **10.06.a — Implement:** Drive crash and slide animations from authoritative crash states.
  - [x] **10.06.b — Verify:** Verify high-speed and low-speed crashes select coherent sequences.
  - [x] **10.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.07 — Implement getting up and on-foot states**

  Completed: [implementation and acceptance evidence](docs/evidence/10.07.md).

  - [x] **10.07.a — Implement:** Implement get-up, standing, walking and remount sequences explicitly.
  - [x] **10.07.b — Verify:** Check end-of-sequence transitions and interrupted recovery.
  - [x] **10.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.08 — Preserve palette-based rider identity**

  Completed: [implementation and acceptance evidence](docs/evidence/10.08.md).

  - [x] **10.08.a — Implement:** Apply rider remaps before enhancement compositing according to catalog policy.
  - [x] **10.08.b — Verify:** Verify distinct riders retain consistent colors across all states.
  - [x] **10.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.09 — Match shadow placement and transparency**

  Completed: [implementation and acceptance evidence](docs/evidence/10.09.md).

  - [x] **10.09.a — Implement:** Position shadow independently using ground contact and height cues.
  - [x] **10.09.b — Verify:** Test airborne riders and slopes without shadows floating with the sprite.
  - [x] **10.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.10 — Test mirrored sprites for asymmetric details**

  Completed: [implementation and acceptance evidence](docs/evidence/10.10.md).

  - [x] **10.10.a — Implement:** Audit left/right mirroring for logos, weapons and asymmetrical clothing.
  - [x] **10.10.b — Verify:** Author explicit exceptions and test both facing directions.
  - [x] **10.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.11 — Provide animation contact sheet playback**

  Completed: [implementation and acceptance evidence](docs/evidence/10.11.md).

  - [x] **10.11.a — Implement:** Add frame stepping, loop speed and state transition preview tools.
  - [x] **10.11.b — Verify:** Export a labeled sheet with timing and pivot for each animation family.
  - [x] **10.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **10.12 — Validate transitions without frame popping**

  Completed: [implementation and acceptance evidence](docs/evidence/10.12.md).

  - [x] **10.12.a — Implement:** Define transition blending or hard-cut policy per state pair.
  - [x] **10.12.b — Verify:** Review attack-to-crash and get-up-to-walk sequences without pivot jumps.
  - [x] **10.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/10.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of rider animation and palette fidelity, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 11 — Combat and contact

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 06 stable scheduling, 09 movement and actor state definitions. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Tick commands, actor states, attack definitions and contact data.

**Required outputs:** Combat rules, ownership transitions and isolated replay fixtures. Completed: [Part 11 evidence](docs/evidence/part-11.md), [combat contract](docs/COMBAT.md).

**Invariant:** Resolve gameplay once; audiovisual events only report the result.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/race_actions.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_actions.cpp), [src/crash.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/crash.cpp), [src/race_helpers.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_helpers.cpp).

- [x] **11.01 — Specify attack startup active and recovery windows**

  - [x] **11.01.a — Implement:** Represent attack phases in simulation ticks with data-driven durations.
  - [x] **11.01.b — Verify:** Test first/last active tick and cancellation on crash.
  - [x] **11.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.02 — Define left and right reach**

  - [x] **11.02.a — Implement:** Define attack volumes relative to actor orientation and lateral side.
  - [x] **11.02.b — Verify:** Test targets just inside/outside reach at different relative speeds.
  - [x] **11.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.03 — Resolve target selection deterministically**

  - [x] **11.03.a — Implement:** Choose targets by explicit eligibility and stable tie-break rules.
  - [x] **11.03.b — Verify:** Test two equally close opponents and ensure identical replay outcomes.
  - [x] **11.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.04 — Implement punch damage and reactions**

  - [x] **11.04.a — Implement:** Apply punch damage and hit reaction once per allowed contact window.
  - [x] **11.04.b — Verify:** Test repeated overlap does not multiply a single hit unexpectedly.
  - [x] **11.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.05 — Implement kick displacement and reactions**

  - [x] **11.05.a — Implement:** Apply kick impulse and state response using domain units.
  - [x] **11.05.b — Verify:** Compare left/right kicks and edge-of-road targets.
  - [x] **11.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.06 — Implement weapon ownership and theft**

  - [x] **11.06.a — Implement:** Model weapon ownership as a unique relationship with transfer rules.
  - [x] **11.06.b — Verify:** Test simultaneous steal attempts and prevent duplicate ownership.
  - [x] **11.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.07 — Implement stun and health depletion**

  - [x] **11.07.a — Implement:** Separate health, stun duration and crash-trigger thresholds.
  - [x] **11.07.b — Verify:** Test exact zero health and stun expiry on the same tick as a new hit.
  - [x] **11.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.08 — Resolve simultaneous attacks consistently**

  - [x] **11.08.a — Implement:** Define resolution order for mutual hits and actor removal.
  - [x] **11.08.b — Verify:** Replay simultaneous attacks repeatedly with equal results.
  - [x] **11.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.09 — Separate cosmetic effects from hit logic**

  - [x] **11.09.a — Implement:** Emit sound/particle/camera events after gameplay resolution.
  - [x] **11.09.b — Verify:** Disable all effects and verify identical combat state hashes.
  - [x] **11.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.10 — Validate bike-to-bike contact**

  - [x] **11.10.a — Implement:** Resolve bike contact using relative position and speed criteria.
  - [x] **11.10.b — Verify:** Test overtaking, side contact and stopped obstacles.
  - [x] **11.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **11.11 — Record isolated combat scenarios**

  - [x] **11.11.a — Implement:** Create isolated cases for every attack and weapon interaction.
  - [x] **11.11.b — Verify:** Record expected health, displacement, owner and state after each tick range.
  - [x] **11.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/11.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.



**Exit evidence:** a reproducible demonstration of combat and contact, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 12 — Crashes and recovery

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 09 handling, 11 impacts and 07 route topology. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Separated rider/bike states and local course geometry.

**Required outputs:** Crash/recovery state machine and recovery scenario suite. Completed: [Part 12 evidence](docs/evidence/part-12.md), [recovery contract](docs/RECOVERY.md).

**Invariant:** Recovery must remain valid across collisions, arrest, finish and branch transitions.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/crash.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/crash.cpp), [src/race_schedule.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_schedule.cpp).

- [x] **12.01 — Separate rider and bike crash states**

  - [x] **12.01.a — Implement:** Model rider and bike as separate recoverable entities with linked IDs.
  - [x] **12.01.b — Verify:** Test one object moving while the other is stationary.
  - [x] **12.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.02 — Implement slide and tumble behavior**

  - [x] **12.02.a — Implement:** Implement crash motion with explicit velocity decay and state progression.
  - [x] **12.02.b — Verify:** Compare travel distance and timing at representative entry speeds.
  - [x] **12.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.03 — Implement off-road collision consequences**

  - [x] **12.03.a — Implement:** Classify obstacles and terrain collision consequences.
  - [x] **12.03.b — Verify:** Test road edge, solid obstacle and repeated impact cases.
  - [x] **12.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.04 — Track detached bike position**

  - [x] **12.04.a — Implement:** Keep bike position and recoverability in authoritative state after separation.
  - [x] **12.04.b — Verify:** Verify camera culling cannot remove the player's recovery target.
  - [x] **12.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.05 — Implement rider get-up timing**

  - [x] **12.05.a — Implement:** Define get-up delay and interruption conditions.
  - [x] **12.05.b — Verify:** Test additional impact on the final get-up tick.
  - [x] **12.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.06 — Implement on-foot movement and recovery target**

  - [x] **12.06.a — Implement:** Choose recovery route/target using the actual branch and bike location.
  - [x] **12.06.b — Verify:** Test a bike behind the rider and on a neighboring branch.
  - [x] **12.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.07 — Implement remount conditions**

  - [x] **12.07.a — Implement:** Check distance, orientation and bike availability before remount.
  - [x] **12.07.b — Verify:** Test exact thresholds and concurrent collision during mount.
  - [x] **12.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.08 — Handle repeated collision during recovery**

  - [x] **12.08.a — Implement:** Specify whether damage resets, interrupts or preserves recovery phases.
  - [x] **12.08.b — Verify:** Replay repeated impacts without invalid animation states.
  - [x] **12.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.09 — Handle finish and arrest during recovery**

  - [x] **12.09.a — Implement:** Order finish, arrest, crash and recovery transitions explicitly.
  - [x] **12.09.b — Verify:** Test coincident events and verify only one result flow activates.
  - [x] **12.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.10 — Prevent stuck or unreachable recovery loops**

  - [x] **12.10.a — Implement:** Detect impossible recovery states and define a documented fallback.
  - [x] **12.10.b — Verify:** Test inaccessible bike and invalid target without infinite loops.
  - [x] **12.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.11 — Test recovery on slopes and branches**

  - [x] **12.11.a — Implement:** Build cases on steep grades and close to split/merge boundaries.
  - [x] **12.11.b — Verify:** Verify rider navigation and bike association remain on valid route data.
  - [x] **12.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **12.12 — Document deliberate accessibility assists**

  - [x] **12.12.a — Implement:** Put optional fast recovery or assistance behind explicit settings.
  - [x] **12.12.b — Verify:** Verify fidelity mode retains measured timings and replay metadata records assists.
  - [x] **12.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/12.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of crashes and recovery, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 13 — Opponent intelligence

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 06 ordering/RNG, 09 handling, 11 combat and 12 recovery. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Rider traits, routes, nearby actors and seeded randomness.

**Required outputs:** Opponent decision systems and full-field replay coverage. Completed: [Part 13 evidence](docs/evidence/part-13.md), [opponent contract](docs/OPPONENTS.md).

**Invariant:** Track decisions and scheduling independently; a plausible race is not parity evidence.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/race.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race.cpp), [src/race_schedule.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_schedule.cpp), [src/race_helpers.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_helpers.cpp).

- [x] **13.01 — Recover rider scheduling intervals**

  - [x] **13.01.a — Implement:** Define player/opponent cadence from source research and explicit scheduling policy.
  - [x] **13.01.b — Verify:** Test due-tick boundaries and first update after spawn.
  - [x] **13.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.02 — Preserve meaningful actor update order**

  - [x] **13.02.a — Implement:** Use a stable scheduling key instead of arbitrary ECS iteration order.
  - [x] **13.02.b — Verify:** Verify insertions of cosmetic entities cannot reorder AI updates.
  - [x] **13.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.03 — Implement route following**

  - [x] **13.03.a — Implement:** Track route choice and desired lateral path per opponent.
  - [x] **13.03.b — Verify:** Test both branch options and lane transitions.
  - [x] **13.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.04 — Implement speed and cornering decisions**

  - [x] **13.04.a — Implement:** Compute target speed from bike capability, curve and state evidence.
  - [x] **13.04.b — Verify:** Compare braking/acceleration decisions around a known corner.
  - [x] **13.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.05 — Implement overtaking and avoidance**

  - [x] **13.05.a — Implement:** Model candidate overtakes with clearance and relative-speed constraints.
  - [x] **13.05.b — Verify:** Test blocked lanes, slow traffic and two rivals attempting the same gap.
  - [x] **13.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.06 — Implement aggression and target choice**

  - [x] **13.06.a — Implement:** Represent aggression/personality parameters separately from render assets.
  - [x] **13.06.b — Verify:** Test target eligibility when a rival crashes or leaves range.
  - [x] **13.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.07 — Implement combat decisions**

  - [x] **13.07.a — Implement:** Select attack type and timing from explicit decision rules.
  - [x] **13.07.b — Verify:** Test cooldowns and ensure AI cannot attack while an invalid state is active.
  - [x] **13.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.08 — Implement crash recovery decisions**

  - [x] **13.08.a — Implement:** Give crashed opponents the same defined recovery states as applicable.
  - [x] **13.08.b — Verify:** Test prolonged recovery and re-entry into traffic.
  - [x] **13.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.09 — Match rider personalities where evidenced**

  - [x] **13.09.a — Implement:** Map observed personality differences to documented data fields.
  - [x] **13.09.b — Verify:** Leave unsupported traits unknown rather than inventing faithful-sounding values.
  - [x] **13.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.10 — Audit RNG consumption per behavior**

  - [x] **13.10.a — Implement:** Trace RNG calls by actor, tick and decision category in debug mode.
  - [x] **13.10.b — Verify:** Locate first divergence without changing release simulation behavior.
  - [x] **13.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.11 — Test full original field size**

  - [x] **13.11.a — Implement:** Run the measured original field size with all riders active.
  - [x] **13.11.b — Verify:** Verify placement, scheduling and asset identity across the whole field.
  - [x] **13.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **13.12 — Compare long-run position and behavior traces**

  - [x] **13.12.a — Implement:** Compare multi-minute seeded runs and recorded human inputs.
  - [x] **13.12.b — Verify:** Separate scheduler, RNG and arithmetic divergence in the evidence report.
  - [x] **13.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/13.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of opponent intelligence, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 14 — Traffic pedestrians and police

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 07 course events, 06 scheduling and 09/12 actor interaction contracts. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Event intervals, actor definitions and pursuit conditions.

**Required outputs:** Traffic/pedestrian/police systems with scenario coverage. Completed: [Part 14 evidence](docs/evidence/part-14.md), [road-user contract](docs/ROAD_USERS.md).

**Invariant:** Keep spawn order, branch assignment and terminal-event ordering explicit.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/traffic.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/traffic.cpp), [src/pedestrians.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/pedestrians.cpp), [src/police.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/police.cpp).

- [x] **14.01 — Decode traffic spawn and density events**

  - [x] **14.01.a — Implement:** Decode traffic density, speed and intersection event parameters.
  - [x] **14.01.b — Verify:** Test entry and exit of each event zone without duplicate spawning.
  - [x] **14.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.02 — Implement lane following and crossing traffic**

  - [x] **14.02.a — Implement:** Implement route-aware traffic lane movement and intersection controllers.
  - [x] **14.02.b — Verify:** Test crossing vehicles against approaching racers on both branches.
  - [x] **14.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.03 — Implement traffic collision responses**

  - [x] **14.03.a — Implement:** Resolve rider/vehicle impacts with documented speed and orientation effects.
  - [x] **14.03.b — Verify:** Test rear, head-on and lateral contacts separately.
  - [x] **14.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.04 — Implement pedestrian spawning and crossing**

  - [x] **14.04.a — Implement:** Spawn pedestrians from source rules and assign crossing trajectories.
  - [x] **14.04.b — Verify:** Test route boundaries and pedestrians interrupted by traffic.
  - [x] **14.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.05 — Implement pedestrian hit outcomes**

  - [x] **14.05.a — Implement:** Implement pedestrian reactions and relevant scoring/state effects.
  - [x] **14.05.b — Verify:** Verify one contact does not repeatedly trigger consequences each frame.
  - [x] **14.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.06 — Implement police patrol and pursuit**

  - [x] **14.06.a — Implement:** Model police patrol, detection, chase and disengagement states.
  - [x] **14.06.b — Verify:** Test suspect entering and leaving detection conditions.
  - [x] **14.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.07 — Implement siren and warning cues**

  - [x] **14.07.a — Implement:** Emit siren start/stop events based on pursuit lifecycle.
  - [x] **14.07.b — Verify:** Pause, restart and despawn police without orphaned looping audio.
  - [x] **14.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.08 — Implement arrest conditions**

  - [x] **14.08.a — Implement:** Specify arrest eligibility by speed, distance and player state.
  - [x] **14.08.b — Verify:** Test near-threshold cases and simultaneous police contact.
  - [x] **14.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.09 — Implement arrest while on foot**

  - [x] **14.09.a — Implement:** Handle arrest during standing, walking, get-up and remount states.
  - [x] **14.09.b — Verify:** Verify the result transition cleans up the recovery state.
  - [x] **14.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.10 — Handle route splits for all actors**

  - [x] **14.10.a — Implement:** Preserve branch ownership for vehicles, pedestrians and pursuit targets.
  - [x] **14.10.b — Verify:** Test actors near merges without teleporting across disconnected roads.
  - [x] **14.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.11 — Match spawn ordering and RNG use**

  - [x] **14.11.a — Implement:** Record traffic/pedestrian/police scheduling and random choices.
  - [x] **14.11.b — Verify:** Compare seeded dense scenes and identify update-order differences.
  - [x] **14.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **14.12 — Test dense scenes and simultaneous incidents**

  - [x] **14.12.a — Implement:** Create stress cases combining attacks, traffic collision and police pursuit.
  - [x] **14.12.b — Verify:** Verify bounded actor counts and deterministic outcome ordering.
  - [x] **14.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/14.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of traffic pedestrians and police, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 15 — Race rules and results

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 09–14 race actors and 07 finish/progress semantics. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Race setup, actor progress and terminal events.

**Required outputs:** [Race/session contract](docs/RACE_RULES.md), immutable standings/results and [Part 15 evidence](docs/evidence/part-15.md).

**Invariant:** Apply outcomes and financial consequences exactly once.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/race.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race.cpp), [src/session.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/session.cpp), [src/race_screen.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_screen.cpp).

- [x] **15.01 — Implement countdown and race start**

  - [x] **15.01.a — Implement:** Implement pre-race setup, countdown and control unlock at defined ticks.
  - [x] **15.01.b — Verify:** Test held input during countdown and restart.
  - [x] **15.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.01.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.02 — Compute progress across route branches**

  - [x] **15.02.a — Implement:** Calculate comparable race progress using route topology and finish rules.
  - [x] **15.02.b — Verify:** Test rivals on unequal branches without naive distance ranking.
  - [x] **15.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.02.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.03 — Implement placement and standings**

  - [x] **15.03.a — Implement:** Maintain rank with stable tie-breaking and finish status.
  - [x] **15.03.b — Verify:** Test overtakes, crashes and equal-progress actors.
  - [x] **15.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.03.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.04 — Implement finish detection**

  - [x] **15.04.a — Implement:** Detect crossing of the finish condition even with large per-tick travel.
  - [x] **15.04.b — Verify:** Test exact-boundary and overshoot cases.
  - [x] **15.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.04.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.05 — Resolve simultaneous finish events**

  - [x] **15.05.a — Implement:** Assign finish order using documented within-tick resolution rules.
  - [x] **15.05.b — Verify:** Replay ties and verify consistent rewards/results.
  - [x] **15.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.05.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.06 — Implement race failure conditions**

  - [x] **15.06.a — Implement:** Define damaged-bike, arrest, timeout or other evidenced failure paths.
  - [x] **15.06.b — Verify:** Test each terminal state routes to the correct frontend outcome.
  - [x] **15.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.06.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.07 — Implement pause resume restart quit**

  - [x] **15.07.a — Implement:** Specify pause ownership and restart/reset behavior.
  - [x] **15.07.b — Verify:** Pause during countdown, combat and results without advancing simulation.
  - [x] **15.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.07.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.08 — Implement results presentation**

  - [x] **15.08.a — Implement:** Populate results from immutable completed-race data.
  - [x] **15.08.b — Verify:** Verify names, positions, rewards and status correspond to the finished run.
  - [x] **15.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.08.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.09 — Record race rewards and penalties**

  - [x] **15.09.a — Implement:** Apply rewards and penalties exactly once in a career transaction.
  - [x] **15.09.b — Verify:** Revisit results and ensure funds are not duplicated.
  - [x] **15.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.09.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.10 — Verify race-to-menu cleanup**

  - [x] **15.10.a — Implement:** Despawn race-scoped entities and stop race-scoped audio on exit.
  - [x] **15.10.b — Verify:** Return to menus repeatedly without stale actors or cameras.
  - [x] **15.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.10.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.11 — Test repeat races without leaked state**

  - [x] **15.11.a — Implement:** Run repeated races with changing courses and bikes.
  - [x] **15.11.b — Verify:** Compare memory, actor counts and RNG initialization after each reset.
  - [x] **15.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.11.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **15.12 — Verify intended outcome and frontend state transitions**

  - [x] **15.12.a — Implement:** Maintain an outcome table for win, loss, arrest and wreck paths.

  - [x] **15.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/15.12.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of race rules and results, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 16 — Menus career and economy

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 01 screen inventory, 15 result contract and 17 save schema coordination. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Localized content, bike catalog and career records.

**Required outputs:** [Typed frontend/career contract](docs/FRONTEND.md), economy/flow tests, [complete inventory audit](docs/evidence/frontend-menu-audit.json) and [Part 16 evidence](docs/evidence/part-16.md).

**Invariant:** Keep frontend state transitions independent from currently spawned race entities.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/frontend.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/frontend.cpp), [src/career.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/career.cpp), [src/native_screens.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/native_screens.cpp), [src/native_strings.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/native_strings.cpp).

- [x] **16.01 — Build explicit frontend state graph**

  - [x] **16.01.a — Implement:** Describe frontend states, transitions and required context as typed data.
  - [x] **16.01.b — Verify:** Test back/cancel behavior from every reachable screen.
  - [x] **16.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.01.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.02 — Implement Thrash flow**

  - [x] **16.02.a — Implement:** Implement quick-race/Thrash selection through results and return.
  - [x] **16.02.b — Verify:** Complete consecutive races without creating unintended career progress.
  - [x] **16.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.02.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.03 — Implement Big Game career flow**

  - [x] **16.03.a — Implement:** Implement career creation, resume and between-race flow.
  - [x] **16.03.b — Verify:** Finish a multi-race progression path and verify persistence.
  - [x] **16.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.03.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.04 — Implement character selection**

  - [x] **16.04.a — Implement:** Map character choice to identity, portrait and gameplay data.
  - [x] **16.04.b — Verify:** Test every option and invalid saved character IDs.
  - [x] **16.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.04.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.05 — Implement course and level selection**

  - [x] **16.05.a — Implement:** Apply unlock and selection rules to course/level menus.
  - [x] **16.05.b — Verify:** Test locked, newly unlocked and completed states.
  - [x] **16.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.05.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.06 — Implement bike showroom and purchase**

  - [x] **16.06.a — Implement:** Display bike specifications, affordability and purchase confirmation.
  - [x] **16.06.b — Verify:** Test insufficient funds and selecting the currently owned bike.
  - [x] **16.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.06.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.07 — Implement repair and money changes**

  - [x] **16.07.a — Implement:** Apply repair costs, damage changes and money updates atomically.
  - [x] **16.07.b — Verify:** Test exact affordability and repeated confirmation inputs.
  - [x] **16.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.07.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.08 — Implement Schmooze interactions**

  - [x] **16.08.a — Implement:** Represent dialogue options and their effect on career state.
  - [x] **16.08.b — Verify:** Test cancel, repeat visits and unavailable choices.
  - [x] **16.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.08.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.09 — Implement unlock and progression rules**

  - [x] **16.09.a — Implement:** Encode progression thresholds and unlock conditions in testable data.
  - [x] **16.09.b — Verify:** Test just-below, exact and above-threshold outcomes.
  - [x] **16.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.09.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.10 — Implement results and return transitions**

  - [x] **16.10.a — Implement:** Pass a completed-race summary into the frontend without live-world references.
  - [x] **16.10.b — Verify:** Test quit, fail and finish return paths.
  - [x] **16.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.10.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.11 — Implement settings credits and jukebox**

  - [x] **16.11.a — Implement:** Implement settings, credits and music menus with explicit return destinations.
  - [x] **16.11.b — Verify:** Test audio state and focus after leaving each screen.
  - [x] **16.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.11.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **16.12 — Audit every menu item against evidence ledger**

  - [x] **16.12.a — Implement:** Use the screen inventory to tick off every item and transition.
  - [x] **16.12.b — Verify:** Report missing screens separately from approximate visual matches.
  - [x] **16.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in [completion receipt](docs/evidence/16.12.md). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of menus career and economy, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 17 — Saves settings and migration

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 01 progression scope and 05 stable IDs; coordinate schema with 16. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Versioned domain records and user settings.

**Required outputs:** Persistence adapter, schema migrations and recovery fixtures. Implemented: see [Part 17 evidence](docs/evidence/part-17.md).

**Invariant:** Validate before mutation and preserve old valid data through write failure.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/career.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/career.cpp), [src/settings.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/settings.cpp), [src/atomic_file.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/atomic_file.cpp).

- [x] **17.01 — Define versioned save schema**

  - [x] **17.01.a — Implement:** Define save records with schema version, stable IDs and bounded collections.
  - [x] **17.01.b — Verify:** Round-trip a representative career without engine entity IDs.
  - [x] **17.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.01.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.02 — Separate settings from career progress**

  - [x] **17.02.a — Implement:** Store preferences separately from progression and per-run state.
  - [x] **17.02.b — Verify:** Reset graphics settings without deleting the player's career.
  - [x] **17.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.02.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.03 — Use atomic save writes**

  - [x] **17.03.a — Implement:** Write a temporary save, flush as required and replace safely.
  - [x] **17.03.b — Verify:** Simulate failures before replacement and retain the prior valid file.
  - [x] **17.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.03.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.04 — Keep recoverable backup**

  - [x] **17.04.a — Implement:** Keep a documented backup rotation and recovery choice.
  - [x] **17.04.b — Verify:** Corrupt the newest save and demonstrate restoration without silent data loss.
  - [x] **17.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.04.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.05 — Validate loaded data before world mutation**

  - [x] **17.05.a — Implement:** Validate money, IDs, progression and collection limits before applying state.
  - [x] **17.05.b — Verify:** Reject malformed values with a useful diagnostic.
  - [x] **17.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.05.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.06 — Test truncated and corrupt saves**

  - [x] **17.06.a — Implement:** Create short, invalid and partially encoded save fixtures.
  - [x] **17.06.b — Verify:** Load them without panic or modifying the active session.
  - [x] **17.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.06.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.07 — Implement explicit schema migrations**

  - [x] **17.07.a — Implement:** Write explicit migrations for each supported older schema.
  - [x] **17.07.b — Verify:** Test old-to-current conversion and avoid reapplying a migration twice.
  - [x] **17.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.07.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.08 — Record content pack compatibility**

  - [x] **17.08.a — Implement:** Record required content IDs and pack versions/hashes where necessary.
  - [x] **17.08.b — Verify:** Load with a missing mod and present a defined recovery policy.
  - [x] **17.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.08.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.09 — Persist input bindings and audio levels**

  - [x] **17.09.a — Implement:** Persist bindings, volume and presentation settings with defaults.
  - [x] **17.09.b — Verify:** Test absent fields and unsupported old key names.
  - [x] **17.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.09.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.10 — Test interrupted writes**

  - [x] **17.10.a — Implement:** Inject failure at each save stage using a controlled test adapter.
  - [x] **17.10.b — Verify:** Verify resulting files are either old-valid or new-valid.
  - [x] **17.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.10.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.11 — Decide original-save import scope**

  - [x] **17.11.a — Implement:** Decide whether original save import is a release requirement or later feature.
  - [x] **17.11.b — Verify:** Document the decision and avoid claiming binary compatibility without tests.
  - [x] **17.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.11.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **17.12 — Document save location and reset procedure**

  - [x] **17.12.a — Implement:** Document per-platform paths, backups, migration and reset operations.
  - [x] **17.12.b — Verify:** Follow recovery instructions in a clean user profile.
  - [x] **17.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/17.12.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** [Part 17 completed receipt](docs/evidence/part-17.md), linked task proofs, tests, source comparison and declared limits. Required full-game gaps remain in Part 19.

</details>

### Phase 18 — Audio music and video

<details>
<summary>Completed — expand checklist and evidence</summary>

**Prerequisites:** 04 media readers, 05 catalog and 06/15 event lifecycles. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Original local media and resolved gameplay/frontend events.

**Required outputs:** Audio/video adapters, cue map and synchronization scenarios. Implemented: see [Part 18 evidence](docs/evidence/part-18.md).

**Invariant:** Missing devices and skipped video must not corrupt race or menu state.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/sound.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/sound.cpp), [src/mixer.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/mixer.cpp), [src/race_music.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_music.cpp), [src/avi.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/avi.cpp), [src/cinepak.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/cinepak.cpp), [src/ima_adpcm.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/ima_adpcm.cpp).

- [x] **18.01 — Inventory sound effects and cue mappings**

  - [x] **18.01.a — Implement:** Catalog effect IDs, source WAVs, loop flags and triggering gameplay events.
  - [x] **18.01.b — Verify:** Verify every required cue resolves and duplicates are intentional.
  - [x] **18.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.01.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.02 — Implement engine pitch and throttle response**

  - [x] **18.02.a — Implement:** Drive engine loops/pitch from speed, gear and throttle with bounded transitions.
  - [x] **18.02.b — Verify:** Compare idle, acceleration, shifting and deceleration without audible discontinuity.
  - [x] **18.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.02.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.03 — Implement collision and combat cues**

  - [x] **18.03.a — Implement:** Map hit/crash categories to cues with controlled overlap and priority.
  - [x] **18.03.b — Verify:** Test simultaneous impacts without clipping or repeated retrigger storms.
  - [x] **18.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.03.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.04 — Implement police siren lifecycle**

  - [x] **18.04.a — Implement:** Tie siren voices to police IDs and pursuit state.
  - [x] **18.04.b — Verify:** Stop every voice when pursuit ends, a race exits or audio resets.
  - [x] **18.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.04.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.05 — Implement music selection and looping**

  - [x] **18.05.a — Implement:** Implement soundtrack selection, advancement and loop policy.
  - [x] **18.05.b — Verify:** Test end-of-track, missing media and menu/race transitions.
  - [x] **18.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.05.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.06 — Expose independent volume buses**

  - [x] **18.06.a — Implement:** Create master, music, effects and engine volume controls.
  - [x] **18.06.b — Verify:** Verify mute and restore affect only the intended buses.
  - [x] **18.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.06.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.07 — Pause and resume audio consistently**

  - [x] **18.07.a — Implement:** Preserve playback policy across pause and resume.
  - [x] **18.07.b — Verify:** Test repeated pauses without duplicated sources or lost loop state.
  - [x] **18.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.07.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.08 — Handle missing audio devices**

  - [x] **18.08.a — Implement:** Handle device loss and initialization failure without stopping gameplay.
  - [x] **18.08.b — Verify:** Test a system with no available audio output.
  - [x] **18.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.08.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.09 — Decode supported video and audio tracks**

  - [x] **18.09.a — Implement:** Decode supported AVI video/audio streams through a bounded adapter.
  - [x] **18.09.b — Verify:** Compare frame count, duration and decoded samples on selected fixtures.
  - [x] **18.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.09.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.10 — Synchronize video playback timing**

  - [x] **18.10.a — Implement:** Use a media clock and defined frame-drop policy for synchronization.
  - [x] **18.10.b — Verify:** Test slow decoding and long playback for drift.
  - [x] **18.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.10.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.11 — Implement skip and return flow**

  - [x] **18.11.a — Implement:** Allow skip only through documented transition points and cleanup media resources.
  - [x] **18.11.b — Verify:** Skip immediately, near the end and after pause.
  - [x] **18.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.11.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **18.12 — Compare cues and audiovisual timing**

  - [x] **18.12.a — Implement:** Record versioned engine cue timestamps and source-media playback durations.
  - [x] **18.12.b — Verify:** Compare timing separately from subjective timbre differences.
  - [x] **18.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/18.12.md` . For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** [Part 18 completed receipt](docs/evidence/part-18.md), linked task proofs, tests, source comparison and declared limits. Required full-game gaps remain in Part 19.

</details>

### Phase 19 — Complete the native-asset offline game

**Prerequisites:** Parts 01–18 provide implemented subsystems; their declared fidelity and functionality gaps are inputs to this completion gate. Original Windows execution remains optional under the owner's earlier scope decision.

**Inputs:** The source corpus, current gap/audit records, real input/replay tools and a fresh profile. Use original textures throughout.

**Required outputs:** A complete offline functional candidate, a closed required-feature manifest and a native-only end-to-end career demonstration. This gate is not complete merely because subsystem scaffolds have checked boxes.

**Invariant:** No AI-generated or enhanced texture is necessary for any required game feature. Required functionality cannot be silently reclassified as visual polish.

**Execution rule:** Every required feature must have code, a command-driven or ordinary-input scenario, expected results and a completion receipt. Compare against the pinned source/native APIs and recorded intended rules; do not reintroduce waived original-executable comparisons.

- [x] **19.01 — Freeze the native-only functional acceptance manifest**

  - [x] **19.01.a — Implement:** Enumerate every offline feature, all 25 course/level combinations, all 15 bikes, eight riders, required menu items, media, save paths and supported controls. Link every known gap from Parts 01–18; distinguish intentional documented policy from an absent required function.
  - [x] **19.01.b — Verify:** Run the manifest with all enhanced packs disabled. Each required entry has an executable scenario and an expected result; unresolved required functions remain open.
  - [x] **19.01.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.01.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [x] **19.02 — Complete the original player action set**

  - [x] **19.02.a — Implement:** Implement and bind remaining required controls, including nitro/boost and intentional dismount; resolve steering/lean semantics, inventory limits, ground/air/recovery eligibility and pause/input ownership from the pinned source or an explicit approved rule.
  - [x] **19.02.b — Verify:** Exercise press, hold, release, depletion, simultaneous controls and crash/remount transitions. Verify actions through the real input adapter and include their state in replays.
  - [x] **19.02.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.02.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [x] **19.03 — Complete actor appearances and animation states**

  - [x] **19.03.a — Implement:** Replace first-drawable pedestrian previews with state-appropriate CANS timing; audit all rider, vehicle, cop and pedestrian appearances, mirrors, pivots, shadows and interaction poses. Preserve the seven confirmed absent native appearances as evidenced exclusions.
  - [x] **19.03.b — Verify:** Run the complete actor/state matrix on every course. No invisible collidable actor, missing required transition, incorrect ground contact or unexplained animation gap remains.
  - [x] **19.03.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.03.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [ ] **19.04 — Close traffic and police functional gaps**

  - [ ] **19.04.a — Implement:** Audit and complete required zone/controller, lane-change, crossing, pedestrian avoidance, pursuit, cop-hit/recovery and arrest behaviors. Resolve the current simplified policies explicitly, retaining topology ownership and bounded pools.
  - [ ] **19.04.b — Verify:** Drive scripted dense, split-route, intersection and simultaneous-incident cases. Required behaviors work with native graphics, deterministic identities and no stale contacts or sirens.
  - [ ] **19.04.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.04.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [x] **19.05 — Close complete race and terminal outcome flows**

  - [x] **19.05.a — Implement:** Verify full-length route progress, unequal branches, finish overshoot/order, combat/recovery, timeout policy and every win/loss/arrest/wreck/quit return. Remove required debug-only/manual shortcuts from the normal player flow.
  - [x] **19.05.b — Verify:** Complete real command-driven races from countdown through results on the native setup matrix; do not substitute fabricated terminal requests for the end-to-end finish acceptance.
  - [x] **19.05.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.05.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [x] **19.06 — Complete offline frontend and source dialogue**

  - [x] **19.06.a — Implement:** Audit every source offline menu/control after the save/media work. Implement remaining required screen paths, complete credits, source dialogue/attitude/relationship selection and character/showroom data; make disabled or outside-scope network items explicit.
  - [x] **19.06.b — Verify:** Run every offline item and Back/cancel route from a clean profile. Report source-art layout differences separately, but leave no required item inert or replaced by placeholder advice.
  - [x] **19.06.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.06.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [x] **19.07 — Complete source media invocation coverage**

  - [x] **19.07.a — Implement:** Audit intro, career level/finale and all result video variants, artist/track submenus, ambient/race soundtrack choices and actor-specific effects. Resolve source MIDI/SBK or alternative media requirements explicitly; retain skip, mute, missing-device and native fallback behavior.
  - [x] **19.07.b — Verify:** Exercise each required invocation with the original local corpus. Verify owner/return screen, cue timing, audio completion, subtitle hooks and cleanup; unsupported required media stays open.
  - [x] **19.07.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.07.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [x] **19.08 — Finish save recovery and user-profile usability**

  - [x] **19.08.a — Implement:** Verify all career slots, new/resume/save-as, content compatibility, settings/bindings, backups, migrations and restart recovery through the packaged player flow. State original binary-save import scope without implying compatibility.
  - [x] **19.08.b — Verify:** Complete profile creation, several races, close/reopen, purchase/repair, corruption recovery and settings reset while preserving funds, unlocks and bindings exactly once.
  - [x] **19.08.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.08.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [ ] **19.09 — Complete a genuine native-asset Big Game career**

  - [x] **19.09.a — Implement:** Record a complete 25-race progression path using real driving/combat commands, source setups and original textures. Include inter-race save/resume, different bikes and failure/recovery branches.
  - [ ] **19.09.b — Verify:** Reach level-five finale and reopen the final career. Compare saved money, qualifications and purchases against the transaction ledger; accept no missing required content or synthetic finish shortcut.
  - [ ] **19.09.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.09.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [ ] **19.10 — Burn down required functionality defects**

  - [ ] **19.10.a — Implement:** Maintain a defect register with reproducer, subsystem owner, severity and regression scenario. Import all earlier declared missing functionality and distinguish required fixes from optional visual polish or source-layout differences.
  - [ ] **19.10.b — Verify:** No blocker or required functional gap remains unassigned, waived implicitly or hidden under a completed scaffold receipt. Closed defects have passing regression evidence.
  - [ ] **19.10.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.10.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [ ] **19.11 — Demonstrate the game without development tools**

  - [ ] **19.11.a — Implement:** Exercise installer/import, menus, a full race, career, media, recovery and exit using ordinary keyboard/controller navigation on the first supported platform, with no console, inspector, test-only command or enhanced pack required.
  - [ ] **19.11.b — Verify:** Have a reviewer follow the written clean-profile route and record failures. Resolve every required interruption before handing the candidate to UI and release verification.
  - [ ] **19.11.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.11.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

- [ ] **19.12 — Freeze the functional candidate for native release verification**

  - [ ] **19.12.a — Implement:** Freeze the native-only feature manifest, content identity, regression recipes and remaining nonblocking differences. Hand the candidate to Parts 20–22; keep G5 pending until the packaged native release is accepted.
  - [ ] **19.12.b — Verify:** Confirm every required feature has implementation and evidence, that Parts 20–22 have explicit acceptance owners, and that Part 24 production is still locked until G5 signoff.
  - [ ] **19.12.c — Attach completion evidence:** Link interfaces, exact native/authored fixtures or recorded input, expected/actual results, error paths and implementation revision in `docs/evidence/19.12.md`. Keep raw commercial assets private. Leave this task open while a required acceptance fails.

**Exit evidence:** a native-asset functional candidate for UI/localization, independent verification and packaging. Final native-only release signoff is G5 in Part 22. Texture enhancement production in Part 24 remains locked until then.



### Phase 20 — UI accessibility and localization

<details>
<summary>Completed — detailed checklist and evidence</summary>

**Prerequisites:** Functional offline candidate from 19, 16 frontend graph, 15 HUD data and 05 language inventory. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Stable text keys, focus graph and presentation settings.

**Required outputs:** Navigable localized UI and persisted accessibility options.

**Invariant:** Accessibility changes must be explicit in replay metadata if they change gameplay.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/hud.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/hud.cpp), [src/text.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/text.cpp), [src/native_strings.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/native_strings.cpp).

- [x] **20.01 — Implement HUD speed health damage and rank**

  - [x] **20.01.a — Implement:** Read speed, health, bike damage and rank from snapshots with defined formatting.
  - [x] **20.01.b — Verify:** Compare displayed values against simulation at checkpoints.
  - [x] **20.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.01.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.02 — Support scalable UI and safe areas**

  - [x] **20.02.a — Implement:** Anchor UI to safe areas with scale rules for window sizes and DPI.
  - [x] **20.02.b — Verify:** Test minimum window size, ultrawide and high-DPI layouts.
  - [x] **20.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.02.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.03 — Support keyboard navigation and controller focus**

  - [x] **20.03.a — Implement:** Define focus order, selected state and navigation actions for every menu.
  - [x] **20.03.b — Verify:** Complete selection and confirmation flows using keyboard and controller only.
  - [x] **20.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.03.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.04 — Expose readable modern text option**

  - [x] **20.04.a — Implement:** Provide readable text sizing/contrast while retaining optional classic artwork.
  - [x] **20.04.b — Verify:** Test long labels and minimum supported UI scale.
  - [x] **20.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.04.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.05 — Audit color-dependent information**

  - [x] **20.05.a — Implement:** Identify health/rank/status cues conveyed only by color.
  - [x] **20.05.b — Verify:** Add shape, icon or text equivalents and verify grayscale readability.
  - [x] **20.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.05.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.06 — Provide reduced flashes and camera motion**

  - [x] **20.06.a — Implement:** Expose reduction settings for flashing, shake and motion where relevant.
  - [x] **20.06.b — Verify:** Verify disabled effects do not alter simulation or hide required state.
  - [x] **20.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.06.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.07 — Implement subtitle policy for dialogue**

  - [x] **20.07.a — Implement:** Catalog spoken content and decide subtitle coverage and timing.
  - [x] **20.07.b — Verify:** Test subtitles at the longest line and during skipped media.
  - [x] **20.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.07.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.08 — Inventory five supplied language resources**

  - [x] **20.08.a — Implement:** Inventory strings and assets for each supplied language independently.
  - [x] **20.08.b — Verify:** Record gaps rather than assuming all screens are translated.
  - [x] **20.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.08.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.09 — Separate text keys from layouts**

  - [x] **20.09.a — Implement:** Use stable localization keys with parameterized values.
  - [x] **20.09.b — Verify:** Test plural/number formatting and avoid string-concatenation layout assumptions.
  - [x] **20.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.09.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.10 — Test long translated strings and glyph coverage**

  - [x] **20.10.a — Implement:** Render long translated labels with actual selected fonts.
  - [x] **20.10.b — Verify:** Verify missing glyph handling and wrapping without clipped controls.
  - [x] **20.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.10.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.11 — Persist accessibility settings**

  - [x] **20.11.a — Implement:** Persist accessibility preferences independently from career data.
  - [x] **20.11.b — Verify:** Restart the game and verify every option remains active.
  - [x] **20.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.11.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **20.12 — Run complete flow without mouse**

  - [x] **20.12.a — Implement:** Navigate import, menus, race, pause and results without mouse input.
  - [x] **20.12.b — Verify:** Record any inaccessible control as a release-blocking UI issue for the chosen target.
  - [x] **20.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/20.12.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of ui accessibility and localization, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

</details>

### Phase 21 — Verification and performance

**Prerequisites:** Native-only functional candidate from 19, UI/localization from 20, the 06 replay harness and each system under comparison. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Versioned engine fixtures and built game on declared hardware.

**Required outputs:** Coverage matrix, trace/image reports and performance receipts.

**Invariant:** Distinguish exact state equality, measured tolerances and subjective visual review.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [docs/VERIFICATION.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/VERIFICATION.md), [docs/PERFORMANCE.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/PERFORMANCE.md), [tests/road_test.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/tests/road_test.cpp).

- [x] **21.01 — Build internal regression capture protocol**

  - [x] **21.01.a — Implement:** Specify engine build/content hashes, capture environment, replay commands and timing method.
  - [x] **21.01.b — Verify:** Repeat an engine baseline capture and quantify measurement variance.
  - [x] **21.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.01.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **21.02 — Record engine/content build, course, bike, seed and inputs**

  - [x] **21.02.a — Implement:** Store course, level, bike, rider, seed and initial state with each run.
  - [x] **21.02.b — Verify:** Reject comparisons whose initial conditions do not match.
  - [x] **21.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.02.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **21.03 — Compare first divergent simulation tick**

  - [x] **21.03.a — Implement:** Compare canonical state at successive ticks and report first differing fields.
  - [x] **21.03.b — Verify:** Classify arithmetic, scheduling, RNG and content mismatches separately.
  - [x] **21.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.03.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **21.04 — Compare fixed-camera image regions**

  - [x] **21.04.a — Implement:** Capture identical viewport/camera/tick regions using only original assets and fixed native presentation settings.
  - [x] **21.04.b — Verify:** Justify every mask and avoid masking actual fidelity defects.
  - [x] **21.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.04.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **21.05 — Set explicit numerical tolerances**

  - [x] **21.05.a — Implement:** Choose exact equality for integer state and justified tolerances for measurements.
  - [x] **21.05.b — Verify:** Record units and reasons for every nonzero tolerance.
  - [x] **21.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.05.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **21.06 — Run all courses and levels in coverage matrix**

  - [ ] **21.06.a — Implement:** Track every course-level combination with scenario status and evidence path.
  - [ ] **21.06.b — Verify:** Distinguish untested, skipped, failed and passed cases.
  - [ ] **21.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.06.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **21.07 — Measure CPU and GPU frame times separately**

  - [ ] **21.07.a — Implement:** Measure simulation, extraction, rendering and GPU work with representative scenes.
  - [ ] **21.07.b — Verify:** Report percentiles and hardware rather than a single average FPS.
  - [ ] **21.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.07.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **21.08 — Measure memory and texture residency**

  - [ ] **21.08.a — Implement:** Track CPU heap, GPU textures, atlas size and streaming peaks.
  - [ ] **21.08.b — Verify:** Run the native asset set on every course, identify the largest contributors and freeze the native residency budget. Enhanced-pack comparisons belong to Part 24 after G5.
  - [ ] **21.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.08.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **21.09 — Test repeated native import race and unload cycles**

  - [ ] **21.09.a — Implement:** Loop import/start/race/exit and inspect retained allocations and resources.
  - [ ] **21.09.b — Verify:** Confirm counts stabilize instead of increasing after every cycle.
  - [ ] **21.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.09.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **21.10 — Test loading stalls and shader warmup**

  - [ ] **21.10.a — Implement:** Profile loading, first-use pipelines and texture upload stalls.
  - [ ] **21.10.b — Verify:** Repeat on cold and warm caches and set measured budgets.
  - [ ] **21.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.10.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **21.11 — Run soak tests on declared hardware**

  - [ ] **21.11.a — Implement:** Run long races on declared physical hardware with detailed logs.
  - [ ] **21.11.b — Verify:** Document thermal/power settings and any driver-specific failures.
  - [ ] **21.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.11.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **21.12 — Publish results with skips and limitations**

  - [ ] **21.12.a — Implement:** Publish commands, build revision, environment and failures alongside successes.
  - [ ] **21.12.b — Verify:** Avoid claiming coverage for checks that were not actually executed.
  - [ ] **21.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/21.12.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of verification and performance, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.



### Phase 22 — Packaging and native-asset release

**Prerequisites:** Functional completion in 19, UI/localization in 20, verification in 21 and G3. Enhanced art and mod production are not prerequisites. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Tagged source, locked dependencies, the native-only acceptance manifest and an approved packaging manifest.

**Required outputs:** Native-asset distributable, checksums, notices and signed G5 functional release acceptance; all required game functions must work with no enhancement pack. Technical RC4 outputs pass; the outstanding project-owner decision is explicit in [the G5 record](docs/evidence/release-signoff.json).

**Invariant:** Test shipped artifacts from a fresh profile without development caches.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [docs/USER_GUIDE.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/USER_GUIDE.md), [docs/CURRENT_STATUS.md](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/docs/CURRENT_STATUS.md).

- [x] **22.01 — Package clean desktop application**

  - [x] **22.01.a — Implement:** Build application packages from a tagged source and locked dependencies.
  - [x] **22.01.b — Verify:** Launch the packaged binary rather than only a development build.
  - [x] **22.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.01.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **22.02 — Run first-launch import on clean machine**

  - [x] **22.02.a — Implement:** Test import and initial play under a fresh user account.
  - [x] **22.02.b — Verify:** Verify missing-disc instructions and successful import without development paths.
  - [x] **22.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.02.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **22.03 — Verify package contains no private game files**

  - [x] **22.03.a — Implement:** Scan package manifests for original files and derived private artwork.
  - [x] **22.03.b — Verify:** Verify only permitted authored fixtures and application resources ship.
  - [x] **22.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.03.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **22.04 — Include applicable third-party notices**

  - [x] **22.04.a — Implement:** Generate dependency notices from actual packaged dependencies.
  - [x] **22.04.b — Verify:** Check required texts and distinguish source licensing from game data.
  - [x] **22.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.04.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **22.05 — Verify save and settings paths**

  - [x] **22.05.a — Implement:** Use platform-appropriate user directories and document overrides.
  - [x] **22.05.b — Verify:** Test unwritable install directory and multiple user accounts.
  - [x] **22.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.05.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **22.06 — Document minimum tested hardware**

  - [x] **22.06.a — Implement:** Publish the lowest hardware configuration actually tested.
  - [x] **22.06.b — Verify:** Record resolution, settings and frame-time evidence for that configuration.
  - [x] **22.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.06.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **22.07 — Test upgrade and uninstall behavior**

  - [x] **22.07.a — Implement:** Test install over old version, save migration and uninstall retention policy.
  - [x] **22.07.b — Verify:** Verify user saves and separately imported data follow documented behavior.
  - [x] **22.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.07.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [x] **22.08 — Provide troubleshooting and log collection**

  - [x] **22.08.a — Implement:** Document common graphics/audio/import errors and log locations.
  - [x] **22.08.b — Verify:** Reproduce each troubleshooting step without exposing private data unnecessarily.
  - [x] **22.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.08.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **22.09 — Create release notes and known issues**

  - [ ] **22.09.a — Implement:** Summarize supported features, known deviations and open hardware gaps.
  - [ ] **22.09.b — Verify:** Link each material limitation to a tracked issue or evidence record.
  - [ ] **22.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.09.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **22.10 — Tag reproducible source revision**

  - [ ] **22.10.a — Implement:** Tag the release commit and record build toolchain/dependency identities.
  - [ ] **22.10.b — Verify:** Rebuild from the tag and compare intended reproducibility outputs.
  - [ ] **22.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.10.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **22.11 — Attach checksums to release artifacts**

  - [ ] **22.11.a — Implement:** Hash final downloadable artifacts after packaging/signing steps.
  - [ ] **22.11.b — Verify:** Verify downloaded bytes against published checksums.
  - [ ] **22.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.11.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **22.12 — Complete native-only full career and functional release acceptance**

  - [ ] **22.12.a — Implement:** Complete the campaign and release matrix using packaged builds.
  - [ ] **22.12.b — Verify:** Leave the release gate open for any unresolved blocking scenario.
  - [ ] **22.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/22.12.md`. For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of packaging and release, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 23 — Mod support and content contracts

**Prerequisites:** G5 native-only functional release in 22; stable 05/06/07/09/17 content, replay and save contracts. Data-only examples may use independently authored placeholder art; production enhancements remain gated in 24. These are phase-entry dependencies, not a requirement to finish every numbered phase strictly in order.

**Inputs:** Base catalog plus explicitly enabled content packs.

**Required outputs:** Mod manifest/resolver, validation reports and authored example pack. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Resolve the complete content graph before activating gameplay-affecting changes.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/assets.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/assets.hpp), [src/upscale_cache.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/upscale_cache.hpp).

- [ ] **23.01 — Define versioned mod manifest**

  - [ ] **23.01.a — Implement:** Define manifest fields for ID, version, dependencies, engine range and content entries.
  - [ ] **23.01.b — Verify:** Validate unknown required fields and malformed versions predictably.
  - [ ] **23.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.02 — Define stable content IDs**

  - [ ] **23.02.a — Implement:** Use namespaced IDs with stable semantics for base and mod content.
  - [ ] **23.02.b — Verify:** Rename a file without breaking saves or references to the same ID.
  - [ ] **23.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.03 — Implement deterministic load priority**

  - [ ] **23.03.a — Implement:** Define base-to-mod precedence and dependency ordering.
  - [ ] **23.03.b — Verify:** Load the same mod set in different filesystem enumeration orders with equal results.
  - [ ] **23.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.04 — Report conflicts with winning source**

  - [ ] **23.04.a — Implement:** Produce a conflict report listing every candidate and selected winner.
  - [ ] **23.04.b — Verify:** Test two mods replacing the same texture and expose the override chain.
  - [ ] **23.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.05 — Validate dependency and engine version requirements**

  - [ ] **23.05.a — Implement:** Resolve dependencies before asset loading with clear incompatibility diagnostics.
  - [ ] **23.05.b — Verify:** Reject missing dependencies and dependency cycles.
  - [ ] **23.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.06 — Support texture replacement first**

  - [ ] **23.06.a — Implement:** Implement texture overrides through the catalog rather than hardcoded file probing.
  - [ ] **23.06.b — Verify:** Replace one texture and preserve its original pivot and gameplay bounds.
  - [ ] **23.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.07 — Support bike and handling data overrides**

  - [ ] **23.07.a — Implement:** Expose constrained bike data overrides through versioned schemas.
  - [ ] **23.07.b — Verify:** Test invalid mass/speed values and record modified simulation configuration.
  - [ ] **23.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.08 — Support courses after schema stabilizes**

  - [ ] **23.08.a — Implement:** Define course graph schema only after core traversal semantics stabilize.
  - [ ] **23.08.b — Verify:** Load an authored tiny course and reject dangling graph links.
  - [ ] **23.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.09 — Validate all numeric ranges and references**

  - [ ] **23.09.a — Implement:** Validate dimensions, numbers, IDs and referenced files before activation.
  - [ ] **23.09.b — Verify:** Report all actionable errors without partially applying the pack.
  - [ ] **23.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.10 — Provide authored example mod**

  - [ ] **23.10.a — Implement:** Create an original placeholder texture/bike mod with setup documentation.
  - [ ] **23.10.b — Verify:** Install it on a clean clone and confirm no commercial content is required.
  - [ ] **23.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.11 — Include active mod hashes in replay metadata**

  - [ ] **23.11.a — Implement:** Include resolved load order and content hashes in replay manifests.
  - [ ] **23.11.b — Verify:** Reject or clearly label a replay with mismatched simulation-affecting mods.
  - [ ] **23.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **23.12 — Document unsupported script and native-code loading**

  - [ ] **23.12.a — Implement:** State that initial mod support is data/assets only unless separately approved.
  - [ ] **23.12.b — Verify:** Verify content loading cannot execute arbitrary scripts or native libraries.
  - [ ] **23.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/23.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of mod support and content contracts, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 24 — Texture-by-texture enhancement

**Prerequisites:** G5 native-asset release is accepted after 19 functional completion, 20 UI/localization, 21 verification and 22 packaging; 23 mod contracts are stable. **Hard gate:** no AI upscaling, candidate generation, texture enhancement production or enhanced-art release before that native-only acceptance. This production gate is mandatory; enhanced-art work must wait for accepted G5.

**Inputs:** Source assets plus per-candidate generation metadata.

**Required outputs:** Immutable masters, review ledger, approved pack and fallback demonstration. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Approve by semantic asset/family; never silently overwrite native masters.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/upscale_import.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/upscale_import.cpp), [src/upscale_cache.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/upscale_cache.cpp), [src/mmpx.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/mmpx.cpp).

- [ ] **24.01 — Export immutable native-resolution masters**

  - [ ] **24.01.a — Implement:** Export native masters with original palette, alpha and provenance intact.
  - [ ] **24.01.b — Verify:** Verify their hashes never change when candidates are regenerated.
  - [ ] **24.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.02 — Prioritize high-visibility assets**

  - [ ] **24.02.a — Implement:** Rank candidates by screen coverage, frequency, gameplay importance and family dependencies.
  - [ ] **24.02.b — Verify:** Complete a small pilot before committing to full-corpus scaling.
  - [ ] **24.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.03 — Create one review item per texture or animation family**

  - [ ] **24.03.a — Implement:** Create review records keyed by asset ID and linked animation family.
  - [ ] **24.03.b — Verify:** Ensure each candidate has an explicit owner/status and no ambiguous filename-only identity.
  - [ ] **24.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.04 — Record AI model version and generation settings**

  - [ ] **24.04.a — Implement:** Record model/service revision, scale, denoise and relevant generation options.
  - [ ] **24.04.b — Verify:** Reopen a candidate record and identify exactly how it was produced.
  - [ ] **24.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.05 — Record prompt seed and input hash**

  - [ ] **24.05.a — Implement:** Store prompt, seed when available, source hash and output hash.
  - [ ] **24.05.b — Verify:** Mark unavailable generation metadata explicitly instead of inventing reproducibility.
  - [ ] **24.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.06 — Preserve alpha edges and pivot metadata**

  - [ ] **24.06.a — Implement:** Retain logical dimensions, pivot and alpha boundaries outside generated pixels.
  - [ ] **24.06.b — Verify:** Compare ground contact and silhouette before/after replacement.
  - [ ] **24.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.07 — Preserve palette-remap masks separately**

  - [ ] **24.07.a — Implement:** Keep semantic recoloring masks independent of enhanced RGB detail.
  - [ ] **24.07.b — Verify:** Preview every rider remap and reject color leakage across clothing regions.
  - [ ] **24.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.08 — Reject invented text and changed silhouettes**

  - [ ] **24.08.a — Implement:** Compare signs, logos, faces and recognizable silhouettes at native and target size.
  - [ ] **24.08.b — Verify:** Reject hallucinated letters, altered pose or changed bike geometry.
  - [ ] **24.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.09 — Review tiled textures for seams**

  - [ ] **24.09.a — Implement:** Preview repeated road/terrain tiles across several repeats and mip levels.
  - [ ] **24.09.b — Verify:** Reject edge seams, directional discontinuities and excessive high-frequency noise.
  - [ ] **24.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.10 — Review animation sequences for flicker**

  - [ ] **24.10.a — Implement:** Play complete animation cycles with native/enhanced toggle at gameplay speed.
  - [ ] **24.10.b — Verify:** Reject frame-to-frame changing details and pivot/outline shimmer.
  - [ ] **24.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.11 — Approve or reject each candidate explicitly**

  - [ ] **24.11.a — Implement:** Require a recorded decision, rationale and approved output hash per asset.
  - [ ] **24.11.b — Verify:** Ensure rejected/superseded candidates cannot be packaged accidentally.
  - [ ] **24.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **24.12 — Package approved overrides with native fallback**

  - [ ] **24.12.a — Implement:** Build override packs from approved records and validate source identity.
  - [ ] **24.12.b — Verify:** Remove an override and confirm native rendering continues without missing assets.
  - [ ] **24.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/24.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of texture-by-texture enhancement, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

### Phase 25 — Future 3D preparation only

**Prerequisites:** 05 asset semantics, 06 domain boundary and 07 topology. This entire future 3D phase remains deferred and outside the current implementation scope.

**Inputs:** Recovered course data, source views and uncertainty annotations.

**Required outputs:** Convention notes and optional schema extension policy only. Names in this roadmap are proposed artifacts until implemented.

**Invariant:** Full 3D production is deferred and cannot become a dependency of this release.

**Execution rule:** Resolve each task's exact constants and file layout from the pinned references below and the evidence ledger. If original behavior is unmeasured, create a research result before declaring implementation parity. The `.a` item creates the deliverable, `.b` specifies its task-specific acceptance, and `.c` records reproducible proof.


Research: [src/course.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course.hpp), [src/actor_descriptors.hpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/actor_descriptors.hpp).

- [ ] **25.01 — Document coordinate axes and handedness**

  - [ ] **25.01.a — Implement:** Define domain forward/right/up directions and adapter handedness explicitly.
  - [ ] **25.01.b — Verify:** Test a canonical straight road and left turn in the classic adapter.
  - [ ] **25.01.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.01.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.02 — Preserve route elevation and branch topology**

  - [ ] **25.02.a — Implement:** Retain sampled heights, node links and semantic branches in source-independent data.
  - [ ] **25.02.b — Verify:** Export/import a course fixture without flattening topology.
  - [ ] **25.02.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.02.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.03 — Keep visual representation separate from collision semantics**

  - [ ] **25.03.a — Implement:** Define collision and surface categories independently from visual meshes/sprites.
  - [ ] **25.03.b — Verify:** Replace a renderer placeholder and verify unchanged contact results.
  - [ ] **25.03.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.03.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.04 — Store physical dimensions separately from pixel size**

  - [ ] **25.04.a — Implement:** Store logical world dimensions separately from pixel rectangles.
  - [ ] **25.04.b — Verify:** Resize an enhanced sprite without changing physical reach or obstacle width.
  - [ ] **25.04.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.04.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.05 — Record sprite view direction and camera assumptions**

  - [ ] **25.05.a — Implement:** Record view angle, camera assumption and uncertainty for each source view.
  - [ ] **25.05.b — Verify:** Mark inferred angles rather than presenting them as measured geometry.
  - [ ] **25.05.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.05.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.06 — Preserve neutral texture masters and masks**

  - [ ] **25.06.a — Implement:** Keep source pixels, alpha, remap masks and approved derivatives separate.
  - [ ] **25.06.b — Verify:** Show a future material tool can access masters without undoing atlas packing.
  - [ ] **25.06.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.06.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.07 — Record material meaning and baked lighting**

  - [ ] **25.07.a — Implement:** Label baked shading, specular highlights and uncertain material regions.
  - [ ] **25.07.b — Verify:** Prevent a sprite crop from being silently classified as clean PBR albedo.
  - [ ] **25.07.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.07.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.08 — Reserve optional mesh and rig fields in content schema**

  - [ ] **25.08.a — Implement:** Allow optional future mesh/rig references without requiring them at runtime.
  - [ ] **25.08.b — Verify:** Load the same asset schema with those fields absent.
  - [ ] **25.08.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.08.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.09 — Specify glTF import conventions when needed**

  - [ ] **25.09.a — Implement:** Write a future convention note for glTF units, axes and texture color spaces.
  - [ ] **25.09.b — Verify:** Validate the convention with an authored cube only if the later project begins.
  - [ ] **25.09.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.09.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.10 — Define attachment semantics for grips feet and weapons**

  - [ ] **25.10.a — Implement:** Define semantic grip, foot, weapon and seat attachment names.
  - [ ] **25.10.b — Verify:** Map names to current sprite semantics without building a full rig now.
  - [ ] **25.10.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.10.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.11 — Document hidden surfaces as unknown reconstructions**

  - [ ] **25.11.a — Implement:** Record hidden surfaces and depth as unknown or authored interpretations.
  - [ ] **25.11.b — Verify:** Keep inferred 3D material separate from recovered source evidence.
  - [ ] **25.11.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.11.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.

- [ ] **25.12 — Defer mesh production 3D physics and full 3D renderer to separate scope**

  - [ ] **25.12.a — Implement:** List mesh production, rigging, 3D camera and physics as deferred deliverables.
  - [ ] **25.12.b — Verify:** Ensure no current release gate depends on those deferred systems.
  - [ ] **25.12.c — Attach completion evidence:** Link the changed module or document, exact fixture/capture used, expected versus actual result, and implementation revision in `docs/evidence/25.12.md` (proposed path). For manual review, include the review procedure and outcome; for automated checks, include the command and result. Record remaining deviations and do not close this parent item while its acceptance check fails.


**Exit evidence:** a reproducible demonstration of future 3d preparation only, linked implementation revision, recorded checks and known limitations. Dependent phases may prototype early, but this gate stays open until its checks pass.

## 7. One-texture-at-a-time production procedure — locked until G5

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

A replay manifest should include game build, simulation schema, source corpus hash, content pack hashes, settings, course/level/bike/character, initial seed, tick-indexed commands and expected state hashes. Compare first divergence rather than only final finishing order. Capture screenshot differences at fixed simulation ticks with controlled camera and viewport. Keep exact numerical tests separate from visual tolerances. Baselines refer to this engine; running the original program is optional research.

Suggested feature issue template:

```text
Feature / roadmap ID:
Intended behavior and source evidence:
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

## 10. Next work: complete the native-only game

- [ ] Close the current required-defect register and renewed career acceptance in Part 19; retain successful action, actor, dialogue and media checks as scoped evidence.
- [ ] Revalidate the complete career after the building collision and AI changes. The 67-attempt RC4 career remains historical evidence.
- [ ] Revalidate the new presentation and simulation against the native matrix, performance and soak requirements in Part 21.
- [ ] Build and verify a replacement for RC4 before Part 22/G5 acceptance. Building rendering/collision, AI and UI fixes are documented in [the correction report](docs/evidence/native-gameplay-fixes.md).
- [ ] Stabilize optional mod contracts in Part 23, then start Part 24 enhancement production only after recorded G5 acceptance.

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
