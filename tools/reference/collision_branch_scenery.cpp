// Actual native scenery at selected and off-launch branch offsets, beyond Dual windows.
#include "race.hpp"
#include "assets.hpp"
#include "course.hpp"
#include "crash.hpp"
#include "animation.hpp"
#include "track_ring.hpp"
#include <iostream>
#include <algorithm>
int main(int argc,char**argv){try{if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto tables=rr::read_physics_tables(assets.read("ROADRASH.EXE"));auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(assets.read("ROADRASH.EXE"));rr::ActorAnimationLibrary library(assets.read("ROADRASH.EXE"),assets.read("DATA/GLOBAL.RSC"));bool first=true;int id=0;std::cout<<"[";int courseid=0;
for(const char*stem:{"CITY","HIWAY","MEDLY","NAPA","CANYN"}){auto course=rr::read_course(assets.read(std::string("DATA/COURSES/")+stem+".CRS"));auto splits=rr::split_nodes(course);for(size_t mask=0;mask<(size_t(1)<<splits.size());++mask){std::vector<int> choices(splits.size());for(size_t bit=0;bit<choices.size();++bit)choices[bit]=int((mask>>bit)&1);auto route=rr::make_route_choices(course,choices);auto split=std::find_if(route.begin(),route.end(),[](const auto&p){return p.event==rr::RoadEvent::split;});if(split==route.end())throw std::runtime_error("Missing fork");
for(int phase:{0,100})for(int kind:{0,3,16})for(int height:{0,13055,13056,13057}){size_t sample=size_t(split-route.begin())+size_t(phase);auto&p=route[sample];rr::Race race(route,route,tables,specs[0],75);race.set_ride_library(&library,nullptr);race.set_crash_library(library,crash);race.set_ticks(100);rr::AnimationFrame frame;frame.boxes.push_back({-1024,0,1024,4096});race.set_static_frames([&](uint32_t,uint32_t,uint8_t,bool){return &frame;});rr::RingStatics statics;if(kind==16){statics.borr_count=1;statics.borr[0]=0;}else{statics.billboards=1;statics.billboard[0].type=uint8_t(kind);statics.billboard[0].selector=7;}race.ring_statics(int(sample),p.section,p.local,statics);
auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.frozen=false;r.route=0;r.sample=r.previous_sample=sample;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=int32_t(sample*256+128);b.i32(0x28)=b.i32(0x3c)=p.offset*256;b.i32(0x20)=b.i32(0x34)=height;b.i32(0xec)=b.i32(0x108)=5000;b.i32(0xf0)=b.i32(0x10c)=0;b.i32(0xf4)=b.i32(0x110)=0;b.i32(0x2f0)=150;b.u8(0xc0)=7;race.collide_riders();if(!first){std::cout<<",";}first=false;std::cout<<"["<<id++<<","<<courseid<<","<<mask<<","<<phase<<","<<kind<<","<<height<<","<<sample<<","<<p.node<<","<<p.local<<","<<p.offset;for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x2f0,0x298})std::cout<<","<<b.i32(off);std::cout<<","<<r.stamina.max<<","<<r.stamina.current<<","<<(r.crash_active?1:0)<<","<<r.half_object.i32(0x1ec)<<"]";}
}++courseid;}std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
