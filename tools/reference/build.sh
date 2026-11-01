#!/usr/bin/env bash
set -euo pipefail
rr_repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$rr_repo"
rr_revision=7aecf6044f2dd773b925b325823f30d648d61c89
rr_source="$rr_repo/local/reference-cpp"
rr_build="$rr_repo/local/reference-build"
if [[ -f local/reference-tools/env.sh ]]; then source local/reference-tools/env.sh; fi
command -v cmake >/dev/null || { echo 'CMake is required. Fedora: dnf install cmake gcc-c++ SDL3-devel (or use the local bootstrap in docs/NATIVE_REFERENCE.md).' >&2; exit 1; }
if [[ ! -d "$rr_source/.git" ]]; then
    git clone https://github.com/arkits/road-rash.git "$rr_source"
    git -C "$rr_source" checkout --detach "$rr_revision"
fi
[[ "$(git -C "$rr_source" rev-parse HEAD)" == "$rr_revision" ]] || { echo 'Reference checkout has a different revision; preserve it and use the pinned revision in another directory.' >&2; exit 1; }
if git -C "$rr_source" apply --check "$rr_repo/tools/reference/fedora-gcc16.patch" 2>/dev/null; then
    git -C "$rr_source" apply "$rr_repo/tools/reference/fedora-gcc16.patch"
else
    git -C "$rr_source" apply --reverse --check "$rr_repo/tools/reference/fedora-gcc16.patch"
fi
cmake -S "$rr_source" -B "$rr_build" -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXE_LINKER_FLAGS="${LDFLAGS:-}"
cmake --build "$rr_build" --parallel "${RR_BUILD_JOBS:-6}"
# This separately authored probe calls the upstream public APIs directly.
"${CXX:-g++}" -O2 -std=c++20 -I "$rr_source/src" -I "${rr_tools_root:-}/usr/include" \
    tools/reference/measure.cpp "$rr_build/librr-resource.a" "$rr_build/jpeg-scalar/build/libjpeg.a" \
    "$rr_build/freetype/build/libfreetype.a" -lSDL3 -pthread -o "$rr_build/rr-measure"
printf 'Built %s\n' "$rr_build/rr-game"
