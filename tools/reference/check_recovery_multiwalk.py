#!/usr/bin/env python3
"""Compare full LIVE crowded recovery motion/contact episodes; RR_RECOVERY_MULTIWALK_FULL enables the144-case matrix."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_encounters import REVISION


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--data', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    data, out = a.data.resolve(), a.output.resolve()
    if data == out or data in out.parents or out in data.parents:
        raise ValueError('Output overlaps immutable game data')
    source, build = Path('local/reference-cpp'), Path('local/reference-build')
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != REVISION:
        raise ValueError('Unpinned reference')
    dirty = subprocess.check_output(['git', '-C', str(source), 'diff', '--name-only', 'HEAD'], text=True).splitlines()
    if any(s.startswith('src/') and s != 'src/hud.cpp' for s in dirty):
        raise ValueError('Compared native source modified')
    out.mkdir(parents=True, exist_ok=False)
    include = out / 'include'
    include.mkdir()
    (include / 'race.hpp').write_text((source / 'src/race.hpp').read_text().replace('  private:', '  public:'))
    (include / 'traffic.hpp').write_bytes((source / 'src/traffic.hpp').read_bytes())
    binary = out / 'native-recovery-multiwalk'
    subprocess.run([os.environ.get('CXX', 'g++'), '-O2', '-std=c++20', '-Wall', '-Wextra', '-Werror', '-I', str(include), '-I', str(source / 'src'), 'tools/reference/recovery_multiwalk.cpp', str(build/'librr-resource.a'), str(build/'jpeg-scalar/build/libjpeg.a'), str(build/'freetype/build/libfreetype.a'), '-lSDL3', '-pthread', '-o', str(binary)], check=True)
    native = subprocess.check_output([str(binary), str(data)])
    (out/'native.json').write_bytes(native)
    env = os.environ.copy()
    env.update(RR_NATIVE_DATA=str(data), RR_NATIVE_RECOVERY_MULTIWALK=str(out/'native.json'), RR_NATIVE_RECOVERY_MULTIWALK_OUTPUT=str(out), CARGO_TARGET_DIR=str(Path('../target').resolve()))
    run = subprocess.run(['cargo', 'test', '--locked', '-p', 'rr-simulation', '--lib', 'source_recovery_multiwalk_native', '--', '--ignored'], env=env, check=False)
    receipt = json.loads((out/'recovery-multiwalk-comparison.json').read_text())
    receipt.update(reference_revision=revision, reference_dirty_paths=dirty, native_trace_sha256=hashlib.sha256(native).hexdigest(), native_library_sha256=hashlib.sha256((build/'librr-resource.a').read_bytes()).hexdigest(), executable_sha256=hashlib.sha256((data/'ROADRASH.EXE').read_bytes()).hexdigest(), original_windows_runtime_compared=False)
    (out/'comparison.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps(receipt))
    if run.returncode or receipt['mismatches'] or receipt['pre_mismatches']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
