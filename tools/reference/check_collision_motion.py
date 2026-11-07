#!/usr/bin/env python3
"""Compare actual native mounted contact motion, ordinary and validated initial-speed extremes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_encounters import compare_cases, REVISION


def motion_evidence(rows):
    from collections import defaultdict
    from fractions import Fraction
    cases = defaultdict(list)
    for row in rows:
        cases[row[0]].append(row)
    controls = {(r[1], r[2], r[3], r[5], r[16]): r for r in rows if r[4] == 0}
    diagonal_cases = 0
    unchanged_diagonal_ticks = 0
    source_warmed_diagonal_cases = 0
    source_warmed_exact_near_misses = 0
    full_crossing_cases = 0
    source_warmed_crossings = 0
    max_step = {0: 0, 30000: 0, 1000000: 0}
    max_lateral = {0: 0, 30000: 0, 1000000: 0}
    for sequence in cases.values():
        proven_miss = False
        full_crossing = False
        previous = (12800, 0)
        unchanged = True
        for r in sequence:
            ax, ay = previous
            bx, by = r[6:8]
            speed = r[16]
            max_step[speed] = max(max_step[speed], bx - ax)
            max_lateral[speed] = max(max_lateral[speed], abs(by - ay))
            previous = (bx, by)
            if r[4] == 1 and ax < 15424 and bx > 15552:
                full_crossing = True
            if r[4] == 3:
                low_y, high_y = r[17] * 256 - 8704, r[17] * 256 + 8704
                bbox = min(ax, bx) <= 15552 and max(ax, bx) >= 15424 and min(ay, by) <= high_y and max(ay, by) >= low_y
                entry, leave = Fraction(0), Fraction(1)
                for a, b, low, high in [(ax, bx, 15424, 15552), (ay, by, low_y, high_y)]:
                    if a == b:
                        if not low <= a <= high:
                            leave = Fraction(-1)
                    else:
                        near, far = sorted((Fraction(low - a, b - a), Fraction(high - a, b - a)))
                        entry, leave = max(entry, near), min(leave, far)
                proven_miss |= bbox and entry > leave
                unchanged &= r[6:16] == controls[(r[1], r[2], r[3], r[5], r[16])][6:16]
        if sequence[0][4] == 3:
            if not unchanged or (sequence[0][16] != 0 and not proven_miss):
                raise ValueError('Diagonal fixture must match its obstacle-free native control; injected fixture must prove a swept-box false positive')
            diagonal_cases += proven_miss
            source_warmed_diagonal_cases += sequence[0][16] == 0
            source_warmed_exact_near_misses += proven_miss and sequence[0][16] == 0
            unchanged_diagonal_ticks += len(sequence)
        full_crossing_cases += full_crossing
        source_warmed_crossings += full_crossing and sequence[0][16] == 0
    return dict(trajectories=len(cases), source_warmup_ticks=3000, source_warmup_marker=0, moderate_injected_initial_speed=30000, injected_validated_initial_speed=1000000,
                maximum_forward_step_q8=max_step, maximum_lateral_step_q8=max_lateral,
                exact_diagonal_near_miss_trajectories=diagonal_cases, source_warmed_diagonal_trajectories=source_warmed_diagonal_cases, source_warmed_swept_box_false_positives=source_warmed_exact_near_misses, unchanged_diagonal_ticks=unchanged_diagonal_ticks,
                full_narrow_obstacle_crossing_trajectories=full_crossing_cases, source_warmed_full_narrow_obstacle_crossings=source_warmed_crossings,
                crossing_scope='Marker0 begins from actual native3000tick throttle-from-rest state (then translated to fixture coordinates);30k and1m are injected initial speeds. The source detector remains endpoint-only.')

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
    binary = out / 'native-collision-motion'
    subprocess.run([os.environ.get('CXX', 'g++'), '-O2', '-std=c++20', '-Wall', '-Wextra', '-Werror', '-I', str(include), '-I', str(src / 'src'), 'tools/reference/collision_motion.cpp', str(build/'librr-resource.a'), str(build/'jpeg-scalar/build/libjpeg.a'), str(build/'freetype/build/libfreetype.a'), '-lSDL3', '-pthread', '-o', str(binary)], check=True)
    native = subprocess.check_output([str(binary), str(data)])
    envelope_binary = out / 'native-speed-envelope'
    subprocess.run([os.environ.get('CXX', 'g++'), '-O2', '-std=c++20', '-Wall', '-Wextra', '-Werror', '-I', str(include), '-I', str(src / 'src'), 'tools/reference/collision_speed_envelope.cpp', str(build/'librr-resource.a'), str(build/'jpeg-scalar/build/libjpeg.a'), str(build/'freetype/build/libfreetype.a'), '-lSDL3', '-pthread', '-o', str(envelope_binary)], check=True)
    envelope = subprocess.check_output([str(envelope_binary), str(data)])
    (out/'native-speed-envelope.json').write_bytes(envelope)
    (out/'native.json').write_bytes(native)
    env = os.environ.copy()
    env.update(RR_NATIVE_DATA=str(data), RR_NATIVE_COLLISION_MOTION=str(out/'native.json'), RR_NATIVE_OUTPUT=str(out), CARGO_TARGET_DIR=str(Path('../target').resolve()))
    subprocess.run(['cargo', 'test', '--locked', '-p', 'rr-simulation', 'native_cases::source_contact_motion_native', '--', '--ignored'], env=env, check=True)
    actual = json.loads((out/'collision-motion-rust.json').read_text())
    result = compare_cases('mounted_contact_motion', json.loads(native), actual)
    metrics = motion_evidence(json.loads(native))
    envelope_rows = json.loads(envelope)
    metrics['native_reachable_flat_envelope'] = dict(specifications=15, throttle_and_nitro_trajectories=len(envelope_rows), ticks_per_trajectory=12000, initial_speed=0, maximum_speed=max(r[2] for r in envelope_rows), maximum_forward_step_q8=max(r[3] for r in envelope_rows), nitro_active_ticks=sum(r[5] for r in envelope_rows), trace_sha256=hashlib.sha256(envelope).hexdigest(), scope='Actual native reachability measurement only; not additional differential observation count. Flat road, no steering, source specifications and charge limits; not a bound on every authored course trajectory.')
    receipt = dict(outcome=result['outcome'], reference_revision=revision, reference_dirty_paths=dirty, native_library_sha256=hashlib.sha256((build/'librr-resource.a').read_bytes()).hexdigest(), native_trace_sha256=hashlib.sha256(native).hexdigest(), executable_sha256=hashlib.sha256((data/'ROADRASH.EXE').read_bytes()).hexdigest(), checks=[result], motion_evidence=metrics, original_windows_runtime_compared=False, rust_legacy_setup=os.environ.get('RR_MOTION_LEGACY')=='1')
    (out/'comparison.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps(receipt))
    if receipt['outcome'] != 'passed':
        raise SystemExit(1)

if __name__ == '__main__':
    main()
