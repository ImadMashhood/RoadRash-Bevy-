#!/usr/bin/env python3
"""Exercise negative CI cases in a disposable source copy, never the real tree."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='rr-ci-guards-') as directory:
    copy = Path(directory)
    for name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'):
        shutil.copy2(root/name, copy/name)
    for name in ('apps', 'crates', 'docs'):
        shutil.copytree(root/name, copy/name)
    main = copy/'apps/game/src/main.rs'
    main.write_text(main.read_text()+'\nfn deliberately_unformatted( ){ }\n')
    formatting = subprocess.run(['cargo', 'fmt', '--all', '--check'], cwd=copy, capture_output=True, text=True)
    assert formatting.returncode != 0 and 'deliberately_unformatted' in formatting.stdout+formatting.stderr
    manifest = copy/'crates/simulation/Cargo.toml'
    manifest.write_text(manifest.read_text().replace('[dependencies]', '[dependencies]\nrr-formats = { path = "../formats" }'))
    locked = subprocess.run(['cargo', 'check', '--locked', '--offline', '-p', 'rr-simulation'], cwd=copy, capture_output=True, text=True)
    assert locked.returncode != 0 and ('lock file' in locked.stderr or 'lockfile' in locked.stderr), locked.stderr
    print(json.dumps({'formatting_violation_rejected': True, 'lockfile_mismatch_rejected': True, 'repository_modified': False}))
