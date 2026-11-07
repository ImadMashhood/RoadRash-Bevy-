// Actual native mixed bike/half callback walks from controlled recovery poses.
#include "race.hpp"
#include "assets.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include <iostream>
#include <cstdlib>
namespace rr {using AttackCodes=std::array<std::array<std::array<int8_t,4>,2>,2>;AttackCodes read_attack_codes(const std::vector<uint8_t>&);}

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
        std::cout << ",";
        animation(r.crash_active ? r.crash_actor.animation : r.ride_animation);
        std::cout << "]";
    }
    std::cout << "]]";
}
int main(int argc,char**argv) {try{
 if(argc!=2)return 2;
 rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);
 auto codes=rr::read_attack_codes(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(exe);
 rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));
 rr::RouteSample sample{{0,0,5000,5000,512,512},0,0,0};std::vector<rr::RouteSample>route(2000,sample);
 const bool full=std::getenv("RR_RECOVERY_MULTIWALK_FULL")!=nullptr;
 const std::vector<int>spacings=full?std::vector<int>{0,40,128}:std::vector<int>{0,40};
 const std::vector<int>laterals=full?std::vector<int>{2000,8000}:std::vector<int>{2000};
 const std::vector<int>reasons=full?std::vector<int>{1,4}:std::vector<int>{4};
 const std::vector<int>inputs=full?std::vector<int>{0,1,2}:std::vector<int>{0,2};
 bool first=true;int id=0;std::cout<<"[";
 for(int count:{3,4})for(int spacing:spacings)for(int lateral:laterals)for(int speed:{0,5000})for(int reason:reasons)for(int held:inputs){
  std::vector<rr::Opponent>opponents;for(int i=1;i<count;++i)opponents.push_back(rr::Opponent{specs[0],75,0,0,0,i});
  rr::Race race(route,route,tables,specs[0],75,opponents,0);race.set_ride_library(&library,&codes);race.set_crash_library(library,crash);race.set_live_schedule(true);race.set_ticks(0);race.set_rng_state(1);
  for(int i=0;i<count;++i){auto&r=race.riders()[size_t(i)];auto&b=r.bike;r.ai=false;r.frozen=false;r.sample=r.previous_sample=size_t((2688+i*spacing)>>8);r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;b.u8(0xc0)=7;b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=2688+i*spacing;b.i32(0x28)=b.i32(0x3c)=(i-1)*lateral;b.i32(0x20)=b.i32(0x34)=0;b.i32(0xec)=b.i32(0x108)=speed;b.i32(0xf0)=b.i32(0x10c)=0;b.i32(0xf4)=b.i32(0x110)=0;b.i32(0x2f0)=150;race.crash_reason(r,reason,reason==4?std::array<int32_t,3>{-80,0,0}:std::array<int32_t,3>{-256,0,1792},reason==4?std::array<int32_t,3>{0,0,0}:std::array<int32_t,3>{0,0,1024});}
  race.sort_schedule();race.update_camera();race.previous_camera_position_=race.camera_position();
  if(!first){std::cout<<",";}first=false;
  std::cout<<"{\"input\":["<<id++<<","<<count<<","<<spacing<<","<<lateral<<","<<speed<<","<<reason<<","<<held<<"],\"pre\":";state(race);std::cout<<",\"steps\":[";
  int all_remount=-1;std::vector<bool>remounted(size_t(count),false);
  for(int tick=1;tick<=3000;++tick){race.set_ticks(tick);for(auto&r:race.riders()){r.sched_last=r.half_sched_last=tick-1;r.bike.i32(0x18)=1;uint32_t input=&r==&race.riders()[0]?(held==1?rr::control_accelerate:held==2?rr::control_accelerate|rr::control_left|rr::control_punch:0):0;race.run_half_node(r,input,false);race.run_bike_node(r,input,false);}race.sort_schedule();race.collide_riders();race.sort_schedule();race.update_camera();race.previous_camera_position_=race.camera_position();if(tick>1)std::cout<<",";state(race);bool all=true;for(int i=0;i<count;++i){remounted[size_t(i)]=remounted[size_t(i)]||!race.riders()[size_t(i)].crash_active;all=all&&remounted[size_t(i)];}if(all&&all_remount<0)all_remount=tick;if(all_remount>=0&&tick>=all_remount+30)break;}
  std::cout<<"],\"first_all_remounted_tick\":"<<all_remount<<"}";
 }
 std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
