// Actual mounted crash decisions and their initial linked body/bike images.
#include "race.hpp"
#include "assets.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include <iostream>
#include <string>
static void state(const rr::Rider& r){const auto&b=r.bike;std::cout<<"[";bool first=true;auto n=[&](int64_t v){if(!first)std::cout<<",";first=false;std::cout<<v;};for(int o:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x24,0x114,0x288,0x2f0,0x2f4,0x23c})n(b.i32(o));n(b.u8(0x164));n(b.skid_ticks);n(r.stamina.max);n(r.stamina.current);n(r.crash_active);if(r.crash_active){const auto&m=r.crash_actor.motion;for(int o:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x24,0x128,0x5c})n(m.i32(o));n(r.crash_actor.animation.animation);}else for(int i=0;i<10;++i)n(0);const auto&a=r.crash_bike_animation;n(a.animation);n(a.frame);n(a.queued);n(a.loops);n(a.deadline);n(a.selected?int(a.selected->at(0)):-1);n(a.selected?int(a.selected->at(1)):-1);n(b.i32(0x5c));std::cout<<"]";}
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));
 bool first=true;int id=0;std::cout<<"[";
 for(int kind=0;kind<12;++kind)for(int speed:{0,300,5000,12000})for(int lateral_speed:{-512,0,512})for(int variation=0;variation<4;++variation){
  rr::RouteSample rs{{0,0,500,500,512,512},0,0,0};rs.path.shoulder_left=rs.path.shoulder_right=250;rs.path.elevation=0;
  if(kind==5||kind==10){rs.path.event_type=5;rs.path.event_parameters={0,0,129};}
  std::vector<rr::RouteSample> route(512,rs);for(size_t i=0;i<route.size();++i)route[i].local=uint32_t(i);
  rr::Race race(route,route,tables,specs[0],75);race.set_crash_library(library,crash);race.set_ride_library(&library,nullptr);race.set_live_schedule(true);race.set_ticks(0);
  rr::TerrainSlice terrain{};terrain.type=(kind==4||kind==9)?0:255;terrain.selectors={0,0};terrain.walls[0]={100,500,300};terrain.walls[1]={100,500,300};race.set_terrain_source([&](const rr::RouteSample&)->const rr::TerrainSlice&{return terrain;});
  auto&r=race.riders()[0];r.ai=false;r.sample=r.previous_sample=50;r.stamina.base=r.stamina.max=3000;r.stamina.current=variation==0?1:variation==1?64:variation==2?65:3000;
  auto&b=r.bike;b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=12800;b.i32(0x28)=b.i32(0x3c)=0;b.i32(0xec)=b.i32(0x108)=speed;b.i32(0xf0)=b.i32(0x10c)=kind>=8?lateral_speed*16:lateral_speed;b.i32(0xf4)=0;b.i32(0x20)=0;b.i32(0x24)=0;b.i32(0x160)=tables.bike_object[11];b.i32(0x2f0)=variation==0?1:160;b.i32(0x114)=(variation-1)*32;b.i32(0x240)=0;b.i32(0x288)=0;b.i32(0x2f4)=0;b.i32(0x2f8)=0;b.u8(0xc0)=7;
  if(kind==3){b.i32(0x288)=b.i32(0x29c);b.skid_ticks=24+variation;b.i32(0x160)=1;}
  if(kind==4){b.i32(0x28)=b.i32(0x3c)=750*256-crash.normal_dimensions[0]+variation*60*256;b.i32(0xf0)=7000;}
  if(kind==5){b.i32(0x28)=b.i32(0x3c)=-750*256;b.i32(0x160)=0;b.i32(0x20)=variation==0?32767:variation==1?32768:variation==2?32700:0;}
  if(kind==6){b.i32(0x20)=variation==0?0:variation==1?101:variation==2?3000:16000;b.i32(0xf4)=variation<2?-1000:-10000;b.i32(0x160)=0;}
  race.advance_ride_animation(r,true);
  if(kind==10){b.i32(0x114)=64;}
  if(kind>=8){race.rider_move(r,true);}
  if(kind==9){b.i32(0x28)=750*256-crash.normal_dimensions[0]+variation*60*256;b.i32(0xf0)=7000;}
  if(kind==10){b.i32(0x28)=-750*256;b.i32(0x160)=0;b.i32(0x20)=variation==0?32767:variation==1?32768:variation==2?32700:0;}
  const auto v=race.sub_slice(r,50);
  if(!first){std::cout<<",";}first=false;std::cout<<"{\"id\":"<<id++<<",\"kind\":"<<kind<<",\"speed\":"<<speed<<",\"lateral_speed\":"<<lateral_speed<<",\"variation\":"<<variation<<",\"yaw_gain\":"<<b.i32(0x2b8)<<",\"yaw\":"<<b.i32(0x5c)<<",\"boundary\":["<<int(v.type)<<","<<v.e0<<","<<v.e4<<","<<v.e8<<","<<v.ec<<","<<int(v.flags)<<","<<int(v.sel[0])<<","<<int(v.sel[1]);for(int n:v.base)std::cout<<","<<n;for(int n:v.top)std::cout<<","<<n;for(int n:v.height)std::cout<<","<<n;std::cout<<","<<int(v.event_type)<<","<<int(v.p3a)<<","<<int(v.b3b)<<"],\"pre\":";state(r);
  if(kind==0||kind==1)race.knock_off(r,variation==3?47:1,kind==1?1:0,kind==0?4:5);
  else if(kind==2||kind==8||kind==11)race.crash_car(r);
  else if(kind==3||kind==6)race.rider_move(r,true);
  else if(kind==4||kind==5||kind==9||kind==10)race.walls(r);
  else race.crash_reason(r,4,{0,0,0},{0,0,0}); // repo voluntary action policy, no native input claimed
  if(kind==11){if(variation==2){b.i32(0x20)=b.i32(0x16c)+1;b.i32(0xf4)=-1;}race.set_rng_state(1);race.crash_bike_animation(r);race.crash_bike_motion(r,true);}
  std::cout<<",\"reason\":"<<b.i32(0x2f8)<<",\"post\":";state(r);std::cout<<"}";
 }std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
