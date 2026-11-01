# Native reference on Fedora

The pinned [C++/SDL3 port](https://github.com/arkits/road-rash/tree/7aecf6044f2dd773b925b325823f30d648d61c89) now builds and runs natively on this Fedora 44 x86_64 machine with the same verified private import as the Rust game. It is a reference implementation, **not the original Windows executable**. The owner's retired original-executable comparison gates remain retired.

## Run the installed build

From this repository:

```sh
./tools/reference/run.sh
./tools/reference/run.sh --course 1 --level 1 --bike 0
./tools/reference/run.sh --version
```

The application menu also contains **Road Rash Native Reference**, pointing to this workspace launcher. Keep this workspace in place or update that desktop entry after moving it. The launcher resolves the active local import and isolates native saves/settings in `local/reference-prefs/interactive/`. It never needs Wine or the Windows AWE DLL. The default window is 1280×960; `--window-scale 1` selects 640×480. Up/W accelerates, Down/S brakes, arrows/A/D steer; Z/X/C attacks and P/Escape pauses. These are the native port's controls, separate from the Rust action map.

The native CLI course indices are **0 Napa, 1 City, 2 Highway, 3 Canyon, 4 Medley**. Source resource names remain CITY/HIWAY/MEDLY/NAPA/CANYN. Do not confuse the two orderings.

## Build again

On Fedora with system development packages, the dependencies are CMake, a C++20 compiler, SDL3 development headers and the runtime libraries used by the pinned port. The usual system command is `sudo dnf install cmake gcc-c++ SDL3-devel`; it was **not** executed for this build. This host already supplied GCC C support and desktop runtime libraries. The following alternative downloads/extracts Fedora RPM development files into the user's own toolchain directory without installing system packages:

```sh
python3 tools/reference/bootstrap-fedora.py
source local/reference-tools/env.sh
./tools/reference/build.sh
```

Bootstrap is specific to Fedora x86_64 with GCC already available; it is not an installer for other distributions. It uses the enabled Fedora/updates repositories, records downloaded RPMs under ignored `local/reference-tools/rpms/`, places tools under `~/.local/share/roadrash-native/toolchain/`, and writes an ignored environment file. It does not request a password. Do not source this compiler environment in an unrelated shell when you want another toolchain.

The build script checks out revision `7aecf6044f2dd773b925b325823f30d648d61c89` in `local/reference-cpp/`, builds Release in `local/reference-build/`, and applies only [two explicit `<climits>` includes](../tools/reference/fedora-gcc16.patch) required by GCC 16's headers. It preserves unfamiliar changes and refuses a different revision. This patch makes the upstream provenance report `dirty: true`; no native gameplay or rendering logic was patched. Pinned FreeType, scalar JPEG and network dependencies are obtained by upstream CMake. SDL3 3.4.18 comes from this Fedora installation.

The same build also compiles our separately authored [measurement probe](../tools/reference/measure.cpp), which calls upstream projection, actor and scene APIs. Source game files stay in the existing immutable import. Nothing under `local/` is published.

## Verify and compare

For upstream disc-dependent tests, provide its expected relative corpus path without copying it:

```sh
mkdir -p local/cdrom
# Point this symlink at your active imported ROADRASH directory.
ln -s ../game-install/installs/YOUR_GENERATION/ROADRASH local/cdrom/ROADRASH
source local/reference-tools/env.sh
ctest --test-dir local/reference-build --output-on-failure --parallel 4
```

This machine registered 134 tests: **119 passed, 15 skipped, zero failed**. The skipped private oracle/font/image/save-fixture tests are listed in [the build receipt](evidence/native-linux-build.json). We do not turn unavailable original-runtime fixtures into successful comparisons.

Generate repeatable fixed-pose native/Rust comparisons:

```sh
cargo build --locked --release -p rr-render
python3 tools/reference/compare.py \
  --assets local/cdrom/ROADRASH --install local/game-install --catalog local/catalog \
  --native local/reference-build/rr-measure --rust target/release/rr-render \
  --output local/native-comparison
```

Pillow is needed only by this comparison tool to convert the native probe's PPM captures into PNGs. Fifteen poses (source samples 50/75/120 across all five courses) now match rider rectangles exactly and all 495 road vectors within 1/1024 pixel, which covers the native JSON diagnostic's six-significant-digit precision. [Measurements](evidence/render-native-sizing.json) include geometry and image hashes; captures stay local. This is a source-port geometry/registration comparison, not whole-frame pixel parity. Terrain random-call order, texture subdivision/filtering/group selection, sky details, animation, shadows and HUD remain visible differences.

The user's sizing report exposed four actual errors: using BOB storage dimensions rather than RRAN logical geometry, substituting a bottom-center hotspot, using an authored camera depth, and reversing the vertical placement around non-bottom hotspots. The renderer now reads the source frame geometry, uses actor size 512 and player projection depth 512, and preserves bitmap extensions below their ground anchor. A depth regression checks both complete sprite visibility over road and occlusion by a nearer hill. Another comparison exposed Canyon walls selecting category-3 tunnel art instead of category-1 landscape art; wall faces/caps now resolve the landscape frames. A colored authored fixture guards that binding.

Frame 2 is logically 29×64 with hotspot (19,52), while the high-resolution BOB stores 58×128 pixels. At City sample 50, the same frame's 640×480 rectangle is **(287,198)–(345,326)** in both renderers. Source sprite metadata, not enhancement pixel dimensions, controls that rectangle. `--asset` selections also resolve their own RRAN frame registration. The source camera follows one sample behind the player; the first boundary already has depth 256, so the player boundary projects at depth 512. The millimetre/source-unit bridge remains provisional until the later Part 9 handling work.

## Measure native performance

```sh
python3 tools/reference/benchmark.py --assets local/cdrom/ROADRASH \
  --output local/native-benchmark
```

This launches three original-mode City runs with 14 opponents, eight rendering workers, a 640×480 surface, OpenGL and vsync disabled. Each run skips 4,000 warmup frames, then times 3,000 frames. Settings/saves are isolated under the output directory. Avoid other builds/heavy workloads during timing. Reports include frame percentiles, phases, simulation ticks and clock drift. Larger warmups may be needed on other hardware to get beyond the initial countdown.

On this Ryzen 5 3600 machine, the three means were **306.26, 299.82 and 300.77 fps**, with mean frame times **3.265, 3.335 and 3.325 ms**. Each measured interval covered 590–603 simulation ticks after warmup. [Raw timing metadata](evidence/native-linux-benchmark.json) records the exact settings and percentiles. These are short desktop throughput measurements, not vsync/display latency, an extended soak, original executable parity or a feature-equivalent speed comparison against Rust. The first 300-frame/90-warmup smoke run was still in the countdown and is deliberately excluded from these reported measurements.
