#!/usr/bin/env python3
"""Replay every real career attempt at four schedules; audit the complete money ledger.

Inputs and full replay reports remain private. Only the aggregate receipt is
suitable for publication. No replay commands or setup values are synthesized.
"""
import argparse
import collections
import concurrent.futures
import hashlib
import json
from pathlib import Path
import subprocess
import sys


SCHEDULES = ('30', '60', '144', 'irregular')
AUTHORITY = {'combat': 4, 'recovery': 2, 'contact_walk': 2,
             'opponents': 14, 'police': True, 'recovery_assists': False}


def require(condition, message):
    # Explicit checks also remain active when Python is launched with -O.
    if not condition:
        raise AssertionError(message)


def saved_career(path):
    saved = json.loads(path.read_text())
    checksum = hashlib.sha256(json.dumps(saved['body'], separators=(',', ':'),
                                        ensure_ascii=False).encode()).hexdigest()
    require(saved['schema_version'] == 2 and checksum == saved['sha256'],
            'Saved career checksum mismatch')
    return saved['body']['career'], checksum


def replay_profile(replay, row):
    require(replay['schema_version'] == 6, 'Wrong replay schema')
    rules = replay['encounter']['rules']
    require(rules['assists'] == {'fast_recovery': False, 'rescue_unreachable': False},
            'Assisted career')
    require((rules['combat_version'], rules['body']['recovery_version'],
             rules['contact_walk_version']) == (4, 2, 2)
            and len(rules['body']['source_angles']) == 257,
            'Wrong current interaction profile')
    seeds = replay['encounter']['seeds']
    require(len(seeds) == 15 and {s['id'] for s in seeds} == set(range(1, 16))
            and bool(replay['population'].get('police')), 'Incomplete source roster or police')
    require(row.get('authority_profile') == AUTHORITY, 'Missing strict career authority receipt')
    require(replay['ticks'] == row['ticks'] and replay['expected_hash'] == row['state_sha256'],
            'Replay identity differs from campaign ledger')
    for command in replay['commands']:
        c = command['input']
        require(not (c.get('terminal') or c.get('impact') or c.get('damage', 0)
                     or c.get('weapon') or c.get('pose')), 'Injected state instead of real controls')


def shop_ledger(old, row, prices):
    cash = old['cash']
    bike = row['bike'] - 1
    condition, maximum = old['condition'], old['maximum_condition']
    require(0 <= bike < len(prices), 'Invalid purchased bike')
    transactions = []
    purchase_cost = repair_cost = 0
    if bike != old['bike']:
        purchase_cost = prices[bike] - prices[old['bike']] // 2
        require(purchase_cost <= cash, 'Unaffordable purchase')
        transactions.append({'kind': 'purchase', 'from_bike': old['bike'], 'to_bike': bike,
                             'price': prices[bike], 'trade_in': prices[old['bike']] // 2,
                             'cash_before': cash, 'cash_after': cash - purchase_cost})
        cash -= purchase_cost
        condition = maximum
    quote = ((prices[bike] // 10) * (maximum - condition) + maximum - 1) // maximum
    if quote <= cash:
        repair_cost = quote
        transactions.append({'kind': 'repair', 'quote': quote, 'paid': quote,
                             'cash_before': cash, 'cash_after': cash - quote,
                             'condition_before': condition, 'condition_after': maximum})
        cash -= quote
        condition = maximum
    require(row['shop_transactions'] == transactions, 'Shop transaction ledger mismatch')
    require(cash == row['money_before'], 'Purchase/repair ledger mismatch')
    require(condition == row['condition_initial_percent'], 'Launch condition differs from shop ledger')
    return cash, bike, maximum, purchase_cost, repair_cost


def settlement_money(row, cash, bike, prices, prizes, starting_cash):
    outcome, place, s = row['outcome'], row['place'], row['settlement']
    require(outcome in ('Won', 'Lost', 'Arrested', 'Wrecked', 'Quit', 'TimedOut'),
            'Unknown outcome')
    require(1 <= place <= 15 and (outcome != 'Won' or place <= 3)
            and (outcome != 'Lost' or place > 3), 'Invalid outcome/place')
    require(s['outcome'] == outcome and s['place'] == place, 'Settlement identity mismatch')
    prize = prizes[place - 1] * row['level'] if outcome in ('Won', 'Lost') else 0
    penalty = (row['level'] * 400 if outcome == 'Arrested' else prices[bike] // 10
               if outcome == 'Wrecked' else 0)
    game_over = penalty > cash
    require((s['prize'], s['penalty'], s['game_over']) == (prize, penalty, game_over),
            'Settlement financial fields mismatch')
    return starting_cash if game_over else cash + prize - penalty


def owned_bike_spec(row, handling):
    """Validate the isolated player damage spec without changing original capacity."""
    specs = handling['specs']
    require(len({spec['id'] for spec in specs}) == len(specs), 'Duplicate handling spec IDs')
    originals = [spec for spec in specs if spec['id'] == row['bike']]
    require(len(originals) == 1, 'Missing original owned bike spec')
    original = originals[0]
    require(original['condition'] > 0 and row['condition_capacity'] == original['condition'],
            'Derived damaged spec used as condition maximum')
    initial = row['condition_initial_percent']
    require(0 <= initial <= 100, 'Invalid initial condition percent')
    derived = [spec for spec in specs if spec['id'] == 16]
    if initial < 100:
        expected = dict(original, id=16,
                        condition=max(1, original['condition'] * initial // 100))
        require(handling['selected'] == 16 and derived == [expected],
                'Invalid derived player damage spec')
    else:
        require(handling['selected'] == row['bike'] and not derived,
                'Replay bike differs from career')
    return original


def terminal_result(row, actors):
    hero = next(a for a in actors if a['id'] == 1)
    terminal = hero['encounter']['recovery']['outcome']
    expected = {'Won': 'finished', 'Lost': 'finished',
                'Arrested': 'arrested', 'Wrecked': 'wrecked'}.get(row['outcome'])
    if expected is not None:
        require(terminal == expected, 'Result lacks the matching real terminal state')
    if expected == 'finished':
        finished = []
        for actor in actors:
            if actor['id'] not in range(1, 16):
                continue
            encounter = actor['encounter']
            if encounter['recovery']['outcome'] == 'finished':
                require(encounter['completed_at'] is not None, 'Missing native finish tick')
                finished.append((encounter['completed_at'], actor['id']))
        # Session standings use native completion tick, then ActorId for ties.
        finished.sort()
        place = 1 + next(i for i, (_, actor_id) in enumerate(finished) if actor_id == 1)
        require(row['place'] == place, 'Claimed place differs from native finish order')
        require(row['outcome'] == ('Won' if place <= 3 else 'Lost'),
                'Claimed win/loss differs from native finish order')
    return hero


def replay_reports(records, root, binary, output, jobs):
    """Bound subprocesses and retained reports to at most three races.

    Only paths/futures are prefetched; giant replay JSONs are read one at a time
    by the auditor. Every canonical checkpoint is retained and compared as before.
    """
    require(1 <= jobs <= 12, 'jobs must be between 1 and 12')
    window = min(3, (jobs + 3) // 4)
    pending = collections.deque()

    def run_rate(index, row, rate):
        report = output / f'current-replay-{index:03}-{rate}.json'
        try:
            subprocess.run([str(binary.resolve()), str((root / row['replay']).resolve()),
                            rate, str(report.resolve())], check=True, stdout=subprocess.DEVNULL)
            return rate, json.loads(report.read_text())
        finally:
            report.unlink(missing_ok=True)

    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        def submit(index):
            pending.append((index, [pool.submit(run_rate, index, records[index], rate)
                                    for rate in SCHEDULES]))
        next_index = 0
        try:
            while next_index < min(window, len(records)):
                submit(next_index)
                next_index += 1
            while pending:
                index, futures = pending[0]
                reports = [future.result() for future in futures]
                yield index, reports
                pending.popleft()
                if next_index < len(records):
                    submit(next_index)
                    next_index += 1
        finally:
            for _, futures in pending:
                for future in futures:
                    future.cancel()
            pool.shutdown(wait=True, cancel_futures=True)


def verify(campaign, binary, data, output, partial=False, jobs=12):
    verifier_sha=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    binary_sha=hashlib.sha256(binary.read_bytes()).hexdigest()
    output.mkdir(parents=True, exist_ok=False)
    root=campaign.parent
    record=json.loads(campaign.read_text())
    if isinstance(record,list):
        if not partial or not record: raise ValueError('Only explicit partial audits accept a nonempty progress ledger')
        after, _ = saved_career(root/f'career-state-{len(record)-1:03}.json')
        record={'completed':after['completed'],'real_commands_only':True,'native_only':True,'races':record,'career':after}
    if (not partial and not record['completed']) or not record['real_commands_only'] or not record['native_only']:
        raise ValueError('A genuinely completed native campaign is required')
    sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
    from research_baseline import PE
    expected=json.loads((Path(__file__).resolve().parents[2]/'docs/baseline/required-files.json').read_text())
    expected_exe=next(f['sha256'] for f in expected['files'] if f['path']=='ROADRASH.EXE')
    if hashlib.sha256((data/'ROADRASH.EXE').read_bytes()).hexdigest()!=expected_exe:raise ValueError('Unverified executable source')
    pe=PE(data/'ROADRASH.EXE')
    prices=pe.words(0x468fc0,15)
    prizes=[pe.words(0x46ab48+i*0x4c,1)[0] for i in range(15)]
    metadata=json.loads((Path(__file__).resolve().parents[2]/'docs/baseline/executable/selectable-actors.json').read_text())
    character=next(c for c in metadata['characters'] if c['character_id']==record['career']['character'])
    old={'cash':character['starting_cash'],'bike':character['bike_ids_by_level'][0],
         'condition':100,'maximum_condition':100,'level':1,'qualified':0,
         'character':character['character_id'],'alias':'','completed':False,'revision':0,'next_race':1,'last_settled':0}
    purchases=repairs=checks=0;rows=[];outcomes=collections.Counter();matrix=set();save_checksums=[]
    require(bool(record['races']), 'Empty campaign ledger')
    reports_iterator = replay_reports(record['races'], root, binary, output, jobs)
    try:
        for i,reports in reports_iterator:
            row=record['races'][i]
            require(not old['completed'], 'Attempt after campaign completion')
            replay_path=root/row['replay']
            if replay_path.stat().st_size>16*1024*1024: raise ValueError('Replay exceeds production reader bound')
            raw=replay_path.read_bytes();replay=json.loads(raw)
            replay_profile(replay, row)
            baseline=None
            for rate,actual in reports:
                require(actual['rate']==rate and actual['ticks']==row['ticks'], 'Replay report identity mismatch')
                expected_ticks=list(range(30,row['ticks']+1,30))
                if row['ticks']%30:expected_ticks.append(row['ticks'])
                require([tick for tick,_ in actual['tick_hashes']]==expected_ticks, 'Incomplete canonical checkpoint array')
                if actual['state_sha256']!=row['state_sha256']:raise AssertionError('Career hash mismatch')
                if baseline is not None and actual['tick_hashes']!=baseline:raise AssertionError('Frame schedule divergence')
                baseline=actual['tick_hashes'];checks+=1
            for key,value in old.items():
                if row['preparation_before'].get(key)!=value:raise AssertionError(f'Initial save ledger differs at {i+1}: {key}')
            if row['attempt']!=i+1:raise AssertionError('Nonmonotonic attempt ledger')
            if row['level']!=old['level']:raise AssertionError('Race level changed without qualification')
            cash,bike,maximum,purchase_cost,repair_cost=shop_ledger(old,row,prices)
            purchases+=int(bike!=old['bike']);repairs+=int(repair_cost>0)
            s=row['settlement'];outcomes[row['outcome']]+=1;matrix.add((row['course'],row['level']))
            after, checksum = saved_career(root/f'career-state-{i:03}.json')
            save_checksums.append(checksum)
            original=owned_bike_spec(row,replay['handling'])
            hero=terminal_result(row,actual['final_snapshot']['actors'])
            actual_condition=max(0,min(100,hero['rider']['bike']['condition']*100//original['condition']))
            if actual_condition!=row['condition_final_percent']:raise AssertionError('Condition normalization mismatch')
            if s['race_id']!=i+1 or after['last_settled']!=i+1 or after['next_race']!=i+2:raise AssertionError('Nonmonotonic settlement ledger')
            expected=settlement_money(row,cash,bike,prices,prizes,character['starting_cash'])
            require(expected==after['cash']==row['money_after'], 'Settlement cash mismatch')
            qualified=old['qualified'];level=old['level'];completed=False
            if s['game_over']:
                qualified=0;level=1
                require(after['bike']==character['bike_ids_by_level'][0], 'Game-over bike reset mismatch')
            elif row['outcome']=='Won':
                course=['CITY','HIWAY','MEDLY','NAPA','CANYN'].index(row['course'])
                qualified|=1<<course
                if qualified==31:
                    qualified=0
                    if level==5:completed=True
                    else:level+=1
            if (after['qualified'],after['level'],after['completed'])!=(qualified,level,completed):raise AssertionError('Invalid qualification/progression transition')
            require(after['qualified']==row['qualified'], 'Qualification receipt mismatch')
            require(s['level_advanced']==(level>old['level']) and s['finale']==completed, 'Settlement progression flags mismatch')
            require(after['character']==character['character_id'] and after['maximum_condition']==maximum, 'Career identity/capacity changed')
            if not s['game_over']:require(after['bike']==bike, 'Settlement changed owned bike')
            shop_revision=int(bike!=old['bike'])+int(repair_cost>0)
            require(after['revision']==row['preparation_before']['revision']+shop_revision+1, 'Career revision ledger mismatch')
            expected_condition=(maximum if row['outcome'] in ('Arrested','Wrecked')
                                else actual_condition if row['outcome'] in ('Won','Lost')
                                else row['condition_initial_percent'])
            require(after['condition']==expected_condition, 'Saved condition differs from real settlement')
            rows.append({'attempt':i+1,'course':row['course'],'level':row['level'],'replay_sha256':hashlib.sha256(raw).hexdigest(),
                         'state_sha256':row['state_sha256'],'ticks':row['ticks'],'purchase_cost':purchase_cost,'repair_cost':repair_cost,
                         'outcome':row['outcome'],'checkpoints_equal_at_four_schedules':len(baseline),'money_audited':True})
            old=after
            print(f'{i+1}/{len(record["races"])} verified',flush=True)
    finally:
        reports_iterator.close()
    require(old==record['career'], 'Final career differs from last saved career')
    if not partial:
        require(old['completed'], 'Campaign is incomplete')
        require(matrix=={(course,level) for course in ('CITY','HIWAY','MEDLY','NAPA','CANYN') for level in range(1,6)}, 'Missing course-level coverage')
    require(binary_sha==hashlib.sha256(binary.read_bytes()).hexdigest(), 'Replay binary changed during audit')
    require(verifier_sha==hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'Verifier changed during audit')
    receipt={'outcome':'partial_audit_passed' if partial else 'passed','native_only':True,'real_controls_only':True,'races':len(rows),'replay_runs':checks,
             'frame_schedules':['30','60','144','irregular'],'course_level_cases':len(matrix),'outcomes':dict(outcomes),
             'purchases':purchases,'paid_repairs':repairs,'final_cash':old['cash'],'final_level':old['level'],
             'final_hash':rows[-1]['state_sha256'],'rows':rows,'binary_sha256':binary_sha,
             'verifier_sha256':verifier_sha,
             'source_executable_sha256':expected_exe,'save_body_sha256':save_checksums,
             'authority_profile':AUTHORITY,'source_fields':{'bike_prices_va':'0x468fc0','prize_table_va':'0x46ab48','prize_stride':'0x4c','source_angles_count':257},
             'audit_flags':{'save_schema_2_sha256':True,'shop_transactions':True,'settlement_finances':True,
                            'terminal_outcomes':True,'native_finish_place':True,'full_checkpoint_arrays':True,'complete_campaign_required':not partial},
             'jobs':jobs,'prefetch_races':min(3,(jobs+3)//4)}
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    return receipt

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('campaign',type=Path);p.add_argument('output',type=Path)
    p.add_argument('--binary',type=Path,default=Path('../target/release/rr-replay'))
    p.add_argument('--data',type=Path,required=True)
    p.add_argument('--partial',action='store_true',help='Audit a progress snapshot; never marks campaign complete')
    p.add_argument('--jobs',type=int,choices=range(1,13),default=12)
    a=p.parse_args();verify(a.campaign,a.binary,a.data,a.output,a.partial,a.jobs)
