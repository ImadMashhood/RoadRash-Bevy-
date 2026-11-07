// Compare actual native cop sheet selection, RRAN registration and decoded pixels.
#include "assets.hpp"
#include "actor_descriptors.hpp"
#include "rider_sprites.hpp"
#include <iostream>
#include <iomanip>
int main(int argc, char** argv) { try {
    if (argc != 2) return 2;
    rr::Assets assets(argv[1]);
    const auto exe = assets.read("ROADRASH.EXE");
    rr::ActorAnimationLibrary library(exe, assets.read("DATA/GLOBAL.RSC"));
    const auto sheets = rr::load_rider_sprites(assets, exe, true);
    const auto palette = assets.read("DATA/PALETTE.RAW");
    bool first = true;
    std::cout << '[';
    for (int bank : {1, 3}) for (int frame = 0; frame < rr::cop_frames; ++frame) {
        const auto texture = rr::rider_texture(sheets, bank == 1 ? 0xde : 0xdf, frame);
        if (!first) std::cout << ',';
        first = false;
        std::cout << '[' << bank << ',' << frame;
        if (!texture) { std::cout << ",0]"; continue; }
        const auto& image = (bank == 1 ? sheets.cop : sheets.shadoc)[size_t(frame)];
        const auto& registration = library.graphics[size_t(bank)].frames[size_t(frame)];
        uint64_t hash = 14695981039346656037ull;
        for (const auto index : image.pixels) for (int channel = 0; channel < 4; ++channel) {
            const uint8_t byte = channel == 3 ? (index == 0 ? 0 : 255) : palette[size_t(index) * 4 + size_t(channel)];
            hash = (hash ^ byte) * 1099511628211ull;
        }
        std::cout << ",1," << image.width << ',' << image.height << ','
                  << registration.width << ',' << registration.height << ','
                  << registration.hotspot[0] << ',' << registration.hotspot[1] << ",\""
                  << std::hex << std::setw(16) << std::setfill('0') << hash << std::dec << "\"]";
    }
    std::cout << "]\n";
} catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; } }
