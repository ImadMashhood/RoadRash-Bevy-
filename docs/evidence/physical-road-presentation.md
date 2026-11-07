# Physical road presentation bridge

Status: public regressions and 1,980 native mounted camera observations pass.
Ten ordinary-command source fork approaches pass at four schedules.

The native source uses absolute Q8 lateral positions for simulation and subtracts
the actor's sampled road offset before projection (`game_types.cpp`,
`traffic.cpp`, `pedestrians.cpp`). `road.cpp` restores the sampled road offset
when positioning its road strip. Passing the absolute actor lateral value to
that strip applies the offset twice. The prior Rust bridge did this for other
riders and road users. It also passed absolute player lateral as a camera
coordinate where the road projector expects a road-relative value.

The authored uniform-translation regression moves every road center and source
rider by 500 world units. Before correction this changed 18,753 image bytes;
after correction the image is identical. A separate colored traffic fixture
checks the same invariant and checks that the vehicle is actually visible.
Original failure: `local/collision-offset-before.log`.

Physical `(node, local progress)` coordinates now determine visible positions in
the selected route and its dual windows. Objects outside that cache have no
visible position. This also allows an off-route detached body to become visible
when its physical road enters a dual window, independently of an old projected
coordinate. The presentation adapter follows the player's authoritative source
branch mask and retains the selected scene across shared sections. Physical-node
inference remains available for older snapshots. A public fork fixture has
200- and 220-sample branches; its shared tail maps 20 samples apart, proving that
physical coordinates determine the displayed position after the merge.

The physical camera bridge trails the mapped player by 256 Q8 units, as
`Race::camera_position()` does. A projected road slice begins at depth 256, and
the player depth is 512; previously grounding that player from an unshifted
camera sampled one road interval ahead. The old standalone `snapshot_view`
helper remains available for historical authored adapters.

These changes affect presentation, not simulation authority. Source whole-race
camera timing, contact-push lag, lateral follow during recovery, arrest pull-back and
detached-body camera motion require separate trajectory evidence; these public
coordinate checks do not claim those cases are verified.

The controlled `camera_pose.cpp` producer invokes the actual pinned
`Race::update_camera` and `Race::camera_position`. Its 330 mounted, no-contact
cases cover three road offsets, symmetric/asymmetric lane widths, lateral
follow/cap thresholds and positive/negative sub-unit Q8 fractions. The valid
before run failed 321 observations; the corrected bridge passes all 330, with
the same native trace SHA-256
`5406b1e4abe5876caf120d4d7d2e20b036a7835b10eae96bb4cb2ff4e27e3553`.
The camera now uses the source's integer lateral follow and 200-unit cap before
conversion to a road-relative camera. Endpoint camera values interpolate for
display only. Reproduce with `tools/reference/check_camera_pose.py`, after
sourcing `local/reference-tools/env.sh`.

Valid receipts: `local/camera-pose-before-mounted/comparison.json` and
`local/camera-pose-fixed-mounted/comparison.json`. The earlier `camera-pose-before`
directory contains a producer compile error; `camera-pose-before-valid` is an
invalid adapter run that copied a crash-body mirror into a mounted fixture.
Neither is counted as engine evidence. Native Windows runtime was not compared.

The private real-art presentation inventory passed 132 controlled poses across
all 26 branch masks in five courses (104.90 seconds, command exit zero). Each
split and merge is sampled two road intervals beyond its transition, and spaced
physical-node coordinates are checked throughout each selected route. Example
images from City, Napa and Sierra Nevada were visually inspected. These are
constructed presentation poses, not driven trajectories or native pixel
comparisons. Their receipt and commercial-art images remain in
`local/collision-topology-integration/branch-render`.

The final mounted camera matrix expands to 1,980 observations with ground
heights 0, 149, 150, 151, 500 and 5,000. The constant-height baseline fails 990
cases; the corrected native integer follow and ground-height camera pass all
1,980 against trace `da4b81f024fb8ecb7306c22cece3a1ec7114e5e495d1432db36acfb164d7fa14`.
Receipts are `local/camera-height-before-expanded` and
`local/camera-height-fixed-expanded`. This replaces the smaller camera matrix
for aggregate counts; it does not add the earlier 330 again. The per-sample
terrain inventory contains hill heights above 1,000 and mountain/cliff heights
up to 5,057, so the arbitrary former 1,000-unit visual camera limit was expanded
to the bounded source-ground range.

New-profile rider/shadow anchors use the authoritative physical ground value,
interpolated only in display copies. Legacy profiles retain their old adapter.
The snapshot also exposes the already-owned scenery half-mirror follow pose and
pull-back amount. Mounted contact pushes change the actor's depth relative to
that frozen camera instead of moving the camera early. Public checks cover a
seven-Q8 shove and a 900-Q8 pull-back; the latter needs a depth beyond the old
1,024 visual limit. Canonical bytes do not serialize these derived snapshot
fields and remain unaffected. Full recovery/terminal camera timing remains
separate work. Source-scene caches release their retained assets on race exit;
a weak-reference regression checks the actual presentation system.

Ten actual source approaches now enter both first-fork arms in every course.
Each runs 45 ordinary-control ticks with all 15 riders, traffic and static
contacts retained and assists disabled; all four replay schedules match. Fifty
native-art frames accompany the recordings. City left/right and Napa right
frames were inspected. The test uses level 5 because City's first split lies
beyond its level-1 finish; the earlier level-1 attempt is invalid as a City
driving fixture. The initial real run also exposed an offbranch traffic
projection panic, corrected by the physical per-target vehicle bridge. Starting
3.9 units from a fork edge did not guarantee a requested turn under real
curvature and contacts; three missed turns were a fixture expectation, not
native parity failures. The final controlled approach gives 160 units of room
and still uses only throttle and steering. No branch override or result command
is injected. These approaches are not a complete career or a player walkthrough.

Current release binary `8f34d0d5249bedf201cc2ee6da62b364b81e2a45e2e34ce6b58d836eb6282d77` independently replays all ten recordings with retained per-tick reports.
Actual Bevy City/Napa right-arm captures also match their headless final hashes;
both were inspected. Napa's held-right approach continues onto the grass, so
that image is not evidence of a centered driving line. Receipts remain private
under `local/collision-topology-integration/branch-release-replays` and
`graphical-branches-current.json`.
