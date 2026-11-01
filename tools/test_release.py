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
