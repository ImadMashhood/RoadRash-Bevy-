#!/usr/bin/env python3
"""Build a private reviewable Linux candidate. Never labels an unaccepted game as released."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
from notices import collect

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(target, output):
    output = output.resolve()
    if output.exists():
        raise ValueError('Use a new package directory.')
    if not (ROOT / 'local').resolve() in output.parents:
        raise ValueError('Unaccepted candidate artifacts must stay under ignored local/.')
    folder = output / 'RoadRash-Rust'
    (folder / 'bin').mkdir(parents=True)
    for name in ['rr-game', 'rr-import', 'rr-content']:
        shutil.copy2(target / name, folder / 'bin' / name)
    shutil.copy2(ROOT / 'tools/release/launcher.py', folder / 'RoadRash.py')
    (folder / 'RoadRash').write_text('#!/bin/sh\nexec python3 "$(dirname -- "$0")/RoadRash.py" "$@"\n')
    (folder / 'RoadRash').chmod(0o755)
    (folder / 'README.txt').write_text(
        'Road Rash Rust — native-art Linux candidate, not an accepted release.\n'
        'Run ./RoadRash. First launch asks for your own ISO or ZIP, verifies the import and prepares original assets.\n'
        'Dependencies: Python 3, zenity, Vulkan/X11 graphics, ALSA/PipeWire; FFmpeg/ffprobe for videos.\n'
        'Game data and saves live in your user data/config directories, outside this application.\n'
        'Upgrading: replace this application directory. Uninstall: delete this directory; user data remains.\n'
        'Troubleshooting: ~/.local/share/roadrash-rust/launcher.log (or the XDG_DATA_HOME equivalent).\n'
        'Keep imported game files and captures private. No original game media ships in this candidate.\n')
    shutil.copyfile(ROOT / 'docs/PLAYER_GUIDE.md', folder / 'PLAYER-GUIDE.md')
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1',
        '--filter-platform', 'x86_64-unknown-linux-gnu'], cwd=ROOT))
    dependencies = collect(metadata, folder / 'third-party', ROOT / 'local/license-cache')
    font_notice = ROOT / 'tools/release/FiraMono-OFL.txt'
    if not font_notice.is_file():
        raise ValueError('Missing embedded Fira Mono font notice')
    shutil.copyfile(font_notice, folder / 'third-party/FiraMono-OFL.txt')
    (folder / 'LICENSE-SCOPE.txt').write_text(
        'Dependency licenses and the embedded Fira Mono OFL are under third-party/.\n'
        'Each dependency retains its own licensing; no original game content or license is included.\n'
        'The project owner has not selected a project-wide license for the recreation source.\n'
        'The supplied original game data remains separately owned and is imported only locally.\n')
    source_files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=ROOT).split(b'\0')
    source_hash = hashlib.sha256()
    for name in sorted(set(source_files)):
        if name and (ROOT / name.decode()).is_file():
            source_hash.update(name + b'\0' + (ROOT / name.decode()).read_bytes() + b'\0')
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    manifest = {'schema_version': 1, 'status': 'unaccepted development candidate', 'base_revision': revision,
                'worktree_diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'], cwd=ROOT)).hexdigest(),
                'toolchain': subprocess.check_output(['rustc', '--version'], text=True).strip(),
                'cargo_lock_sha256': digest(ROOT / 'Cargo.lock'), 'private_game_payloads': False,
                'third_party_notices_complete': True, 'dependency_notice_count':len(dependencies), 'source_files_sha256':source_hash.hexdigest(), 'files': []}
    for path in sorted(folder.rglob('*')):
        if path.is_file():
            assert path.suffix.lower() not in {'.iso', '.png', '.wav', '.rra', '.avi', '.dll', '.exe'}
            manifest['files'].append({'path': str(path.relative_to(folder)), 'bytes': path.stat().st_size, 'sha256': digest(path)})
    (folder / 'candidate-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    archive = output / 'RoadRash-Rust-linux-candidate.tar.gz'
    with tarfile.open(archive, 'w:gz') as bundle:
        bundle.add(folder, arcname='RoadRash-Rust')
    (output / 'SHA256SUMS').write_text(digest(archive) + '  ' + archive.name + '\n')
    return archive


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--target', type=Path, default=ROOT.parent / 'target/release')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    print(build(a.target.resolve(), a.output))
