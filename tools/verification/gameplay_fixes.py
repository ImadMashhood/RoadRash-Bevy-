#!/usr/bin/env python3
"""Private native building-contact probe; source art and setups stay in OUTPUT.

This deliberately positions a native bike near a wall and removes other actors
so a contact can be attributed. It is not a career or performance acceptance.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess


def run(args, log):
    with log.open('w') as stream:
        subprocess.run([str(v) for v in args], stdout=stream,
                       stderr=subprocess.STDOUT, check=True)


def verify(a):
    for protected in [a.install, a.catalog]:
        if a.output == protected or protected in a.output.parents or a.output in protected.parents:
            raise ValueError('Probe output overlaps immutable imported content')
    a.output.mkdir(parents=True, exist_ok=False)
    base = [a.game, '--install-root', a.install, '--catalog', a.catalog,
            '--course', 'CITY', '--frontend', 'off', '--audio', 'off',
            '--videos', 'off', '--window-size', '640x480']
    fresh = a.output / 'fresh.json'
    run(base + ['--benchmark-pilot', 'on', '--benchmark-seconds', '12',
                '--smoke-frames', '1000000', '--record-replay', fresh,
                '--performance-report', a.output / 'smoke.json',
                '--capture', a.output / 'race.png', '--capture-frame', '200',
                '--profile-root', a.output / 'profile'], a.output / 'fresh.log')
    record = json.loads(fresh.read_text())
    encounter = record['encounter']
    obstacles = encounter['rules']['obstacles']
    buildings = [o for o in obstacles if o['kind'] == 'building']
    assert len(buildings) > 1000, 'Native factory did not import building volumes'
    assert record['population']['tactics_version'] == 1
    assert encounter['rules']['combat_version'] == 3
    assert encounter['rules']['collision_version'] == 2
    assert encounter['rules']['body']['motion_version'] == 1
    assert encounter['rules']['body']['recovery_version'] == 1
    assert record['handling']['motion_version'] == 2
    contact = copy.deepcopy(record)
    e, p = contact['encounter'], contact['population']
    start = contact['handling']['start_sample'] * 256
    wall = next(o for o in buildings if o['right'] < 0
                and start <= o['progress'] < start + 256
                and e['topology']['paths'][str(o['path'])]['selected_start'] == -start)
    width, length, _ = e['rules']['dimensions']
    seed = e['seeds'][0]
    seed.update(distance=wall['progress'] - start, speed=12000,
                lateral=wall['right'] + width + 1024)
    seed['location'] = dict(path=wall['path'], progress=wall['progress'],
                            lateral=seed['lateral'], height=0, heading=0)
    e['seeds'] = [seed]
    e['inventory']['items'] = {}
    p.update(brains={}, zones=[], spawns=[], traffic=False, periodic_police=False)
    contact.update(ticks=45, expected_hash=None,
                   commands=[dict(tick=0, actor=1, sequence=0, input=dict(
                       throttle=True, brake=False, steer=0, steer_q15=-32767, fork=None))])
    path = a.output / 'wall.json'
    path.write_text(json.dumps(contact))
    reference = None
    rates = []
    for rate in ['30', '60', '144', 'irregular']:
        report = a.output / f'wall-{rate}.json'
        run([a.replay, path, rate, report, '--every-tick'], a.output / f'wall-{rate}.log')
        result = json.loads(report.read_text())
        if reference is None:
            reference = result
        assert result['tick_hashes'] == reference['tick_hashes'], 'Replay divergence'
        actor = result['final_snapshot']['actors'][0]
        unit, bike = actor['encounter'], actor['rider']['bike']
        assert unit['fighter']['hit_count'] == 0, 'A boundary brush invented condition/stamina damage'
        assert unit['recovery']['phase'] == 'mounted' and bike['speed'] > 0, 'Side brush stopped forward motion'
        at = unit['recovery']['bike_location']
        boundaries=e['rules']['building_boundaries']
        boundary=next(b for b in boundaries if b['path']==at['path'] and b['sample']==at['progress']//256)
        if boundary['building'] and not boundary['flags'] & 0x38:
            widen=(((at['progress']&255)*250//3)>>8)
            left=boundary['outer'][0]-(widen if boundary['selectors'][0]==255 else 0)
            right=boundary['outer'][1]+(widen if boundary['selectors'][1]==255 else 0)
            assert bike['lateral']-width >= left*256 and bike['lateral']+width <= right*256, 'Bike crossed an active native boundary'
        rates.append(rate)
    run(base + ['--replay', path, '--render-hz', '60', '--capture-frame', '22',
                '--capture', a.output / 'wall.png', '--report', a.output / 'rendered.json'],
        a.output / 'rendered.log')
    rendered = json.loads((a.output / 'rendered.json').read_text())
    assert rendered['state_sha256'] == reference['state_sha256']
    receipt = dict(outcome='passed', building_volumes=len(buildings),
                   static_scenery_volumes=sum(o['kind'] == 'solid' for o in obstacles),
                   source_factory_tactics_version=1, combat_version=3, collision_version=2,
                   motion_version=2, recovery_version=1, native_contact_ticks=45,
                   wall_incidents=0, final_speed=bike['speed'], bike_inside_native_boundary=True,
                   building_boundaries=len(e['rules']['building_boundaries']),
                   source_physical_scenery_boxes=sum(o.get('source_box') is not None for o in obstacles),
                   schedules=rates, rendered_hash_equal=True,
                   state_sha256=reference['state_sha256'],
                   binary_sha256=hashlib.sha256(a.game.read_bytes()).hexdigest(),
                   scope='Controlled native boundary brush and isolated actors; forward speed preserved, no invented damage or terminal injection. Twelve-second full-field smoke is not career/performance acceptance.')
    (a.output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['game', 'replay', 'install', 'catalog', 'output']:
        parser.add_argument('--' + name, type=lambda v: Path(v).resolve(), required=True)
    verify(parser.parse_args())
