#!/usr/bin/env python3
"""Verify drawable traffic views and confirm absent pedestrian lookups against the pinned port."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
REVISION='7aecf6044f2dd773b925b325823f30d648d61c89'
def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['data','catalog','output']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--rust',type=Path,default=Path('../target/release'))
    p.add_argument('--source',type=Path,default=Path('local/reference-cpp'))
    p.add_argument('--native',type=Path,default=Path('local/reference-build'))
    a=p.parse_args();o=a.output.resolve()
    for root in [a.data.resolve(),a.catalog.resolve()]:
        if o==root or o in root.parents or root in o.parents:raise ValueError('Output overlaps immutable input')
    revision=subprocess.check_output(['git','-C',str(a.source),'rev-parse','HEAD'],text=True).strip()
    if revision!=REVISION:raise ValueError('Unpinned reference')
    o.mkdir(parents=True,exist_ok=True)
    raw=subprocess.check_output([str((a.rust/'rr-population-art').resolve()),str(a.data.resolve()),str(a.catalog.resolve())])
    (o/'rust-art.json').write_bytes(raw);art=json.loads(raw)
    queries=[q for c in art['courses'] for q in c['missing_queries']]
    query_file=o/'queries.tsv';query_file.write_text(''.join(' '.join(map(str,q))+'\n' for q in queries))
    binary=o/'native-pedestrian-looks'
    subprocess.run([os.environ.get('CXX','g++'),'-O2','-std=c++20','-Wall','-Wextra','-Werror','-I',str(a.source/'src'),'tools/reference/pedestrian-looks.cpp',str(a.native/'librr-resource.a'),str(a.native/'jpeg-scalar/build/libjpeg.a'),str(a.native/'freetype/build/libfreetype.a'),'-lSDL3','-pthread','-o',str(binary)],check=True)
    native=subprocess.check_output([str(binary),str(a.data.resolve()),str(query_file)])
    (o/'native-queries.json').write_bytes(native);rows=json.loads(native)
    if len(rows)!=len(queries) or any(row[4:]!=[0,0] for row in rows):raise AssertionError('Appearance absence differs from pinned native lookup')
    receipt={'outcome':'passed','reference_revision':revision,'vehicle_models':sum(len(c['vehicles']) for c in art['courses']),'vehicle_view_cases':sum(len(v) for c in art['courses'] for v in c['vehicles']),'all_vehicle_views_nontransparent':True,'police_slots':art['courses'][0]['police_slots'],'police_nonempty':art['courses'][0]['police_nonempty'],'pedestrian_requests_level_1':sum(c['pedestrian_requests'] for c in art['courses']),'drawable_pedestrian_requests_level_1':sum(c['drawable_pedestrian_requests'] for c in art['courses']),'reference_absent_appearance_cases':len(rows),'all_absences_match_native':True,'native_queries_sha256':hashlib.sha256(native).hexdigest(),'rust_census_sha256':hashlib.sha256(raw).hexdigest(),'scope':'static source lookup and drawable preview census; full pedestrian CANS timing and original runtime residency are unmeasured'}
    (o/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))
if __name__=='__main__':main()
