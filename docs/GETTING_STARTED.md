# Build and verify the foundation

Run commands from the repository root. Rust 1.99.1 is selected by rust-toolchain.toml.

```sh
cargo test --locked -p rr-formats -p rr-simulation -p rr-import -p rr-content -p rr-course -p rr-render
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
python3 -m unittest discover -s tools -p 'test_*.py' -v
cargo run --locked -p rr-game
```

The last command opens an authored classic-style road made of horizontal bands and a red actor placeholder. Up/W accelerates, Down/S brakes, and Left/Right or A/D steers. The logical viewport is 640×480 with resizable presentation. This command uses the controllable fallback scaffold with authored handling constants. To render imported source courses and rider art, pass both data roots as shown under “Play the current classic source view” below.

Linux launch needs an X11/XWayland display, Vulkan-capable graphics driver, linker/compiler tools and Bevy/winit platform development libraries. The local Linux smoke launch and screenshot are recorded in docs/evidence/parts-01-03.md. Other desktop hardware remains untested. No 3D rendering, lighting, physics or audio feature is enabled in this foundation.

For a read-only corpus audit, select your own extracted ISO:

```sh
python3 tools/verify_disc.py /absolute/path/to/source.iso --report /absolute/path/to/report.json
```

This tool validates the ISO directory structure and hashes required files. It does not extract, install or activate assets. Keep source media and captures in ignored `local/` directories. Generated manifests in docs contain hashes and metadata only. Never commit media files to make a test pass.

Workspace dependencies flow from the game composition root to the simulation and presentation crates. Presentation consumes snapshots from the simulation schema. The format and simulation crates have no renderer dependency. The existing local prototype outside this clone is preserved separately.

## Import your own data

```sh
cargo run --locked -p rr-import -- /absolute/path/source.iso /absolute/path/private-install
# Or pass the extracted ROADRASH directory instead of the ISO.
cargo run --locked -p rr-import -- --active /absolute/path/private-install
cargo run --locked -p rr-game -- --install-root /absolute/path/private-install
```

Press Ctrl+C to cancel import. The old active install remains selected on pre-activation failure. See [IMPORT.md](IMPORT.md) for layout, recovery and limits. Private game data is optional for the authored placeholder. `--install-root` validates the import; `--catalog` separately selects converted rider visuals.

For a bounded graphical smoke run, add `--smoke-frames 180 --capture local/foundation.png`. The screenshot is authored placeholder content. Use an ignored local output directory. Run `python3 tools/check_workflow_guards.py` to exercise formatting and lockfile failure cases without changing the repository.

## Decode and inspect assets

Pass the importer root containing `active.json`; the converter resolves and validates the active generation:

```sh
cargo run --locked --release -p rr-content -- build /absolute/private-install local/catalog
python3 -m http.server 8789 --bind 127.0.0.1 --directory local/catalog
cargo run --locked --release -p rr-game -- --catalog local/catalog
cargo run --locked --release -p rr-simulation --bin rr-replay -- docs/fixtures/replay.json irregular local/replay.json
cargo run --locked --release -p rr-game -- --catalog local/catalog --replay docs/fixtures/replay.json --render-hz 144 --report local/rendered.json
```

Open http://127.0.0.1:8789/inspector.html for contact sheets and frame/palette/alpha controls. To test an authored override, run `cargo run --locked --release -p rr-content --example authored_override -- local/authored-override.png`, then add `--enhanced-image local/authored-override.png --switch-textures-at 60` to the graphical replay. This is test art, not an approved enhancement pack. See [CONTENT.md](CONTENT.md) and [SIMULATION.md](SIMULATION.md) for cache/schema/replay contracts. All generated art and captures stay ignored.

## Reconstruct and inspect courses

```sh
cargo run --locked --release -p rr-course -- inspect /absolute/private-install local/courses
python3 -m http.server 8790 --bind 127.0.0.1 --directory local/courses
```

Open http://127.0.0.1:8790/index.html. Choose any course, level or branch route; jump to its finish, step samples and inspect before/at/after split and merge panels. The command validates the active import and exports all 26 branches with coverage.json. Generated course definitions and screenshots stay private. See [COURSES.md](COURSES.md) for units, endpoint conventions, reproducibility and source semantics and rendering boundaries.

## Play the current classic source view

```sh
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course CITY
cargo run --locked --release -p rr-game -- --install-root local/game-install --catalog local/catalog --course HIWAY --level 2 --branch-mask 1 --framing wide --window-size 1280x720
```

Up/W accelerates, Down/S brakes and Left/Right or A/D steers with the current authored handling. Source rendering is active when both install and catalog roots are provided. Add `--integer-scale on` for whole-pixel enlargement, or use `--camera-sample NUMBER` for a source inspection offset. The [renderer contract](RENDERING.md) documents exact policies and remaining gameplay scope.

```sh
cargo run --locked --release -p rr-render -- authored local/render-authored
cargo run --locked --release -p rr-render -- suite local/game-install local/catalog local/render-checkpoints
cargo run --locked --release -p rr-render -- diff local/expected.png local/actual.png local/diff.png
```

Authored scenes work without commercial data. Source-derived captures/diffs stay ignored; public CI checks authored pixel hashes and bounded rendering behavior.
