// Actual native route crossing and physical ground edges, independently read source courses.
#include "race.hpp"
#include "assets.hpp"
#include "course.hpp"
#include <iostream>
#include <algorithm>
int main(int argc,char**argv){try{if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto tables=rr::read_physics_tables(assets.read("ROADRASH.EXE"));auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));bool first=true;int id=0;std::cout<<"[";int courseid=0;
for(const char*stem:{"CITY","HIWAY","MEDLY","NAPA","CANYN"}){auto course=rr::read_course(assets.read(std::string("DATA/COURSES/")+stem+".CRS"));std::vector<uint32_t> splits;for(auto&[key,node]:course.nodes)if(node.kind==1)splits.push_back(key);
for(size_t mask=0;mask<(size_t(1)<<splits.size());++mask){std::vector<int> choices(splits.size());for(size_t bit=0;bit<choices.size();++bit)choices[bit]=int((mask>>bit)&1);auto route=rr::make_route_choices(course,choices);
for(size_t sample=1;sample<route.size();++sample){if(route[sample].event!=rr::RoadEvent::split)continue;auto node=course.nodes.at(route[sample-1].node);auto split=course.nodes.at(node.next);auto found=std::find(splits.begin(),splits.end(),split.offset);if(found==splits.end())throw std::runtime_error("Missing native split identity");size_t bit=size_t(found-splits.begin());auto leftchoices=choices;leftchoices[bit]=0;auto rightchoices=choices;rightchoices[bit]=1;auto left=rr::make_route_choices(course,leftchoices);auto right=rr::make_route_choices(course,rightchoices);int edge=left[sample].offset+left[sample].path.right;
for(int side:{-1,0,1}){rr::Race race(left,right,tables,specs[0],75);auto&r=race.riders()[0];auto&b=r.bike;r.route=0;r.sample=r.previous_sample=sample-1;b.i32(0x1c)=b.i32(0x30)=int32_t(sample*256-1);b.i32(0x28)=b.i32(0x3c)=edge*256+side;race.move_along(r,2);const auto&point=(r.route?right:left)[r.sample];auto edges=race.edges(r,r.sample);
if(!first){std::cout<<",";}first=false;std::cout<<"["<<id++<<","<<courseid<<","<<mask<<","<<bit<<","<<sample<<","<<side<<","<<edge<<","<<b.i32(0x1c)<<","<<r.route<<","<<point.node<<","<<point.local<<","<<point.offset<<","<<edges.left<<","<<edges.right<<"]";}
}}++courseid;}std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
