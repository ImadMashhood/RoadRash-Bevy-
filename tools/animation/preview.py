#!/usr/bin/env python3
"""Export private rider contact sheets and a standalone, offline playback inspector."""
import argparse
import base64
import html
import json
from pathlib import Path
import subprocess
from PIL import Image, ImageDraw


def build(output):
    metadata = json.loads((output / 'riders.json').read_text())
    images = {}
    for sprite in metadata['sprites']:
        if sprite:
            for name in [sprite['image']] + ([sprite['shadow']['image']] if sprite['shadow'] else []):
                images[name] = 'data:image/png;base64,' + base64.b64encode((output / name).read_bytes()).decode()
    sheets = []
    for sequence in metadata['sequences']:
        # Every distinct selected frame is labeled; timing remains the complete 60 Hz timeline.
        selected = list(dict.fromkeys(t['frame'] for t in sequence['ticks']))
        columns = 6
        sheet = Image.new('RGBA', (columns * 180, max(1, (len(selected) + columns - 1) // columns) * 230), '#192532')
        draw = ImageDraw.Draw(sheet)
        for i, frame in enumerate(selected):
            x, y = i % columns * 180, i // columns * 230
            sprite = metadata['sprites'][frame]
            draw.line((x + 8, y + 180, x + 172, y + 180), fill='#647e8e')
            if not sprite:
                draw.text((x + 5, y + 188), f"Code {sequence['code']} / BOB {frame}\nEmpty source slot: no draw", fill='white')
                continue
            image = Image.open(output / sprite['image']).convert('RGBA')
            logical, pivot = sprite['logical'], sprite['pivot']
            size = tuple(max(1, round(v * 1.6)) for v in logical)
            image = image.resize(size, Image.Resampling.NEAREST)
            sheet.alpha_composite(image, (round(x + 90 - size[0] * pivot[0]), round(y + 180 - size[1] * pivot[1])))
            timings = [t['tick'] for t in sequence['ticks'] if t['frame'] == frame]
            draw.text((x + 5, y + 188), f"Code {sequence['code']} / BOB {frame}\nticks {min(timings)}..{max(timings)}\npivot {pivot[0]:.3f}, {pivot[1]:.3f}", fill='white')
        name = f"sequence-{sequence['code']:02}.png"
        sheet.save(output / name)
        sheets.append((sequence['code'], name))
    payload = json.dumps(metadata, separators=(',', ':')).replace('<', '\\u003c')
    image_payload = json.dumps(images)
    page = '''<!doctype html><meta charset="utf-8"><title>Road Rash rider inspector</title>
<style>body{background:#101b29;color:#eef4f7;font:16px system-ui;margin:30px;max-width:1200px}button,select,input{font:inherit;margin:6px;padding:8px}canvas{background:#20354a;border:1px solid #647e8e;max-width:100%;image-rendering:pixelated}a{color:#7ed6ed}label{display:inline-block}output{display:block;padding:12px;white-space:pre-wrap}.sheets img{max-width:100%}</style>
<h1>Rider animation inspector</h1><p>Imported timing at 60 simulation ticks per second. State changes cut at a fixed ground anchor. Playback controls affect this preview only.</p>
<label for="state">Sequence</label><select id="state"></select><button id="play">Pause</button><button id="previous">Previous tick</button><button id="next">Next tick</button>
<label>Playback <select id="speed"><option value="0.25">0.25×</option><option value="0.5">0.5×</option><option selected value="1">1×</option><option value="2">2×</option></select></label>
<label>Tick <input id="tick" type="range" min="0" value="0"></label><label>Height preview <input id="height" type="range" min="0" max="120" value="0"></label>
<canvas id="view" width="960" height="520"></canvas><output id="info" aria-live="polite"></output><h2>Contact sheets</h2><div class="sheets">SHEETS</div>
<script>const data=DATA, sources=IMAGES, sprites={};
const el=id=>document.getElementById(id),state=el('state'),tick=el('tick'),canvas=el('view'),ctx=canvas.getContext('2d');let playing=true,debt=0,last=0;
for(const s of data.sequences){const o=document.createElement('option');o.value=s.code;o.textContent='Sequence '+s.code;state.append(o)}
function sequence(){return data.sequences.find(s=>s.code===Number(state.value))}function change(){tick.max=Math.max(0,sequence().ticks.length-1);tick.value=0;debt=0;draw()}
function image(sprite,anchorX,anchorY,mirror,factor=1){const bitmap=sprites[sprite.image];if(!bitmap)return;const w=sprite.logical[0]*3*factor,h=sprite.logical[1]*3*factor;ctx.save();ctx.translate(anchorX,anchorY);if(mirror)ctx.scale(-1,1);ctx.drawImage(bitmap,-w*sprite.pivot[0],-h*sprite.pivot[1],w,h);ctx.restore()}
function draw(){const s=sequence(),t=s.ticks[Number(tick.value)];ctx.fillStyle='#20354a';ctx.fillRect(0,0,960,520);ctx.strokeStyle='#688b99';ctx.beginPath();ctx.moveTo(0,410);ctx.lineTo(960,410);ctx.moveTo(480,0);ctx.lineTo(480,520);ctx.stroke();if(!t)return;const sprite=data.sprites[t.frame],factor=((t.view_flags&65536)?2:1)*((t.view_flags&131072)?4:1);if(sprite){if(sprite.shadow)image(sprite.shadow,480,410,t.mirror,factor);image(sprite,480,410-Number(el('height').value)*3,t.mirror,factor)}el('info').textContent=`Sequence ${s.code} · tick ${t.tick} / ${s.ticks.length-1} · frame ${t.frame}\nContact flag: ${t.contact} · source mirror: ${t.mirror} · palette: ${data.color}\nLogical size: ${sprite?.logical ?? "empty source slot (no draw)"} · pivot: ${sprite?.pivot ?? "none"}\n${data.transition_policy}`;canvas.dataset.sequence=s.code;canvas.dataset.frame=t.frame;canvas.dataset.tick=t.tick}
state.onchange=change;tick.oninput=()=>{playing=false;el('play').textContent='Play';draw()};el('height').oninput=draw;el('play').onclick=()=>{playing=!playing;el('play').textContent=playing?'Pause':'Play'};function step(d){playing=false;el('play').textContent='Play';tick.value=(Number(tick.value)+d+Number(tick.max)+1)%(Number(tick.max)+1);draw()}el('previous').onclick=()=>step(-1);el('next').onclick=()=>step(1);
function frame(now){if(last&&playing){debt+=(now-last)*60/1000*Number(el('speed').value);const n=Math.floor(debt);if(n){debt-=n;tick.value=(Number(tick.value)+n)%(Number(tick.max)+1);draw()}}last=now;requestAnimationFrame(frame)}
Promise.all(Object.entries(sources).map(([name,url])=>new Promise((resolve,reject)=>{const im=new Image;im.onload=()=>{sprites[name]=im;resolve()};im.onerror=reject;im.src=url}))).then(()=>{ctx.imageSmoothingEnabled=false;change();requestAnimationFrame(frame)});
</script>'''
    page = page.replace('SHEETS', ''.join(f'<details><summary>Sequence {code}</summary><img alt="Sequence {code}: frames, timing and pivots" src="{html.escape(name)}"></details>' for code, name in sheets)).replace('DATA', payload).replace('IMAGES', image_payload)
    (output / 'index.html').write_text(page)
    receipt = {'outcome': 'passed', 'sequences': len(sheets), 'sprites': sum(s is not None for s in metadata['sprites']), 'tick_hz': 60, 'color': metadata['color'], 'all_contact_sheets_written': all((output / name).exists() for _, name in sheets)}
    (output / 'preview-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--color', type=int, default=0, choices=range(15))
    parser.add_argument('--binary', type=Path, default=Path('target/release/rr-riders'))
    args = parser.parse_args()
    data_path = args.data.resolve()
    output_path = args.output.resolve()
    if data_path == output_path or data_path in output_path.parents or output_path in data_path.parents:
        raise ValueError('Output overlaps immutable source data')
    args.output.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(args.binary.resolve()), str(args.data.resolve()), str(args.output.resolve()), str(args.color)], check=True)
    print(json.dumps(build(args.output)))
    print((args.output / 'index.html').resolve())


if __name__ == '__main__':
    main()
