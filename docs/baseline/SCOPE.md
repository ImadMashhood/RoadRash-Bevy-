# Scope and implementation baseline

## Product direction

The user has authorized implementation through roadmap Phase 23 and has now explicitly deferred 3D. Build the classic PC game experience in Rust/Bevy with modern module boundaries, validated content loading, deterministic simulation and replaceable 2D presentation. The separate local 3D prototype is historical research input. Current work has no mesh, rig, lighting or 3D physics requirements.

First target: Linux x86_64 desktop, X11/XWayland and Vulkan-capable GPU. Windows and macOS are later targets until compile and hardware receipts exist. Initial modes: offline Thrash and Big Game. Online multiplayer, mobile, browser, and realistic motorcycle physics are outside initial release scope.

## Supported source revision

Only the supplied 1997 rerelease corpus is supported initially. ROADRASH.EXE is 541184 bytes with SHA-256 `73d7f9744b5b1a9482b3909950cdf3fe34119fe13d023b031e9c6739a60f18a2`. Required data consists of 341 files from the pinned reference manifest. Alternate discs require a separately reviewed manifest and executable-table adapter; no best-effort offset decoding of unknown binaries.

Reference source revision: `7aecf6044f2dd773b925b325823f30d648d61c89` from https://github.com/arkits/road-rash. Upstream source is research evidence. No C++ implementation is incorporated here. Original captures and full behavior certification remain pending.

## Fidelity policy

Simulation targets observed original handling, combat, schedules, progression and outcomes. Presentation-only texture changes must leave tick hashes unchanged. Optional assists and changed rules require explicit configuration and replay metadata. Keep native indexed images, palette/remap masks and upscaled derivatives distinct. 3D production is deferred.

Classic comparison target: 640×480, 4:3, original assets and controlled camera checkpoints; this is an initial comparison convention, not a measured claim about every original display mode. Current presentation target: resizable window and classic road projection with original sprites and optional approved texture replacements. Camera calibration and actual graphics/device acceptance are unverified.

## Execution policy

Work task-by-task. Do not claim a phase complete while its behavioral evidence is missing. Dependencies may be prototyped to unblock measurement, but remain labeled as partial. Commits and evidence use stable README task IDs. The implementing agent owns active work; release acceptance belongs to the project owner. No calendar commitment is implied.
