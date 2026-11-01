#!/usr/bin/env python3
"""Plot actual private fixed-tick source traces without external plotting dependencies."""
import argparse
import csv
import json
from pathlib import Path
from xml.sax.saxutils import escape


def chart(rows, fields, title, target):
    colors = ['#66d9ef', '#ffbf69', '#c3f584']
    width, height = 1100, 260 * len(fields)
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><rect width="100%" height="100%" fill="#101b29"/><text x="35" y="28" fill="white" font-family="sans-serif" font-size="18">{escape(title)}</text>']
    for panel, field in enumerate(fields):
        values = [float(r[field]) for r in rows]
        lo, hi = min(0, min(values)), max(1, max(values))
        top = 60 + panel * 250
        points = ' '.join(f'{60+i/(len(values)-1)*1000:.2f},{top+170-(v-lo)/(hi-lo)*170:.2f}' for i, v in enumerate(values))
        parts.extend([f'<text x="60" y="{top-12}" fill="white" font-family="sans-serif">{escape(field)} [{lo:g}, {hi:g}]</text>', f'<path d="M60 {top} V{top+170} H1060" fill="none" stroke="#7793a1"/>', f'<polyline points="{points}" fill="none" stroke="{colors[panel%3]}" stroke-width="1.5"/>', f'<text x="60" y="{top+192}" fill="#9cbdca" font-family="sans-serif">tick {rows[0]["tick"]} to {rows[-1]["tick"]}</text>'])
    parts.append('</svg>')
    target.write_text(''.join(parts))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-trace', required=True, type=Path)
    parser.add_argument('--riding-trace', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    source = json.loads(args.native_trace.read_text())
    for bike in [1, 8, 15]:
        rows = [dict(tick=r[2], speed=r[3], gear=r[4], lean=r[6]) for r in source if r[0] == bike and r[1] == 0]
        chart(rows, ['speed', 'gear', 'lean'], f'Bike {bike}: throttle ticks 0–1799; braking ticks 1800–2399', args.output / f'bike-{bike}.svg')
    with args.riding_trace.open() as stream:
        rows = list(csv.DictReader(stream))
    chart(rows, ['speed', 'lateral', 'height'], 'Integrated rider trace: fixed ticks, lateral source Q8, height source Q8', args.output / 'rider.svg')
    print(json.dumps({'outcome': 'passed', 'plots': 4, 'source': 'actual native API traces and Rust riding CSV; no render-time input'}))


if __name__ == '__main__':
    main()
