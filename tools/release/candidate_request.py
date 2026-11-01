#!/usr/bin/env python3
"""Validate an explicit candidate request before selecting source or publishing a tag."""
import json
import os
from pathlib import Path
import re
import subprocess


def validate(request, head):
    source, tag = request['source'], request['tag']
    if request.get('status') != 'review-only' or request.get('g5_accepted') is not False:
        raise ValueError('This workflow publishes review candidates only, never G5 acceptance.')
    if not re.fullmatch(r'[0-9a-f]{40}', source):
        raise ValueError('Candidate source must be a complete immutable commit SHA.')
    if not re.fullmatch(r'native-functional-\d{4}-\d{2}-\d{2}-rc\d+', tag):
        raise ValueError('Invalid candidate tag.')
    subprocess.run(['git', 'merge-base', '--is-ancestor', source, head], check=True)
    existing = subprocess.run(['git', 'show-ref', '--verify', 'refs/tags/' + tag], capture_output=True)
    if existing.returncode == 0:
        raise ValueError('Candidate tag already exists; create a new immutable candidate.')
    return source, tag


if __name__ == '__main__':
    request = json.loads(Path('release/candidate.json').read_text())
    source, tag = validate(request, os.environ['GITHUB_SHA'])
    with Path(os.environ['GITHUB_OUTPUT']).open('a') as output:
        output.write(f'source={source}\ntag={tag}\n')
