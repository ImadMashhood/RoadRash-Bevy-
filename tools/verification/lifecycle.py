#!/usr/bin/env python3
"""Private rendered start/quit/unload fixture; separate from real-race completion."""
import argparse
import json
from pathlib import Path
import subprocess


def run(a):
    a.output.mkdir(parents=True, exist_ok=False)
    events = []
    for i in range(10):
        base = i * 180
        for frame, action in [(1, {'begin': 'Thrash'}), (2, {'character': i % 8 + 1}),
                              (3, {'course': i % 5 + 1}), (60, {'open': 'Paused'}),
                              (61, 'quit_race'), (62, 'continue'), (63, 'main')]:
            events.append(dict(frame=base + frame, action=action))
    script = a.output / 'script.json'
    script.write_text(json.dumps(events))
    with (a.output / 'run.log').open('w') as log:
        subprocess.run([str(a.binary), '--install-root', str(a.install), '--catalog', str(a.catalog),
                        '--frontend', 'on', '--audio', 'off', '--videos', 'off',
                        '--profile-root', str(a.output / 'profile'), '--ui-script', str(script),
                        '--performance-report', str(a.output / 'report.json'), '--smoke-frames', '1800'],
                       check=True, stdout=log, stderr=subprocess.STDOUT)
    report = json.loads((a.output / 'report.json').read_text())
    samples = report['memory_samples']
    assert len(samples) >= 6, 'Insufficient lifetime observations'
    # Fonts/renderer may populate caches initially. Compare the retained resource
    # ceilings after both five-course passes; RSS is not an allocation counter.
    count = lambda key: max(s[key] for s in samples)
    result = dict(outcome='passed', cycles=10, courses=5, character_selections=8,
                  scenario='Begin/real native factory/countdown/pause/player quit/results/Main; no synthetic finish.',
                  max_entity_index_slots=count('entities'), max_cpu_images=count('cpu_images'),
                  max_gpu_images=count('gpu_images'), sample_count=len(samples),
                  binary_sha256=report['binary_sha256'], samples=samples,
                  limitation='The schema-2 entities field is Entities.len: allocated index slots, not spawned entities (Bevy 0.20). This isolates scene/resource lifetime, not race victory or CPU frame-time acceptance. Telemetry retains bounded observations.')
    warm_slots = [s['entities'] for s in samples[len(samples)//2:]]
    result['warmed_entity_index_slots'] = sorted(set(warm_slots))
    assert max(warm_slots) == min(warm_slots), 'Entity index storage grows after the full warmup pass'
    assert result['max_cpu_images'] <= 16 and result['max_gpu_images'] <= 16
    (a.output / 'receipt.json').write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    for name in ['binary', 'install', 'catalog', 'output']:
        p.add_argument('--' + name, type=lambda v: Path(v).resolve(), required=True)
    run(p.parse_args())
