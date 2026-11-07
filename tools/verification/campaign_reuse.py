#!/usr/bin/env python3
"""Fail-closed proof that a fresh campaign reuses exactly audited input bytes.

This never invokes an engine or claims another independent replay execution.
The caller must pin an already accepted public source receipt SHA256.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re


DEPENDENCIES = ('tools/verification/campaign.py', 'tools/research_baseline.py',
                'docs/baseline/required-files.json',
                'docs/baseline/executable/selectable-actors.json')
SCHEDULES = ['30', '60', '144', 'irregular']
AUTHORITY = {'combat': 4, 'recovery': 2, 'contact_walk': 2,
             'opponents': 14, 'police': True, 'recovery_assists': False}


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def strict_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'Duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs)


def mask_wall(raw):
    record = strict_json(raw)
    require(len(record['races']) == 31, 'Expected exactly 31 audited attempts')
    for row in record['races']:
        require(type(row['wall_ms']) is int and row['wall_ms'] >= 0, 'Invalid wall time')
    masked, count = re.subn(rb'(?m)^([ \t]*"wall_ms":[ \t]*)[0-9]+([ \t]*,?[ \t]*)$',
                            rb'\g<1>0\g<2>', raw)
    require(count == 31, 'Unexpected wall-time encoding')
    return masked


def prove(reference, reference_sha, source_audit, source_campaign, actual_campaign,
          source_data, actual_data, binary, repo):
    checker_sha = sha(Path(__file__))
    require(sha(reference) == reference_sha, 'Unpinned source receipt')
    trusted = strict_json(reference.read_bytes())
    audit = strict_json(source_audit.read_bytes())
    require(sha(source_audit) == trusted['audit_receipt_sha256']
            and audit == trusted['audit'], 'Source audit receipt differs')
    require(trusted['outcome'] == audit['outcome'] == 'passed'
            and trusted['audit_exit_code'] == 0, 'Source audit did not pass')
    require((audit['races'], audit['replay_runs'], audit['course_level_cases']) == (31, 124, 25)
            and audit['frame_schedules'] == SCHEDULES, 'Incomplete source audit')
    require(audit['native_only'] is True and audit['real_controls_only'] is True,
            'Wrong source audit scope')
    require(audit['authority_profile'] == AUTHORITY, 'Wrong source interaction profile')
    for flag in ('save_schema_2_sha256', 'shop_transactions', 'settlement_finances',
                 'terminal_outcomes', 'native_finish_place', 'full_checkpoint_arrays',
                 'complete_campaign_required'):
        require(audit['audit_flags'].get(flag) is True, 'Missing audit gate: ' + flag)
    require(set(trusted['audit_dependencies']) == set(DEPENDENCIES), 'Missing audit dependency')
    def dependencies():
        for name, digest in trusted['audit_dependencies'].items():
            require(sha(repo / name) == digest, 'Changed dependency: ' + name)
        require(sha(binary) == audit['binary_sha256'] == trusted['replay_binary_sha256'],
                'Changed replay companion')
        require(audit['verifier_sha256'] == trusted['verifier_sha256']
                == trusted['audit_dependencies'][DEPENDENCIES[0]], 'Changed auditor')
    dependencies()
    require(source_campaign.parent.resolve() != actual_campaign.parent.resolve(),
            'Campaign output directory is not fresh')
    source_raw, actual_raw = source_campaign.read_bytes(), actual_campaign.read_bytes()
    require(sha(source_campaign) == trusted['campaign_sha256'], 'Source campaign changed')
    require(mask_wall(source_raw) == mask_wall(actual_raw), 'Campaign authority bytes differ')
    record = strict_json(source_raw)
    require(record['completed'] is True and len(audit['rows']) == 31
            and len(trusted['save_file_sha256']) == 31, 'Incomplete reference corpus')
    hashes = []
    for i, row in enumerate(record['races']):
        replay = f'race-{i:03}.json'
        require(row['replay'] == replay, 'Unexpected replay path')
        source = source_campaign.parent / replay
        actual = actual_campaign.parent / replay
        digest = sha(source)
        require(digest == audit['rows'][i]['replay_sha256'] == sha(actual),
                f'Replay differs at attempt {i + 1}')
        save = f'career-state-{i:03}.json'
        require(sha(source_campaign.parent / save) == trusted['save_file_sha256'][i]
                == sha(actual_campaign.parent / save), f'Save differs at attempt {i + 1}')
        hashes.append(digest)
    require(sha(source_campaign.parent / 'career.json') == trusted['final_save_sha256']
            == sha(actual_campaign.parent / 'career.json'), 'Final save differs')
    manifest_path = repo / DEPENDENCIES[2]
    files = strict_json(manifest_path.read_bytes())['files']
    require(len(files) == 341 and len({f['path'] for f in files}) == 341,
            'Incomplete raw source inventory')
    executable = next(f['sha256'] for f in files if f['path'] == 'ROADRASH.EXE')
    require(executable == audit['source_executable_sha256']
            == trusted['source_fixture']['source_executable_sha256'], 'Changed pricing source')
    require(source_data.resolve() != actual_data.resolve(), 'Raw import is not fresh')
    for item in files:
        name = Path(item['path'])
        require(not name.is_absolute() and '..' not in name.parts, 'Unsafe source path')
        for data in (source_data, actual_data):
            path = data / name
            require(path.is_file() and not path.is_symlink()
                    and path.stat().st_size == item['size'] and sha(path) == item['sha256'],
                    'Raw source file differs: ' + str(name))
    dependencies()
    require(sha(reference) == reference_sha
            and sha(source_audit) == trusted['audit_receipt_sha256']
            and sha(source_campaign) == trusted['campaign_sha256']
            and actual_campaign.read_bytes() == actual_raw, 'Inputs changed during proof')
    require(sha(Path(__file__)) == checker_sha, 'Checker changed during proof')
    return {'outcome': 'passed', 'scope': 'Exact-byte evidence reuse; no new engine execution.',
            'new_independent_replay_executions': 0, 'reused_independent_replay_executions': 124,
            'reused_unique_replay_inputs': 31, 'frame_schedules': SCHEDULES,
            'source_reference_sha256': reference_sha,
            'source_audit_receipt_sha256': trusted['audit_receipt_sha256'],
            'source_campaign_sha256': trusted['campaign_sha256'],
            'actual_campaign_sha256': sha(actual_campaign),
            'replay_sha256': hashes, 'save_file_sha256': trusted['save_file_sha256'],
            'final_save_sha256': trusted['final_save_sha256'], 'raw_source_files_verified': 341,
            'replay_binary_sha256': audit['binary_sha256'],
            'verifier_sha256': audit['verifier_sha256'],
            'audit_dependencies': trusted['audit_dependencies'],
            'checker_sha256': checker_sha}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('reference', 'source_audit', 'source_campaign', 'actual_campaign',
                 'source_data', 'actual_data', 'binary', 'repo', 'output'):
        parser.add_argument('--' + name.replace('_', '-'), type=Path, required=True)
    parser.add_argument('--reference-sha256', required=True)
    args = vars(parser.parse_args())
    output = args.pop('output')
    args['reference_sha'] = args.pop('reference_sha256')
    proof = prove(**args)
    with output.open('x') as handle:
        handle.write(json.dumps(proof, indent=2) + '\n')
