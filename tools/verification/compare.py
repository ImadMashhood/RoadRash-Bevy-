"""Strict internal regression comparison; no original-executable oracle is required."""
import hashlib
import json
from pathlib import Path

INITIAL_KEYS = ('content_sha256', 'course', 'level', 'bike', 'character', 'seed',
                'inputs_sha256', 'simulation_schema', 'camera', 'native_only')


def compare(left, right):
    for key in INITIAL_KEYS:
        if key not in left['initial'] or key not in right['initial']:
            raise ValueError('Missing initial condition: ' + key)
        if left['initial'][key] != right['initial'][key]:
            raise ValueError('Incompatible initial condition: ' + key)
    for record in (left, right):
        if not record.get('engine_sha256'):
            raise ValueError('Missing engine build identity')
    differences = {}
    for channel in ('ticks', 'scheduler', 'rng', 'motion'):
        a, b = left.get(channel, []), right.get(channel, [])
        first = next((i for i, pair in enumerate(zip(a, b)) if pair[0] != pair[1]), None)
        if first is None and len(a) != len(b):
            first = min(len(a), len(b))
        differences[channel] = None if first is None else {
            'index': first, 'left': a[first] if first < len(a) else None,
            'right': b[first] if first < len(b) else None}
    return {'outcome': 'equal' if not any(differences.values()) else 'different',
            'first_differences': differences,
            'classification': 'First unequal observation per channel; causality requires inspection. Content mismatches reject comparison.',
            'tolerance': {'state': 0, 'rng': 0, 'scheduling_ticks': 0, 'fixed_point_motion': 0},
            'engine_changed': left['engine_sha256'] != right['engine_sha256']}


def image_regions(left, right):
    """Exact engine image comparison, unmasked. Decode PNG to ignore encoder metadata."""
    from PIL import Image
    with Image.open(left) as a, Image.open(right) as b:
        if a.size != b.size:
            raise ValueError('Incompatible image dimensions')
        a, b = a.convert('RGBA'), b.convert('RGBA')
        width, height = a.size
        rows = []
        for name, top, bottom in [('sky', 0, height * 3 // 8),
                                  ('road_and_actors', height * 3 // 8, height),
                                  ('full', 0, height)]:
            ap = list(a.crop((0, top, width, bottom)).getdata())
            bp = list(b.crop((0, top, width, bottom)).getdata())
            errors = [abs(x-y) for p, q in zip(ap, bp) for x, y in zip(p, q)]
            rows.append({'region': name, 'bounds': [0, top, width, bottom], 'mask': None,
                         'different_pixels': sum(p != q for p, q in zip(ap, bp)),
                         'maximum_channel_error_8bit': max(errors, default=0),
                         'mean_channel_error_8bit': sum(errors) / max(1, len(errors))})
        return {'regions': rows, 'passed': all(r['different_pixels'] == 0 for r in rows),
                'tolerance_8bit': 0, 'reason': 'Internal fixed-camera rasterizer is deterministic; no defect masks.'}
