#!/usr/bin/env python3
"""Local, native-only repeat capture. Keep the output directory private."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from compare import compare, image_regions


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(a):
    a.output.mkdir(parents=True, exist_ok=False)
    replay = json.loads(a.replay.read_text())
    replay['ticks'] = min(900, replay['ticks'])
    replay['expected_hash'] = None
    replay['commands'] = [c for c in replay['commands'] if c['tick'] < replay['ticks']]
    private = a.output / 'setup.json'
    private.write_text(json.dumps(replay))
    initial = dict(content_sha256=sha(a.data / 'ROADRASH.EXE'),
                   course=a.course, level=a.level, bike=a.bike, character=a.character,
                   seed=replay['seed'], inputs_sha256=sha(private),
                   simulation_schema=replay['schema_version'],
                   camera={'size': [640, 480], 'framing': 'classic'}, native_only=True)
    traces = []
    for rate in ['30', '144']:
        path = a.output / f'trace-{rate}.json'
        subprocess.run([str(a.target / 'rr-replay'), str(private), rate, str(path), '--every-tick'], check=True)
        report = json.loads(path.read_text())
        traces.append(dict(engine_sha256=sha(a.target / 'rr-replay'), initial=initial,
            ticks=report['tick_hashes'],
            rng=[[r['tick'], r['rng']] for r in report['forensic']],
            scheduler=[[r['tick'], r['scheduler_sha256']] for r in report['forensic']],
            motion=[[r['tick'], r['motion_sha256']] for r in report['forensic']]))
    state = compare(*traces)
    assert state['outcome'] == 'equal'
    for i in range(2):
        command = [str(a.target / 'rr-game'), '--install-root', str(a.install),
                   '--catalog', str(a.catalog), '--course', a.course, '--level', str(a.level),
                   '--bike', str(a.bike), '--replay', str(private), '--render-hz', '60',
                   '--frontend', 'off', '--audio', 'off', '--videos', 'off', '--window-size', '640x480',
                   '--capture', str(a.output / f'capture-{i}.png'), '--capture-frame', '300']
        with (a.output / f'capture-{i}.log').open('w') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    images = image_regions(a.output / 'capture-0.png', a.output / 'capture-1.png')
    assert images['passed']
    receipt = dict(outcome='passed', initial=initial, simulation_build_sha256=sha(a.target / 'rr-replay'),
                   game_build_sha256=sha(a.target / 'rr-game'), ticks=replay['ticks'],
                   trace=state, repeated_image=images, source_setup_sha256=sha(private))
    (a.output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    for name in ['replay', 'output', 'target', 'data', 'install', 'catalog']:
        p.add_argument('--' + name, type=lambda v: Path(v).resolve(), required=True)
    p.add_argument('--course', default='CITY')
    p.add_argument('--level', type=int, default=1)
    p.add_argument('--bike', type=int, default=1)
    p.add_argument('--character', type=int, default=3)
    run(p.parse_args())
