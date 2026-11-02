# Native Linux review candidates

The release workflow builds an immutable source commit with locked dependencies,
runs formatting, Python/Rust checks and Clippy, collects dependency/font notices,
packages only the application, and verifies SHA256SUMS before publishing a
**prerelease**. Original game data is supplied locally at first launch.

`release/candidate.json` is an explicit request in a follow-up commit. It points
to the frozen source commit, which is an ancestor of the request. Only full
40-character SHAs and `native-functional-YYYY-MM-DD-rcN` tags are allowed. Existing
tags are rejected; the workflow never silently replaces a candidate. The build job
has read access; only the separate publish job has contents write access. Checkout
credentials are not retained for dependency build scripts. Publishing does not
mark G5 accepted or enable texture production.

The candidate contains three application binaries, a Python/Zenity launcher,
player guide, license-scope statement, 369 locked Linux dependency notice records
and the embedded font's OFL. It contains no original or converted commercial
images, source DLLs/EXE, sound, films, ISO or replay setups. The original game data
and the project's still-unselected source license are distinct from dependency
licenses. FFmpeg/graphics/audio system runtimes are documented external
prerequisites and are not redistributed as part of this archive.

Archive owner/group names/IDs and timestamps are normalized to the source commit
epoch. Two packages with identical candidate file bytes produce identical archive
bytes. A clean rebuild also verifies intended deterministic engine outputs and
content/notices identity. Cross-distro/compiler-path binary identity is not
promised; the published artifact has its own exact checksum and is downloaded
and tested on Fedora. The local clean-build archive and Ubuntu CI archive are
identified separately.

RC2 freezes functional code at `0c9a544bab00c90b96fb924cb85dc1972411ee0e`.
RC3 adds the cold-registry CI guard fix at
`bccd1d303e084e84a80674a5454b8966c55b276f`; its game source is unchanged.
The clean Fedora rebuild's game bytes match the frozen soak binary exactly
(`9e6e3c748425022d1fc3df727a508c753ab80a15e1dc11dace3308f5900109dc`).
The earlier CI headless guard failed before testing the intended lock mismatch
because its offline registry was empty. It now fetches the valid locked graph
before constructing the negative fixture. The desktop and candidate build checks
are recorded separately from that historical failure.

Download the archive and its adjacent SHA256SUMS, then run `sha256sum --check
SHA256SUMS` in the download directory. The acceptance check additionally verifies
every extracted file against `candidate-manifest.json`. Extract into a directory
you own; run `./RoadRash`. No Cargo or developer checkout is required.

The source-hidden sandbox test mounts the workspace out of view, removes Cargo
from PATH and runs a read-only copied application with a fresh data/profile root.
Own-disc import verifies 341 files and converts the native catalog. A correct
120-frame startup passes. A deliberately invalid 90-frame smoke argument was
rejected in an earlier probe; that failure is retained in the receipt and is not
counted as a successful startup. Missing-disc guidance is tested separately.
Upgrade preserves the exact career/preferences/import pointer; deleting only the
application preserves those same user files. The original ISO is never deleted.

The ordinary packaged keyboard walkthrough and explicit corruption recovery are
separate from instrumented smoke tests. The downloaded build also replays the
complete level-five finale on Fedora and must produce the audited canonical hash.
The final release receipt names the exact tested artifact, platform, checksums,
known issues and remaining owner acceptance. [Player guide](PLAYER_GUIDE.md),
[native acceptance](NATIVE_ACCEPTANCE.md).

The workflow follows GitHub's documented
[GITHUB_TOKEN permissions](https://docs.github.com/en/actions/tutorials/authenticate-with-github_token)
and [prerelease CLI](https://cli.github.com/manual/gh_release_create).

Actual RC3 build/publish and both headless/desktop CI jobs pass: [candidate run](https://github.com/ImadMashhood/RoadRash-Bevy-/actions/runs/41358300204), [foundation run](https://github.com/ImadMashhood/RoadRash-Bevy-/actions/runs/41358300172), [step-level receipt](evidence/native-ci.json). These runs identify the frozen application/request commits; a later evidence-only follow-up is not silently included in those pass claims.

The current review artifact is [RC4](https://github.com/ImadMashhood/RoadRash-Bevy-/releases/tag/native-functional-2026-11-01-rc4) from source `234337749774a5a5a6d3fc9cafa88b9c1d55f3dd`, archive SHA-256 `02821ad359c978cebf82350e62d79bafddc159e945177ef47e694d56162d3f11`, downloaded game `c744f04d7a18c636371e5147ce43210c5c8a7ec50ec800aa2b6dc5e815f5a0c8`. The clean Fedora build matches local frozen game `1c02e7700d3fb5b516dd8fdbd5c41c9d9a444c7dc312c73ccca2df0b1dc03035`; two local normalized archives match `882a940a22515984411377ae2aa270b6e58ebf3cacbd942b0a4b826ae3fa5699`. These local bytes are distinguished from the Ubuntu artifact. All current [CI jobs](evidence/native-ci.json), [download hashes](evidence/native-download-rc4.json), [source-hidden launch](evidence/native-package-sandbox-rc4.json), [full packaged career](evidence/native-packaged-career.json), [graphical finale](evidence/native-delivered-runtime.json) and [real hour](evidence/native-performance.json) pass for their stated scope. Earlier RC2/RC3 receipts above are history. Public CI skips private ISO checks explicitly; the separate actual local `--require` audit passes 341/341.
