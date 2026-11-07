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
    assert encounter['rules']['combat_version'] == 4
    assert encounter['rules']['collision_version'] == 2
    assert encounter['rules']['body']['motion_version'] == 1
    assert encounter['rules']['body']['recovery_version'] == 2
    assert record['handling']['motion_version'] == 3
    assert encounter['rules']['contact_walk_version'] == 2
    assert encounter['rules']['source_roads']['version'] == 1
    contact = copy.deepcopy(record)
    e, p = contact['encounter'], contact['population']
    start = contact['handling']['start_sample'] * 256
    width, length, _ = e['rules']['dimensions']
    roads = e['rules']['source_roads']
    profile = roads['profiles'][roads['initial']]
    boundaries = {(b['path'], b['sample']): b for b in e['rules']['building_boundaries']}
    # A brush needs one continuous facade for its complete travel horizon. The
    # former first-sample fixture crossed an opening and hit its end face.
    candidates = []
    horizon = 10
    for path, first in profile['selected'].items():
        path = int(path)
        ground = roads['ground'][str(path)]
        for sample in range(len(ground) - horizon):
            distance = (first + sample) * 256 + 128 - start
            if not 0 <= distance < e['rules']['finish_progress'] - horizon * 256:
                continue
            panel = [boundaries[path, j] for j in range(sample, sample + horizon)]
            if any(not b['building'] or b['flags'] & 0x38 or b['selectors'][0] == 255
                   or b['outer'][0] != panel[0]['outer'][0] for b in panel):
                continue
            identities = profile['identities'][str(path)]
            offset = identities[sample][2]
            if any(row[2] != offset for row in identities[sample:sample + horizon]):
                continue
            slope = max(abs(g['rise']) for g in ground[sample:sample + horizon])
            change = max(abs(ground[j + 1]['rise'] - ground[j]['rise'])
                         for j in range(sample, sample + horizon - 1))
            candidates.append((slope, change, path, sample, distance, offset, panel[0]))
    assert candidates, 'No continuous source facade supports the brush horizon'
    _, _, path_id, sample, distance, offset, boundary = min(candidates, key=lambda c: c[:5])
    seed = e['seeds'][0]
    seed.update(distance=distance, speed=12000,
                lateral=(boundary['outer'][0] + offset) * 256 + width + 1024)
    seed['location'] = dict(path=path_id, progress=sample * 256 + 128,
                            lateral=seed['lateral'], height=0, heading=0)
    e['seeds'] = [seed]
    e['inventory']['items'] = {}
    p.update(brains={}, zones=[], spawns=[], traffic=False, periodic_police=False)
    # One ordinary neutral physics tick establishes native shoulder ground before
    # the 45 held-steering brush ticks; the constructor ground value is zero.
    contact.update(ticks=46, expected_hash=None,
                   commands=[dict(tick=0, actor=1, sequence=0, input=dict(
                       throttle=False, brake=False, steer=0, steer_q15=0, fork=None)),
                       dict(tick=1, actor=1, sequence=1, input=dict(
                       throttle=True, brake=False, steer=0, steer_q15=-32767, fork=None))])
    path = a.output / 'wall.json'
    path.write_text(json.dumps(contact))
    settled = copy.deepcopy(contact)
    settled['ticks'] = 1
    settled['commands'] = settled['commands'][:1]
    settled_path = a.output / 'settled.json'
    settled_path.write_text(json.dumps(settled))
    settled_report = a.output / 'settled-result.json'
    run([a.replay, settled_path, '60', settled_report], a.output / 'settled.log')
    settled_actor = json.loads(settled_report.read_text())['final_snapshot']['actors'][0]
    settled_ground = settled_actor['rider']['bike']['previous_ground']
    assert settled_ground == boundary['surface']['side_height'][0], 'Neutral initialization did not establish native shoulder ground'
    assert settled_actor['encounter']['recovery']['phase'] == 'mounted'
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
        assert unit['recovery']['incident'] == 0, 'Brush crashed and recovered before the final snapshot'
        at = unit['recovery']['bike_location']
        assert at['path'] == path_id and sample <= at['progress'] // 256 < sample + horizon, 'Brush left its verified continuous facade'
        assert at['progress'] > seed['location']['progress'], 'Brush did not advance'
        assert bike['previous_ground'] == boundary['surface']['side_height'][0], 'Brush has incorrect native shoulder ground'
        assert bike['lateral'] - width == (boundary['outer'][0] + offset) * 256, 'Fixture never reached the continuous facade'
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
                   source_factory_tactics_version=1, combat_version=4, collision_version=2,
                   motion_version=3, contact_walk_version=2, source_roads_version=1,
                   recovery_version=2, native_contact_ticks=45,
                   settling_ticks=1, settled_native_ground=settled_ground, brush_path=path_id, brush_start_sample=sample,
                   continuous_facade_samples=horizon, native_shoulder_ground=bike['previous_ground'],
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
