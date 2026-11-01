#!/usr/bin/env python3
"""Read-only ISO corpus verifier. Does not execute or publish game data.
This research tool is not the production importer (no extraction/activation).
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

SECTOR = 2048
MAX_DEPTH = 32
MAX_ENTRIES = 10000


def inventory(image):
    size = image.stat().st_size
    result = {}
    visited = set()
    with image.open('rb') as stream:
        def read(offset, length):
            if offset < 0 or length < 0 or offset + length > size:
                raise ValueError('ISO extent outside image')
            stream.seek(offset)
            data = stream.read(length)
            if len(data) != length:
                raise ValueError('truncated ISO read')
            return data

        pvd = read(16 * SECTOR, SECTOR)
        if pvd[:7] != b'\x01CD001\x01':
            raise ValueError('unsupported ISO primary volume descriptor')

        def extent(record):
            if len(record) < 34:
                raise ValueError('short directory record')
            position = struct.unpack_from('<I', record, 2)[0]
            length = struct.unpack_from('<I', record, 10)[0]
            if position != struct.unpack_from('>I', record, 6)[0] or length != struct.unpack_from('>I', record, 14)[0]:
                raise ValueError('inconsistent ISO both-endian extent')
            if position * SECTOR + length > size:
                raise ValueError('ISO extent outside image')
            return position, length

        def walk(record, parent='', depth=0):
            if depth > MAX_DEPTH:
                raise ValueError('directory depth limit exceeded')
            position, length = extent(record)
            if (position, length) in visited:
                raise ValueError('cyclic or aliased directory')
            visited.add((position, length))
            if length > 16 * 1024 * 1024:
                raise ValueError('directory allocation limit exceeded')
            block = read(position * SECTOR, length)
            offset = 0
            while offset < len(block):
                count = block[offset]
                if count == 0:
                    offset = (offset // SECTOR + 1) * SECTOR
                    continue
                if count < 34 or offset + count > len(block) or offset % SECTOR + count > SECTOR:
                    raise ValueError('invalid ISO directory record size')
                entry = block[offset:offset + count]
                offset += count
                name_length = entry[32]
                if 33 + name_length > len(entry):
                    raise ValueError('invalid identifier length')
                raw = entry[33:33 + name_length]
                if raw in (b'\x00', b'\x01'):
                    continue
                name = raw.decode('ascii').split(';')[0]
                if not name or name in ('.', '..') or any(c in name for c in '/\\\x00'):
                    raise ValueError('unsafe ISO identifier')
                path = parent + '/' + name
                if entry[25] & 2:
                    walk(entry, path, depth + 1)
                else:
                    if len(result) >= MAX_ENTRIES or path in result:
                        raise ValueError('too many or duplicate file records')
                    start, length = extent(entry)
                    if entry[25] & 0x80:
                        raise ValueError('multi-extent files not supported')
                    digest = hashlib.sha256()
                    for at in range(0, length, 1024 * 1024):
                        digest.update(read(start * SECTOR + at, min(1024 * 1024, length - at)))
                    result[path] = {'size': length, 'sha256': digest.hexdigest()}
        walk(pvd[156:156 + pvd[156]])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('iso', type=Path)
    parser.add_argument('--manifest', type=Path, default=Path(__file__).resolve().parents[1] / 'docs/baseline/required-files.json')
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    files = inventory(args.iso)
    manifest = json.loads(args.manifest.read_text())
    failures = []
    for item in manifest['files']:
        actual = files.get('/ROADRASH/' + item['path'].upper())
        if actual is None or actual['sha256'] != item['sha256']:
            failures.append(item['path'])
    with args.iso.open('rb') as stream:
        iso_hash = hashlib.file_digest(stream, 'sha256').hexdigest()
    report = {'schema_version': 1, 'iso_sha256': iso_hash, 'files_in_image': len(files), 'required_files': len(manifest['files']), 'matched_files': len(manifest['files']) - len(failures), 'failures': failures, 'upstream_revision': manifest['upstream_revision'], 'mode': 'read-only audit; not production import'}
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
