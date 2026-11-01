#!/usr/bin/env python3
"""Native-art desktop launcher: private first-run import, conversion and game startup."""
from pathlib import Path
import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import zipfile


def user_root():
    value = Path(os.environ.get('XDG_DATA_HOME', Path.home() / '.local/share'))
    if not value.is_absolute():
        value = Path.home() / '.local/share'
    return value / 'roadrash-rust'


def dialog(*args):
    result = subprocess.run(['zenity', *args], capture_output=True, text=True)
    if result.returncode == 1:
        return None
    if result.returncode != 0:
        raise RuntimeError('The graphical file picker failed. See the launch log.')
    return result.stdout.rstrip('\n')


def run_stage(command, label, log, graphical):
    """Cancellation sends SIGINT to the transactional importer before reaping it."""
    process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    progress = subprocess.Popen(['zenity', '--progress', '--pulsate', '--auto-close',
                                 '--title=Road Rash Rust', '--text=' + label],
                                stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL) if graphical else None
    try:
        while process.poll() is None:
            if progress and progress.poll() is not None:
                process.send_signal(signal.SIGINT)
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                raise RuntimeError(label + ' canceled; the previous valid import is retained.')
            time.sleep(0.1)
        if process.returncode:
            raise RuntimeError(label + ' failed. See the launch log.')
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        if progress and progress.poll() is None:
            progress.terminate()
            progress.wait()


def source_iso(source, temporary):
    source = source.resolve(strict=True)
    if source.suffix.lower() != '.zip':
        return source
    with zipfile.ZipFile(source) as archive:
        candidates = [i for i in archive.infolist() if not i.is_dir() and i.filename.lower().endswith('.iso')]
        if len(candidates) != 1 or not 0 < candidates[0].file_size <= 1024 ** 3:
            raise ValueError('Choose a ZIP containing exactly one ISO of at most 1 GiB.')
        output = temporary / 'source.iso'
        with archive.open(candidates[0]) as reader, output.open('xb') as writer:
            remaining = 1024 ** 3
            while chunk := reader.read(min(1024 ** 2, remaining + 1)):
                if len(chunk) > remaining:
                    raise ValueError('ZIP ISO exceeds the extraction limit.')
                writer.write(chunk)
                remaining -= len(chunk)
        return output


def launch(package, root, source=None, graphical=True, prepare_only=False, game_args=()):
    package, root = package.resolve(strict=True), root.resolve()
    root.mkdir(parents=True, exist_ok=True)
    install, catalog = root / 'import', root / 'catalog-rr-content-3'
    log_path = root / 'launcher.log'
    if log_path.exists() and log_path.stat().st_size > 1024 ** 2:
        log_path.replace(root / 'launcher.previous.log')
    with log_path.open('a') as log, tempfile.TemporaryDirectory(prefix='rr-import-', dir=root) as temp:
        if not (install / 'active.json').exists() or source is not None:
            if source is None:
                if not graphical:
                    raise ValueError('No imported game data. Supply --source PATH to your Road Rash ISO or ZIP.')
                selected = dialog('--file-selection', '--title=Choose your Road Rash ISO or ZIP',
                                  '--file-filter=Game disc | *.iso *.ISO *.zip *.ZIP')
                if not selected:
                    return 0
                source = Path(selected)
            iso = source_iso(Path(source), Path(temp))
            run_stage([str(package / 'bin/rr-import'), str(iso), str(install)],
                      'Verifying and importing your local game data', log, graphical)
        # Always validate the active import; existence alone never authorizes it.
        active = subprocess.run([str(package / 'bin/rr-import'), '--active', str(install)],
                                capture_output=True, text=True, check=True)
        if not active.stdout.strip():
            raise RuntimeError('No verified active import.')
        current = catalog / 'catalog.json'
        rebuild = True
        if current.exists():
            try:
                metadata = json.loads(current.read_text())
                rebuild = metadata['recipe']['converter_version'] != 'rr-content-3'
            except (KeyError, ValueError):
                pass
        if rebuild:
            run_stage([str(package / 'bin/rr-content'), 'build', str(install), str(catalog)],
                      'Preparing original sprites and course data', log, graphical)
        if prepare_only:
            return 0
        command = [str(package / 'bin/rr-game'), '--install-root', str(install), '--catalog', str(catalog),
                   '--frontend', 'on', *game_args]
        result = subprocess.run(command, cwd=root, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode:
            raise RuntimeError('The game could not start or exited with an error. Check desktop graphics/runtime dependencies and the launch log.')
        return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data-root', type=Path, default=user_root())
    parser.add_argument('--source', type=Path)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--headless', action='store_true', help='Automated installer acceptance; no dialogs')
    parser.add_argument('--package-root', type=Path, default=Path(__file__).resolve().parent)
    args, game_args = parser.parse_known_args()
    try:
        if not args.headless and not shutil.which('zenity'):
            raise RuntimeError('Install zenity to enable the graphical first-launch importer.')
        return launch(args.package_root, args.data_root, args.source, not args.headless,
                      args.prepare_only, game_args)
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        message = f'{error}\nLog: {args.data_root / "launcher.log"}'
        if not args.headless and shutil.which('zenity'):
            subprocess.run(['zenity', '--error', '--title=Road Rash Rust', '--text=' + message], check=False)
        else:
            print(message, file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
