#!/usr/bin/env python3
"""Replay every real career attempt at four schedules; audit the complete money ledger.

Inputs and full replay reports remain private. Only the aggregate receipt is
suitable for publication. No replay commands or setup values are synthesized.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def verify(campaign, binary, data, output):
    output.mkdir(parents=True, exist_ok=False)
    root=campaign.parent
    record=json.loads(campaign.read_text())
    if not record['completed'] or not record['real_commands_only'] or not record['native_only']:
        raise ValueError('A genuinely completed native campaign is required')
    sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
    from research_baseline import PE
    pe=PE(data/'ROADRASH.EXE')
    prices=pe.words(0x468fc0,15)
    prizes=[pe.words(0x46ab48+i*0x4c,1)[0] for i in range(15)]
    metadata=json.loads((Path(__file__).resolve().parents[2]/'docs/baseline/executable/selectable-actors.json').read_text())
    character=next(c for c in metadata['characters'] if c['character_id']==record['career']['character'])
    old={'cash':character['starting_cash'],'bike':character['bike_ids_by_level'][0],
         'condition':100,'maximum_condition':100,'level':1,'qualified':0}
    purchases=repairs=checks=0;rows=[];outcomes=collections.Counter();matrix=set()
    for i,row in enumerate(record['races']):
        replay_path=root/row['replay'];raw=replay_path.read_bytes();replay=json.loads(raw)
        if len(raw)>16*1024*1024: raise ValueError('Replay exceeds production reader bound')
        if replay['schema_version']!=6 or replay['encounter']['rules']['assists']!={'fast_recovery':False,'rescue_unreachable':False}:
            raise ValueError('Wrong schema or assisted career')
        for command in replay['commands']:
            c=command['input']
            if c.get('terminal') or c.get('impact') or c.get('damage',0) or c.get('weapon') or c.get('pose'):
                raise ValueError('Injected state instead of real controls')
        baseline=None
        for rate in ['30','60','144','irregular']:
            report=output/'current-replay.json'
            subprocess.run([str(binary.resolve()),str(replay_path.resolve()),rate,str(report.resolve())],
                           check=True,stdout=subprocess.DEVNULL)
            actual=json.loads(report.read_text())
            if actual['state_sha256']!=row['state_sha256']:raise AssertionError('Career hash mismatch')
            if baseline is not None and actual['tick_hashes']!=baseline:raise AssertionError('Frame schedule divergence')
            baseline=actual['tick_hashes'];checks+=1
        cash=old['cash'];bike=row['bike']-1;condition=old['condition'];maximum=old['maximum_condition']
        purchase_cost=repair_cost=0
        if bike!=old['bike']:
            purchase_cost=prices[bike]-prices[old['bike']]//2
            cash-=purchase_cost;condition=maximum;purchases+=1
        quote=((prices[bike]//10)*(maximum-condition)+maximum-1)//maximum
        if quote<=cash:
            cash-=quote;repair_cost=quote;repairs+=int(quote>0)
        if cash!=row['money_before']:raise AssertionError(f'Purchase/repair ledger mismatch at {i+1}')
        s=row['settlement'];outcomes[row['outcome']]+=1;matrix.add((row['course'],row['level']))
        after=json.loads((root/f'career-state-{i:03}.json').read_text())['body']['career']
        expected=cash
        if row['outcome'] in ('Won','Lost'):
            prize=prizes[row['place']-1]*row['level'];assert prize==s['prize'];expected+=prize
        elif row['outcome'] in ('Arrested','Wrecked'):
            penalty=row['level']*400 if row['outcome']=='Arrested' else prices[bike]//10
            assert penalty==s['penalty']
            expected=character['starting_cash'] if penalty>cash else cash-penalty
        assert expected==after['cash']==row['money_after']
        assert after['qualified']==row['qualified']
        rows.append({'attempt':i+1,'course':row['course'],'level':row['level'],'replay_sha256':hashlib.sha256(raw).hexdigest(),
                     'state_sha256':row['state_sha256'],'ticks':row['ticks'],'purchase_cost':purchase_cost,'repair_cost':repair_cost,
                     'outcome':row['outcome'],'checkpoints_equal_at_four_schedules':len(baseline),'money_audited':True})
        old=after
        print(f'{i+1}/{len(record["races"])} verified',flush=True)
    assert old==record['career'] and old['completed']
    assert len(matrix)==25
    receipt={'outcome':'passed','native_only':True,'real_controls_only':True,'races':len(rows),'replay_runs':checks,
             'frame_schedules':['30','60','144','irregular'],'course_level_cases':len(matrix),'outcomes':dict(outcomes),
             'purchases':purchases,'paid_repairs':repairs,'final_cash':old['cash'],'final_level':old['level'],
             'final_hash':rows[-1]['state_sha256'],'rows':rows,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()}
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    (output/'current-replay.json').unlink()

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('campaign',type=Path);p.add_argument('output',type=Path)
    p.add_argument('--binary',type=Path,default=Path('../target/release/rr-replay'))
    p.add_argument('--data',type=Path,required=True)
    a=p.parse_args();verify(a.campaign,a.binary,a.data,a.output)
