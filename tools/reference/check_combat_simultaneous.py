#!/usr/bin/env python3
"""Compare controlled 3/4/8-rider live-list fight transactions with one RNG anchor."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_encounters import REVISION, compare_cases

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--data',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--quick',action='store_true')
    p.add_argument('--profile',choices=['3','4'],default='4')
    a=p.parse_args();data,out=a.data.resolve(),a.output.resolve()
    if data==out or data in out.parents or out in data.parents:raise ValueError('Output overlaps data')
    source,build=Path('local/reference-cpp'),Path('local/reference-build')
    revision=subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()
    if revision!=REVISION:raise ValueError('Unpinned native reference')
    dirty=subprocess.check_output(['git','-C',str(source),'diff','--name-only','HEAD'],text=True).splitlines()
    if any(v.startswith('src/') and v!='src/hud.cpp' for v in dirty):raise ValueError('Compared native sources modified')
    out.mkdir(parents=True,exist_ok=False);include=out/'include';include.mkdir()
    header=(source/'src/race.hpp').read_text()
    if header.count('  private:')!=1:raise ValueError('Header declaration layout changed')
    (include/'race.hpp').write_text(header.replace('  private:','  public:'))
    binary=out/'native-combat-simultaneous'
    subprocess.run([os.environ.get('CXX','g++'),'-O2','-std=c++20','-Wall','-Wextra','-Werror','-I',str(include),'-I',str(source/'src'),'tools/reference/combat_simultaneous.cpp',str(build/'librr-resource.a'),str(build/'jpeg-scalar/build/libjpeg.a'),str(build/'freetype/build/libfreetype.a'),'-lSDL3','-pthread','-o',str(binary)],check=True)
    native=subprocess.check_output([str(binary),str(data)]+(['--quick']if a.quick else []));(out/'native.json').write_bytes(native)
    env=os.environ.copy();env.update(RR_NATIVE_DATA=str(data),RR_SIMULTANEOUS_EXPECTED=str(out/'native.json'),RR_SIMULTANEOUS_OUTPUT=str(out),RR_SIMULTANEOUS_PROFILE=a.profile,CARGO_TARGET_DIR=str(Path('../target').resolve()))
    tested=subprocess.run(['cargo','test','--locked','-p','rr-simulation','native_simultaneous_combat','--','--ignored'],env=env,check=False)
    if not (out/'rust.json').is_file():
        failure={'outcome':'rust_writer_failed','rust_test_exit_code':tested.returncode,'reference_revision':revision,'native_trace_sha256':hashlib.sha256(native).hexdigest(),'scope':'Native observations completed; no Rust comparison trace was produced. See retained test log.'}
        (out/'comparison.json').write_text(json.dumps(failure,indent=2)+'\n')
        raise SystemExit(1)
    expected=json.loads(native);actual=json.loads((out/'rust.json').read_bytes())
    flat=lambda cases:[row for case in cases for row in case['rows']]
    result=compare_cases('simultaneous_combat',flat(expected),flat(actual))
    result.update(cases=len(expected),actor_tick_observations=sum(c['count']*len(c['rows'])for c in expected),combat_profile=int(a.profile),reference_revision=revision,reference_dirty_paths=dirty,native_trace_sha256=hashlib.sha256(native).hexdigest(),rust_trace_sha256=hashlib.sha256((out/'rust.json').read_bytes()).hexdigest(),native_library_sha256=hashlib.sha256((build/'librr-resource.a').read_bytes()).hexdigest(),source_executable_sha256=hashlib.sha256((data/'ROADRASH.EXE').read_bytes()).hexdigest(),rng_policy='Each trajectory anchors MSVC RNG once, then free-runs across all actors and ticks.',scope='Controlled live-list half-target fight/animation transactions, authored positions/commands and source clips; not full Race/AI/physics scheduling or whole-game parity.',original_windows_runtime_compared=False)
    (out/'comparison.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    if tested.returncode or result['outcome']!='passed':raise SystemExit(1)
if __name__=='__main__':main()
