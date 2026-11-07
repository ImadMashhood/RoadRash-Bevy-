// Held steering/coasting reaches native fork crossings through ordinary dt=1 movement.
#include "race.hpp"
#include "assets.hpp"
#include "course.hpp"
#include "crash.hpp"
#include "animation.hpp"
#include <iostream>
#include <algorithm>
int main(int argc,char**argv){try{if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto tables=rr::read_physics_tables(assets.read("ROADRASH.EXE"));auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(assets.read("ROADRASH.EXE"));rr::ActorAnimationLibrary library(assets.read("ROADRASH.EXE"),assets.read("DATA/GLOBAL.RSC"));bool first=true;int id=0;std::cout<<"[";int courseid=0;
for(const char*stem:{"CITY","HIWAY","MEDLY","NAPA","CANYN"}){auto course=rr::read_course(assets.read(std::string("DATA/COURSES/")+stem+".CRS"));auto splits=rr::split_nodes(course);
for(size_t mask=0;mask<(size_t(1)<<splits.size());++mask){std::vector<int> choices(splits.size());for(size_t bit=0;bit<choices.size();++bit)choices[bit]=int((mask>>bit)&1);auto route=rr::make_route_choices(course,choices);
for(size_t sample=1;sample<route.size();++sample){if(route[sample].event!=rr::RoadEvent::split)continue;auto split=course.nodes.at(course.nodes.at(route[sample-1].node).next);size_t bit=size_t(std::find(splits.begin(),splits.end(),split.offset)-splits.begin());auto leftchoices=choices;leftchoices[bit]=0;auto rightchoices=choices;rightchoices[bit]=1;auto left=rr::make_route_choices(course,leftchoices);auto right=rr::make_route_choices(course,rightchoices);int edge=left[sample].offset+left[sample].path.right;
for(int side:{-1000,0,1000})for(int steer:{-1,0,1})for(int speed:{5000,15000,30000}){rr::Race race(left,right,tables,specs[0],75);race.set_ride_library(&library,nullptr);race.set_crash_library(library,crash);auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.frozen=false;r.route=0;r.sample=r.previous_sample=sample-1;b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=int32_t(sample*256-180);b.i32(0x28)=b.i32(0x3c)=edge*256+side;b.i32(0xec)=b.i32(0x108)=speed;b.i32(0x20)=b.i32(0x34)=0;b.i32(0x2f0)=150;b.u8(0xc0)=7;
for(int tick=1;tick<=24;++tick){race.set_ticks(100+tick);race.step_rider(r,steer<0?rr::control_left:steer>0?rr::control_right:0,true);race.collide_riders();auto&point=(r.route?right:left)[r.sample];if(!first){std::cout<<",";}first=false;std::cout<<"["<<id<<","<<courseid<<","<<mask<<","<<bit<<","<<side<<","<<steer<<","<<speed<<","<<tick<<","<<b.i32(0x1c)<<","<<b.i32(0x28)<<","<<b.i32(0x20)<<","<<b.i32(0xec)<<","<<r.route<<","<<point.node<<","<<point.local<<","<<point.offset<<"]";if(r.sample>=sample)break;}++id;}
}}++courseid;}std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
