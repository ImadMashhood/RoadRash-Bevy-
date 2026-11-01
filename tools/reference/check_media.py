#!/usr/bin/env python3
"""Compare source media and sound scalars with the pinned native port; keep outputs private."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
REVISION='7aecf6044f2dd773b925b325823f30d648d61c89'
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--data',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--source',type=Path,default=Path('local/reference-cpp'));p.add_argument('--rust',type=Path,default=Path('../target/release'));p.add_argument('--native',type=Path,default=Path('local/reference-build'))
    a=p.parse_args();data,out=a.data.resolve(),a.output.resolve()
    if data==out or data in out.parents or out in data.parents:raise ValueError('Output overlaps immutable source')
    revision=subprocess.check_output(['git','-C',str(a.source),'rev-parse','HEAD'],text=True).strip()
    if revision!=REVISION:raise ValueError('Unpinned native reference')
    out.mkdir(parents=True,exist_ok=True)
    subprocess.run([str((a.rust/'rr-media-check').resolve()),str(data),str(out)],check=True)
    binary=out/'native-media'
    subprocess.run([os.environ.get('CXX','g++'),'-O2','-std=c++20','-Wall','-Wextra','-Werror','-ffunction-sections','-fdata-sections','-I',str(a.source/'src'),'tools/reference/media.cpp',str(a.native/'librr-resource.a'),str(a.native/'jpeg-scalar/build/libjpeg.a'),str(a.native/'freetype/build/libfreetype.a'),'-Wl,--gc-sections','-lSDL3','-pthread','-o',str(binary)],check=True)
    raw=subprocess.check_output([str(binary),str(data),str(out)]);(out/'media-native.json').write_bytes(raw);expected=json.loads(raw);actual=json.loads((out/'media-rust.json').read_bytes());checks=[]
    for key in ['wav','rpm','pitch','avi']:
        if expected[key]!=actual[key]:
            first=next((i for i,(x,y)in enumerate(zip(expected[key],actual[key]))if x!=y),min(len(expected[key]),len(actual[key])))
            raise AssertionError(f'{key}: mismatch at {first}: native={expected[key][first]} rust={actual[key][first]}')
        checks.append({'family':key,'cases':len(expected[key]),'outcome':'passed'})
    report={'outcome':'passed','reference_revision':revision,'executable_sha256':hashlib.sha256((data/'ROADRASH.EXE').read_bytes()).hexdigest(),'checks':checks,'native_output_sha256':hashlib.sha256(raw).hexdigest(),'decoder':subprocess.check_output(['ffmpeg','-version'],text=True).splitlines()[0],'original_runtime_compared':False,'scope':'source PCM, native RPM/pitch, AVI metadata and IMA audio samples; subjective mix/timbre and complete native mixer scheduling are not claimed'}
    (out/'receipt.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
if __name__=='__main__':main()
