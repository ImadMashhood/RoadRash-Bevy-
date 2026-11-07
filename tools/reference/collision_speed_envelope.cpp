// Reachable flat-road source throttle/nitro envelope. No injected speed.
#include "race.hpp"
#include "assets.hpp"
#include "animation.hpp"
#include <iostream>
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto tables=rr::read_physics_tables(assets.read("ROADRASH.EXE"));auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));rr::ActorAnimationLibrary library(assets.read("ROADRASH.EXE"),assets.read("DATA/GLOBAL.RSC"));
 rr::RouteSample sample{{0,0,5000,5000,512,512},0,0,0};std::vector<rr::RouteSample> route(12000,sample);for(size_t j=0;j<route.size();++j)route[j].local=uint32_t(j);
 bool first=true;std::cout<<"[";for(size_t spec=0;spec<specs.size();++spec)for(int boost:{0,1}){
 rr::Race race(route,route,tables,specs[spec],75);race.set_ride_library(&library,nullptr);auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.frozen=false;r.sample=r.previous_sample=50;b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=12800;b.i32(0xec)=b.i32(0x108)=0;b.i32(0x28)=b.i32(0x3c)=0;b.i32(0x20)=b.i32(0x34)=0;b.i32(0x2f0)=150;b.u8(0xc0)=7;int speed=0,step=0,peak=0,active=0;
 for(int tick=1;tick<=12000;++tick){race.set_ticks(100+tick);const int previous=b.i32(0x1c);race.step_rider(r,rr::control_accelerate|(boost?rr::control_nitro:0),true);if(b.i32(0xec)>speed){speed=b.i32(0xec);peak=tick;}step=std::max(step,b.i32(0x1c)-previous);active+=(b.u8(0xc0)&0x20)!=0;}
 if(!first){std::cout<<",";}first=false;std::cout<<"["<<spec<<","<<boost<<","<<speed<<","<<step<<","<<peak<<","<<active<<"]";
 }std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
