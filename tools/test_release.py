"""Authored package/launcher error-path checks; no original game data."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile
import os
from unittest.mock import patch

def module(name):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).parent/'release'/f'{name}.py')
    value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value

launcher=module('launcher')
class Launcher(unittest.TestCase):
    def test_archive_bytes_ignore_file_timestamps_and_owner_metadata(self):
        with patch.dict('sys.modules', {'notices': module('notices')}):
            package = module('package')
        with tempfile.TemporaryDirectory() as temp:
            d = Path(temp)
            folder = d / 'application'
            folder.mkdir()
            (folder / 'authored.txt').write_text('authored application fixture\n')
            package.archive_tree(folder, d / 'one.tar.gz', 123456789)
            os.utime(folder / 'authored.txt', (987654321, 987654321))
            package.archive_tree(folder, d / 'two.tar.gz', 123456789)
            self.assertEqual((d / 'one.tar.gz').read_bytes(), (d / 'two.tar.gz').read_bytes())

    def test_candidate_request_rejects_shell_text_acceptance_and_existing_tags(self):
        from types import SimpleNamespace
        candidate = module('candidate_request')
        valid = dict(source='a' * 40, tag='native-functional-2026-11-01-rc2',
                     status='review-only', g5_accepted=False)
        for field, bad in [('source', '$(touch /tmp/not-allowed)'), ('tag', 'main'),
                           ('status', 'accepted'), ('g5_accepted', True)]:
            with patch.object(candidate.subprocess, 'run', side_effect=AssertionError('unexpected git')):
                with self.assertRaises(ValueError): candidate.validate({**valid, field: bad}, 'head')
        with patch.object(candidate.subprocess, 'run', side_effect=[SimpleNamespace(returncode=0), SimpleNamespace(returncode=1)]):
            self.assertEqual(candidate.validate(valid, 'head'), (valid['source'], valid['tag']))
        with patch.object(candidate.subprocess, 'run', return_value=SimpleNamespace(returncode=0)):
            with self.assertRaisesRegex(ValueError, 'already exists'): candidate.validate(valid, 'head')

    def test_zip_never_uses_archive_paths_and_rejects_ambiguous_discs(self):
        with tempfile.TemporaryDirectory() as temp:
            d=Path(temp); z=d/'disc.zip'; out=d/'out';out.mkdir()
            with zipfile.ZipFile(z,'w') as f:f.writestr('../../outside.iso',b'authored fake disc')
            p=launcher.source_iso(z,out)
            self.assertEqual(p,out/'source.iso')
            self.assertEqual(p.read_bytes(),b'authored fake disc')
            self.assertFalse((d/'outside.iso').exists())
            with zipfile.ZipFile(z,'w') as f:
                f.writestr('one.iso',b'x');f.writestr('two.iso',b'y')
            with self.assertRaises(ValueError):launcher.source_iso(z,out)
    def test_missing_disc_headless_reports_action_without_opening_a_dialog(self):
        with tempfile.TemporaryDirectory() as temp:
            d=Path(temp)
            with patch.object(launcher,'dialog',side_effect=AssertionError('unexpected dialog')):
                with self.assertRaisesRegex(ValueError,'--source'):
                    launcher.launch(d,d/'profile',graphical=False)
    def test_user_data_respects_absolute_xdg_and_rejects_relative_override(self):
        with patch.dict(os.environ,{'XDG_DATA_HOME':'relative'}):
            self.assertEqual(launcher.user_root(),Path.home()/'.local/share/roadrash-rust')
        with patch.dict(os.environ,{'XDG_DATA_HOME':'/tmp/rr-test-user-data'}):
            self.assertEqual(launcher.user_root(),Path('/tmp/rr-test-user-data/roadrash-rust'))
