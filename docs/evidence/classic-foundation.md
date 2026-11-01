# Classic foundation verification

Scope change: 3D deferred by explicit user instruction. ADR 0002 supersedes the earlier 3D presentation direction.

- `cargo test --locked -p rr-formats -p rr-simulation`: 5 tests passed.
- `python3 -m unittest discover -s tools -p 'test_*.py' -v`: 6 tests passed.
- `cargo fmt --all --check`: passed.
- `cargo check --workspace`: passed (shared local target cache).
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed (shared local target cache).

The app uses a 2D camera and sprites; PBR/glTF/3D renderer features are no longer requested. The placeholder is authored content. Neither original-game handling fidelity nor actual GUI launch is certified by compilation. CI configuration now includes a Linux desktop compile/lint job, but a remote run is not claimed here.
