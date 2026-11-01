#!/bin/sh
# Run inside the pinned official Rust container with /source read-only and /evidence writable.
set -eu
apt-get update
apt-get install -y --no-install-recommends libx11-dev libxi-dev libxkbcommon-dev pkg-config python3
mkdir -p /build
cp /source/Cargo.toml /source/Cargo.lock /source/rust-toolchain.toml /build/
cp -r /source/apps /source/crates /source/docs /source/tools /build/
useradd --uid 1000 --create-home rrbuild
chown -R rrbuild:rrbuild /build
chmod a+w /evidence
su rrbuild -s /bin/sh -c '
export PATH=/usr/local/cargo/bin:$PATH
export CARGO_HOME=/home/rrbuild/.cargo
export RUSTUP_HOME=/usr/local/rustup
cd /build
rustc --version
cargo --version
cargo fmt --all --check
python3 -m unittest discover -s tools -p "test_*.py" -v
cargo test --locked -p rr-formats -p rr-simulation -p rr-import -p rr-content -p rr-course
cargo clippy --locked --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=4 cargo build --locked --release --workspace
cargo package --list --allow-dirty -p rr-game > /evidence/game-package-files.txt
printf "clean-container-build-passed\n" > /evidence/result.txt
'
