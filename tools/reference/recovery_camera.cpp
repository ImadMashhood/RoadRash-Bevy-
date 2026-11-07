// Actual pinned camera callbacks with independent quiet-bike and crash-half poses.
// A copied header exposes access; the reference sources/library are unchanged.
#include "race.hpp"
#include "assets.hpp"
#include <algorithm>
#include <iostream>
int main(int argc, char** argv) { try {
    if (argc != 2) return 2;
    rr::Assets assets(argv[1]);
    const auto tables = rr::read_physics_tables(assets.read("ROADRASH.EXE"));
    const auto specs = rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
    int id = 0; bool first_case = true;
    std::cout << "[";
    for (int base : {-700, 0, 500}) for (int left : {200, 700})
    for (int ground : {0,149,150,151,500,5000}) for (int delta : {-40,-7,11}) {
        std::vector<rr::RouteSample> route;
        for (int i=0;i<128;++i) {
            const int l=left+(i%3)*17, r=1000-l;
            rr::RouteSample sample{{0,0,l,r,l+12,r+12},0,0,uint32_t(i)};
            sample.offset=base+(i%5-2)*30;
            route.push_back(sample);
        }
        rr::Race race(route,route,tables,specs[0],75,{},0);
        auto& rider=race.riders()[0];
        if(!first_case) std::cout << ",";
        first_case=false;
        std::cout << "{\"id\":" << id << ",\"base\":" << base << ",\"left\":" << left << ",\"rows\":[";
        for(int tick=0;tick<640;++tick) {
            const int position=41*256+(tick%17);
            const int lateral=(route[size_t(position>>8)].offset+(tick%2?1200:-801))*256+(tick%3-1);
            const int elevation=ground+(tick%11-5);
            const int follow=position+delta*256+(tick%251);
            const int body_lateral=lateral+(tick%2?-900:700)*256;
            const int body_ground=elevation+700;
            rider.sample=rider.previous_sample=size_t(position>>8);
            rider.bike.i32(0x1c)=position;
            rider.bike.i32(0x28)=lateral;
            rider.bike.i32(0x24)=elevation;
            rider.crash_active=true;
            rider.crash_actor.motion.i32(0x1c)=follow;
            rider.crash_actor.motion.i32(0x28)=body_lateral;
            rider.crash_actor.motion.i32(0x24)=body_ground;
            rider.crash_actor.sample=size_t(follow>>8);
            rider.half_mirror.position=follow;
            rider.half_mirror.lateral=body_lateral;
            rider.half_mirror.sample=size_t(follow>>8);
            // Exercise both terminal latch and arrest ramp, followed by full decay.
            const bool terminal=tick<330;
            rider.bike.u8(0xc0)=uint8_t(terminal && (id%2==0)?0x10:0);
            race.busted_=terminal && (id%2!=0);
            race.update_camera();
            if(tick) std::cout << ",";
            std::cout << "[" << tick << "," << position << "," << lateral << "," << elevation
                << "," << follow << "," << body_lateral << "," << body_ground
                << "," << race.camera_back_ << "," << race.camera_lift_
                << "," << race.camera_position() << "," << race.camera().x << "," << race.camera().height << "]";
        }
        std::cout << "]}"; ++id;
    }
    std::cout << "]\n";
} catch(const std::exception& error) {std::cerr<<error.what()<<"\n";return 1;} }
