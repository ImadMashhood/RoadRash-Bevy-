#!/usr/bin/env python3
"""Exercise the real Linux gamepad backend with two temporary uinput pads.

Only controller axes/buttons are emitted. Requires writable /dev/uinput and
read access to the resulting event devices. No system permission changes.
"""
import argparse,json,os,selectors,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--probe',type=Path,required=True);p.add_argument('--helper',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
if not a.probe.is_file() or not a.helper.is_file():raise SystemExit('Build the probe and native helper first')
if not os.access('/dev/uinput',os.W_OK):raise SystemExit('/dev/uinput is not writable; native backend test unavailable')
a.output.mkdir(parents=True,exist_ok=True);state=a.output/'probe.json';stop=state.with_suffix('.stop');state.unlink(missing_ok=True);stop.unlink(missing_ok=True)
steps=[];probe_log=(a.output/'probe.log').open('w');helper_log=(a.output/'helper.log').open('w')
probe=subprocess.Popen([str(a.probe.resolve()),str(state.resolve())],stdout=probe_log,stderr=subprocess.STDOUT)
helper=subprocess.Popen([str(a.helper.resolve())],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=helper_log,text=True)
selector=selectors.DefaultSelector();selector.register(helper.stdout,selectors.EVENT_READ)
def send(command):
    helper.stdin.write(command+'\n');helper.stdin.flush()
    if not selector.select(timeout=3):raise RuntimeError('uinput helper acknowledgement timed out')
    reply=helper.stdout.readline().strip()
    if not reply.startswith('ok '):raise RuntimeError(f'uinput helper failed: {reply}')
def rows():
    if not state.exists():return []
    return json.loads(state.read_text())['observations']
def wait(name,predicate,start=0):
    deadline=time.monotonic()+6
    while time.monotonic()<deadline:
        for row in rows()[start:]:
            if predicate(row['input']):
                steps.append({'check':name,'observed':row['input'],'command':row['command']});print(name,'passed',flush=True);return row['input']
        if probe.poll() is not None:raise RuntimeError(f'backend probe exited {probe.returncode}')
        time.sleep(.02)
    raise RuntimeError(f'timed out: {name}; latest={rows()[-1:]}; see probe/helper logs')
def is_pad(row):return isinstance(row['device'],dict)and 'Gamepad'in row['device']
try:
    baseline=wait('backend_started',lambda r:True)['connected_gamepads']
    send('create 0');wait('first_controller_connected',lambda r:r['connected_gamepads']==baseline+1)
    start=len(rows());send('axis 0 16384');first=wait('partial_right_stick_reaches_q15',lambda r:is_pad(r)and 0<r['drive']['steer_q15']<32767,start)['device']
    start=len(rows());send('south 0 1');wait('south_button_accelerates',lambda r:r['device']==first and r['drive']['throttle'],start)
    send('create 1');wait('second_controller_connected_without_stealing',lambda r:r['connected_gamepads']==baseline+2 and r['device']==first and r['drive']['throttle'])
    start=len(rows());send('axis 1 -32768');second=wait('second_controller_takes_over_on_input',lambda r:is_pad(r)and r['device']!=first and r['drive']['steer_q15']<=-32765 and not r['drive']['throttle'],start)['device']
    start=len(rows());send('axis 1 0');wait('neutral_active_pad_does_not_inherit_other_held_throttle',lambda r:r['device']==second and r['drive']=={'throttle':False,'brake':False,'steer_q15':0},start)
    start=len(rows());send('destroy 1');wait('active_disconnect_signalled_and_controls_cleared',lambda r:r['active_disconnected']and r['device']=='Keyboard'and not r['drive']['throttle']and r['drive']['steer_q15']==0,start)
    send('south 0 0');time.sleep(.08);start=len(rows());send('trigger 0 255');wait('analog_right_trigger_accelerates',lambda r:r['device']==first and r['drive']['throttle'],start)
    send('trigger 0 0');time.sleep(.08);start=len(rows());send('start 0 1');wait('start_button_pause_resume_edge',lambda r:r['pause_pressed']and r['resume_pressed'],start);send('start 0 0')
    send('create 1');wait('controller_reconnects',lambda r:r['connected_gamepads']==baseline+2)
    # Force a fresh neutral report: some backends retain the previous axis
    # value across reconnect until the new evdev device emits a change.
    send('axis 1 1');time.sleep(.04);send('axis 1 0');time.sleep(.08)
    start=len(rows());send('axis 1 -16384');wait('reconnected_controller_partial_left_input',lambda r:is_pad(r)and r['device']!=first and -32767<r['drive']['steer_q15']<0,start)
    start=len(rows());send('destroy 0');wait('inactive_disconnect_preserves_active_input',lambda r:r['connected_gamepads']==baseline+1 and not r['active_disconnected']and r['drive']['steer_q15']<0,start)
    start=len(rows());send('destroy 1');wait('last_active_disconnect_clears_input',lambda r:r['connected_gamepads']==baseline and r['active_disconnected']and r['device']=='Keyboard',start)
    stop.write_text('complete\n');assert probe.wait(timeout=5)==0
    (a.output/'receipt.json').write_text(json.dumps({'outcome':'passed','backend':'real Linux uinput -> evdev -> Gilrs -> Bevy InputPlugin -> game adapter -> tick command','virtual_devices':True,'physical_controller_tested':False,'logical_controllers':2,'kernel_device_creations':3,'baseline_connected_devices':baseline,'full_axis_endpoint_tolerance_q15':2,'checks':steps,'observations':rows()},indent=2)+'\n')
finally:
    if helper.stdin:helper.stdin.close()
    try:helper.wait(timeout=3)
    except subprocess.TimeoutExpired:helper.kill();helper.wait()
    if probe.poll()is None:
        stop.write_text('cleanup\n')
        try:probe.wait(timeout=3)
        except subprocess.TimeoutExpired:probe.kill();probe.wait()
    selector.close();probe_log.close();helper_log.close()
