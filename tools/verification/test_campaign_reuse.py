"""Synthetic fail-closed evidence-reuse checks; never substitute for an engine audit."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import campaign_reuse as reuse


class ReuseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source, self.actual = [self.root / n for n in ('source', 'actual')]
        self.data, self.newdata = [self.root / n for n in ('data', 'newdata')]
        for path in (self.source, self.actual, self.data, self.newdata):
            path.mkdir()
        self.binary = self.root / 'binary'
        self.binary.write_bytes(b'fixture companion')
        inventory = []
        for i in range(341):
            name = 'ROADRASH.EXE' if i == 0 else f'file-{i:03}'
            raw = str(i).encode()
            for directory in (self.data, self.newdata):
                (directory / name).write_bytes(raw)
            inventory.append({'path': name, 'size': len(raw),
                              'sha256': hashlib.sha256(raw).hexdigest()})
        for name in reuse.DEPENDENCIES:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('fixture dependency')
        (self.root / reuse.DEPENDENCIES[2]).write_text(json.dumps({'files': inventory}))
        dependencies = {name: reuse.sha(self.root / name) for name in reuse.DEPENDENCIES}
        rows, audit_rows, saves = [], [], []
        for i in range(31):
            replay = f'race-{i:03}.json'
            raw = json.dumps({'ordinary_input_fixture': i}).encode()
            save = f'career-state-{i:03}.json'
            for directory in (self.source, self.actual):
                (directory / replay).write_bytes(raw)
                (directory / save).write_text('fixture save ' + str(i))
            rows.append({'replay': replay, 'wall_ms': i + 1, 'money': 1000 + i})
            audit_rows.append({'replay_sha256': reuse.sha(self.source / replay)})
            saves.append(reuse.sha(self.source / save))
        self.record = {'completed': True, 'races': rows, 'career': {'cash': 1030}}
        for directory in (self.source, self.actual):
            (directory / 'campaign.json').write_text(json.dumps(self.record, indent=2))
            (directory / 'career.json').write_bytes(b'final fixture save')
        self.audit = {'outcome': 'passed', 'races': 31, 'replay_runs': 124,
                      'course_level_cases': 25, 'frame_schedules': reuse.SCHEDULES,
                      'native_only': True, 'real_controls_only': True,
                      'authority_profile': reuse.AUTHORITY,
                      'source_executable_sha256': reuse.sha(self.data / 'ROADRASH.EXE'),
                      'binary_sha256': reuse.sha(self.binary),
                      'verifier_sha256': dependencies[reuse.DEPENDENCIES[0]],
                      'audit_flags': dict.fromkeys(('save_schema_2_sha256', 'shop_transactions',
                                                   'settlement_finances', 'terminal_outcomes',
                                                   'native_finish_place', 'full_checkpoint_arrays',
                                                   'complete_campaign_required'), True),
                      'rows': audit_rows}
        self.auditpath = self.root / 'audit.json'
        self.auditpath.write_text(json.dumps(self.audit))
        self.reference = self.root / 'reference.json'
        self.trusted = {'outcome': 'passed', 'audit_exit_code': 0, 'audit': self.audit,
                        'audit_receipt_sha256': reuse.sha(self.auditpath),
                        'audit_dependencies': dependencies,
                        'replay_binary_sha256': self.audit['binary_sha256'],
                        'verifier_sha256': self.audit['verifier_sha256'],
                        'campaign_sha256': reuse.sha(self.source / 'campaign.json'),
                        'save_file_sha256': saves,
                        'final_save_sha256': reuse.sha(self.source / 'career.json')}
        self.trusted['source_fixture'] = {
            'source_executable_sha256': self.audit['source_executable_sha256']}
        self.reference.write_text(json.dumps(self.trusted))
        self.pin = reuse.sha(self.reference)

    def proof(self):
        return reuse.prove(self.reference, self.pin, self.auditpath,
                           self.source / 'campaign.json', self.actual / 'campaign.json',
                           self.data, self.newdata, self.binary, self.root)

    def test_only_wall_values_may_differ_and_no_second_execution_claim(self):
        changed = copy.deepcopy(self.record)
        for row in changed['races']:
            row['wall_ms'] += 900
        (self.actual / 'campaign.json').write_text(json.dumps(changed, indent=2))
        proof = self.proof()
        self.assertEqual(proof['new_independent_replay_executions'], 0)
        self.assertEqual(proof['reused_independent_replay_executions'], 124)
        self.assertEqual(proof['raw_source_files_verified'], 341)

    def test_campaign_authority_or_even_nonwall_encoding_change_rejected(self):
        path = self.actual / 'campaign.json'
        original = path.read_bytes()
        for raw in (original.replace(b'1000', b'1001'), original.replace(b'1000', b'1000.0'),
                    original.replace(b'"completed": true', b'"completed": true, "completed": true')):
            path.write_bytes(raw)
            with self.assertRaises((ValueError, KeyError)):
                self.proof()
        path.write_bytes(original)

    def test_every_replay_save_and_final_save_is_checked(self):
        for name in ('race-030.json', 'career-state-030.json', 'career.json'):
            path = self.actual / name
            original = path.read_bytes()
            path.write_bytes(original + b' ')
            with self.assertRaises(ValueError):
                self.proof()
            path.write_bytes(original)

    def test_dependency_and_binary_changes_rejected(self):
        for path in [self.binary] + [self.root / name for name in reuse.DEPENDENCIES]:
            original = path.read_bytes()
            path.write_bytes(original + b' ')
            with self.assertRaises(ValueError):
                self.proof()
            path.write_bytes(original)

    def test_reference_and_audit_must_remain_pinned(self):
        for path in (self.reference, self.auditpath, self.source / 'campaign.json'):
            original = path.read_bytes()
            path.write_bytes(original + b' ')
            with self.assertRaises(ValueError):
                self.proof()
            path.write_bytes(original)

    def test_raw_source_change_and_symlink_rejected(self):
        path = self.newdata / 'file-340'
        original = path.read_bytes()
        path.write_bytes(b'wrong')
        with self.assertRaises(ValueError):
            self.proof()
        path.unlink()
        path.symlink_to(self.data / 'file-340')
        with self.assertRaises(ValueError):
            self.proof()
        path.unlink()
        path.write_bytes(original)

    def test_reusing_same_output_or_raw_import_is_rejected(self):
        with self.assertRaises(ValueError):
            reuse.prove(self.reference, self.pin, self.auditpath,
                        self.source / 'campaign.json', self.source / 'campaign.json',
                        self.data, self.newdata, self.binary, self.root)
        with self.assertRaises(ValueError):
            reuse.prove(self.reference, self.pin, self.auditpath,
                        self.source / 'campaign.json', self.actual / 'campaign.json',
                        self.data, self.data, self.binary, self.root)

    def test_partial_or_wrong_profile_cannot_become_reuse(self):
        for key, value in [('outcome', 'partial_audit_passed'), ('replay_runs', 120),
                           ('authority_profile', dict(reuse.AUTHORITY, police=False))]:
            changed = copy.deepcopy(self.audit)
            changed[key] = value
            self.auditpath.write_text(json.dumps(changed))
            trusted = copy.deepcopy(self.trusted)
            trusted['audit'] = changed
            trusted['audit_receipt_sha256'] = reuse.sha(self.auditpath)
            self.reference.write_text(json.dumps(trusted))
            self.pin = reuse.sha(self.reference)
            with self.assertRaises(ValueError):
                self.proof()


if __name__ == '__main__':
    unittest.main()
