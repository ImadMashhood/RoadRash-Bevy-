// Independently authored public-API probes. No decompiled source or data embedded.
#include "race.hpp"
#include "assets.hpp"
#include "crash.hpp"
#include "actor_descriptors.hpp"
#include "race_setup.hpp"
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
    std::cout<<"]}\n";
}catch(std::exception& e){std::cerr<<e.what()<<"\n";return 1;}}
