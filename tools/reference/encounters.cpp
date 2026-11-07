// Independently authored public-API probes. No decompiled source or data embedded.
#include "race.hpp"
#include "assets.hpp"
#include "crash.hpp"
#include "actor_descriptors.hpp"
#include "traffic.hpp"
#include "family.hpp"
#include "resource.hpp"
#include "animation.hpp"
namespace rr {
using AttackCodes=std::array<std::array<std::array<int8_t,4>,2>,2>;
AttackCodes read_attack_codes(const std::vector<uint8_t>&);
}
#include <iostream>
#include <algorithm>
int main(int argc,char**argv){try{
    if(argc!=2)return 2;
    rr::Assets assets(argv[1]);
    auto tables=rr::read_physics_tables(assets.read("ROADRASH.EXE"));
    auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
    std::cout<<"{\"motion\":["; bool first=true;
    for(int speed:{-10000,-1,0,1200,5000,1000000})for(int acceleration:{-500,0,300})for(int heading:{0,0x100000,0x400000,0x800000,0xc00000,-0x100000}){
        rr::Bike actor; actor.i32(0x18)=1; actor.i32(0xec)=speed;actor.accel=acceleration;actor.i32(0x128)=heading;
        int forward=0;rr::advance_actor_motion(actor,tables,[&](int d){forward=d;});
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<speed<<","<<acceleration<<","<<heading<<","<<forward<<","<<actor.i32(0x28)<<"]";
    }
    std::cout<<"],\"damage\":[";first=true;
    rr::RouteSample sample{{0,0,500,500,512,512},0,0,0};
    std::vector<rr::RouteSample> route(2000,sample);
    for(int reason=0;reason<7;++reason)for(int amount:{0,1,7,32,100}){
        rr::Race race(route,route,tables,specs[0],75);
        auto& rider=race.riders()[0];rider.stamina.base=2500;rider.stamina.max=2500;rider.stamina.current=2500;
        race.knock_off(rider,amount,0,reason);
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<reason<<","<<amount<<","<<rider.stamina.max<<","<<rider.stamina.current<<"]";
    }
    std::cout<<"],\"attack_timing\":[";first=true;
    rr::ActorAnimationLibrary library(assets.read("ROADRASH.EXE"),assets.read("DATA/GLOBAL.RSC"));
    const auto codes=rr::read_attack_codes(assets.read("ROADRASH.EXE"));
    for(int side=0;side<2;++side)for(int kind=0;kind<2;++kind)for(int weapon=0;weapon<4;++weapon){
        const int8_t code=codes[size_t(side)][size_t(kind)][size_t(weapon)];
        rr::ActorAnimationState state;
        rr::set_animation_with_fallback(state,library.playback[0],code,0,library.fallback);
        rr::queue_animation_with_fallback(state,library.playback[0],2,library.fallback);
        int duration=1;
        for(const auto& frame:*library.playback[0][size_t(state.animation)])duration+=int(frame.duration);
        for(int age=0;age<duration;++age){
            const auto* selected=rr::advance_animation(state,library.playback[0],uint32_t(age));
            const bool active=(selected && (selected->flags & 0x600000)) || state.frame==2;
            if(!first){std::cout<<",";}first=false;
            std::cout<<"["<<side<<","<<kind<<","<<weapon<<","<<age<<","<<(active?1:0)<<"]";
        }
    }
    std::cout<<"],\"crash_entry\":[";first=true;
    const auto crash_config=rr::read_crash_config(assets.read("ROADRASH.EXE"));
    for(int reason:{1,2,4,5})for(int speed:{0,5000,20000}){
        rr::Bike bike;bike.i32(0xec)=speed;bike.i32(0xf0)=123;bike.i32(0xf4)=50;
        bike.i32(0x20)=100;bike.i32(0x16c)=1000;
        rr::CrashActor body;rr::init_crash_actor(body,crash_config);
        const std::array<int32_t,3> rider=reason==4?std::array<int32_t,3>{-80,0,0}:std::array<int32_t,3>{-256,0,1792};
        const std::array<int32_t,3> machine=reason==4?std::array<int32_t,3>{0,0,0}:std::array<int32_t,3>{0,0,1024};
        for(int repeat=0;repeat<2;++repeat){
            const bool entered=rr::begin_crash(bike,body,crash_config,library.playback[0],library.fallback,uint32_t(repeat*12),reason,rider,machine);
            if(!first){std::cout<<",";}first=false;
            std::cout<<"["<<reason<<","<<speed<<","<<repeat<<","<<(entered?1:0)<<","<<bike.i32(0xec)<<","<<bike.i32(0x20)<<","<<bike.i32(0xf4)<<","<<body.motion.i32(0xec)<<","<<body.motion.i32(0x20)<<","<<body.motion.i32(0xf4)<<","<<int(body.animation.animation)<<"]";
        }
    }
    std::cout<<"],\"damage_transaction\":[";first=true;
    for(int reason=0;reason<7;++reason)for(int amount:{0,1,7,32,100})for(int condition:{1,10,100})for(int health:{1,2500}){
        rr::Race race(route,route,tables,specs[0],75);
        auto& rider=race.riders()[0];rider.stamina.base=2500;rider.stamina.max=2500;rider.stamina.current=health;rider.stamina.strength=7;rider.bike.i32(0x2f0)=condition;
        const bool down=race.knock_off(rider,amount,amount+3,reason);
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<reason<<","<<amount<<","<<condition<<","<<health<<","<<rider.stamina.max<<","<<rider.stamina.current<<","<<rider.bike.i32(0x2f0)<<","<<(down?1:0)<<"]";
    }
    std::cout<<"],\"recovery_motion\":[";first=true;
    // Isolate horizontal motion: fixed dt=1, no impulses/acceleration/drag.
    // Positive speed is a control, negative speeds are crash-entry throw values.
    for(int speed:{-256,-80,5000}){
        rr::Bike body;body.i32(0x18)=1;body.i32(0xec)=speed;body.accel=0;
        int progress=1000;
        for(int tick=1;tick<=3;++tick){
            rr::advance_actor_motion(body,tables,[&](int delta){progress+=delta;});
            if(!first){std::cout<<",";}first=false;
            std::cout<<"["<<speed<<","<<tick<<","<<progress<<","<<body.i32(0xec)<<"]";
        }
    }
    std::cout<<"],\"recovery_drag\":[";first=true;
    for(int initial:{-10000,-256,-80,0,1200,5000,30000})for(int contact:{0,74})for(int surface=0;surface<4;++surface){
        rr::Bike body;body.i32(0x18)=1;body.i32(0xec)=initial;body.i32(0x150)=130;body.i32(0x154)=70;body.i32(0x160)=contact;
        for(int tick=1;tick<=30;++tick){rr::integrate_speed(body,tables,surface);
            if(!first){std::cout<<",";}first=false;
            std::cout<<"["<<initial<<","<<contact<<","<<surface<<","<<tick<<","<<body.i32(0xec)<<","<<body.accel<<","<<body.i32(0x15c)<<"]";
        }
    }
    std::cout<<"],\"fight_impact\":[";first=true;
    for(int side=0;side<2;++side)for(int kind=0;kind<2;++kind)for(int weapon=0;weapon<4;++weapon)for(int level:{0,4})for(int edge:{-1,0,1})for(int gap:{0,63,64}){
        if(weapon==3 && kind==1)continue;
        rr::Opponent opponent{specs[0],75,0,0,0,1};
        rr::Race race(route,route,tables,specs[0],75,{opponent},level);
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(-1);
        for(int actor=0;actor<2;++actor){auto& r=race.riders()[size_t(actor)];r.ai=false;r.frozen=actor==1;r.stamina.strength=7;r.stamina.base=r.stamina.max=r.stamina.current=2500;
            r.bike.i32(0x18)=1;r.bike.i32(0xec)=0;r.bike.i32(0x1c)=1000+(actor?gap:0);r.bike.i32(0x28)=actor?(side?1:-1)*(rr::weapon_reach[weapon]*4096+edge):0;r.bike.i32(0x2f0)=150;r.sample=r.previous_sample=3;
        }
        race.riders()[0].weapon=weapon==3?0:weapon;
        int hit=-1;
        for(int tick=0;tick<70;++tick){race.step(tick==0?(weapon==3?rr::control_kick:(kind?rr::control_backhand:rr::control_punch)):0);
            if(race.riders()[1].stamina.current<2500){hit=tick;break;}
        }
        const auto& t=race.riders()[1];
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<side<<","<<kind<<","<<weapon<<","<<level<<","<<edge<<","<<gap<<","<<hit<<","<<t.stamina.max<<","<<t.stamina.current<<","<<t.bike.i32(0x2f0)<<"]";
    }
    std::cout<<"],\"fight_sequences\":[";first=true;int fight_id=0;
    for(int side:{0,1})for(int kind:{0,1})for(int weapon:{0,1,2,3})for(int mode:{0,1,2})for(int dynamic:{0,1,2,3})for(uint32_t seed:{3u,4u}){
        if(weapon==3 && kind==1)continue;
        rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(route,route,tables,specs[0],75,{opponent},0);
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);
        for(auto& r:race.riders()){r.ai=false;r.stamina.strength=1;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.bike.i32(0x18)=1;r.bike.i32(0x2f0)=150;}
        race.riders()[0].weapon=weapon==3?0:weapon;
        const int id=fight_id++;
        for(int tick=0;tick<160;++tick){
            race.set_ticks(tick);race.set_rng_state(seed);
            const int gap=(dynamic==1 && tick<12)||(dynamic==2 && tick>=12 && tick<40)?64:0;
            const int lateral=(side?1:-1)*((dynamic==3 && tick>=12)?-18000:18000);
            for(int actor=0;actor<2;++actor){auto& r=race.riders()[size_t(actor)];r.sample=r.previous_sample=3;r.bike.i32(0x1c)=1000+(actor?gap:0);r.bike.i32(0x28)=actor?lateral:0;r.bike.i32(0xec)=0;r.bike.i32(0x20)=0;r.bike.i32(0x288)=0;}
            bool input=tick==0 || mode==1 || (mode==2 && tick%40==0);
            race.fight(race.riders()[0],input?(weapon==3?rr::control_kick:kind?rr::control_backhand:rr::control_punch):0,0);
            race.fight(race.riders()[1],0,1);
            for(int actor=0;actor<2;++actor)race.advance_ride_animation(race.riders()[size_t(actor)],actor==0);
            const auto& a=race.riders()[0];const auto& t=race.riders()[1];
            if(!first){std::cout<<",";}first=false;
            std::cout<<"["<<id<<","<<side<<","<<kind<<","<<weapon<<","<<mode<<","<<dynamic<<","<<tick<<","<<seed<<","<<t.stamina.max<<","<<t.stamina.current<<","<<t.bike.i32(0x2f0)<<","<<t.hit_until<<","<<int(a.ride_animation.animation)<<","<<int(a.ride_animation.frame)<<"]";
        }
    }
    std::cout<<"],\"strike_transactions\":[";first=true;int case_id=0;
    for(int side=0;side<2;++side)for(int kind=0;kind<2;++kind)for(int weapon=0;weapon<4;++weapon)for(int deadline:{99,100,101})for(int reaction:{0,1})for(int health:{64,2500})for(int condition:{1,150}){
        if(weapon==3 && kind==1)continue;
        rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(route,route,tables,specs[0],75,{opponent},0);
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(100);
        for(int actor=0;actor<2;++actor){auto& r=race.riders()[size_t(actor)];r.ai=false;r.stamina.strength=7;r.stamina.base=r.stamina.max=2500;r.stamina.current=actor?health:2500;
            r.bike.i32(0x18)=1;r.bike.i32(0x1c)=1000;r.bike.i32(0x28)=actor?(side?18000:-18000):0;r.bike.i32(0xec)=5000;r.bike.i32(0x2f0)=actor?condition:150;r.bike.i32(0x288)=0;
            rr::set_animation_with_fallback(r.ride_animation,library.playback[0],actor&&reaction?9:2,100,library.fallback);
        }
        auto& a=race.riders()[0];auto& t=race.riders()[1];a.attack.weapon=weapon;a.attack.right=side!=0;a.attack.backhand=kind!=0;t.hit_until=deadline;
        race.resolve_hit(a,0);
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<case_id++<<","<<side<<","<<kind<<","<<weapon<<","<<deadline<<","<<reaction<<","<<health<<","<<condition<<","<<t.stamina.max<<","<<t.stamina.current<<","<<t.bike.i32(0x2f0)<<","<<t.bike.i32(0xec)<<","<<t.bike.i32(0x288)<<","<<(t.bike.i32(0x2f4)!=0?1:0)<<","<<int(t.crash_active?t.crash_actor.animation.animation:(t.contact_animation>=0?t.contact_animation:t.ride_animation.animation))<<","<<t.hit_until<<","<<a.bike.i32(0xec)<<","<<a.bike.i32(0x288)<<"]";
    }
    std::cout<<"],\"weapon_theft\":[";first=true;
    for(int side=0;side<2;++side)for(int weapon:{1,2})for(int target_side=0;target_side<2;++target_side)for(int frame:{0,2})for(int deadline:{99,100})for(int bits:{0,3}){
        rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(route,route,tables,specs[0],75,{opponent},0,0,{},assets.read("ROADRASH.EXE"));
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(100);race.set_rng_state(bits==3?4:3);
        auto& a=race.riders()[0];auto& t=race.riders()[1];
        for(int actor=0;actor<2;++actor){auto& r=race.riders()[size_t(actor)];r.ai=false;r.stamina.strength=7;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.bike.i32(0x1c)=1000;r.bike.i32(0x28)=actor?(side?18000:-18000):0;}
        a.weapon=0;a.attack.weapon=0;a.attack.right=side!=0;t.weapon=weapon;t.attack.weapon=weapon;t.attack.timer=1;t.steal_until=deadline;
        rr::set_animation_with_fallback(a.ride_animation,library.playback[0],codes[size_t(side)][0][0],100,library.fallback);
        rr::set_animation_with_fallback(t.ride_animation,library.playback[0],codes[size_t(target_side)][0][size_t(weapon)],100,library.fallback);t.ride_animation.frame=int8_t(frame);
        race.resolve_hit(a,0);const bool stolen=a.weapon!=0;
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<side<<","<<weapon<<","<<target_side<<","<<frame<<","<<deadline<<","<<bits<<","<<a.weapon<<","<<t.weapon<<","<<a.steal_until<<","<<t.stamina.current<<","<<int(a.ride_animation.animation)<<","<<int(t.contact_animation>=0?t.contact_animation:t.ride_animation.animation)<<","<<(stolen?int(a.ride_animation.queued):-99)<<","<<(stolen?int(t.ride_animation.queued):-99)<<"]";
    }
    std::cout<<"],\"separation\":[";first=true;int sep_id=0;
    for(int type:{4,8})for(int partner:{4,0x10,1})for(int axis:{0,1,2,3})for(int speed:{-1000,0,5000,11000,11001,30000})for(int side:{-1,1})for(int short_box:{0,1}){
        rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(route,route,tables,specs[0],75,{opponent},0);
        auto& a=race.riders()[0];auto& b=race.riders()[1];
        int id=sep_id++;
        for(int i=0;i<2;++i){auto& r=race.riders()[size_t(i)];auto& p=r.bike;r.sample=r.previous_sample=10;
            p.i32(0x18)=1;p.i32(0x1c)=2560+i*20;p.i32(0x28)=i*side*4000;p.i32(0x20)=i*2000;
            p.i32(0x30)=p.i32(0x1c);p.i32(0x3c)=p.i32(0x28);p.i32(0x34)=p.i32(0x20);
            p.i32(0x174)=i&&partner!=4?16384:7680;p.i32(0x178)=64;p.i32(0x17c)=i&&short_box?4096:8960;
            p.i32(0x108)=i?5000:speed;p.i32(0x10c)=i?-700:1200;p.i32(0x110)=i?300:-100;
            p.i32(0x194)=i?(partner==1?0x1000000:partner==0x10?1800:256):256;
            p.i32(0xc4)=i?partner:type;
        }
        if(axis==0)a.bike.i32(0x30)=b.bike.i32(0x30)+side*129;
        if(axis==1)a.bike.i32(0x3c)=b.bike.i32(0x3c)+side*(a.bike.i32(0x174)+b.bike.i32(0x174)+1);
        if(axis==2)a.bike.i32(0x34)=b.bike.i32(0x34)+side*22000;
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<id<<","<<type<<","<<partner;
        const int in_offsets[]={0x1c,0x28,0x20,0x30,0x3c,0x34,0x178,0x174,0x17c,0x108,0x10c,0x110,0x194};
        for(const auto* r:{&a,&b})for(int off:in_offsets)std::cout<<","<<r->bike.i32(off);
        race.separate(a,uint32_t(type),b,uint32_t(partner));
        const int out_offsets[]={0x1c,0x28,0x20,0x30,0x3c,0x34,0x184,0x188,0x18c};
        for(const auto* r:{&a,&b}){for(int off:out_offsets)std::cout<<","<<r->bike.i32(off);std::cout<<","<<int(r->bike.u8(0x19c));}
        std::cout<<"]";
    }
    std::cout<<"],\"bike_contacts\":[";first=true;int contact_id=0;
    for(int axis:{0,1,2,3})for(int speed:{-1000,0,5000,10999,11000,11001,20000,30000})for(int side:{-1,1})for(int offset:{0,3839,3840,10000})for(int fallen:{0,1})for(int crashed:{0,1})for(int condition:{1,150}){
        rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(route,route,tables,specs[0],75,{opponent},0);
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(100);
        auto& a=race.riders()[0];auto& b=race.riders()[1];
        for(int i=0;i<2;++i){auto& r=race.riders()[size_t(i)];auto& p=r.bike;r.ai=false;r.sample=r.previous_sample=10;
            r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
            const bool down=i?fallen:crashed;const auto& dims=down?crash_config.bike_dimensions:crash_config.normal_dimensions;
            p.i32(0x18)=1;p.i32(0x1c)=2560+i*20;p.i32(0x28)=i*side*offset;p.i32(0x20)=i*2000;
            p.i32(0x30)=p.i32(0x1c);p.i32(0x3c)=p.i32(0x28);p.i32(0x34)=p.i32(0x20);
            for(int j=0;j<3;++j)p.i32(0x174+j*4)=dims[size_t(j)];
            p.i32(0xec)=p.i32(0x108)=i?5000:speed;p.i32(0xf0)=p.i32(0x10c)=i?-700:1200;p.i32(0xf4)=p.i32(0x110)=i?300:-100;
            p.i32(0x2f4)=down?1:0;p.i32(0x2f0)=condition;p.i32(0x288)=500;p.i32(0xc4)=4;
        }
        if(axis==0)a.bike.i32(0x30)=b.bike.i32(0x30)+side*129;
        if(axis==1)a.bike.i32(0x3c)=b.bike.i32(0x3c)+side*(a.bike.i32(0x174)+b.bike.i32(0x174)+1);
        if(axis==2)a.bike.i32(0x34)=b.bike.i32(0x34)+side*22000;
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<contact_id++<<","<<axis<<","<<speed<<","<<side<<","<<fallen<<","<<crashed<<","<<condition;
        const int in_offsets[]={0x1c,0x28,0x20,0x30,0x3c,0x34,0x178,0x174,0x17c,0x108,0x10c,0x110,0x194};
        for(const auto* r:{&a,&b})for(int off:in_offsets)std::cout<<","<<r->bike.i32(off);
        race.separate(a,4,b,4);race.contact(a,b);race.contact(b,a);
        const int out_offsets[]={0x1c,0x28,0x20,0xec,0xf0,0xf4,0x288,0x2f0};
        for(const auto* r:{&a,&b}){for(int off:out_offsets)std::cout<<","<<r->bike.i32(off);
            std::cout<<","<<(r->bike.i32(0x2f4)!=0?1:0)<<","<<r->stamina.max<<","<<r->stamina.current;
            std::cout<<","<<(r->crash_active?int(r->crash_actor.animation.animation):-1);
            for(int off:{0xec,0x20,0xf4})std::cout<<","<<(r->crash_active?r->crash_actor.motion.i32(off):0);
        }
        std::cout<<"]";
    }
    std::cout<<"],\"car_contacts\":[";first=true;int car_contact_id=0;
    for(int axis:{0,1,2,3})for(int speed:{-1000,0,5000,10999,11000,11001,20000,30000})for(int side:{-1,1})for(int offset:{0,12543,12544,20000})for(int car_speed:{0,5000,15000})for(int direction:{-1,1})for(int crashed:{0,1})for(int condition:{1,150}){
        rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(route,route,tables,specs[0],75,{opponent},0);
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(100);
        auto& a=race.riders()[0];auto& b=race.riders()[1];
        for(int i=0;i<2;++i){auto& r=race.riders()[size_t(i)];auto& p=r.bike;r.ai=false;r.sample=r.previous_sample=10;
            r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
            const bool down=i?false:crashed;const auto& dims=down?crash_config.bike_dimensions:crash_config.normal_dimensions;
            p.i32(0x18)=1;p.i32(0x1c)=2560+i*20;p.i32(0x28)=i*side*offset;p.i32(0x20)=i*2000;
            p.i32(0x30)=p.i32(0x1c);p.i32(0x3c)=p.i32(0x28);p.i32(0x34)=p.i32(0x20);
            for(int j=0;j<3;++j)p.i32(0x174+j*4)=dims[size_t(j)];
            p.i32(0xec)=i?car_speed:speed;p.i32(0x108)=i?car_speed*direction:speed;p.i32(0xf0)=p.i32(0x10c)=i?0:1200;p.i32(0xf4)=p.i32(0x110)=i?0:-100;
            if(i){p.i32(0x174)=16384;p.i32(0x178)=64;p.i32(0x17c)=8960;p.i32(0x194)=1800;p.i32(0xc4)=16;}
            p.i32(0x2f4)=down?1:0;p.i32(0x2f0)=condition;p.i32(0x288)=500;p.i32(0xc4)=i?16:4;
        }
        if(axis==0)a.bike.i32(0x30)=b.bike.i32(0x30)+side*129;
        if(axis==1)a.bike.i32(0x3c)=b.bike.i32(0x3c)+side*(a.bike.i32(0x174)+b.bike.i32(0x174)+1);
        if(axis==2)a.bike.i32(0x34)=b.bike.i32(0x34)+side*22000;
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<car_contact_id++<<","<<axis<<","<<speed<<","<<side<<","<<car_speed*direction<<","<<crashed<<","<<condition;
        const int in_offsets[]={0x1c,0x28,0x20,0x30,0x3c,0x34,0x178,0x174,0x17c,0x108,0x10c,0x110,0x194};
        for(const auto* r:{&a,&b})for(int off:in_offsets)std::cout<<","<<r->bike.i32(off);
        race.separate(a,4,b,16);
        rr::TrafficCar car;car.position=b.bike.i32(0x1c);car.lateral=b.bike.i32(0x28);car.speed=car_speed;
        race.contact_car(a,car);
        const int out_offsets[]={0x1c,0x28,0x20,0xec,0xf0,0xf4,0x288,0x2f0};
        for(const auto* r:{&a}){for(int off:out_offsets)std::cout<<","<<r->bike.i32(off);
            std::cout<<","<<(r->bike.i32(0x2f4)!=0?1:0)<<","<<r->stamina.max<<","<<r->stamina.current;
            std::cout<<","<<(r->crash_active?int(r->crash_actor.animation.animation):-1);
            for(int off:{0xec,0x20,0xf4})std::cout<<","<<(r->crash_active?r->crash_actor.motion.i32(off):0);
        }
        for(int off:{0x1c,0x28,0x20})std::cout<<","<<b.bike.i32(off);
        std::cout<<"]";
    }
    std::cout<<"],\"static_contacts\":[";first=true;int static_contact_id=0;
    for(int axis:{0,1,2,3})for(int speed:{-1000,0,5000,10999,11000,11001,20000,30000})for(int side:{-1,1})for(int offset:{0,12543,12544,20000})for(int object_height:{4096,17920,50000})for(int crashed:{0,1})for(int condition:{1,150}){
        rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(route,route,tables,specs[0],75,{opponent},0);
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(100);
        auto& a=race.riders()[0];auto& b=race.riders()[1];
        for(int i=0;i<2;++i){auto& r=race.riders()[size_t(i)];auto& p=r.bike;r.ai=false;r.sample=r.previous_sample=10;
            r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
            const bool down=i?false:crashed;const auto& dims=down?crash_config.bike_dimensions:crash_config.normal_dimensions;
            p.i32(0x18)=1;p.i32(0x1c)=2560+i*20;p.i32(0x28)=i*side*offset;p.i32(0x20)=i*2000;
            p.i32(0x30)=p.i32(0x1c);p.i32(0x3c)=p.i32(0x28);p.i32(0x34)=p.i32(0x20);
            for(int j=0;j<3;++j)p.i32(0x174+j*4)=dims[size_t(j)];
            p.i32(0xec)=i?0:speed;p.i32(0x108)=i?0:speed;p.i32(0xf0)=p.i32(0x10c)=i?0:1200;p.i32(0xf4)=p.i32(0x110)=i?0:-100;
            if(i){p.i32(0x174)=16384;p.i32(0x178)=64;p.i32(0x17c)=object_height;p.i32(0x194)=0x1000000;p.i32(0xc4)=1;}
            p.i32(0x2f4)=down?1:0;p.i32(0x2f0)=condition;p.i32(0x288)=500;p.i32(0xc4)=i?1:4;
        }
        if(axis==0)a.bike.i32(0x30)=b.bike.i32(0x30)+side*129;
        if(axis==1)a.bike.i32(0x3c)=b.bike.i32(0x3c)+side*(a.bike.i32(0x174)+b.bike.i32(0x174)+1);
        if(axis==2)a.bike.i32(0x34)=b.bike.i32(0x34)+side*22000;
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<static_contact_id++<<","<<axis<<","<<speed<<","<<side<<","<<object_height<<","<<crashed<<","<<condition;
        const int in_offsets[]={0x1c,0x28,0x20,0x30,0x3c,0x34,0x178,0x174,0x17c,0x108,0x10c,0x110,0x194};
        for(const auto* r:{&a,&b})for(int off:in_offsets)std::cout<<","<<r->bike.i32(off);
        race.separate(a,4,b,1);race.contact_static(a,b);
        const int out_offsets[]={0x1c,0x28,0x20,0xec,0xf0,0xf4,0x288,0x2f0};
        for(const auto* r:{&a}){for(int off:out_offsets)std::cout<<","<<r->bike.i32(off);
            std::cout<<","<<(r->bike.i32(0x2f4)!=0?1:0)<<","<<r->stamina.max<<","<<r->stamina.current;
            std::cout<<","<<(r->crash_active?int(r->crash_actor.animation.animation):-1);
            for(int off:{0xec,0x20,0xf4})std::cout<<","<<(r->crash_active?r->crash_actor.motion.i32(off):0);
        }
        for(int off:{0x1c,0x28,0x20})std::cout<<","<<b.bike.i32(off);
        std::cout<<"]";
    }
    std::cout<<"],\"scenery_boxes\":[";first=true;
    const auto families=assets.read("DATA/FAMILIES.RSC");
    for(const auto& resource:rr::read_resources(families)){
        if(resource.tag!="FAM ")continue;
        const std::vector<uint8_t> payload(families.begin()+resource.offset,families.begin()+resource.offset+resource.size);
        const auto family=rr::read_family(payload);
        for(const auto& [offset,node]:family.nodes){
            if(node.tag!=0x5252414e)continue;
            const auto animation=rr::read_animation(family.bytes,offset);
            const auto& boxes=animation.frames[std::min(size_t(2),animation.frames.size()-1)].boxes;
            if(!first){std::cout<<",";}first=false;
            std::cout<<"["<<resource.id<<","<<offset<<","<<boxes.size();
            for(const auto& box:boxes)for(int value:box)std::cout<<","<<value;
            std::cout<<"]";
        }
    }
    std::cout<<"],\"building_walls\":[";first=true;int wall_id=0;
    for(int side:{-1,1})for(int depth:{-1,0,1,14,45,60,61,200})for(int fraction:{0,128,255})for(int speed:{0,2499,2500,12000})for(int rate:{-7000,0,7000})for(int old_flags:{0,8,16})for(int flags:{0,8,16})for(int old_selector:{0,255})for(int selector:{0,255}){
        auto samples=route; samples[9].local=9;samples[10].local=10; samples[10].path.marking_flags=uint8_t(flags);samples[9].path.marking_flags=uint8_t(old_flags);
        samples[10].path.shoulder_left=samples[10].path.shoulder_right=250;
        samples[9].path.shoulder_left=samples[9].path.shoulder_right=250;
        rr::Race race(samples,samples,tables,specs[0],75);
        rr::TerrainSlice current,previous;current.type=previous.type=0;current.selectors={uint8_t(selector),uint8_t(selector)};previous.selectors={uint8_t(old_selector),uint8_t(old_selector)};
        race.set_terrain_source([&](const rr::RouteSample& sample)->const rr::TerrainSlice& {return sample.local==9?previous:current;});
        // Give the synthetic samples distinct coordinates for the terrain callback.
        // Race owns copies; sample.local is an authored identity, not game data.
        race.set_crash_library(library,crash_config);race.set_ticks(100);
        auto& r=race.riders()[0];r.ai=false;r.sample=10;r.previous_sample=9;
        auto& b=r.bike;b.i32(0x1c)=2560+fraction;b.i32(0x30)=2304+fraction;
        const int lateral=side*(750*256-7680+depth*256);
        b.i32(0x28)=b.i32(0x3c)=lateral;b.i32(0xec)=speed;b.i32(0xf0)=rate;b.i32(0x288)=500;b.i32(0xf4)=0;b.i32(0x20)=0;
        race.walls(r);
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<wall_id++<<","<<side<<","<<depth<<","<<fraction<<","<<speed<<","<<rate<<","<<old_flags<<","<<flags<<","<<old_selector<<","<<selector;
        for(int off:{0x1c,0x28,0xec,0xf0,0x288,0x20,0xf4})std::cout<<","<<b.i32(off);
        std::cout<<","<<(b.i32(0x2f4)!=0?1:0)<<","<<(r.crash_active?int(r.crash_actor.animation.animation):-1);
        for(int off:{0x1c,0x28,0x20,0xf4})std::cout<<","<<(r.crash_active?r.crash_actor.motion.i32(off):0);
        std::cout<<"]";
    }
    std::cout<<"],\"recovery_trajectory\":[";first=true;int recovery_id=0;
    for(int initial:{-1000,0,5000,15000})for(int reason:{1,2,4,5})for(int rise:{-16,0,16})for(int height:{0,20000})for(uint32_t roll:{0u,1u,2u,3u})for(int air_side:{-1,0,1}){
        if(initial<0 && roll!=0)continue;
        if(air_side && (initial<=0 || rise!=0 || height!=20000 || roll!=0))continue;
        auto recovery_route=route;for(auto& sample:recovery_route)sample.path.rise=int8_t(rise);
        rr::Race race(recovery_route,recovery_route,tables,specs[0],75);
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(0);
        auto& r=race.riders()[0];auto& b=r.bike;r.ai=false;r.sample=r.previous_sample=10;
        b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=2560;b.i32(0x28)=b.i32(0x3c)=0;b.i32(0xec)=initial;b.i32(0xf0)=b.i32(0xf4)=0;b.i32(0x2f0)=150;b.i32(0x20)=b.i32(0x34)=height;
        race.crash_reason(r,reason,reason==4?std::array<int32_t,3>{-80,0,0}:std::array<int32_t,3>{-256,0,1792},reason==4?std::array<int32_t,3>{0,0,0}:std::array<int32_t,3>{0,0,1024});
        if(air_side){b.i32(0x160)=0;b.i32(0x104)=air_side*1800;}
        const int id=recovery_id++;
        for(int tick=1;tick<=2500;++tick){
            race.set_ticks(tick);
            if(b.i32(0x2f4)==1 || b.i32(0x2f4)==2)
                rr::advance_crash_bike_animation(r.crash_bike_animation,library.playback[0],library.fallback,b.i32(0xec),uint32_t(tick),[&]{return roll;});
            rr::CrashRiderAnimationInput input;input.crash=b.i32(0x2f4);input.speed=r.crash_actor.motion.i32(0xec);input.height_q8=r.crash_actor.motion.i32(0x20);
            rr::advance_crash_rider_animation(r.crash_actor.animation,library.playback[0],library.fallback,input,uint32_t(tick),[&]{return roll;});
            if(b.i32(0x2f4)!=input.crash)r.crash_timer=0;
            b.i32(0x2f4)=input.crash;if(input.crash==2)++r.crash_timer;
            race.crash_bike_motion(r,true);race.crash_body_motion(r,1);race.crash_recovery(r,true,0);
            if(!first){std::cout<<",";}first=false;
            std::cout<<"["<<id<<","<<initial<<","<<reason<<","<<tick<<","<<rise<<","<<height<<","<<roll<<","<<air_side;
            for(int off:{0x1c,0x28,0x20,0xec,0xf4,0x160})std::cout<<","<<b.i32(off);
            std::cout<<","<<(r.crash_active?1:0)<<","<<b.i32(0x2f4);
            for(int off:{0x1c,0x28,0x20,0xec,0xf4,0x160,0x128})std::cout<<","<<(r.crash_active?r.crash_actor.motion.i32(off):0);
            std::cout<<","<<(r.crash_active?int(r.crash_actor.animation.animation):-1)<<","<<(r.crash_active?int(r.crash_actor.animation.frame):0)<<","<<(r.crash_active?int(r.crash_actor.animation.loops):0)<<"]";
            if(!r.crash_active)break;
        }
    }
    std::cout<<"],\"recovery_regeneration\":[";first=true;
    for(int player:{0,1})for(int phase:{1,2,3})for(int health:{0,2498,2500}){
        rr::Race race(route,route,tables,specs[0],75);race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(1);
        auto& r=race.riders()[0];r.ai=false;r.sample=r.previous_sample=10;r.bike.i32(0x18)=1;r.bike.i32(0x1c)=2560;r.bike.i32(0xec)=5000;r.bike.i32(0x2f0)=150;
        race.crash_reason(r,1,{-256,0,1792},{0,0,1024});r.bike.i32(0x2f4)=phase;
        if(phase!=1){rr::set_animation_with_fallback(r.crash_actor.animation,library.playback[0],phase==2?36:41,1,library.fallback);r.crash_actor.animation.queued=phase==2?37:2;}
        r.stamina.base=r.stamina.max=2500;r.stamina.current=health;
        race.step_crash(r,player!=0,0);
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<player<<","<<phase<<","<<health<<","<<r.stamina.current<<"]";
    }
    std::cout<<"],\"half_contacts\":[";first=true;int half_id=0;
    for(int type:{1,2,4,8,16})for(int phase:{1,2,3})for(int speed:{-3000,0,500,501,6000,40000})for(int other_speed:{0,1699,1700,1701})for(int vertical:{0,2999,3000,12000})for(int animation:{33,37,41})for(int own:{0,1})for(uint32_t roll:{0u,32u}){
        if(own && type!=4)continue;
        rr::Race race(route,route,tables,specs[0],75);race.set_crash_library(library,crash_config);race.set_ticks(100);
        uint32_t seed=0;while((((seed*0x343fdu+0x269ec3u)>>16)&0x7fffu)%33!=roll)++seed;race.set_rng_state(seed);
        auto& owner=race.riders()[0];owner.ai=true;owner.bike.i32(0x18)=1;owner.bike.i32(0x2f4)=phase;
        rr::Rider detached,obstacle;auto& d=detached.bike;detached.sample=detached.previous_sample=10;
        d.i32(0x18)=1;d.i32(0x1c)=2560;d.i32(0x28)=0;d.i32(0x20)=100;d.i32(0xec)=speed;d.i32(0xf4)=vertical;d.i32(0x174)=tables.half_object[17];d.i32(0x17c)=tables.half_object[19];
        d.u8(0x19c)=7;d.i32(0x184)=1000;d.i32(0x188)=-2000;d.i32(0x18c)=500;
        auto& other=own?owner:obstacle;auto& o=other.bike;o.i32(0xc4)=type;o.i32(0xec)=other_speed;o.i32(0x20)=0;o.i32(0x28)=6000;
        o.i32(0x174)=type==16?16384:type==8?tables.half_object[17]:7680;
        o.i32(0x17c)=own?crash_config.bike_dimensions[2]:type==1?20000:type==8?tables.half_object[19]:8960;
        rr::set_animation_with_fallback(owner.crash_actor.animation,library.playback[0],int8_t(animation),100,library.fallback);owner.crash_actor.animation.loops=2;
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<half_id++<<","<<type<<","<<phase<<","<<speed<<","<<other_speed<<","<<vertical<<","<<animation<<","<<own<<","<<roll<<","<<o.i32(0x174)<<","<<o.i32(0x17c);
        race.contact_half_static(detached,owner,other);
        for(int off:{0x1c,0x28,0x20,0xec,0xf4})std::cout<<","<<d.i32(off);
        std::cout<<","<<owner.bike.i32(0x2f4)<<","<<int(owner.crash_actor.animation.animation)<<","<<int(owner.crash_actor.animation.queued)<<","<<int(owner.crash_actor.animation.frame)<<","<<int(owner.crash_actor.animation.loops)<<","<<int(d.u8(0x19c));
        for(int off:{0x184,0x188,0x18c})std::cout<<","<<d.i32(off);
        std::cout<<"]";
    }
    std::cout<<"],\"proxy_pairs\":[";first=true;int proxy_id=0;
    for(int role_a:{0,1})for(int role_b:{0,1})for(int own:{0,1})for(int phase:{1,2,3})for(int speed:{-256,1200,8000})for(int other_speed:{0,1700,1701,15000})for(int axis:{0,1,2,3})for(uint32_t seed:{1u,4u}){
        if(own && role_a==role_b)continue;
        rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(route,route,tables,specs[0],75,{opponent},0);
        race.set_ride_library(&library,&codes);race.set_crash_library(library,crash_config);race.set_ticks(100);race.set_rng_state(seed);
        for(int i=0;i<2;++i){auto& owner=race.riders()[size_t(i)];owner.ai=true;owner.sample=owner.previous_sample=10;
            owner.stamina.base=owner.stamina.max=owner.stamina.current=2500;owner.stamina.strength=7;
            owner.bike.i32(0x18)=1;owner.bike.i32(0x2f0)=150;owner.bike.i32(0x20)=owner.bike.i32(0xf4)=0;
        }
        auto& oa=race.riders()[0];auto& ob=race.riders()[size_t(own?0:1)];rr::Rider da,db;
        if(role_a || (own&&role_b)){oa.bike.i32(0x2f4)=phase;oa.crash_active=true;rr::set_animation_with_fallback(oa.crash_actor.animation,library.playback[0],int8_t(phase==1?33:phase==2?37:41),100,library.fallback);}
        if(role_b && !own){ob.bike.i32(0x2f4)=phase;ob.crash_active=true;rr::set_animation_with_fallback(ob.crash_actor.animation,library.playback[0],int8_t(phase==1?33:phase==2?37:41),100,library.fallback);}
        auto& a=role_a?da:oa;auto& b=role_b?db:ob;
        for(int i=0;i<2;++i){auto& r=i?b:a;auto& p=r.bike;const int role=i?role_b:role_a;const auto& owner=i?ob:oa;
            if(role){rr::init_bike_object(p,tables);p.i32(0x194)=tables.half_object[21];p.i32(0xc4)=8;for(int j=0;j<3;++j)p.i32(0x174+j*4)=tables.half_object[size_t(17+j)];}
            else if(owner.bike.i32(0x2f4)){for(int j=0;j<3;++j)p.i32(0x174+j*4)=crash_config.bike_dimensions[size_t(j)];}
            r.sample=r.previous_sample=10;p.i32(0x18)=1;p.i32(0x1c)=p.i32(0x30)=2560+i*20;p.i32(0x28)=p.i32(0x3c)=i*1000;p.i32(0x20)=p.i32(0x34)=0;
            p.i32(0xec)=p.i32(0x108)=i?other_speed:speed;p.i32(0xf0)=p.i32(0x10c)=0;p.i32(0xf4)=p.i32(0x110)=i?300:-100;p.i32(0x288)=role?0:500;
        }
        if(axis==0)a.bike.i32(0x30)=b.bike.i32(0x30)-a.bike.i32(0x178)-b.bike.i32(0x178)-1;
        if(axis==1)a.bike.i32(0x3c)=b.bike.i32(0x3c)-a.bike.i32(0x174)-b.bike.i32(0x174)-1;
        if(axis==2)a.bike.i32(0x34)=b.bike.i32(0x34)+22000;
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<proxy_id++<<","<<role_a<<","<<role_b<<","<<own<<","<<phase<<","<<speed<<","<<other_speed<<","<<axis<<","<<seed;
        const int input[]={0x1c,0x28,0x20,0x30,0x3c,0x34,0x178,0x174,0x17c,0x108,0x10c,0x110,0x194};
        for(const auto* r:{&a,&b})for(int off:input)std::cout<<","<<r->bike.i32(off);
        if(!own)race.separate(a,role_a?8:4,b,role_b?8:4);
        const auto respond=[&](rr::Rider& self,rr::Rider& owner,rr::Rider& other,int role){
            if(role)race.contact_half_static(self,owner,other);
            else if(own && self.bike.i32(0xec)<10 && self.bike.i32(0x20)==0){self.bike.i32(0x184)=self.bike.i32(0x188)=self.bike.i32(0x18c)=0;self.bike.u8(0x19c)=0;self.bike.i32(0xec)=std::clamp(int32_t(self.bike.i32(0xec)),-1000,30000);self.bike.i32(0xf4)=std::clamp(int32_t(self.bike.i32(0xf4)),-10000,1536);}
            else race.contact(self,other);
        };
        respond(a,oa,b,role_a);respond(b,ob,a,role_b);
        const auto output=[&](const rr::Rider& object,const rr::Rider& owner){
            for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x288})std::cout<<","<<object.bike.i32(off);
            std::cout<<","<<owner.bike.i32(0x2f0)<<","<<owner.stamina.max<<","<<owner.stamina.current<<","<<owner.bike.i32(0x2f4)<<","<<(owner.crash_active?int(owner.crash_actor.animation.animation):-1)<<","<<(owner.crash_active?int(owner.crash_actor.animation.queued):-1);
        };
        output(a,oa);output(b,ob);std::cout<<"]";
    }
    std::cout<<"],\"body_building_walls\":[";first=true;int body_wall_id=0;
    for(int side:{-1,1})for(int depth:{-1,0,1,14,45,60,61,200})for(int fraction:{0,128,255})for(int speed:{-1000,0,2499,2500,12000})for(int rate:{-7000,0,7000})for(int old_flags:{0,8,16})for(int flags:{0,8,16})for(int old_selector:{0,255})for(int selector:{0,255}){
        auto samples=route; samples[9].local=9;samples[10].local=10; samples[10].path.marking_flags=uint8_t(flags);samples[9].path.marking_flags=uint8_t(old_flags);
        samples[10].path.shoulder_left=samples[10].path.shoulder_right=250;
        samples[9].path.shoulder_left=samples[9].path.shoulder_right=250;
        rr::Race race(samples,samples,tables,specs[0],75);
        rr::TerrainSlice current,previous;current.type=previous.type=0;current.selectors={uint8_t(selector),uint8_t(selector)};previous.selectors={uint8_t(old_selector),uint8_t(old_selector)};
        race.set_terrain_source([&](const rr::RouteSample& sample)->const rr::TerrainSlice& {return sample.local==9?previous:current;});
        // Give the synthetic samples distinct coordinates for the terrain callback.
        // Race owns copies; sample.local is an authored identity, not game data.
        race.set_crash_library(library,crash_config);race.set_ticks(100);
        auto& r=race.riders()[0];r.ai=false;r.sample=10;r.previous_sample=9;
        auto& b=r.bike;b.i32(0x1c)=2560+fraction;b.i32(0x30)=2304+fraction;
        const int lateral=side*(750*256-tables.half_object[17]+depth*256);
        b.i32(0x28)=b.i32(0x3c)=lateral;b.i32(0xec)=speed;b.i32(0xf0)=rate;b.i32(0x288)=500;b.i32(0xf4)=0;b.i32(0x20)=0;b.i32(0xc4)=8;b.i32(0x174)=tables.half_object[17];b.i32(0x178)=tables.half_object[18];b.i32(0x17c)=tables.half_object[19];b.i32(0x108)=speed;
        race.walls(r);
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<body_wall_id++<<","<<side<<","<<depth<<","<<fraction<<","<<speed<<","<<rate<<","<<old_flags<<","<<flags<<","<<old_selector<<","<<selector;
        for(int off:{0x1c,0x28,0xec,0xf0,0x288,0x20,0xf4})std::cout<<","<<b.i32(off);
        std::cout<<","<<b.i32(0x5c);
        std::cout<<"]";
    }
    std::cout<<"]}\n";
}catch(std::exception& e){std::cerr<<e.what()<<"\n";return 1;}}
