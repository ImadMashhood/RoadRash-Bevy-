#!/usr/bin/env python3
"""Compare narrow combat/recovery primitives; does not claim full race parity."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
REVISION='7aecf6044f2dd773b925b325823f30d648d61c89'
def compare_cases(name, native, rust):
    """Retain every failed family and its first counterexample, including length differences."""
    from itertools import zip_longest
    differences = [(index, left, right) for index, (left, right) in
                   enumerate(zip_longest(native, rust)) if left != right]
    result = dict(family=name, rows=len(rust), native_rows=len(native),
                  outcome='failed' if differences else 'passed',
                  mismatches=len(differences))
    if differences:
        index, left, right = differences[0]
        result['first_divergence'] = dict(index=index, native=left, rust=right)
    return result

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--data',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--rust',type=Path,default=Path('../target/release'))
    p.add_argument('--source',type=Path,default=Path('local/reference-cpp'))
    p.add_argument('--native',type=Path,default=Path('local/reference-build'))
    a=p.parse_args()
    revision=subprocess.check_output(['git','-C',str(a.source),'rev-parse','HEAD'],text=True).strip()
    if revision!=REVISION: raise ValueError('Unpinned native reference')
    dirty=subprocess.check_output(['git','-C',str(a.source),'diff','--name-only','HEAD'],text=True).splitlines()
    if any(path.startswith('src/') and path!='src/hud.cpp' for path in dirty):
        raise ValueError('Compared reference sources are modified')
    d,o=a.data.resolve(),a.output.resolve()
    if d==o or d in o.parents or o in d.parents: raise ValueError('Output overlaps source')
    o.mkdir(parents=True,exist_ok=True)
    subprocess.run([str((a.rust/'rr-encounter-check').resolve()),str(d),str(o)],check=True)
    binary=o/'native-encounters'
    subprocess.run([os.environ.get('CXX','g++'),'-O2','-std=c++20','-Wall','-Wextra','-Werror','-I',str(a.source/'src'),'tools/reference/encounters.cpp',str(a.native/'librr-resource.a'),str(a.native/'jpeg-scalar/build/libjpeg.a'),str(a.native/'freetype/build/libfreetype.a'),'-lSDL3','-pthread','-o',str(binary)],check=True)
    reference=subprocess.check_output([str(binary),str(d)])
    (o/'native.json').write_bytes(reference)
    expected=json.loads(reference)
    checks=[]
    for name in ['motion','damage','attack_timing','crash_entry','damage_transaction','recovery_motion']:
        result=json.loads((o/f'{name}-rust.json').read_bytes())
        checks.append(compare_cases(name,expected[name],result))
    outcome='passed' if all(row['outcome']=='passed' for row in checks) else 'failed'
    receipt={'outcome':outcome,'reference_revision':revision,'reference_dirty_paths':dirty,'compared_reference_sources_clean':True,'native_library_sha256':hashlib.sha256((a.native/'librr-resource.a').read_bytes()).hexdigest(),'executable_sha256':hashlib.sha256((d/'ROADRASH.EXE').read_bytes()).hexdigest(),'checks':checks,'native_trace_sha256':hashlib.sha256(reference).hexdigest(),'physical_original_game_compared':False,'scope':'paired motion/damage/attack/crash primitives plus isolated zero-drag longitudinal recovery; complete contacts, fights and multi-tick full recovery remain separate acceptance'}
    (o/'comparison.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt))
    if outcome != 'passed':
        raise SystemExit(1)
if __name__=='__main__': main()
