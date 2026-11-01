# Supported foundation platform and dependencies

| Target | Compile/lint | Runtime | Requirement |
| --- | --- | --- | --- |
| Linux x86_64, X11/XWayland | Local; clean Debian container job | Local smoke receipt when recorded | Initial target |
| Windows | Not tested | Not tested | Deferred |
| macOS | Not tested | Not tested | Deferred |

CI uses a Linux-only matrix deliberately; a successful container compile is not evidence of a working GPU or physical desktop.

Build prerequisites: pinned Rust toolchain, C/C++ linker toolchain, pkg-config, X11, Xi and xkbcommon development libraries. Debian/Ubuntu: `build-essential pkg-config libx11-dev libxi-dev libxkbcommon-dev python3`. Fedora: `gcc gcc-c++ pkgconf-pkg-config libX11-devel libXi-devel libxkbcommon-devel python3`. Runtime: X11/XWayland display, Vulkan loader and a compatible vendor or Mesa driver. Audio is not enabled and currently requires no ALSA dependency. Additional audio/gamepad dependencies will be added when those features are implemented.

Explicit Bevy features: `std` for standard library integration; `default_app` for baseline app/assets/state facilities; `bevy_log` for logging integration; `2d_bevy_render` for classic sprite rendering; `png` for screenshots and lossless asset previews; `bevy_winit` and `x11` for desktop window/input; `multi_threaded` for engine task scheduling. No 3D renderer is requested. Dependency-internal shared light/math crates are not evidence of 3D gameplay.

Clean build recipe: `podman run --rm --security-opt label=disable -v "$PWD:/source:ro" -v "$PWD/local/clean-build:/evidence" docker.io/library/rust:1.99.1-bookworm sh /source/tools/ci/clean_build.sh`. This copies source/configuration only to an empty container build directory and compiles without host Cargo artifacts. It needs network access for OS packages and crates. Record image digest and actual results in evidence.
