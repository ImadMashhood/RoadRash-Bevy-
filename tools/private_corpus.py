#!/usr/bin/env python3
"""Explicit private check. Print SKIP when no user-supplied ISO is configured."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--require', action='store_true')
parser.add_argument('--report', type=Path, default=Path('local/private-corpus.json'))
args = parser.parse_args()
source = os.environ.get('RR_TEST_ISO')
if not source:
    print('SKIP private-corpus: RR_TEST_ISO is not configured; synthetic tests are separate')
    sys.exit(2 if args.require else 0)
args.report.parent.mkdir(parents=True, exist_ok=True)
result = subprocess.run([sys.executable, str(Path(__file__).with_name('verify_disc.py')), source, '--report', str(args.report)])
if result.returncode == 0:
    report = json.loads(args.report.read_text())
    print(f"PASS private-corpus: {report['matched_files']}/{report['required_files']} required files")
sys.exit(result.returncode)
