# Saves, preferences and recovery

Part 17 implements `rr-session::persistence` and the Bevy frontend storage adapter. New saves use schema 2: bounded career/stable identifiers, a content requirement list and SHA-256 over the canonical body. No Bevy entities, source asset bytes or transient race world are serialized. Maximum file size is 64 KiB; hashes detect corruption, not hostile editing. Validate schema, integrity, money, identifiers, progression, settlement ledger and pack compatibility before replacing the active career.

## Paths and slots

| Platform | Data root | Configuration root |
| --- | --- | --- |
| Linux | `$XDG_DATA_HOME/roadrash-rust` or `~/.local/share/roadrash-rust` | `$XDG_CONFIG_HOME/roadrash-rust` or `~/.config/roadrash-rust` |
| Windows (path policy, untested) | `%APPDATA%/RoadRash-Rust` | Same |
| macOS (path policy, untested) | `~/Library/Application Support/RoadRash-Rust` | Same |

Relative XDG overrides are ignored. `--profile-root local/test-profile` puts data in `data/` and settings in `config/` below that root. The default career is `data-root/careers/default.json`; slots 1–4 are `default-slot-1.json` through `default-slot-4.json`. Slot 0 is the base file. `--career-file PATH.json` changes the base filename; numbered slots use its stem. Preferences live in `config-root/settings.json`; `--settings-file` overrides it. Launch flags explicitly supplied for saved settings take precedence at startup. The saved active slot is used for Big Game resume.

The Restroom offers Save, Save As, Load, Restore Backup, settings and input configuration. Save As selects one of five slots. A successful slot save/load changes the active slot; a failed load leaves the current career and selected file unchanged. Results, purchases and repairs save through the same adapter. Preferences schema 1 stores bindings, volume, framing, native/override selection, video policy and active slot separately. Selected soundtrack playback is transient. Graphics Reset resets framing/override settings while preserving career, audio and bindings. Configure Input validates rebinding conflicts. Unsupported old key names fall back with diagnostics; missing fields use validated defaults. Unknown schemas are rejected.

## Atomicity and backup policy

Writes validate new bytes and any existing primary first. A per-file `create_new` PID lock prevents competing writers. Unique temporary file → write → file sync → atomically prepare previous valid `.bak` → directory sync → replace primary → directory sync. Only one previous valid generation is retained. Guarded paths reject traversal, non-JSON destinations and overlap with the immutable import before creating directories. Owned temporary files and locks are cleaned on ordinary errors. Fault tests cover all six stages. Before replacement, old data survives; after replacement, new data may already be valid even if a later durability sync reports an error. This is a Linux-tested protocol, not a claim of power-loss certification or Windows replacement semantics.

A damaged primary is never silently overwritten. Close other game processes, then choose its slot and Restore Backup in the Restroom. The adapter validates `.bak`, renames the previous primary to `.corrupt.PID.serial`, installs the validated backup and retains `.bak`. If installation fails it attempts to put the preserved primary back. The preserved file can be valid: the suffix means displaced by explicit recovery. Inspect it before deleting it. Oversized corrupt primaries are moved without reading them into memory.

If the process was killed, a `.json.lock` or unique `.tmp.PID.serial` can remain. Ensure **all game processes are closed**, copy the primary, backup and sidecars elsewhere, then remove only that file's stale lock and abandoned temporary files. Relaunch and use explicit recovery. Never remove another running writer's lock. To reset settings, close the game and move `settings.json` and its backup aside; defaults load on restart without touching careers. To start a fresh career, use an unused slot or separately archive the chosen career and its sidecars. These operations were followed in an isolated Linux profile; real user profiles were not modified.

## Migration and content policy

The independently authored [v1 fixture](fixtures/career-v1.json) exercises migration from Part 16's JSON snapshot. Decoding v1 returns a migration record without rewriting the source. The next explicit save emits v2 and preserves the previous valid v1 bytes in `.bak`; reopening v2 does not migrate again. Unsupported future versions reject safely. To resume a previous local bridge, pass `--career-file local/career-session.json` explicitly; default locations changed to OS user directories.

`rr:base`, version 1 records the supported executable digest (or the independently authored fixture identity). Required simulation packs must match ID/version/hash exactly; extra active simulation packs also reject. Missing presentation-only packs warn and fall back to native visuals. This validates the contract, not a completed mod loader; Part 23 supplies stable mod packaging. **Original binary game-save import is deferred and is not a native-release requirement.** Binary headers are identified and rejected with an explicit message; this project does not claim binary compatibility.

[Implementation](../crates/session/src/persistence/mod.rs), [disk adapter](../crates/session/src/persistence/disk.rs), [frontend adapter](../apps/game/src/frontend/storage.rs), [tests](../crates/session/tests/persistence.rs), [Part 17 evidence](evidence/part-17.md).
