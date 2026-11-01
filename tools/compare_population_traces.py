#!/usr/bin/env python3
"""Compare scheduler, RNG and motion observations independently, without inferring causality."""
import argparse
import json
from pathlib import Path

def first_difference(left, right):
    for index, (a, b) in enumerate(zip(left, right)):
        if a != b:
            return {'index':index, 'left':a, 'right':b}
    if len(left) != len(right):
        return {'index':min(len(left),len(right)), 'left_count':len(left), 'right_count':len(right)}
    return None

def compare(left, right):
    a=json.loads((left/'behavior-trace.json').read_text())
    b=json.loads((right/'behavior-trace.json').read_text())
    scheduler=first_difference([[r['tick'],r['decisions']] for r in a],[[r['tick'],r['decisions']] for r in b])
    motion=first_difference([[r['tick'],r['motion']] for r in a],[[r['tick'],r['motion']] for r in b])
    rng=first_difference(json.loads((left/'rng-trace.json').read_text()),json.loads((right/'rng-trace.json').read_text()))
    return {'outcome':'equal' if all(x is None for x in [scheduler,motion,rng]) else 'different', 'scheduler':scheduler,'rng':rng,'motion_arithmetic_candidate':motion,'interpretation':'First unequal observation per channel; a motion mismatch alone is not proof of an arithmetic root cause.'}

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('left',type=Path);p.add_argument('right',type=Path);p.add_argument('output',type=Path)
    a=p.parse_args()
    if a.output.resolve() in [(d/f).resolve() for d in [a.left,a.right] for f in ['behavior-trace.json','rng-trace.json']]:
        raise ValueError('Output overlaps an input trace')
    report=compare(a.left,a.right);a.output.write_text(json.dumps(report,indent=2)+'\n');print(report['outcome'])
if __name__=='__main__': main()
