// Separately authored adapter: fixed API calls, no game payload in this source.
#include "bike.hpp"
#include "assets.hpp"
#include <iostream>
int main(int argc,char**argv){try{if(argc!=2)return 2;rr::Assets a(argv[1]);auto specs=rr::read_bike_specs(a.read("DATA/BIKESPEC.RSC"));auto t=rr::read_physics_tables(a.read("ROADRASH.EXE"));
 std::cout<<"[";bool first=true;
 for(size_t id=0;id<specs.size();++id)for(int scenario=0;scenario<7;++scenario){rr::Bike b;rr::init_bike_object(b,t);rr::apply_spec(b,specs[id],75);b.i32(0x18)=1;b.i32(0x160)=74;
 for(int tick=0;tick<2400;++tick){bool gas=tick<1800,brake=tick>=1800&&scenario==0;int dir=scenario==1?-1:scenario==2?1:0;int surface=scenario>2?scenario-3:0;
 rr::apply_controls(b,t,(gas?rr::control_accelerate:0)|(brake?rr::control_brake:0)|(dir<0?rr::control_left:dir>0?rr::control_right:0));rr::traction(b,t,surface,true);rr::integrate_speed(b,t,surface);rr::shift_gears(b);
 if(!first){std::cout<<",";}first=false;std::cout<<"["<<id+1<<","<<scenario<<","<<tick+1<<","<<b.i32(0xec)<<","<<b.i32(0x23c)<<","<<b.i32(0x240)<<","<<b.i32(0x288)<<","<<b.i32(0xf0)<<","<<b.i32(0x118)<<","<<b.accel<<","<<b.i32(0x2f4)<<"]";
 }}std::cout<<"]\n";
 }catch(std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
