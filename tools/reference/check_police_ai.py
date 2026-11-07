#!/usr/bin/env python3
"""Compare actual native source cop AI decision transactions with Rust."""
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
    modes=p.add_mutually_exclusive_group()
    modes.add_argument('--objects', action='store_true')
    modes.add_argument('--geometry', action='store_true')
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
    for header in (src/'src').glob('*.hpp'):
        (include/header.name).write_text(header.read_text().replace('  private:', '  public:') if header.name=='race.hpp' else header.read_text())
    binary = out / 'native-police-ai'
    subprocess.run([os.environ.get('CXX', 'g++'), '-O2', '-std=c++20', '-Wall', '-Wextra', '-Werror', '-I', str(include), '-I', str(src / 'src'), 'tools/reference/police_ai.cpp', str(build/'librr-resource.a'), str(build/'jpeg-scalar/build/libjpeg.a'), str(build/'freetype/build/libfreetype.a'), '-lSDL3', '-pthread', '-o', str(binary)], check=True)
    native = subprocess.check_output([str(binary), str(data)] + (['objects'] if a.objects else ['geometry'] if a.geometry else []))
    (out/'native.json').write_bytes(native)
    env = os.environ.copy()
    env.update(RR_NATIVE_DATA=str(data), RR_NATIVE_POLICE_AI=str(out/'native.json'), RR_NATIVE_OUTPUT=str(out), CARGO_TARGET_DIR=str(Path('../target').resolve()))
    subprocess.run(['cargo', 'test', '--locked', '-p', 'rr-simulation', 'native_police_ai::source_police_ai_geometry_native' if a.geometry else 'native_police_ai::source_police_ai_native', '--', '--ignored'], env=env, check=True)
    actual = json.loads((out/'police-ai-rust.json').read_text())
    expected = json.loads(native) if a.geometry else [row['output'] for row in json.loads(native)]
    legacy = os.environ.get('RR_POLICE_AI_LEGACY') == '1'
    columns = [0, 4, 6, 17, 18, 19, 20, 21, 22, 24] if legacy else None
    if columns:
        expected = [[row[i] for i in columns] for row in expected]
        actual = [[row[i] for i in columns] for row in actual]
    result = compare_cases('source_police_ai_lane_geometry' if a.geometry else 'source_police_ai_objects' if a.objects else 'source_police_ai', expected, actual)
    if columns:
        result['compared_columns'] = ['rng_state', 'target_lateral', 'target_speed', 'force', 'drive', 'lean', 'previous_lean', 'lateral_speed', 'speed', 'animation_code']
        result['baseline_scope'] = 'Frozen unaccepted scaffold command selection plus its single player-control application; no invented old brain state comparisons'

    receipt = dict(outcome=result['outcome'], reference_revision=revision, reference_dirty_paths=dirty, native_library_sha256=hashlib.sha256((build/'librr-resource.a').read_bytes()).hexdigest(), native_trace_sha256=hashlib.sha256(native).hexdigest(), executable_sha256=hashlib.sha256((data/'ROADRASH.EXE').read_bytes()).hexdigest(), checks=[result], original_windows_runtime_compared=False, rust_legacy_setup=os.environ.get('RR_POLICE_AI_LEGACY')=='1', before_wrap=os.environ.get('RR_POLICE_AI_BEFORE_WRAP')=='1')
    (out/'comparison.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps(receipt))
    if receipt['outcome'] != 'passed':
        raise SystemExit(1)

if __name__ == '__main__':
    main()
