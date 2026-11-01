#!/usr/bin/env python3
"""Read supported local files to produce metadata inventories. No executable is run.
Field offsets are research facts from the pinned upstream references; behavior not inferred as verified.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]

class PE:
    def __init__(self, path):
        self.data = path.read_bytes()
        self.pe = self.u32(60)
        if self.data[:2] != b'MZ' or self.data[self.pe:self.pe+4] != b'PE\0\0':
            raise ValueError('invalid PE signature')
        self.base = self.u32(self.pe+24+28)
        self.sections = []
        table = self.pe + 24 + self.u16(self.pe+20)
        for i in range(self.u16(self.pe+6)):
            at = table + i * 40
            self.sections.append((self.u32(at+12), self.u32(at+16), self.u32(at+20)))
    def u16(self, at):
        return struct.unpack_from('<H', self.data, at)[0]
    def u32(self, at):
        return struct.unpack_from('<I', self.data, at)[0]
    def offset(self, rva, size):
        for va, length, raw in self.sections:
            if va <= rva and rva+size <= va+length:
                at = raw+rva-va
                if at+size > len(self.data):
                    raise ValueError('truncated PE section')
                return at
        raise ValueError('unmapped PE range')
    def read(self, address, size):
        at = self.offset(address-self.base, size)
        return self.data[at:at+size]
    def words(self, address, count, code='I'):
        size = struct.calcsize('<'+code)*count
        return list(struct.unpack('<'+code*count, self.read(address, size)))
    def strings(self):
        rva = self.u32(self.pe+24+112)
        base = self.offset(rva, 16)
        def entries(relative):
            p = base+relative
            count = self.u16(p+12)+self.u16(p+14)
            if count > 4096:
                raise ValueError('resource count too large')
            return [(self.u32(p+16+i*8), self.u32(p+20+i*8)) for i in range(count)]
        strings = {}
        for kind, types in entries(0):
            if kind != 6:
                continue
            for block, names in entries(types & 0x7fffffff):
                for language, value in entries(names & 0x7fffffff):
                    address, length = self.u32(base+value), self.u32(base+value+4)
                    at = self.offset(address,length)
                    end = at+length
                    for i in range(16):
                        n = self.u16(at); at += 2
                        if at+2*n>end:
                            raise ValueError('truncated string block')
                        if n:
                            strings[(block-1)*16+i] = self.data[at:at+2*n].decode('utf-16-le')
                        at += 2*n
        return strings

def generate(data, output):
    manifest = json.loads((ROOT/'docs/baseline/required-files.json').read_text())
    for name in ('ROADRASH.EXE', 'TEXT/ENU.DLL'):
        required = next(f for f in manifest['files'] if f['path'] == name)
        if hashlib.sha256((data/name).read_bytes()).hexdigest() != required['sha256']:
            raise ValueError('unsupported or modified revision: '+name)
    exe = PE(data/'ROADRASH.EXE')
    resources = PE(data/'TEXT/ENU.DLL').strings()
    strings = {}
    count = int(resources[0].split()[1])
    for index in range(1, count+1):
        text = resources.get(index, '')
        seen = 0
        while text.endswith('ÿ'):
            if len(text) < 2 or seen > 64:
                raise ValueError('invalid language string continuation')
            value = text[-2].encode('cp1252')[0]
            continuation = 3000 + (value if value < 128 else value-256)
            text = text[:-2] + resources[continuation]
            seen += 1
        strings[index] = text
    strings[0] = '' 
    output.mkdir(parents=True,exist_ok=True)
    source = {'exe_sha256': hashlib.sha256(exe.data).hexdigest(), 'upstream_revision': manifest['upstream_revision'], 'confidence': 'verified-local', 'method': 'static binary metadata inspection', 'runtime_observations': 'not performed'}
    def write(name, value):
        (output/name).write_text(json.dumps({'schema_version':1, 'source':source, **value},indent=2)+'\n')
    counts = {0x10:0x474668,0x11:0x47466c,0x12:0x465fb0,0x13:0x474670,0xb:0x46ca88,0xd:0x46ce80}
    screens=[]
    for screen in range(36):
        count, address, title, title_format, unused = exe.words(0x46a390+screen*20,5)
        if screen in counts:
            count = exe.words(counts[screen],1)[0]
        if count>64:
            raise ValueError('implausible item count')
        items=[]
        for i in range(count if address else 0):
            b=exe.read(address+i*56,56)
            def u(at): return struct.unpack_from('<I',b,at)[0]
            items.append({'index':i,'target':u(4),'caption_id':u(8),'caption':strings.get(u(8),'[dynamic/missing]'),'rect':list(struct.unpack_from('<iiii',b,12)),'navigation':list(b[28:36]),'flags':u(36),'enabled':bool(b[40]),'handler':f'0x{u(44):08x}','description_id':u(48),'escape_item':bool(u(36)&0x8000),'activation_semantics':'direct target' if not u(44) else 'handler-dependent; requires behavior verification'})
        screens.append({'id':screen,'title_id':title,'title':strings.get(title,'[dynamic/missing]'),'items':items,'entry_conditions':'runtime-dependent; capture required','transition_capture':'pending'})
    def extra_item(address, index):
        b=exe.read(address,56)
        u=lambda at: struct.unpack_from('<I',b,at)[0]
        return {'index':index,'target':u(4),'caption_id':u(8),'caption':strings.get(u(8),'[dynamic/missing]'),'navigation':list(b[28:36]),'flags':u(36),'enabled':bool(b[40]),'handler':f'0x{u(44):08x}','escape_item':bool(u(36)&0x8000),'activation_semantics':'handler-dependent; runtime verification required'}
    popup_tables=[(0x4735c0,4),(0x4736a0,6),(0x4737f0,2),(0x474240,4),(0x474320,3),(0x4743c8,4),(0x473fa0,3),(0x473b38,3),(0x473be0,2),(0x473c50,4),(0x473d30,5),(0x473e48,3),(0x473ef0,3)]
    popups=[{'address':f'0x{address:08x}','items':[extra_item(address+i*56,i) for i in range(count)]} for address,count in popup_tables]
    course_extras=[extra_item(address,i) for i,address in enumerate([0x469130,0x469168,0x4690f8])]
    write('menu-items.json',{'screens':screens,'item_count':sum(len(s['items']) for s in screens),'popups':popups,'course_extras':course_extras,'note':'Static screen, popup and extra-item records; runtime-dependent entry/handler transitions remain pending original capture.'})
    write('control-tables.json',{'keyboard_words':exe.words(0x466618,11,'H'),'joystick_words':exe.words(0x466630,11,'H'),'actions':[strings.get(256+i,'[missing]') for i in range(9)],'note':'Original internal bindings, not SDL scancodes. Axis/button and held/edge semantics require original capture.'})
    bike_names=[0x70,0x71,0x72,0x74,0x73,0x7a,0x7d,0x7b,0x7e,0x7c,0x75,0x78,0x76,0x79,0x77]
    write('bike-records.json',{'bikes':[{'id':i,'legacy_footer_string':strings.get(slot,'[missing]'),'name_slot':slot,'price':price,'availability':'purchase allowed when cash plus half current bike value covers price; upstream-reported career rule'} for i,(slot,price) in enumerate(zip(bike_names,exe.words(0x468fc0,15)))],'character_start_cash':exe.words(0x469000,9,'H'),'character_rider_map':[8,7,2,5,4,6,3,1],'default_bikes_by_rider_then_level':list(exe.read(0x4641c8,50))})
    slot_to_bike=[0,1,2,4,3,10,12,14,11,13,5,7,9,6,8]
    stems=json.loads((ROOT/'docs/baseline/actors.json').read_text())
    prices=exe.words(0x468fc0,15)
    bikes=[]
    for screen in screens:
        if screen['id'] not in (21,22,23):
            continue
        for item in screen['items']:
            slot=item['description_id']
            if not 128 <= slot <= 142:
                continue
            index=slot-128
            bike=slot_to_bike[index]
            stem=stems['bike'][index]['asset_stem']
            bikes.append({'bike_id':bike,'spec_resource_id':bike+1,'showroom_slot':slot,'display_name':item['caption'],'showroom_label':strings.get(slot,''),'category':stem.split('/')[0],'source_image':'IMAGES/BIKES/'+stem.upper()+'.RRI','price':prices[bike],'eligibility':'same-bike purchase returns already-owned; otherwise cash plus half current bike price must cover purchase price','eligibility_confidence':'upstream-reported'})
    character_map=[8,7,2,5,4,6,3,1]
    level_bikes=list(exe.read(0x4641c8,50))
    cash=exe.words(0x469000,9,'H')
    characters=[]
    for index,item in enumerate(screens[14]['items'][:8]):
        stem=stems['character'][index]['asset_stem']
        rider=character_map[index]
        characters.append({'character_id':index+1,'display_name':item['caption'],'personality_id':rider,'starting_cash':cash[index+1],'bike_ids_by_level':level_bikes[rider*5:rider*5+5],'source_images':['IMAGES/CHARS/'+stem.upper()+'-'+m+'.RRI' for m in 'FNH'],'eligibility':'selection handler accepts IDs 1 through 8; offline characters have no additional unlock test in pinned native source','eligibility_confidence':'upstream-reported'})
    write('selectable-actors.json',{'bikes':sorted(bikes,key=lambda b:b['bike_id']),'characters':characters,'behavior_source':'src/career.cpp and src/frontend.cpp at pinned revision; runtime behavior remains unverified'})
    write('course-lengths.json',{'values_tenths_mile':exe.words(0x468f58,25),'note':'Menu distance labels, not finish coordinates. Runtime finish event inventory still required.'})
    return {'screens':len(screens),'items':sum(len(s['items']) for s in screens),'bike_records':15,'control_words':22}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('data',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    print(json.dumps(generate(args.data,args.output)))
if __name__=='__main__':
    main()
