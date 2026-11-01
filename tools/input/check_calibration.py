#!/usr/bin/env python3
"""Compare saved profiles in the real evdev game adapter and native preview."""
import argparse,json,os,selectors,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--input',type=Path,required=True);p.add_argument('--probe',type=Path,required=True);p.add_argument('--preview',type=Path,required=True);p.add_argument('--helper',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
for binary in [a.input,a.probe,a.preview,a.helper]:
    if not binary.is_file():raise SystemExit(f'Build first: {binary}')
if not os.access('/dev/uinput',os.W_OK):raise SystemExit('/dev/uinput unavailable; no native calibration check was run')
a.output.mkdir(parents=True,exist_ok=True);profile=a.output/'authored-profile.json'
if profile.exists():raise SystemExit('Use a fresh output directory; authored-profile.json already exists')
key='fffe:0100:roadrash virtual test pad 0'
def cli(*args):return subprocess.run([str(a.input.resolve()),'calibration',*map(str,args)],check=True,capture_output=True,text=True)
cli('init',profile)
for field,value in [('minimum',-0.82),('center',0.07),('maximum',0.9),('dead_zone',0.03),('saturation',0.96),('response','cubic'),('trigger_threshold',0.3)]:cli('set',profile,key,field,value)
cli('validate',profile);expected=json.loads(profile.read_text())['devices'][key]
state=a.output/'probe.json';state.with_suffix('.stop').unlink(missing_ok=True)
logs=[(a.output/name).open('w')for name in ['helper.log','probe.log','preview.log']]
helper=subprocess.Popen([str(a.helper.resolve())],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=logs[0],text=True)
probe=None;preview=None;steps=[];select=selectors.DefaultSelector();select.register(helper.stdout,selectors.EVENT_READ)
def send(command):
    helper.stdin.write(command+'\n');helper.stdin.flush()
    if not select.select(timeout=3)or not helper.stdout.readline().startswith('ok '):raise RuntimeError('virtual pad command failed')
def observations():return json.loads(state.read_text())['observations']if state.exists()else[]
def wait(name,predicate,start=0):
    end=time.monotonic()+6
    while time.monotonic()<end:
        for row in observations()[start:]:
            for pad in row.get('pads',[]):
                if pad['key']==key and predicate(pad,row):
                    steps.append({'check':name,'raw':pad['raw_stick'],'steer_q15':pad['steer_q15'],'command':row['command']});print(name,'passed',flush=True);return pad,row
        if probe.poll()is not None:raise RuntimeError('native adapter probe exited')
        time.sleep(.02)
    raise RuntimeError(f'timed out: {name}')
try:
    send('create 0')
    probe=subprocess.Popen([str(a.probe.resolve()),str(state.resolve()),str(profile.resolve())],stdout=logs[1],stderr=subprocess.STDOUT)
    wait('profiled_controller_detected',lambda pad,row:True)
    start=len(observations());send('axis 0 2294');wait('calibrated_center_is_neutral',lambda pad,row:0.065<pad['raw_stick']<0.075 and pad['steer_q15']==0,start)
    for raw in [1966,2621]:
        start=len(observations());send(f'axis 0 {raw}');wait('center_drift_is_suppressed',lambda pad,row:0.05<pad['raw_stick']<0.09 and pad['steer_q15']==0,start)
    start=len(observations());send('trigger 0 90');wait('calibrated_trigger_threshold_applied',lambda pad,row:row['command']['throttle'],start)
    start=len(observations());send('trigger 0 0');wait('released_trigger_clears_throttle',lambda pad,row:not row['command']['throttle'],start)
    start=len(observations());send('axis 0 -32768');wait('left_full_deflection',lambda pad,row:pad['steer_q15']==-32767 and row['command']['steer_q15']==-32767,start)
    start=len(observations());send('axis 0 32767');wait('right_full_deflection',lambda pad,row:pad['steer_q15']==32767 and row['command']['steer_q15']==32767,start)
    send('axis 0 19660')
    report=a.output/'preview.json';capture=a.output/'preview.png'
    preview=subprocess.Popen([str(a.preview.resolve()),'--profile',str(profile.resolve()),'--device-key',key,'--smoke-frames','180','--capture',str(capture.resolve()),'--report',str(report.resolve())],stdout=logs[2],stderr=subprocess.STDOUT)
    # Existing devices may not report their initial axes at process startup.
    # Emit changes after startup, then leave both consumers at the same value.
    for _ in range(14):
        if preview.poll()is not None:break
        send('axis 0 19332');time.sleep(.05);send('axis 0 19660');time.sleep(.05)
    assert preview.wait(timeout=15)==0
    ui=json.loads(report.read_text());assert ui['connected']and ui['selected']==key and ui['profile']==expected
    pad,row=wait('gameplay_cubic_profile_applied',lambda pad,row:abs(pad['raw_stick']-ui['raw'])<0.00001 and pad['steer_q15']==ui['steer_q15']and row['command']['steer_q15']==ui['steer_q15'])
    assert 0<ui['steer_q15']<32767 and len(ui['curve'])==201 and capture.is_file()
    steps.append({'check':'live_preview_matches_game_command','raw':ui['raw'],'steer_q15':ui['steer_q15'],'curve_points':len(ui['curve'])});print('live_preview_matches_game_command passed',flush=True)
    state.with_suffix('.stop').write_text('complete\n');assert probe.wait(timeout=5)==0
    (a.output/'receipt.json').write_text(json.dumps({'outcome':'passed','virtual_controller':True,'physical_controller_tested':False,'scope':'saved model profile through real Linux evdev/Gilrs into the same transform in gameplay and native preview','profile':expected,'checks':steps,'preview_png':capture.name},indent=2)+'\n')
finally:
    if helper.stdin:helper.stdin.close()
    try:helper.wait(timeout=3)
    except subprocess.TimeoutExpired:helper.kill();helper.wait()
    if probe is not None and probe.poll()is None:
        state.with_suffix('.stop').write_text('cleanup\n')
        try:probe.wait(timeout=3)
        except subprocess.TimeoutExpired:probe.kill();probe.wait()
    if preview is not None and preview.poll()is None:
        preview.terminate()
        try:preview.wait(timeout=3)
        except subprocess.TimeoutExpired:preview.kill();preview.wait()
    select.close()
    for log in logs:log.close()
