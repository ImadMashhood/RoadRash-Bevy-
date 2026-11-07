#!/usr/bin/env python3
"""Compare the complete mounted terrain motion with pinned Race::step_rider."""
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
    binary = out / 'native-terrain-motion'
    subprocess.run([os.environ.get('CXX', 'g++'), '-O2', '-std=c++20', '-Wall', '-Wextra', '-Werror', '-I', str(include), '-I', str(source / 'src'), 'tools/reference/terrain_motion.cpp', str(build / 'librr-resource.a'), str(build / 'jpeg-scalar/build/libjpeg.a'), str(build / 'freetype/build/libfreetype.a'), '-lSDL3', '-pthread', '-o', str(binary)], check=True)
    native = subprocess.check_output([str(binary), str(data)])
    (out / 'native.json').write_bytes(native)
    env = os.environ.copy()
    env.update(RR_NATIVE_DATA=str(data), RR_NATIVE_TERRAIN_MOTION=str(out / 'native.json'), RR_TERRAIN_MOTION_OUTPUT=str(out), CARGO_TARGET_DIR=str(Path('../target').resolve()))
    tested = subprocess.run(['cargo', 'test', '--locked', '-p', 'rr-simulation', 'source_terrain_motion_native', '--', '--ignored'], env=env, check=False)
    comparison = json.loads((out / 'rust-comparison.json').read_text())
    trace = json.loads(native)
    cadence = dict(observable_boundary_responses=0, continuous_response_episodes=0, continuous_response_across_samples=0,
                   separation_then_reimpact_episodes=0, held_away_then_back_reimpact_episodes=0, stamina_or_condition_damage_ticks=0)
    for case in trace['cases']:
        fixture = trace['fixtures'][case['fixture']]
        run = maximum = 0
        seen = separated = reimpact = crosses = False
        previous = None
        for row in case['rows']:
            response = bool(row[26])
            run = run + 1 if response else 0
            maximum = max(maximum, run)
            if seen and not response:
                separated = True
            if response and separated:
                reimpact = True
            seen |= response
            crosses |= response and previous is not None and bool(previous[26]) and row[2] // 256 != previous[2] // 256
            previous = row
            cadence['observable_boundary_responses'] += response
            cadence['stamina_or_condition_damage_ticks'] += row[10] != 150 or row[16] != 2500 or row[17] != 2500
        cadence['continuous_response_episodes'] += maximum >= 2
        cadence['continuous_response_across_samples'] += crosses
        cadence['separation_then_reimpact_episodes'] += reimpact
        cadence['held_away_then_back_reimpact_episodes'] += reimpact and fixture['mode'] == 5
    comparison['native_cadence'] = cadence
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
