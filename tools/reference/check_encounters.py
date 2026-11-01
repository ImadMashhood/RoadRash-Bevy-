#!/usr/bin/env python3
"""Compare narrow combat/recovery primitives; does not claim full race parity."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
REVISION='7aecf6044f2dd773b925b325823f30d648d61c89'
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
    for name in ['motion','damage']:
        result=json.loads((o/f'{name}-rust.json').read_bytes())
        if result!=expected[name]:
            for i,(left,right) in enumerate(zip(expected[name],result)):
                if left!=right: raise AssertionError(f'{name}[{i}]: {left} != {right}')
            raise AssertionError(f'{name} row count')
        checks.append({'family':name,'rows':len(result),'outcome':'passed'})
    receipt={'outcome':'passed','reference_revision':revision,'executable_sha256':hashlib.sha256((d/'ROADRASH.EXE').read_bytes()).hexdigest(),'checks':checks,'native_trace_sha256':hashlib.sha256(reference).hexdigest(),'physical_original_game_compared':False,'scope':'generic actor displacement and reason-scaled stamina only; broader gameplay validated by authored regression scenarios'}
    (o/'comparison.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt))
if __name__=='__main__': main()
