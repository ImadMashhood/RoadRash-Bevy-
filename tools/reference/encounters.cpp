// Independently authored public-API probes. No decompiled source or data embedded.
#include "race.hpp"
#include "assets.hpp"
#include "crash.hpp"
#include <iostream>
int main(int argc,char**argv){try{
    if(argc!=2)return 2;
    rr::Assets assets(argv[1]);
    auto tables=rr::read_physics_tables(assets.read("ROADRASH.EXE"));
    auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
    std::cout<<"{\"motion\":["; bool first=true;
    for(int speed:{-10000,-1,0,1200,5000,1000000})for(int acceleration:{-500,0,300})for(int heading:{0,0x100000,0x400000,0x800000,0xc00000,-0x100000}){
        rr::Bike actor; actor.i32(0x18)=1; actor.i32(0xec)=speed;actor.accel=acceleration;actor.i32(0x128)=heading;
        int forward=0;rr::advance_actor_motion(actor,tables,[&](int d){forward=d;});
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<speed<<","<<acceleration<<","<<heading<<","<<forward<<","<<actor.i32(0x28)<<"]";
    }
    std::cout<<"],\"damage\":[";first=true;
    rr::RouteSample sample{{0,0,500,500,512,512},0,0,0};
    std::vector<rr::RouteSample> route(2000,sample);
    for(int reason=0;reason<7;++reason)for(int amount:{0,1,7,32,100}){
        rr::Race race(route,route,tables,specs[0],75);
        auto& rider=race.riders()[0];rider.stamina.base=2500;rider.stamina.max=2500;rider.stamina.current=2500;
        race.knock_off(rider,amount,0,reason);
        if(!first){std::cout<<",";}first=false;
        std::cout<<"["<<reason<<","<<amount<<","<<rider.stamina.max<<","<<rider.stamina.current<<"]";
    }
    std::cout<<"]}\n";
}catch(std::exception& e){std::cerr<<e.what()<<"\n";return 1;}}
