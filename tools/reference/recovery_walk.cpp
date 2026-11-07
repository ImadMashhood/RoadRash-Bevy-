// Actual source walk-control wrapper and strict pickup/facing decisions.
#include "race.hpp"
#include "assets.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include <iostream>
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto config=rr::read_crash_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));
 rr::RouteSample sample{{0,0,5000,5000,512,512},0,0,0};std::vector<rr::RouteSample> route(2048,sample);std::cout<<"[";bool first=true;
 for(int player:{0,1})for(int condition:{0,150})for(int gap:{-1000,-10,-9,0,9,10,1000})for(int lateral:{-6001,-6000,0,6000,6001})for(int heading:{0,0x400000,0x800000})for(int longitudinal:{-1,0,1})for(int side:{-1,0,1}){
  const uint32_t controls=(longitudinal<0?0x1000:longitudinal>0?0x800:0)|(side<0?0x4000:side>0?0x2000:0);
  rr::Race race(route,route,tables,specs[0],75);race.set_ride_library(&library,nullptr);race.set_crash_library(library,config);race.set_live_schedule(true);race.set_ticks(100);auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.sample=r.previous_sample=size_t((12800+gap)>>8);b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=12800+gap;b.i32(0x28)=b.i32(0x3c)=6000+lateral;b.i32(0x20)=b.i32(0x34)=0;b.i32(0xec)=b.i32(0x108)=0;b.i32(0x2f0)=condition;
  race.crash_reason(r,4,{0,0,0},{0,0,0});b.i32(0x2f4)=2;r.crash_timer=123;auto&m=r.crash_actor.motion;r.crash_actor.sample=r.crash_actor.previous_sample=50;m.i32(0x1c)=m.i32(0x30)=12800;m.i32(0x28)=m.i32(0x3c)=0;m.i32(0x20)=m.i32(0x34)=0;m.i32(0xec)=400;m.i32(0x128)=m.i32(0x5c)=heading;rr::set_animation_with_fallback(r.crash_actor.animation,library.playback[0],38,100,library.fallback);r.crash_actor.animation.queued=-1;
  race.crash_recovery(r,player!=0,controls);
  if(!first){std::cout<<",";}first=false;std::cout<<"["<<player<<","<<condition<<","<<gap<<","<<lateral<<","<<heading<<","<<controls;for(int off:{0x2f4,0x28,0x2f0})std::cout<<","<<b.i32(off);for(int off:{0x1c,0x28,0xec,0x128,0x5c})std::cout<<","<<m.i32(off);const auto&a=r.crash_actor.animation;std::cout<<","<<int(a.animation)<<","<<int(a.frame)<<","<<int(a.queued)<<","<<int(a.loops)<<","<<r.crash_timer<<","<<(r.crash_active?1:0)<<"]";
 }std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
