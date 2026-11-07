// Actual native find_cop + police_think, including intervening rider halves.
#include "race.hpp"
#include "assets.hpp"
#include "police.hpp"
#include "animation.hpp"
#include <iostream>
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto config=rr::read_police_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));rr::RouteSample sample{{0,0,5000,5000,512,512},0,0,0};std::vector<rr::RouteSample> route(1024,sample);std::vector<rr::Opponent> opponents;for(int i=0;i<2;++i){rr::Opponent o;o.spec=specs[0];o.rider_value=75;o.base_speed=100;o.cur2_speed=60;o.lane=0;o.slot=i+1;opponents.push_back(o);}rr::Race race(route,route,tables,specs[0],75,opponents);race.set_ride_library(&library,nullptr);race.set_police(config,specs,&library);if(!race.spawn_cop(0))throw std::runtime_error("No native cop");race.set_ticks(100);auto&player=race.riders()[0];auto&cop=race.riders().back();bool first=true;int id=0;std::cout<<"[";
 for(int gap:{-2560,-1280,-512,-511,-1,0,1,511,512,1280,2560})for(int crash:{0,1,2,3})for(int speed:{-1800,-1799,299,300,1799,1800})for(int difference:{-5000,-4999,0,4999,5000})for(int cop_crash:{0,1,2})for(int flags:{0,1,2})for(int layout:{0,1,2,3})for(int resident:{0,1}){
  const int pos=12800;for(size_t k=0;k<race.riders().size();++k){auto&r=race.riders()[k];r.crash_active=false;r.half_detached=false;r.sample=r.previous_sample=50;r.bike.i32(0x1c)=pos+10000+int(k)*1000;r.bike.i32(0x28)=0;r.bike.u8(0xc0)=7;r.half_object.u8(0xc0)=4;r.bike.i32(0x2f4)=0;r.half_slice_present=true;r.bike_slice_present=true;}
  player.bike.i32(0x1c)=pos;player.bike.i32(0xec)=speed;player.bike.i32(0x2f4)=crash;player.bike.u8(0xc0)=flags==1?0x10:7;player.half_object.u8(0xc0)=flags==2?0x10:4;player.busted=false;race.busted_=false;
  if(crash){player.crash_active=true;player.crash_actor.motion=player.half_object;player.crash_actor.motion.i32(0x1c)=pos+(layout==3?100:0);player.crash_actor.route=0;player.crash_actor.sample=50;}
  cop.bike.i32(0x1c)=pos+gap;cop.bike.i32(0xec)=speed+difference;cop.bike.i32(0x2f4)=cop_crash;cop.half_slice_present=resident!=0;
  for(int k=1;k<=2;++k){auto&r=race.riders()[size_t(k)];if(layout==1)r.bike.i32(0x1c)=pos+gap*k/3;else if(layout==2)r.bike.i32(0x1c)=pos;else if(layout==3){r.half_detached=true;r.half_position=pos+(k==1?-10:10);r.half_lateral=0;}}
  const bool found=race.find_cop(player)>=0;race.police_think(player);if(!first){std::cout<<",";}first=false;std::cout<<"["<<id++<<","<<gap<<","<<crash<<","<<speed<<","<<difference<<","<<cop_crash<<","<<flags<<","<<layout<<","<<resident<<","<<(found?1:0)<<","<<(race.busted()?1:0)<<"]";
 }std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
