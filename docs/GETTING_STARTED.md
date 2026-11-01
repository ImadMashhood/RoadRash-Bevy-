# Build and verify the foundation

Run commands from the repository root. Rust 1.99.1 is selected by rust-toolchain.toml.

```sh
cargo test --locked -p rr-formats
cargo clippy --locked -p rr-formats --all-targets -- -D warnings
cargo fmt --all --check
python3 -m unittest discover -s tools -p 'test_*.py' -v
cargo run --locked -p rr-game
```

The last command opens an authored road slab and red box with a 3D camera and directional light. It is a foundation scene, not a playable motorcycle or a faithful game. Linux launch needs an X11/XWayland display, Vulkan-capable graphics driver, linker/compiler tools and the development libraries required by Bevy/winit on the chosen distribution. Hardware launch is not yet certified. No audio feature is enabled in this foundation.

For a read-only corpus audit, select your own extracted ISO:

```sh
python3 tools/verify_disc.py /absolute/path/to/source.iso --report /absolute/path/to/report.json
```

This tool validates the ISO directory structure and hashes required files. It does not extract, install or activate assets. Keep source media and captures in ignored `local/` directories. Generated manifests in docs contain hashes and metadata only. Never commit media files to make a test pass.

Workspace dependencies flow from rr-game to rr-formats. The format crate has no renderer dependency. Simulation and presentation crates will be introduced when their first real consumers exist. The existing local prototype outside this clone is preserved separately.
