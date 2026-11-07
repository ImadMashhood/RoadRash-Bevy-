// Actual native mixed bike/half callback walks from controlled recovery poses.
#include "race.hpp"
#include "assets.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include <iostream>

void motion(const rr::Bike& b) {
    std::cout << "[";
    bool first=true;
    for(int off:{0x1c,0x28,0x20,0x30,0x3c,0x34,0xec,0xf0,0xf4,0x108,0x10c,0x110}) {
        if(!first) std::cout << ",";
        first=false; std::cout << b.i32(off);
    }
    std::cout << "]";
}
void animation(const rr::ActorAnimationState& a) {
    // The uninitialized native sentinel and Rust's default code0 both have no
    // selected descriptor. This canonicalizes that constructor representation.
    std::cout << "[" << (a.animation==-1&&!a.selected?0:int(a.animation)) << "," << int(a.frame) << "," << int(a.queued)
              << "," << int(a.loops) << ",";
    if(a.selected) std::cout << "[" << int(a.selected->at(0)) << "," << int(a.selected->at(1)) << "]";
    else std::cout << "null";
    std::cout << "]";
}
void state(rr::Race& race) {
    std::cout << "[" << race.rng_state() << ",[";
    bool first=true;
    for(const auto& r:race.riders()) {
        if(!first) std::cout << ",";
        first=false;
        std::cout << "["; motion(r.bike);
        std::cout << "," << r.bike.i32(0x288) << "," << r.bike.i32(0x2f0)
                  << "," << r.bike.i32(0x2f4) << "," << r.stamina.current << "," << r.stamina.max << ",";
        animation(r.crash_bike_animation);
        std::cout << ",";
        if(r.crash_active) {
            std::cout << "["; motion(r.crash_actor.motion); std::cout << ",";
            animation(r.crash_actor.animation); std::cout << "]";
        } else std::cout << "null";
        std::cout << "]";
    }
    std::cout << "]]";
}
int main(int argc,char**argv) { try {
    if(argc!=2) return 2;
    rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);
    auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(exe);
    rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));
    rr::RouteSample sample{{0,0,5000,5000,512,512},0,0,0};std::vector<rr::RouteSample>route(2000,sample);
    bool first=true;int id=0;std::cout<<"[";
    for(int code:{35,36,37,38,41,2})for(int order:{0,1})for(int speed:{5000,11000,11001,20000})
    for(int side:{-1,1})for(int condition:{1,150})for(int height:{4096,32768}) {
        rr::Race race(route,route,tables,specs[0],75,{},0);race.set_ride_library(&library,nullptr);
        race.set_crash_library(library,crash);race.set_live_schedule(true);race.set_ticks(0);race.set_rng_state(1);
        auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.frozen=false;r.sample=r.previous_sample=10;
        r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
        b.u8(0xc0)=7;b.i32(0x18)=1;b.i32(0x1c)=2688;b.i32(0x30)=2688-side*129;
        b.i32(0x28)=b.i32(0x3c)=0;b.i32(0x20)=b.i32(0x34)=0;
        b.i32(0xec)=b.i32(0x108)=speed;b.i32(0xf0)=b.i32(0x10c)=0;b.i32(0xf4)=b.i32(0x110)=0;
        b.i32(0x2f0)=condition;race.crash_reason(r,4,{0,0,0},{0,0,0});
        b.i32(0x20)=b.i32(0x34)=0;b.i32(0x2f4)=code==35?1:(code==41||code==2)?3:2;
        auto&m=r.crash_actor.motion;m.i32(0x20)=m.i32(0x34)=0;
        rr::set_animation_with_fallback(r.crash_actor.animation,library.playback[0],int8_t(code),0,library.fallback);
        r.crash_actor.animation.queued=-1;r.crash_actor.animation.selected=std::array<int8_t,2>{int8_t(code==2?41:code),0};
        rr::set_animation_with_fallback(r.crash_bike_animation,library.playback[0],48,0,library.fallback);
        r.crash_bike_animation.queued=-1;r.crash_bike_animation.selected=std::array<int8_t,2>{48,0};
        race.sync_half_mirror(r);
        if(!first) { std::cout<<","; }
        first=false;
        std::cout<<"{\"input\":["<<id++<<","<<code<<","<<order<<","<<speed<<","<<side<<","<<condition<<","<<height<<"],\"pre\":";
        state(race);std::cout<<",\"steps\":[";race.set_ticks(1);
        if(order==0)race.hit_static(r,0,{-8192,0,8192,height},nullptr);else race.crash_recovery(r,true,0);
        state(race);std::cout<<",";
        if(order==0)race.crash_recovery(r,true,0);else race.hit_static(r,0,{-8192,0,8192,height},nullptr);
        state(race);std::cout<<"]}";
    }
    std::cout<<"]\n";
} catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
