#!/usr/bin/env python3
"""Compare actual pinned per-route finish ownership with source route profiles."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_encounters import compare_cases, REVISION


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--data', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    a = p.parse_args()
    data, out = a.data.resolve(), a.output.resolve()
    if data == out or data in out.parents or out in data.parents:
        raise ValueError('Output overlaps immutable game data')
    src, build = Path('local/reference-cpp'), Path('local/reference-build')
    revision = subprocess.check_output(['git', '-C', str(src), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != REVISION:
        raise ValueError('Unpinned reference')
    dirty = subprocess.check_output(['git', '-C', str(src), 'diff', '--name-only', 'HEAD'], text=True).splitlines()
    if any(s.startswith('src/') and s != 'src/hud.cpp' for s in dirty):
        raise ValueError('Compared reference sources modified')
    out.mkdir(parents=True, exist_ok=False)
    include = out / 'include'
    include.mkdir()
    (include / 'race.hpp').write_text((src / 'src/race.hpp').read_text().replace('  private:', '  public:'))
    binary = out / 'native-topology-finishes'
    subprocess.run([os.environ.get('CXX', 'g++'), '-O2', '-std=c++20', '-Wall', '-Wextra', '-Werror', '-I', str(include), '-I', str(src / 'src'), 'tools/reference/topology_finishes.cpp', str(build/'librr-resource.a'), str(build/'jpeg-scalar/build/libjpeg.a'), str(build/'freetype/build/libfreetype.a'), '-lSDL3', '-pthread', '-o', str(binary)], check=True)
    native = subprocess.check_output([str(binary), str(data)])
    (out/'native.json').write_bytes(native)
    env = os.environ.copy()
    env.update(RR_NATIVE_DATA=str(data), RR_NATIVE_TOPOLOGY_FINISHES=str(out/'native.json'), RR_NATIVE_OUTPUT=str(out), CARGO_TARGET_DIR=str(Path('../target').resolve()))
    subprocess.run(['cargo', 'test', '--locked', '-p', 'rr-simulation', 'native_cases::source_topology_finishes_native', '--', '--ignored'], env=env, check=True)
    actual = json.loads((out/'topology-finishes-rust.json').read_text())
    result = compare_cases('physical_route_finishes', json.loads(native), actual)
    receipt = dict(outcome=result['outcome'], reference_revision=revision, reference_dirty_paths=dirty, native_library_sha256=hashlib.sha256((build/'librr-resource.a').read_bytes()).hexdigest(), native_trace_sha256=hashlib.sha256(native).hexdigest(), executable_sha256=hashlib.sha256((data/'ROADRASH.EXE').read_bytes()).hexdigest(), checks=[result], original_windows_runtime_compared=False, rust_legacy_setup=os.environ.get('RR_WALK_LEGACY')=='1')
    (out/'comparison.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps(receipt))
    if receipt['outcome'] != 'passed':
        raise SystemExit(1)

if __name__ == '__main__':
    main()
