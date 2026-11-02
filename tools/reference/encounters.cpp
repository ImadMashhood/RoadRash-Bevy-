// Independently authored public-API probes. No decompiled source or data embedded.
#include "race.hpp"
#include "assets.hpp"
#include "crash.hpp"
#include "actor_descriptors.hpp"
namespace rr {
using AttackCodes=std::array<std::array<std::array<int8_t,4>,2>,2>;
AttackCodes read_attack_codes(const std::vector<uint8_t>&);
}
#include <iostream>
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
    std::cout<<"]}\n";
}catch(std::exception& e){std::cerr<<e.what()<<"\n";return 1;}}
