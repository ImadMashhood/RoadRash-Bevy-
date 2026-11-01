#!/usr/bin/env bash
set -euo pipefail
rr_repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$rr_repo"
mkdir -p local
if [[ -n "${RR_CALIBRATION_BINARY:-}" ]]; then
    rr_binary="$RR_CALIBRATION_BINARY"
elif [[ -x "$rr_repo/target/release/rr-calibrate" ]]; then
    rr_binary="$rr_repo/target/release/rr-calibrate"
else
    rr_binary="$rr_repo/../target/release/rr-calibrate"
fi
[[ -x "$rr_binary" ]] || { echo 'Build first: cargo build --locked --release -p rr-game --bin rr-calibrate' >&2; exit 1; }
exec "$rr_binary" --profile "$rr_repo/local/controller-calibration.json" "$@"
