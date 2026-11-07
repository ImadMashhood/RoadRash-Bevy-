// Actual pinned native boundary callbacks; public fixture inputs and output words.
#include "race.hpp"
#include "assets.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include <iostream>
int main(int argc,char** argv) { try {
 if(argc!=2)return 2;
 rr::Assets assets(argv[1]); auto exe=assets.read("ROADRASH.EXE");
 auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
 auto crash=rr::read_crash_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));
 int id=0;bool first=true;std::cout<<"[";
 for(int kind:{0,1,3,4,255})for(int mode=0;mode<4;++mode)for(int side:{-1,1})
 for(int depth:{-1,0,1,16,30,60,120,400})for(int speed:{2499,2500,12000})
 for(int category:{4,8})for(int height:{0,63,1024,8192})for(int player:{0,1}) {
  rr::RouteSample rs{{0,0,500,500,512,512},0,0,0};rs.path.shoulder_left=rs.path.shoulder_right=250;rs.path.elevation=200;
  rs.path.event_type=mode>=2?5:0;rs.path.event_parameters={0,uint8_t(mode==3?12:0),uint8_t((mode==3?28:0)|(side<0?1:2))};
  std::vector<rr::RouteSample> route(128,rs);for(size_t i=0;i<route.size();++i)route[i].local=uint32_t(i);
  route[9].path.marking_flags=mode==1?0x40:0;
  route[11].path.shoulder_left=280;route[11].path.shoulder_right=230;
  rr::Race race(route,route,tables,specs[0],75);race.set_crash_library(library,crash);race.set_ticks(100);
  rr::TerrainSlice terrain;terrain.type=uint8_t(kind);terrain.selectors={0,0};
  terrain.walls[0]={100,500,mode==2?0:300};terrain.walls[1]={80,400,mode==2?-1:400};
  race.set_terrain_source([&](const rr::RouteSample&)->const rr::TerrainSlice&{return terrain;});
  auto&r=race.riders()[0];r.ai=!player;r.sample=10;r.previous_sample=9;
  auto&b=r.bike;const int hw=category==4?crash.normal_dimensions[0]:tables.half_object[17];
  const int hh=category==4?crash.normal_dimensions[2]:tables.half_object[19];
  const int lateral=side*(750*256-hw+depth*256);const int rate=side*(mode==1?0:7000);
  b.i32(0x1c)=10*256+128;b.i32(0x30)=9*256+64;
  b.i32(0x28)=lateral;b.i32(0x3c)=lateral+side*(mode==3?10000:-10000);
  b.i32(0x20)=height;b.i32(0xec)=b.i32(0x108)=speed;b.i32(0xf0)=rate;b.i32(0xf4)=0;b.i32(0x288)=500;
  b.i32(0x160)=mode>=2?0:tables.contact_base;b.i32(0xc4)=category;b.i32(0x174)=hw;b.i32(0x17c)=hh;
  b.i32(0x2f4)=b.i32(0x2f8)=0;b.u8(0xc0)=7;
  rr::Bike owner=b;owner.i32(0x2f4)=1;
  if(!first){std::cout<<",";}first=false;
  std::cout<<"{\"id\":"<<id++<<",\"boundaries\":[";
  bool first_boundary=true;for(size_t sample:{10u,11u,9u,10u}) {
   const auto v=race.sub_slice(r,sample);if(!first_boundary)std::cout<<",";first_boundary=false;
   std::cout<<"["<<int(v.type)<<","<<v.e0<<","<<v.e4<<","<<v.e8<<","<<v.ec<<","<<int(v.flags)<<","<<int(v.sel[0])<<","<<int(v.sel[1]);
   for(int n:v.base){std::cout<<","<<n;}for(int n:v.top){std::cout<<","<<n;}for(int n:v.height){std::cout<<","<<n;}
   std::cout<<","<<int(v.event_type)<<","<<int(v.p3a)<<","<<int(v.b3b)<<"]";
  }
  std::cout<<"],\"query\":["<<b.i32(0x1c)<<","<<b.i32(0x30)<<","<<lateral<<","<<b.i32(0x3c)<<","<<hw<<","<<hh<<","<<crash.bike_dimensions[2]<<","<<b.i32(0x16c)<<","<<height<<","<<b.i32(0x160)<<","<<category<<","<<player<<",1],\"motion\":["<<speed<<","<<rate<<",0,500,0]";
  race.walls(r,category==8?&owner:nullptr);
  std::cout<<",\"expected\":["<<b.i32(0x1c)<<","<<b.i32(0x28)<<","<<b.i32(0xec)<<","<<b.i32(0xf0)<<","<<b.i32(0x288)<<","<<b.i32(0x20)<<","<<b.i32(0xf4)<<","<<(r.crash_active?1:0)<<","<<b.i32(0x2f8);
  if(r.crash_active){const auto&m=r.crash_actor.motion;for(int off:{0x20,0xec,0xf0,0xf4})std::cout<<","<<m.i32(off);std::cout<<","<<int(r.crash_actor.animation.animation);}
  else std::cout<<",0,0,0,0,0";
  std::cout<<","<<b.i32(0x5c)<<"]}";
 }
 std::cout<<"]\n";
 }catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
