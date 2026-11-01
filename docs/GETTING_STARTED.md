# Build and verify the foundation

Run commands from the repository root. Rust 1.99.1 is selected by rust-toolchain.toml.

```sh
cargo test --locked -p rr-formats -p rr-simulation
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
python3 -m unittest discover -s tools -p 'test_*.py' -v
cargo run --locked -p rr-game
```

The last command opens an authored classic-style road made of horizontal bands and a red actor placeholder. Up/W accelerates, Down/S brakes, and Left/Right or A/D steers. The logical viewport is 640×480 with resizable presentation. This is a controllable architecture scaffold: it does not yet load original courses or sprites, and its handling constants are authored placeholders.

Linux launch needs an X11/XWayland display, Vulkan-capable graphics driver, linker/compiler tools and Bevy/winit platform development libraries. Hardware launch has not been certified. No 3D rendering, lighting, physics or audio feature is enabled in this foundation.

For a read-only corpus audit, select your own extracted ISO:

```sh
python3 tools/verify_disc.py /absolute/path/to/source.iso --report /absolute/path/to/report.json
```

This tool validates the ISO directory structure and hashes required files. It does not extract, install or activate assets. Keep source media and captures in ignored `local/` directories. Generated manifests in docs contain hashes and metadata only. Never commit media files to make a test pass.

Workspace dependencies flow from the game composition root to the simulation and presentation crates. Presentation consumes snapshots from the simulation schema. The format and simulation crates have no renderer dependency. The existing local prototype outside this clone is preserved separately.
