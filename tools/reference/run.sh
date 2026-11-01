#!/usr/bin/env bash
set -euo pipefail
rr_repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$rr_repo"
if [[ "${1:-}" == --version ]]; then exec "$rr_repo/local/reference-build/rr-game" --version; fi
rr_assets="$(python3 - <<'PY'
import json,re
from pathlib import Path
root=Path('local/game-install');generation=json.loads((root/'active.json').read_text())['generation']
if not re.fullmatch(r'install-[A-Za-z0-9_-]+',generation):raise SystemExit('Invalid install generation')
p=(root/'installs'/generation/'ROADRASH').resolve()
if not (p/'ROADRASH.EXE').is_file():raise SystemExit('Import game data first')
print(p)
PY
)"
export RR_PREF_OVERRIDE="$rr_repo/local/reference-prefs/interactive/"
mkdir -p "$RR_PREF_OVERRIDE"
exec "$rr_repo/local/reference-build/rr-game" --assets "$rr_assets" --windowed --window-scale 2 "$@"
