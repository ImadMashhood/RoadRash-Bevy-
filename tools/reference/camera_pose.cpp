// Invoke the pinned camera updater; the copied header only exposes probe access.
#include "race.hpp"
#include "assets.hpp"
#include <iostream>
int main(int argc, char** argv) { try {
    if (argc != 2) return 2;
    rr::Assets assets(argv[1]);
    auto tables = rr::read_physics_tables(assets.read("ROADRASH.EXE"));
    auto specs = rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
    int id = 0; bool first = true; std::cout << "[";
    for (int offset : {-1000, 0, 500}) for (int left : {250, 500})
    for (int relative : {-1200,-801,-800,-201,-1,0,1,201,800,801,1200})
    for (int fraction : {-255,-1,0,1,255}) for (int height : {0,149,150,151,500,5000}) {
        const int right = 1000-left;
        rr::RouteSample sample{{0,0,left,right,left+12,right+12},0,0,0};
        sample.offset = offset;
        std::vector<rr::RouteSample> route(128,sample);
        rr::Race race(route,route,tables,specs[0],75,{},0);
        auto& rider = race.riders()[0];
        rider.sample = rider.previous_sample = 51;
        rider.bike.i32(0x1c) = 51*256;
        rider.bike.i32(0x28) = (offset+relative)*256+fraction;
        rider.bike.i32(0x24) = height;
        rider.half_mirror.position = rider.bike.i32(0x1c);
        rider.half_mirror.lateral = rider.bike.i32(0x28);
        rider.half_mirror.sample = rider.sample;
        race.update_camera();
        if (!first) std::cout << ",";
        first = false;
        std::cout << "[" << id++ << "," << offset << "," << left << "," << right
          << "," << rider.bike.i32(0x28) << "," << race.camera_position()
          << "," << race.camera().x << "," << height << "," << race.camera().height << "]";
    }
    std::cout << "]\n";
} catch (const std::exception& error) { std::cerr << error.what() << "\n"; return 1; } }
