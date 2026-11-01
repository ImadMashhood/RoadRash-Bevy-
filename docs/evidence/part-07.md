# Part 07 acceptance receipt

Scope: bounded static course reconstruction, branch-aware traversal, scenery placement and offline inspection. Original-runtime comparison remains waived; 3D remains deferred. Implementation revision is the Git commit containing this receipt and the matching task checkboxes.

## Result

All 12 Part 07 parents and their 36 children are complete. `rr-course` supplies the source-unit graph, section tables, route cursor, split/merge windows, interval stepping, terrain/event/family/scenery records, deterministic placements and an interactive route explorer. The game composition remains the existing authored driving scaffold until Phase 08 integrates classic course rendering; this is not a finished race/game release.

The verified corpus contains **109 sections and 141 nodes** across five courses. All **26 branch combinations** visit every section collectively. All **130 branch/level finish results** match numerical facts from the pinned reference's course-test census. All **132 split/merge seam checks** pass: retained road-edge error is zero, merge separation closes, continuous rise/width/shoulder/side-height interpolation is consistent, and each source path can be read forward/reverse without overrun. Raw source header resets remain separately reported.

## Commands and outcomes

```sh
cargo test --locked -p rr-course -p rr-formats -p rr-content -p rr-import -p rr-simulation
cargo test --locked --release -p rr-presentation --target-dir ../target
cargo clippy --locked --workspace --all-targets --target-dir ../target -- -D warnings
cargo build --locked --release --workspace --bins --examples --target-dir ../target -j4
cargo fmt --all --check
python3 -m unittest discover -s tools -p 'test_*.py' -v
python3 tools/check_workflow_guards.py
../target/release/rr-course inspect local/game-install local/courses
../target/release/rr-course inspect local/game-install local/courses-repro
python3 -m http.server 8790 --bind 127.0.0.1 --directory local/courses
```

Passed: **73 Rust tests** (20 new course tests plus 53 existing tests), **10 Python tests**, formatting, workspace Clippy and linked release build. CI and clean-container recipes now include rr-course in headless tests/lints; these local results do not claim that a remote Actions or new clean-container run has completed.

Course tests cover contextual dangling references, invalid roots/kinds/counts/aliases, genuine forward cycles versus intentional backlinks, every truncation of the authored course container, 3,000 deterministic nested-table mutations with zero panics, split/merge/dual windows, signed bend/rise, exact Q8 and direct section endpoint conventions, lane/side units, initial/equal/gap interval quirks, signed hills/cliff bases and unknown flags, event boundaries/first finish, deterministic level gating and stable repeated object identities. The exporter rejects source/output overlap before creating a directory in immutable data.

The private CLI enumerates and validates every route, checks exact expected lengths/finishes from [pinned census metadata](../fixtures/course-expectations.json), visits every whole sample forward/reverse and exercises fractional positions on both sides of all seams. [Coverage](course-coverage.json) preserves per-route and per-course results. [Reproducibility](course-reproducibility.json) compares all seven generated files from independent output directories byte-for-byte. No original source or decoded course payload is committed.

## Browser and visual verification

At http://127.0.0.1:8790/index.html, selected all 25 course/level combinations and used Jump to finish; every cursor matched its displayed finish. Selected all 26 branch routes and checked their lengths and merge views. Selected every seam in every branch and saved **132 full-page screenshots**, each containing fixed-camera before/at/after panels. The displayed retained-edge error was zero and continuous_geometry_passed was true in every case. Course switching, level selection, branch selection, finish jumping, sample stepping and source/unknown metadata were exercised through the browser UI. A responsive grid overflow found in visual review was corrected and the final layout rechecked.

[Browser QA metadata](course-browser-qa.json) records course/level results, branch lengths and the 132 local screenshot filenames/hashes. Images remain under ignored `local/course-seams/`; the final overview is `local/course-explorer.png`. They are review artifacts, not original-executable screenshots or a claim of final road-renderer fidelity.

## Deliberate policies and remaining work

Source per-section elevation is preserved. The continuous route-elevation integral and fractional blending are explicit inspection policies; raw header resets and side-height steps remain visible. The plan map's bend/512 heading scale is schematic. Full random hill/wall geometry, textured road/terrain rendering, billboard/family animations and source-course driving integration are Phase 08 or later gameplay work. Reverse travel uses the selected route's branch history and cached validated samples; direct backwards section initializers are available separately, rather than falsely claiming every original iterator's hidden state was reproduced.

Offline placement uses an isolated seed-1 stream and does not claim full original race RNG interleaving. Unresolved family slot 0 is explicit: five roadside placement instances summed over Pacific Highway's levels. They remain records with source IDs; no substitute art is fabricated. Other applicable family IDs resolve to source family resources. Pedestrian/police spawn requests keep raw words and no guessed lateral position. Police spawning uses its separate gameplay assets, so its scenery-family field is absent rather than incorrectly reported missing. Unknown terrain flags and record words remain inspectable.

Pinned source facts come from [course.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/course.cpp), [road.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/road.cpp), [terrain.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/terrain.cpp), [scenery.cpp](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/scenery.cpp) and the numerical census in tests/course_test.cpp. No upstream C++ implementation was copied; upstream runtime test claims are not presented as this project's results.
