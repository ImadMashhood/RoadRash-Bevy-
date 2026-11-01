#!/usr/bin/env python3
"""Capture fixed native/Rust poses; publish geometry/hash metadata only."""
from pathlib import Path
import argparse,hashlib,json,subprocess
from PIL import Image
p=argparse.ArgumentParser();p.add_argument('--assets',type=Path,required=True);p.add_argument('--install',type=Path,required=True);p.add_argument('--catalog',type=Path,required=True);p.add_argument('--native',type=Path,required=True);p.add_argument('--rust',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
records=[]
for course in ['CITY','HIWAY','MEDLY','NAPA','CANYN']:
 for sample in [50,75,120]:
  stem=f'{course}-{sample}';cpp=a.output/f'cpp-{stem}';rust=a.output/f'rust-{stem}.png'
  raw=subprocess.check_output([str(a.native),str(a.assets),course,str(sample),str(cpp.with_suffix('.ppm'))],text=True);c=json.loads(raw);cpp.with_suffix('.json').write_text(raw)
  Image.open(cpp.with_suffix('.ppm')).save(cpp.with_suffix('.png'))
  subprocess.run([str(a.rust),'frame',str(a.install),str(a.catalog),course,str(sample),str(rust)],check=True,capture_output=True)
  r=json.loads(rust.with_suffix('.json').read_text());assert len(c['road'])==len(r['projection'])
  error=max(abs(expected[key]-actual[key]) for expected,actual in zip(c['road'],r['projection']) for key in expected)
  bounds_error=max(abs(x-y) for x,y in zip(c['bounds'],r['render']['rider_bounds']))
  assert error<=1/1024,(stem,'road',error)
  assert bounds_error<=1/1024,(stem,'rider',bounds_error)
  records.append({'case':stem,'camera_sample':sample,'course':course,'camera_height':150,'dimensions':[640,480],'rider_depth_q8':c['actor_depth'],'rider_logical':c['rider_logical'],'rider_hotspot':c['rider_hotspot'],'native_bounds':c['bounds'],'rust_bounds':r['render']['rider_bounds'],'road_slices':len(c['road']),'maximum_road_geometry_error':error,'maximum_rider_bounds_error':bounds_error,'native_png_sha256':hashlib.sha256(cpp.with_suffix('.png').read_bytes()).hexdigest(),'rust_png_sha256':hashlib.sha256(rust.read_bytes()).hexdigest()})
  print(stem,'geometry matched',flush=True)
(a.output/'comparison.json').write_text(json.dumps({'outcome':'passed','native_revision':'7aecf6044f2dd773b925b325823f30d648d61c89','native_patch':'two missing <climits> includes for GCC 16; no gameplay/render changes','reference':'compiled native C++ port, not the original EXE','coverage':'15 fixed integer-sample poses, 495 road slice vectors and rider registration/projection; full image equality not asserted','geometry_tolerance':1/1024,'tolerance_reason':'native diagnostic JSON prints six significant digits','records':records},indent=2)+'\n')
