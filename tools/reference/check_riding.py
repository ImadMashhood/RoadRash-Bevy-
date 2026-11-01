#!/usr/bin/env python3
"""Compare the Rust source-handling/animation APIs with the pinned native port."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

REVISION = '7aecf6044f2dd773b925b325823f30d648d61c89'


def run(data, output, rust, source, native):
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != REVISION:
        raise RuntimeError('Native reference is not the pinned revision')
    data_path = data.resolve()
    output_path = output.resolve()
    if data_path == output_path or data_path in output_path.parents or output_path in data_path.parents:
        raise ValueError('Output overlaps immutable source data')
    output.mkdir(parents=True, exist_ok=True)
    checks = []
    for family in ['handling', 'animation', 'vertical', 'palettes', 'road-motion']:
        binary = output / f'native-{family}'
        subprocess.run([os.environ.get('CXX', 'g++'), '-O2', '-std=c++20', '-Wall', '-Wextra', '-Werror', '-I', str(source / 'src'), str(Path('tools/reference') / f'{family}.cpp'), str(native / 'librr-resource.a'), str(native / 'jpeg-scalar/build/libjpeg.a'), str(native / 'freetype/build/libfreetype.a'), '-lSDL3', '-pthread', '-o', str(binary)], check=True)
        reference = output / f'{family}-native.json'
        actual = output / f'{family}-rust.json'
        with reference.open('w') as stream:
            subprocess.run([str(binary.resolve()), str(data.resolve())], stdout=stream, check=True)
        subprocess.run([str((rust / f'rr-{family}').resolve()), str(data.resolve()), str(actual.resolve())], check=True)
        expected = json.loads(reference.read_text())
        result = json.loads(actual.read_text())
        if len(expected) != len(result):
            raise AssertionError(f'{family} row count: {len(expected)} != {len(result)}')
        for i, (left, right) in enumerate(zip(expected, result)):
            if left != right:
                raise AssertionError(f'{family}, row {i}: {left} != {right}')
        checks.append({'family': family, 'outcome': 'passed', 'rows': len(result), 'native_sha256': hashlib.sha256(reference.read_bytes()).hexdigest(), 'rust_sha256': hashlib.sha256(actual.read_bytes()).hexdigest(), 'angle_cases': len(result) * 9 if family == 'animation' else 0})
    receipt = {'outcome': 'passed', 'reference_revision': revision, 'physical_original_game_compared': False, 'specifications': 15, 'checks': checks, 'scope': 'source-port APIs, imported specs/tables, synthetic input schedules and vertical impulses; no original executable parity claim'}
    (output / 'comparison.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--rust', type=Path, default=Path('target/release'))
    parser.add_argument('--source', type=Path, default=Path('local/reference-cpp'))
    parser.add_argument('--native', type=Path, default=Path('local/reference-build'))
    args = parser.parse_args()
    run(args.data, args.output, args.rust, args.source, args.native)


if __name__ == '__main__':
    main()
