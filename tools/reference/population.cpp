// Independently authored adapters around pinned public native APIs.
#include "race.hpp"
#include "traffic.hpp"
#include "police.hpp"
#include "assets.hpp"
#include <iostream>
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");
 std::cout<<"{\"cadence\":[";bool first=true;
 for(bool slice:{false,true})for(int position:{-10000,-1024,-512,0,768,1024,2048,10000})for(int speed:{0,2048,4096,6144,10000})for(int tick:{0,299,300})for(bool human:{false,true}){
 if(!first){std::cout<<",";}first=false;std::cout<<rr::live_interval(slice,position,0,speed,tick,human);
 }
 std::cout<<"],\"timers\":[";first=true;rr::TrafficTimers timers;rr::traffic_configure(timers,6);
 for(int density:{-25,0,3,12,-10,0}){rr::traffic_set_density(timers,density);if(!first){std::cout<<",";}first=false;std::cout<<"["<<timers.cruise<<","<<timers.rate<<","<<timers.oncoming_reload<<","<<timers.other_reload<<","<<timers.oncoming<<","<<timers.behind<<","<<timers.ahead<<","<<timers.density<<"]";}
 std::cout<<"],\"arrest\":[";first=true;
 for(int crash:{0,1,2,3})for(int speed:{-1800,-300,0,299,300,1799,1800})for(int cop_speed:{-5000,0,4999,5000})for(int delta:{0,511,512,-511,-512})for(bool found:{false,true})for(bool crashed:{false,true}){
 rr::PoliceArrestInput input;input.crash=crash;input.bike_speed=speed;input.cop_speed=cop_speed;input.rider_position=1000;input.cop_position=1000+delta;input.found_cop=found;input.cop_crashed=crashed;
 if(!first){std::cout<<",";}first=false;std::cout<<(rr::police_arrest(input)?1:0);
 }
 std::cout<<"],\"personalities\":[";first=true;auto profiles=rr::read_personalities(exe);
 for(int id=0;id<10;++id)for(int level=0;level<5;++level){const auto& p=profiles[size_t(id)];if(!first){std::cout<<",";}first=false;std::cout<<"["<<id<<","<<level<<","<<p.cur_levels[size_t(level)]<<","<<p.cur2_levels[size_t(level)]<<","<<int(p.kick_roll)<<","<<int(p.backhand_roll)<<","<<p.overtake_index<<"]";}
 std::cout<<"],\"vehicle_choices\":[";first=true;
 for(const char* stem:{"CITY","HIWAY","MEDLY","NAPA","CANYN"}){rr::VehicleLibrary vehicles(assets.read(std::string("DATA/CARS/")+stem+".CAR"),exe);for(int kind=0;kind<4;++kind)for(int random=0;random<32;++random){if(!first){std::cout<<",";}first=false;std::cout<<vehicles.choose(uint8_t(kind),uint32_t(random));}}
 std::cout<<"]}\n";
}catch(std::exception& e){std::cerr<<e.what()<<"\n";return 1;}}
