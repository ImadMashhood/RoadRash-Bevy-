#!/usr/bin/env python3
"""Compare the terrain boundary response with pinned Race::walls."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_encounters import REVISION


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    data, out = args.data.resolve(), args.output.resolve()
    if data == out or data in out.parents or out in data.parents:
        raise ValueError('Output overlaps immutable game data')
    source, build = Path('local/reference-cpp'), Path('local/reference-build')
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != REVISION:
        raise ValueError('Unpinned reference')
    dirty = subprocess.check_output(['git', '-C', str(source), 'diff', '--name-only', 'HEAD'], text=True).splitlines()
    if any(path.startswith('src/') and path != 'src/hud.cpp' for path in dirty):
        raise ValueError('Compared source modified')
    out.mkdir(parents=True, exist_ok=False)
    include = out / 'include'
    include.mkdir()
    (include / 'race.hpp').write_text((source / 'src/race.hpp').read_text().replace('  private:', '  public:'))
    binary = out / 'native-terrain-boundaries'
    subprocess.run([os.environ.get('CXX', 'g++'), '-O2', '-std=c++20', '-Wall', '-Wextra', '-Werror', '-I', str(include), '-I', str(source / 'src'), 'tools/reference/terrain_boundaries.cpp', str(build / 'librr-resource.a'), str(build / 'jpeg-scalar/build/libjpeg.a'), str(build / 'freetype/build/libfreetype.a'), '-lSDL3', '-pthread', '-o', str(binary)], check=True)
    native = subprocess.check_output([str(binary), str(data)])
    (out / 'native.json').write_bytes(native)
    env = os.environ.copy()
    env.update(RR_NATIVE_DATA=str(data), RR_NATIVE_BOUNDARIES=str(out / 'native.json'), RR_BOUNDARY_OUTPUT=str(out), CARGO_TARGET_DIR=str(Path('../target').resolve()))
    tested = subprocess.run(['cargo', 'test', '--locked', '-p', 'rr-simulation', 'source_boundary_native', '--', '--ignored'], env=env, check=False)
    comparison = json.loads((out / 'rust-comparison.json').read_text())
    comparison.update(reference_revision=revision, reference_dirty_paths=dirty,
                      native_trace_sha256=hashlib.sha256(native).hexdigest(),
                      native_library_sha256=hashlib.sha256((build / 'librr-resource.a').read_bytes()).hexdigest(),
                      native_executable_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                      source_executable_sha256=hashlib.sha256((data / 'ROADRASH.EXE').read_bytes()).hexdigest(),
                      original_windows_runtime_compared=False)
    (out / 'comparison.json').write_text(json.dumps(comparison, indent=2) + '\n')
    print(json.dumps(comparison))
    if comparison['mismatches'] or tested.returncode:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
