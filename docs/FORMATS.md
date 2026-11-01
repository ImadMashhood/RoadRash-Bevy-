# Source formats and decoding

`rr-formats` is safe Rust without Bevy. Source facts follow the pinned [reference revision](https://github.com/arkits/road-rash/tree/7aecf6044f2dd773b925b325823f30d648d61c89/src), especially resource.cpp, cel.cpp, bob.cpp, family.cpp, rri.cpp and exe_tables.cpp. No upstream C++ implementation was copied. The supported EXE hash is recorded in the baseline manifest. Unknown binaries cannot use version-specific address tables.

CRSR/LBTR directories use explicit little-endian reads and checked ranges. Exact payload aliases are allowed; partial overlaps and overlap with metadata are rejected. Raw header and directory words remain available in serialized inspection metadata; pointer-like fields are never dereferenced. CEL CCB/PLUT/PDAT chunks and unknown chunk headers are preserved. Indexed texture mip data starts at offset 28; odd mip heights round upward. Small legacy CEL drawing records remain opaque rather than being assigned invented pixel semantics.

BOB frames retain source order, dimensions and original indices, including intentional empty slots. HIBOB and LOBOB each contain 326 slots; LOCOP contains 45. The supplied disc has no HICOP, so LOCOP is the documented police fallback. Shadows are separate sheets, with source index zero transparent even when a remap changes its palette value. Ten supported rider remaps are extracted through bounded PE virtual-address translation. Palette data is RGBX; output alpha comes from the declared source-index policy.

Headerless BOB images use explicit source-family widths: high/low horizons 720/360, dashboards 640/320, BIGMETER 190, LILMETER 95 and MATTE 640. Cloud layouts remain unknown and their source bytes are retained. These widths are format facts, not runtime measurements.

RRI files are JPEG streams. The image crate supplies bounded standard RGB decoding; no indexed palette is invented for RGB data. Original legacy quantization is not emulated, so this does not promise pixel identity with the original executable. RGBA PNG exports round-trip exactly to our decoded pixels.

Compressed resource/family envelopes use [explode 0.1.2](https://docs.rs/explode/0.1.2/explode/) for PKWARE DCL with bounded reads and CRC verification. Family parsing walks array/OFST/CLGP relative offsets and extracts texture groups. Unknown leaves remain inspection records and source masters. Limits include 16 MiB decompression, 64 MiB decoded family pixels, 65,536 edges, bounded nodes/textures, and depth 32. Counts, dimensions, allocation products and offsets are checked before access.

Run `cargo test --locked -p rr-formats`. Tests include every-prefix truncation, bad magic/endian, overflow, alias/overlap policy, raw metadata JSON round-trip, known pixel/mip/remap/alpha vectors, empty frames, PE gating, DCL/CRC, family cycles, fanout and decompression limits. `cargo run --locked --release -p rr-formats --bin rr-format-fuzz -- 10000 local/format-fuzz.json` performs deterministic bounded mutation fuzzing over ten parsers. It is a reproducible smoke fuzz campaign, not an exhaustive security result or coverage-guided libFuzzer run. No panic occurred in the recorded campaign, so there is no failing input to minimize.

See [corpus coverage](evidence/corpus-04-06.json) and [acceptance receipt](evidence/parts-04-06.md). Private source masters and previews stay outside Git.
