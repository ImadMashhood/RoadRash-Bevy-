# Foundation verification receipt

Commands run in the workspace using pinned Rust 1.99.1:

- `cargo test --locked -p rr-formats`: 3 passed, 0 failed.
- `cargo clippy --locked -p rr-formats --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `python3 -m unittest discover -s tools -p 'test_*.py' -v`: 6 passed, 0 failed.
- `cargo check --locked -p rr-game`: passed (local shared target directory used for compilation cache).
- `python3 tools/verify_disc.py local/source.iso --report docs/evidence/disc-corpus.json`: 341 matched, 0 failures.

Original input/camera/menu capture, production import activation, gameplay and actual graphical launch have not passed acceptance. No phase is declared complete from these checks.
