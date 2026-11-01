#!/usr/bin/env python3
"""Compare pinned native career APIs with the Rust session domain."""
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
    if revision!=REVISION: raise ValueError('Unpinned reference')
    data,out=a.data.resolve(),a.output.resolve()
    if data==out or data in out.parents or out in data.parents:raise ValueError('Output overlaps immutable source')
    out.mkdir(parents=True,exist_ok=True)
    subprocess.run([str((a.rust/'rr-session-check').resolve()),str(data),str(out)],check=True)
    binary=out/'native-economy'
    subprocess.run([os.environ.get('CXX','g++'),'-O2','-std=c++20','-Wall','-Wextra','-Werror','-ffunction-sections','-fdata-sections','-I',str(a.source/'src'),'tools/reference/economy.cpp',str(a.source/'src/career.cpp'),str(a.native/'librr-resource.a'),str(a.native/'jpeg-scalar/build/libjpeg.a'),str(a.native/'freetype/build/libfreetype.a'),'-Wl,--gc-sections','-lSDL3','-pthread','-o',str(binary)],check=True)
    raw=subprocess.check_output([str(binary),str(data)])
    (out/'economy-native.json').write_bytes(raw)
    expected=json.loads(raw);actual=json.loads((out/'economy-rust.json').read_bytes());checks=[]
    for family,rows in expected.items():
        if actual[family]!=rows:
            first=next((i for i,(x,y)in enumerate(zip(rows,actual[family]))if x!=y),min(len(rows),len(actual[family])))
            raise AssertionError(f'{family}: mismatch at {first}: native={rows[first]} rust={actual[family][first]}')
        checks.append({'family':family,'cases':len(rows),'outcome':'passed'})
    receipt={'outcome':'passed','reference_revision':revision,'executable_sha256':hashlib.sha256((data/'ROADRASH.EXE').read_bytes()).hexdigest(),'checks':checks,'native_output_sha256':hashlib.sha256(raw).hexdigest(),'original_runtime_compared':False,'scope':'character/default-bike decoding, bike prices, prizes, trade-in and settlement APIs; not visual or full-session trajectory parity'}
    (out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))
if __name__=='__main__':main()
