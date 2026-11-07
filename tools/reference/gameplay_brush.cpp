// Actual source course mounted brush, through first crash; private regression diagnostic.
#include "race.hpp"
#include "assets.hpp"
#include "course_scene.hpp"
#include "terrain_ground.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include <iostream>
int main(int argc,char**argv){try{
 if(argc!=6 && argc!=7){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto course=rr::read_course(assets.read("DATA/COURSES/CITY.CRS"));auto route=rr::make_route_choices(course,std::vector<int>(rr::split_nodes(course).size(),0));rr::CourseScene scene(assets,exe,course,"CITY",1,0);uint32_t seed=1;auto random=[&](){seed=seed*214013u+2531011u;return(seed>>16)&0x7fffu;};std::vector<std::vector<rr::TerrainSlice>> terrain;
 for(size_t section=0;section<course.sections.size();++section){const auto&data=course.sections[section];rr::TerrainIterator iterator(data.terrain,0,random);terrain.emplace_back();for(size_t local=0;local<data.samples.size();++local){if(local){iterator.step(256);}const auto&p=data.samples[local];auto t=iterator.slice(-p.left-p.shoulder_left,p.right+p.shoulder_right);if(t.type==4&&p.marking_flags&&!(p.marking_flags&0x40)){for(auto&point:t.points){point[1]=0;}}if(p.marking_flags&0x40){t.type=3;}if(t.type==0){t.selectors=scene.terrain(uint32_t(section),uint32_t(local)).selectors;}terrain.back().push_back(t);}}
 rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));auto crash=rr::read_crash_config(exe);rr::Race race(route,route,tables,specs[0],75);race.set_ride_library(&library,nullptr);race.set_crash_library(library,crash);race.set_terrain_source([&](const rr::RouteSample&p)->const rr::TerrainSlice&{return terrain[course.nodes.at(p.node).link][p.local];});auto&r=race.riders()[0];auto&b=r.bike;const int position=std::stoi(argv[2]);r.ai=false;r.frozen=false;r.sample=r.previous_sample=size_t(position>>8);b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=position;b.i32(0x28)=b.i32(0x3c)=std::stoi(argv[3]);b.i32(0xec)=b.i32(0x108)=std::stoi(argv[4]);b.i32(0x20)=b.i32(0x34)=0;b.i32(0x24)=b.i32(0x38)=0;b.u8(0xc0)=7;
 std::cout<<"[";bool first=true;for(int tick=argc==7?0:1;tick<=std::stoi(argv[5]);++tick){race.set_ticks(tick);race.step_rider(r,tick==0?0:rr::control_accelerate|rr::control_left,true);race.collide_riders();if(!first){std::cout<<",";}first=false;std::cout<<"["<<tick;for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x114,0x288,0x2f0,0x24,0x160,0x2f4})std::cout<<","<<b.i32(off);std::cout<<","<<int(b.u8(0x164))<<"]";if(r.crash_active)break;}std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
