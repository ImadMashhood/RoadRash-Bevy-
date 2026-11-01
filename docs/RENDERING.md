# Classic renderer contract

The `rr-render` crate draws the classic course view entirely in safe Rust. Bevy owns the application lifecycle, input, fixed scheduling, window, GPU upload and presentation. `rr-presentation::CoursePresentation` displays the resulting RGBA image through a nearest-sampled Bevy sprite. The same drawing kernel produces headless checkpoints; there is no separate test-only road implementation. 3D remains deferred.

The renderer consumes immutable `rr-course` graphs/routes and read-only simulation snapshots. Source/catalog loading, family resolution, terrain generation and static placements happen at startup. Drawing performs no filesystem access and consumes no gameplay RNG. Previous/current snapshots are copied and interpolated into presentation values; they never feed back into simulation commands, contacts, progression or canonical hashing.

## Camera and source units

| Property | Active contract |
| --- | --- |
| Classic logical image | 640×480, corresponding to source 320×240 view coordinates at 2× |
| Wide logical image | 852×480, Hor+: same vertical scale, additional horizontal scenery |
| Horizon | Source row 90, output row 180 |
| Camera height | 150 source world units above current road; fixed height policy |
| Near plane | Positive source depth 1 Q8 unit; geometry crossing it is clipped |
| Lookahead | 33 sampled slices; depth increases monotonically |
| Projection | 4,352 source Q16 scale words at EXE VA 0x0046f098, indexed by depth/2 |
| Supported binary | Existing SHA-256-gated SupportedExe contract; arbitrary EXEs cannot supply addresses |
| Player image | Catalog rider frame 2 by default; `--asset` may select another typed rider ID |
| Player visual depth | Q8 depth 512, derived from the source one-sample camera trail and first-boundary convention |
| Sprite size/contact | Source RRAN logical dimensions/hotspot and actor size 512, independent of HI/LO/enhanced bitmap dimensions |
| Pixel sampling | Nearest; original indexed mip levels selected for minified source textures |
| Viewport | Aspect-preserving fit, centered letterbox/pillarbox; optional integer scale when it fits |
| HUD | Authored speed digits anchored 16 pixels inside the centered classic-safe 640-pixel region |

The source depth table has a nonlinear near region: it is **not** replaced by a generic focal-length/depth formula. A regression found that generic interpolation could push the foreground rider off-screen. `contact_anchor` now binds the sprite foot to the same projected strip and inverse-depth convention as the road. Source scale clamps and perspective contact have separate acceptance vectors.

Imported live races now use schema-3 source Q8 positions, lateral displacement and above-ground height directly. Legacy schemas retain the original authored bridge (eight scenario millimetres per source Q8 unit; ten lateral millimetres per world unit). Default start is source sample 50; `--camera-sample` selects another validated start. The new coordinate model and grade/bend laws are documented in [HANDLING.md](HANDLING.md). No physical-metre conversion is claimed. Presentation interpolation consumes copies and never changes authoritative state.

Camera lateral position follows interpolated player lateral position. Course bend accumulates source heading for horizon scrolling; `View.heading` is an additional presentation offset. Infinity-distance horizon imagery does not shift under pure lateral translation. Source-course selection and branch-mask selection are launch configuration in this phase; a complete gameplay branch/input controller is later handling/race work.

## Projection, surfaces and clipping

Projection consumes the selected branch-aware route cache, preserving node, section, local index, road half and source lane/shoulder values. It integrates signed bend and rise separately and exposes both roadway halves in split/merge windows. The other half is initialized from the validated separation/derivative state rather than compared as an unrelated global Z. At a transition, shared incoming geometry connects to the exposed outgoing halves. No camera calculation mutates source data.

Road and terrain panels become textured source-space polygons. The near-plane clipper interpolates geometry/UVs before projection. The rasterizer bounds screen rectangles to the actual framebuffer, rejects nonfinite/oversized vertices and degenerate triangles, and uses perspective-correct UV/depth interpolation with a depth buffer. Behind-camera sprites are discarded. Alpha-zero texels neither paint nor write depth. Frames are bounded to 1920×1080 storage; camera and decoded image bounds are validated separately.

The eight road-material slots and three marked-ground bindings are recovered from the supported EXE descriptor table at 0x0046dfb0 and GLOBAL.RSC CEL resources. The source alias for the middle marked lane is preserved. Lane position, Q8 lane counts and event marking flags select normal, marked or alternate materials. Exact source event endpoints are tested through the course decoder-to-material pipeline, including the initial sample and inclusive end behavior. Source mip levels are converted with the original global palette and chosen for minification. The modern rasterizer is not a bit-for-bit reimplementation of every original CCB/affine drawing effect.

The high-resolution horizon is read through the source course-to-horizon pointer table at 0x00467aa0 (City, Sierra, Penin, Napa, PCH), not guessed from CRS filenames. The 720×320 indexed image is palette-converted, centered and wrapped with signed-safe modular addressing. Wide mode exposes more columns at the same scale; extreme heading values stay in bounds. Course loads create fresh drawing state, so a previous course's background cannot survive into a new frame.

## Terrain and scenery

Building, hill and wall/cliff categories use the decoded section semantics. Shoulder width/height and selectors remain independent of road widths. Building height comes from the resolved source group; facades and simple roof planes use the source family texture. Hills walk bounded signed source point profiles; walls walk their source parameter records and signed bases. Low wall heights expose cliff geometry down to source elevation zero, with the source water material at the foot. Terrain categories have isolated authored visual fixtures before combined source-course captures.

Procedural terrain uses a private seed-1 MSVC-style stream during scene construction. Scenery uses the existing isolated placement stream from Part 07. Neither stream is consumed during rendering, and neither participates in gameplay random interleaving. This gives a reproducible presentation baseline, not a claim that original race-wide random-call interleaving has been recovered.

Static scenery resolves source family selectors to RRAN view pixels and source hotspots. Three-view pixel aliases are handled without replacing the selected view's hotspot; stale/uninterpreted drawing leaves remain explicit notes. Scenery instances retain their source IDs, logical size, mirror flag and fixed source-local placements. Terrain ground height and projected-strip contact determine their anchors.

Required road, horizon, rider and supported binary failures abort startup. Optional unresolved scenery/terrain/shoulder references are reported before drawing and in capture metadata; terrain/shoulder material gaps use declared native-ground fallbacks, while unresolved static sprite pixels are omitted. The complete per-course notes are preserved in [checkpoint metadata](evidence/render-checkpoints.json). An explicit fallback is not a claim that an unknown leaf was decoded. Group resolution currently chooses the highest stored cell with native mip selection; exact legacy group thresholds, shading/blend effects, rails and elaborate tunnel/roof details remain fidelity refinements. Source flags and original data remain available for that work.

Pedestrian/police spawn requests remain gameplay records, not invented static scenery. The player now uses imported tick-driven [rider animations and ground shadows](ANIMATION.md). Combat/AI, detached recovery physics and traffic behavior remain their later roadmap phases. The renderer already supports arbitrary visual billboards and tests racers/scenery/traffic at equal depths; this does not claim a completed traffic or opponent system.

## Stable sprite order and interpolation

Billboards sort far-to-near, then by class (shadow, scenery, traffic, racer), then stable ID ascending. Later equal-depth entries paint on top. The depth buffer preserves occlusion by road/terrain, and transparent texels leave earlier content intact. This is a documented deterministic ordering policy, not an unmeasured original draw-list claim.

Actor positions are interpolated only within the same route/branch/section domain; domain changes retain the new pose until a race-aware transition adapter exists. Other actors compare local progress only within the player's domain; cross-section/branch visibility is deferred rather than flattening unrelated coordinates. Ground contact follows the actually rendered shoulder ramp as well as the road.

Rider dimensions/hotspots come from GLOBAL.RSC ANIM 1 RRAN frame records rather than catalog BOB storage dimensions. Frame 2 is 29×64 with hotspot (19,52); its source actor size factor is 2. The general billboard pivot uses world-up coordinates, and the image portion below a hotspot uses ground-anchor occlusion so road depth cannot amputate it. Nearer hills still occlude it. Other art uses its decoded source logical metadata. A larger native/enhanced PNG therefore cannot enlarge collision bounds, move the pivot or change gameplay. `--enhanced-image` is a renderer override; missing enhanced art falls back to native. The test override is authored checker art, not an AI-enhanced or approved pack.

Bevy input, simulation and snapshot publication keep their existing explicit ordering. Presentation interpolates owned previous/current visual positions using accumulator alpha, prepares the classic frame, uploads it, and requests screenshots after drawing. A headless Bevy adapter test exercises actual image updates while asserting unchanged authoritative snapshots and simulation hashes. Render-frequency, aspect ratio and texture selection are independently tested against the same command replay.

## Run the source course view

Run from the repository root after importing/converting your own data:

```sh
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course CITY
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course NAPA --level 3 --branch-mask 5 --framing wide --window-size 1280x720
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course CITY --integer-scale on --window-size 1280x960
```

Supported source names: CITY, HIWAY, MEDLY, NAPA, CANYN. Levels are 1–5; branch bits are the Part 07 sorted split-node order. With both install and catalog supplied, course defaults to CITY. Without the full source configuration, the previous authored fallback remains available; explicit `--course` requires both inputs. Up/W accelerates, Down/S brakes, Left/Right or A/D steers using source handling in imported races.

Other flags: `--camera-sample NUMBER`, `--asset rr:...`, `--texture-mode native|enhanced`, `--enhanced-image PNG`, `--switch-textures-at TICK`, `--replay JSON`, `--render-hz 30|60|144|irregular`, `--report JSON`, `--capture PNG`, `--smoke-frames N`. Boolean integer scaling is `--integer-scale on|off`. Camera/level/branch/window arguments are bounded and invalid source references fail before launch.

## Visual checkpoints and image diffs

```sh
cargo run --locked --release -p rr-render -- authored local/render-authored
cargo run --locked --release -p rr-render -- frame local/game-install local/catalog CITY 50 local/city.png 0 1 classic
cargo run --locked --release -p rr-render -- suite local/game-install local/catalog local/render-checkpoints
cargo run --locked --release -p rr-render -- diff local/expected.png local/actual.png local/diff.png
```

The seven authored scenes (straight, bend, crest, valley, hill, wall/cliff, building) have visually reviewed [RGBA golden hashes](fixtures/render-goldens.json) in public CI. No commercial pixel data is needed. To intentionally change a baseline, regenerate the scenes, inspect every image, review the changed geometry/policy and update the hashes in the same change; never regenerate expected values merely to silence a failure.

The private suite renders **620 fixed-camera checkpoints** covering every course and all 26 branches, before/at/after all 132 split/merge transitions, 130 level-specific finish views with actual level placement settings, starts and terrain-category views. These are frozen inspection scenarios at declared tick 0 and source positions, not complete-race outcome tests. Each record identifies camera, resolution, level, branch, source position, statistics and hashes. A repeated frame is compared over the entire RGBA image and a real difference image is exported. There are no masked nondeterministic regions. RGB differences use absolute channel values; alpha-only changes are marked white. A mismatch returns nonzero after retaining the diff.

Two independent full exports match in PNG bytes and metadata. The Bevy matrix additionally runs four virtual presentation schedules, wide/integer/portrait/small-window cases, native/override/live-switch cases and every source course. GPU screenshots record requested frame, simulation tick, interpolation alpha, camera, start offset and window dimensions in `.capture.json` sidecars. They are current-engine captures, not screenshots of the original executable. Virtual 30/60/144/irregular schedules are not hardware FPS benchmarks.

All original-derived PNGs, decoded data and comparison images stay in ignored local directories. Only code, authored fixture definitions, hashes and metadata receipts are published. Source/output overlap is rejected before writing under an immutable install. See [Part 08 acceptance](evidence/part-08.md).

Source rider frames retain their own pivots, mirroring and size flags, body roll and independent shadow registration. Empty source frame slots are preserved. Generic single-image overrides continue to serve legacy preview snapshots; complete animated enhancement packs remain Phase 24.
