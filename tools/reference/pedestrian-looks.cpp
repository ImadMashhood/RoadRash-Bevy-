// Static native appearance census; never runs the original executable.
#include "course_scene.hpp"
#include "assets.hpp"
#include <iostream>
#include <fstream>
#include <tuple>
int main(int argc,char**argv){try{if(argc!=2 && argc!=3){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");if(argc==3){std::ifstream stream(argv[2]);std::vector<std::tuple<std::string,uint32_t,uint32_t,uint32_t>> queries;std::string name;uint32_t section,local,selector;while(stream>>name>>section>>local>>selector){queries.emplace_back(name,section,local,selector);}std::cout<<"[";bool comma=false;for(const char* stem:{"CITY","HIWAY","MEDLY","NAPA","CANYN"}){auto course=rr::read_course(assets.read(std::string("DATA/COURSES/")+stem+".CRS"));rr::CourseScene scene(assets,exe,course,stem,1,0);for(auto& [name,s,l,c]:queries){if(name!=stem){continue;}auto look=scene.pedestrian_look(s,l,uint8_t(c));bool drawable=false;if(look){for(const auto& frame:look->graphics.frames){if(frame.texture){drawable=true;break;}}}if(comma){std::cout<<",";}comma=true;std::cout<<"[\""<<name<<"\","<<s<<","<<l<<","<<c<<","<<(look?1:0)<<","<<(drawable?1:0)<<"]";}}std::cout<<"]\n";return 0;}
std::cout<<"[";bool first=true;
 for(const char* stem:{"CITY","HIWAY","MEDLY","NAPA","CANYN"}){auto course=rr::read_course(assets.read(std::string("DATA/COURSES/")+stem+".CRS"));rr::CourseScene scene(assets,exe,course,stem,1,0);size_t requested=0,loaded=0,drawable=0,failed=0;
 for(size_t section=0;section<course.sections.size();++section)for(size_t local=0;local<course.sections[section].samples.size();++local)for(const auto& actor:scene.dzhr_actors(uint32_t(section),uint32_t(local))){if((actor.word0&15)>=11){continue;}++requested;try{auto look=scene.pedestrian_look(uint32_t(section),uint32_t(local),uint8_t(actor.word0>>8));if(!look){continue;}++loaded;bool found=false;for(const auto& frame:look->graphics.frames){if(frame.texture){found=true;break;}}if(found){++drawable;}}catch(const std::exception&){++failed;}}
 if(!first){std::cout<<",";}first=false;std::cout<<"{\"course\":\""<<stem<<"\",\"requests\":"<<requested<<",\"loaded\":"<<loaded<<",\"drawable\":"<<drawable<<",\"failed\":"<<failed<<"}";
 }std::cout<<"]\n";}catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}}
