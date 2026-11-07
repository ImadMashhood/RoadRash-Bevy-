// Actual LIVE player think: backhand speed change precedes police_think.
#include "race.hpp"
#include "assets.hpp"
#include "police.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include <iostream>
namespace rr {using AttackCodes=std::array<std::array<std::array<int8_t,4>,2>,2>;AttackCodes read_attack_codes(const std::vector<uint8_t>&);}
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto ps=rr::read_personalities(exe);auto police=rr::read_police_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));auto codes=rr::read_attack_codes(exe);auto crash=rr::read_crash_config(exe);bool first=true;std::cout<<"[";
 for(int speed:{-1,0,299,300})for(int gap:{-512,-511,0,511,512})for(int active:{0,1})for(int phase:{0,1,2,3,4,5}){
 if(phase && active){continue;}
 rr::RouteSample sample{{0,0,500,500,512,512},0,0,0};std::vector<rr::RouteSample> route(2048,sample);rr::Opponent o;o.spec=specs[0];o.rider_value=75;o.base_speed=100;o.cur2_speed=60;o.slot=1;std::vector<rr::Opponent> opponents{o};rr::Race race(route,route,tables,specs[0],75,opponents,0,0,ps,exe);race.set_ride_library(&library,&codes);race.set_crash_library(library,crash);race.set_police(police,specs,&library);
 for(size_t n=0;n<race.riders().size();++n){auto&r=race.riders()[n];r.ai=false;r.sample=r.previous_sample=50;r.bike.i32(0x1c)=r.bike.i32(0x30)=12800+(n?63:0);r.bike.i32(0x28)=r.bike.i32(0x3c)=n?16000:0;r.bike.i32(0xec)=r.bike.i32(0x108)=n?0:speed;r.bike.i32(0x18)=1;}
 race.set_live_schedule(true);race.set_rng_state(1);if(!race.spawn_cop(31))throw std::runtime_error("cop spawn");auto&cop=race.riders().back();cop.bike.i32(0x1c)=cop.bike.i32(0x30)=12800+gap;cop.half_mirror.position=12800+gap;cop.sample=cop.previous_sample=size_t((12800+gap)>>8);cop.bike.i32(0xec)=cop.bike.i32(0x108)=0;race.sort_schedule();auto&p=race.riders()[0];
 if(active){p.attack.timer=1;p.attack.fired=false;p.attack.weapon=0;p.attack.backhand=true;p.attack.right=true;p.ride_animation.animation=codes[1][1][0];p.ride_animation.frame=2;p.ride_animation.deadline=200;}
 if(phase){race.set_ticks(0);race.crash_reason(p,4,{-80,0,0},{0,0,0});auto&m=p.crash_actor.motion;m.i32(0x1c)=m.i32(0x30)=12800;m.i32(0x28)=m.i32(0x3c)=0;m.i32(0xec)=m.i32(0x108)=phase==1?1800:0;m.i32(0x20)=m.i32(0x24)=m.i32(0xf4)=0;p.crash_actor.sample=p.crash_actor.previous_sample=50;p.bike.i32(0x2f4)=phase==1?1:(phase==3||phase==5)?3:2;p.crash_actor.animation.animation=phase==1?35:phase==2||phase==4?38:phase==3?41:2;p.crash_actor.animation.frame=0;p.crash_actor.animation.deadline=200;p.bike.i32(0x1c)=p.bike.i32(0x30)=phase==2?13800:12800;p.sample=p.previous_sample=size_t(p.bike.i32(0x1c)>>8);p.bike.i32(0x28)=p.bike.i32(0x3c)=phase==4?6000:0;p.bike.i32(0xec)=p.bike.i32(0x108)=speed;p.half_mirror.position=12800;race.sort_schedule();}
 race.set_ticks(100);p.bike.i32(0x18)=1;p.sched_last=p.half_sched_last=99;race.run_half_node(p,0,false);race.run_bike_node(p,0,false);
 if(!first){std::cout<<",";}first=false;std::cout<<"["<<speed<<","<<gap<<","<<active<<","<<phase<<","<<int(p.busted)<<","<<p.bike.i32(0xec)<<","<<race.riders()[1].stamina.current<<"]";
 }std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
