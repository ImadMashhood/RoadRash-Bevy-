#!/usr/bin/env python3
"""Opt-in process-level import acceptance on user-owned data; outputs stay local."""
import argparse
import hashlib
import json
from pathlib import Path
import signal
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--iso', type=Path, required=True)
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--game', type=Path)
parser.add_argument('--workspace', type=Path, required=True)
args = parser.parse_args()
args.binary = args.binary.resolve()
if args.game:
    args.game = args.game.resolve()
root = args.workspace.resolve()
root.mkdir(parents=True, exist_ok=False)
install = root/'install raíz'

def run(source, target, success=True):
    result = subprocess.run([str(args.binary), str(source), str(target)], capture_output=True, text=True)
    if (result.returncode == 0) != success:
        raise RuntimeError(result.stderr)
    return result

def active(target):
    a = json.loads((target/'active.json').read_text())
    return target/'installs'/a['generation']/'ROADRASH'

run(args.iso.resolve(), install)
assert len(list(active(install).rglob('*'))) >= 341
checks = ['full ISO import: 341 files']
for sig, label in [(signal.SIGINT, 'cancel'), (signal.SIGKILL, 'forced termination')]:
    pointer = (install/'active.json').read_bytes()
    with (root/(label.replace(' ', '-')+'.log')).open('w') as log:
        p = subprocess.Popen([str(args.binary), str(args.iso.resolve()), str(install)], stdout=log, stderr=log)
        deadline = time.monotonic()+20
        while time.monotonic() < deadline:
            staged = list((install/'installs').glob('.rr-staging-*'))
            if staged and len(list(staged[0].rglob('*'))) >= 5:
                break
            if p.poll() is not None:
                raise RuntimeError('import completed before interruption fixture could trigger')
            time.sleep(.001)
        else:
            p.kill(); p.wait(); raise RuntimeError('no staging work observed')
        p.send_signal(sig)
        assert p.wait(timeout=20) != 0
    assert (install/'active.json').read_bytes() == pointer
    assert active(install).joinpath('ROADRASH.EXE').is_file()
    if args.game:
        subprocess.run([str(args.game), '--install-root', str(install), '--smoke-frames', '120'], check=True, stdout=subprocess.DEVNULL)
    checks.append(label+': previous pointer and launch preserved')
run(args.iso.resolve(), install)
assert not list((install/'installs').glob('.rr-staging-*'))
checks.append('next import recovers interrupted staging')
source = active(install)
exe = source/'ROADRASH.EXE'
original = exe.read_bytes()
try:
    modified = bytearray(original); modified[-1] ^= 1; exe.write_bytes(modified)
    pointer = (install/'active.json').read_bytes()
    result = run(source, root/'modified-revision', success=False)
    assert 'wrong-hash' in result.stderr and 'ROADRASH.EXE' in result.stderr
    assert (install/'active.json').read_bytes() == pointer
finally:
    exe.write_bytes(original)
checks.append('one-byte modified original executable rejected with file diagnostic')
for file in source.rglob('*'):
    if file.is_file():
        file.chmod(0o444)
for directory in sorted((p for p in source.rglob('*') if p.is_dir()), reverse=True):
    directory.chmod(0o555)
source.chmod(0o555)
second = root/'folder install 私の'
run(source, second)
assert hashlib.sha256((active(second)/'ROADRASH.EXE').read_bytes()).hexdigest() == '73d7f9744b5b1a9482b3909950cdf3fe34119fe13d023b031e9c6739a60f18a2'
checks.append('read-only extracted folder and Unicode/space destination: 341 files')
receipt = json.loads((active(second).parent/'receipt.json').read_text())
report = {'schema_version': 1, 'checks': checks, 'required_files': len(receipt['files']), 'importer_version': receipt['importer_version'], 'importer_build_sha256': receipt['importer_build_sha256'], 'manifest_sha256': receipt['manifest_sha256'], 'recipe_version': receipt['recipe_version'], 'outcome': 'passed'}
(root/'report.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report))
