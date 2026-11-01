#!/usr/bin/env python3
"""Repeat the upstream race benchmark with isolated original-mode settings."""
from pathlib import Path
import argparse,json,os,subprocess
p=argparse.ArgumentParser();p.add_argument('--assets',type=Path,required=True);p.add_argument('--binary',type=Path,default=Path('local/reference-build/rr-game'));p.add_argument('--output',type=Path,required=True);p.add_argument('--frames',type=int,default=3000);p.add_argument('--warmup',type=int,default=4000);p.add_argument('--runs',type=int,default=3);a=p.parse_args()
if min(a.frames,a.runs)<1 or a.warmup<0:raise SystemExit('frames/runs must be positive; warmup nonnegative')
a.output.mkdir(parents=True,exist_ok=True);pref=a.output/'prefs';pref.mkdir(exist_ok=True)
(pref/'settings.txt').write_text('widescreen=0\ntexture_detail=0\nenh_interpolate=0\nenh_perspective=0\nenh_full_colour=0\nenh_filter=0\nenh_fog=0\nenh_grade=0\nenh_draw_slices=33\nenh_bloom=0\nenh_vignette=0\nenh_motion_blur=0\n')
env=os.environ.copy();env['RR_PREF_OVERRIDE']=str(pref.resolve())+'/'
reports=[]
for run in range(1,a.runs+1):
 report=a.output/f'run-{run}.json'
 with (a.output/f'run-{run}.log').open('w')as log:
  subprocess.run([str(a.binary),'--assets',str(a.assets.resolve()),'--course','1','--opponents','14','--render-scale','1','--window-scale','1','--bench',str(a.frames),'--bench-warmup',str(a.warmup),'--bench-out',str(report.resolve()),'--bench-label',f'city-full-field-original-{run}'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
 reports.append(json.loads(report.read_text()));print(f'Run {run}: {reports[-1]["fps_mean"]} fps',flush=True)
(a.output/'summary.json').write_text(json.dumps({'scope':'native C++ full-field City race throughput; vsync off; not original EXE parity or a Rust performance comparison','runs':reports},indent=2)+'\n')
