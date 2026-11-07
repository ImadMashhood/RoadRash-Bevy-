// Actual native callback displacements can choose a road at a split boundary.
#include "race.hpp"
#include "assets.hpp"
#include "course.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include <algorithm>
#include <iostream>
int main(int argc,char**argv){try{if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));bool first=true;int courseid=0;std::cout<<"[";
for(const char*stem:{"CITY","HIWAY","MEDLY","NAPA","CANYN"}){auto course=rr::read_course(assets.read(std::string("DATA/COURSES/")+stem+".CRS"));auto splits=rr::split_nodes(course);for(size_t mask=0;mask<(size_t(1)<<splits.size());++mask){std::vector<int> choices(splits.size());for(size_t bit=0;bit<choices.size();++bit)choices[bit]=int((mask>>bit)&1);auto route=rr::make_route_choices(course,choices);for(size_t sample=1;sample<route.size();++sample){if(route[sample].event!=rr::RoadEvent::split)continue;auto split=course.nodes.at(course.nodes.at(route[sample-1].node).next);size_t bit=size_t(std::find(splits.begin(),splits.end(),split.offset)-splits.begin());auto lc=choices;lc[bit]=0;auto rc=choices;rc[bit]=1;auto left=rr::make_route_choices(course,lc);auto right=rr::make_route_choices(course,rc);const int edge=left[sample].offset+left[sample].path.right;
for(int side:{-1,0,1}){rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(left,right,tables,specs[0],75,{opponent},0);race.set_crash_library(library,crash);race.set_ticks(100);for(int i=0;i<2;++i){auto&r=race.riders()[size_t(i)];auto&b=r.bike;r.ai=true;r.frozen=false;r.sample=r.previous_sample=sample-1;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;const int x=int(sample*256)-(i?5:40);b.i32(0x18)=1;b.i32(0x1c)=x;b.i32(0x30)=x-(i?0:117);b.i32(0x28)=b.i32(0x3c)=edge*256+side;b.i32(0x20)=b.i32(0x34)=0;b.i32(0xec)=b.i32(0x108)=i?0:30000;b.i32(0xf0)=b.i32(0x10c)=0;b.i32(0xf4)=b.i32(0x110)=0;b.i32(0x2f0)=150;b.u8(0xc0)=7;}
race.collide_riders();if(!first){std::cout<<",";}first=false;std::cout<<"["<<courseid<<","<<mask<<","<<bit<<","<<side<<","<<sample<<","<<edge;for(auto&r:race.riders()){auto&b=r.bike;auto&p=(r.route?right:left)[r.sample];std::cout<<","<<r.route<<","<<p.node<<","<<p.local;for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x2f0})std::cout<<","<<b.i32(off);std::cout<<","<<(r.crash_active?1:0);}std::cout<<"]";}
}}++courseid;}std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
