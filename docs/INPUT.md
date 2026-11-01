# Action bindings

Part 9 has started with task 09.01. `rr-input` owns versioned, renderer-independent action maps and configuration I/O. The game translates Bevy key codes at its composition boundary; simulation commands contain actions rather than physical keys. Keyboard mappings are active now. Gamepad devices, analog calibration and source handling remain open tasks 09.02 onward.

```sh
cargo run --locked -p rr-input -- init local/bindings.json
cargo run --locked -p rr-input -- set local/bindings.json race accelerate Space
cargo run --locked -p rr-input -- validate local/bindings.json
cargo run --locked -p rr-game -- --bindings local/bindings.json \
  --install-root local/game-install --catalog local/catalog --course CITY
cargo run --locked -p rr-input -- reset local/bindings.json race
```

`set` replaces that action's alternatives; separate alternatives with commas, such as `ArrowUp,KeyW`. Unknown physical key/action/context names are rejected. The [default file](fixtures/default-bindings.json) lists supported actions in `race`, `menu`, and `paused` contexts. Supported keys are arrows, A–Z (`KeyA` etc.), digits (`Digit0` etc.), Space, Escape, Enter, Tab, Backspace, left/right Shift/Control/Alt and F1–F12. Changes take effect on the next launch; an in-game binding editor and menus are later UI work.

All required actions must remain bound. Each action accepts one to four distinct alternatives. Duplicate/conflicting physical keys are rejected **within a context**, with both action names in the error; different contexts may reuse a key. For example, W accelerates in the race map and navigates up in the menu map. Menus consume their map when implemented in the later UI phase; no front-end menu is claimed here. Resetting one context preserves customizations in the others; `reset FILE all` restores the entire map.

Reads are capped at 64 KiB. Unknown fields/schema versions are rejected. Edits validate before changing state or touching disk. Writes use a same-directory exclusive temporary file, file sync and rename on the supported Linux platform. Invalid edits preserve the previous complete file; interrupted temporary files are never read as active configuration. `init` refuses to overwrite an existing file. Parent directories must already exist.

Default driving controls remain Up/W, Down/S, Left/A and Right/D. Simultaneous left/right cancels; braking precedence stays in the simulation. Escape/P pauses. Enter/P resumes after movement controls return to neutral. Losing window focus also pauses and clears the pending command; regaining focus alone never resumes. The window title shows the configured Resume alternatives. These keyboard lifecycle checks are preparatory work for task 09.11, which remains open until active-controller disconnect behavior is implemented and verified.

Recorded replays bypass physical keyboard/pause sampling and retain their existing canonical hashes. No handling constants, simulation units or replay schema changed in this input slice. Five pure input tests, two actual Bevy adapter tests and [CLI acceptance](evidence/input-bindings-cli.json) cover defaults, context reuse, conflict reporting, rejected-edit rollback, schema/size limits, durable complete-file replacement, context-specific reset, actual remapped tick commands, opposed steering and focus/neutral/explicit resume. Physical gamepad hardware has not been tested.
