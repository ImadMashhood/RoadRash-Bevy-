// Native rail draw-window flags while the terminal camera pulls back.
#include "race.hpp"
#include "assets.hpp"
#include <algorithm>
#include <iostream>
int main(int argc, char** argv) { try {
    if (argc != 2) return 2;
    rr::Assets assets(argv[1]);
    const auto tables = rr::read_physics_tables(assets.read("ROADRASH.EXE"));
    const auto specs = rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
    bool first = true; std::cout << "[";
    for (int follow : {0,128,512,12800,32640}) for (int back : {0,3,6,255,768,897,900})
    for (int delta : {-1,0,1}) for (int flags : {0,1,2,3,252,255}) for (int height : {4,128,252,256}) {
        rr::PathSample p{}; p.left=p.right=500; p.shoulder_left=p.shoulder_right=50;
        p.event_type=5; p.event_parameters[2]=uint8_t(flags);
        rr::RouteSample sample{p,0,0,0}; std::vector<rr::RouteSample> route(512,sample);
        rr::Race race(route,route,tables,specs[0],75,{},0);
        rr::TerrainSlice terrain{}; terrain.type=255;
        race.set_terrain_source([&](const rr::RouteSample&)->const rr::TerrainSlice& { return terrain; });
        race.set_rail_height_source([&](const rr::RouteSample&) { return height; });
        race.riders()[0].half_mirror.position=follow; race.camera_back_=back;
        const int camera=int(race.camera_position());
        const int index=std::clamp((camera>>8)+31+delta,0,511);
        const auto view=race.sub_slice(race.riders()[0],size_t(index));
        if(!first) { std::cout << ","; }
        first=false;
        std::cout << "[" << follow << "," << back << "," << index << "," << flags << "," << height << "," << camera << "," << int(view.b3b) << "]";
    }
    std::cout << "]\n";
} catch(const std::exception& error) { std::cerr << error.what() << "\n"; return 1; } }
