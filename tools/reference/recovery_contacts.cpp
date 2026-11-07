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
    rr::Assets assets(argv[1]); auto exe=assets.read("ROADRASH.EXE");
    auto tables=rr::read_physics_tables(exe); auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
    auto crash=rr::read_crash_config(exe);
    rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));
    rr::RouteSample sample{{0,0,5000,5000,512,512},0,0,0};
    std::vector<rr::RouteSample> route(2000,sample);
    bool first=true; int id=0; std::cout << "[";
    for(int code:{35,36,37,38,41}) for(int count:{3,4}) for(int down:{1,3,4})
    for(int speed:{0,1699,1700,1701,8000,20000}) for(int side:{-1,1}) for(int layout:{0,1}) {
        if(down>count) continue;
        std::vector<rr::Opponent> opponents;
        for(int i=1;i<count;++i) opponents.push_back(rr::Opponent{specs[0],75,0,0,0,i});
        rr::Race race(route,route,tables,specs[0],75,opponents,0);
        race.set_ride_library(&library,nullptr); race.set_crash_library(library,crash);
        race.set_live_schedule(true); race.set_ticks(0); race.set_rng_state(1);
        for(int i=0;i<count;++i) {
            auto& r=race.riders()[size_t(i)]; auto& b=r.bike;
            r.ai=false; r.frozen=false; r.sample=r.previous_sample=10;
            r.stamina.base=r.stamina.max=r.stamina.current=2500; r.stamina.strength=7;
            b.u8(0xc0)=7; b.i32(0x18)=1;
            b.i32(0x1c)=b.i32(0x30)=2688+i*15;
            b.i32(0x28)=b.i32(0x3c)=side*(i-1)*(layout?8000:2000);
            b.i32(0x20)=b.i32(0x34)=0;
            b.i32(0xec)=b.i32(0x108)=i%2?speed:-speed/2;
            b.i32(0xf0)=b.i32(0x10c)=0; b.i32(0xf4)=b.i32(0x110)=0;
            b.i32(0x2f0)=150;
            if(i<down) {
                race.crash_reason(r,1,{0,0,0},{0,0,0});
                auto& body=r.crash_actor.motion;
                body.i32(0x1c)=body.i32(0x30)=2688+i*15+(layout?20:-10);
                body.i32(0x28)=body.i32(0x3c)=b.i32(0x28)+side*(layout?-6000:1000);
                body.i32(0x20)=body.i32(0x34)=0;
                body.i32(0xec)=body.i32(0x108)=i%2?speed:-speed/2;
                body.i32(0xf0)=body.i32(0x10c)=0; body.i32(0xf4)=body.i32(0x110)=0;
                b.i32(0x20)=b.i32(0x34)=0;
                b.i32(0x2f4)=code==35?1:code==41?3:2;
                rr::set_animation_with_fallback(r.crash_actor.animation,library.playback[0],int8_t(code),0,library.fallback);
                r.crash_actor.animation.queued=-1;
                race.sync_half_mirror(r);
            }
        }
        if(!first) std::cout << ",";
        first=false;
        std::cout << "{\"input\":[" << id++ << "," << code << "," << count << "," << down
                  << "," << speed << "," << side << "," << layout << "],\"pre\":";
        state(race); std::cout << ",\"pre_camera\":" << race.camera_position() << ",\"steps\":[";
        std::vector<uint32_t> cameras;
        for(int pass=1;pass<=4;++pass) {
            race.set_ticks(pass);
            for(auto& r:race.riders()) { r.sched_last=pass; r.half_sched_last=pass; }
            race.sort_schedule();
            race.collide_riders();
            if(pass>1) std::cout << ",";
            state(race);
            cameras.push_back(race.camera_position());
        }
        std::cout << "],\"cameras\":[";
        for(size_t i=0;i<cameras.size();++i) { if(i) std::cout << ","; std::cout << cameras[i]; }
        std::cout << "]}";
    }
    std::cout << "]\n";
} catch(const std::exception& e) { std::cerr << e.what() << "\n"; return 1; } }
