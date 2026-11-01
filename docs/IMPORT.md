# Local game-data importer

`rr-import <source.iso | extracted ROADRASH directory> <private installation root>` imports only the supported, hash-pinned 341-file corpus. Unknown or modified executable/data revisions are rejected with per-file diagnostics. The directory argument must point directly to ROADRASH, not its parent. Unicode/spaces are supported in the selected source/destination path; original logical asset paths use case-insensitive matching. Case collisions, symlinks and special source files are rejected.

The implementation is Rust, independent of Bevy. Bounded ISO parsing lives in `crates/import/src/iso.rs`, validation/installation in its library, and signal/argument handling in its CLI. JSON and hashes use explicit versioned records. Errors distinguish missing, wrong-size, wrong-hash, unreadable, unsupported input, cancellation and lock contention.

## Transaction layout

```text
private-install/
  import.lock                    OS advisory lock; released automatically on process exit
  active.json                    atomically replaced active-generation record
  installs/
    install-<unique-id>/
      ROADRASH/                  verified files, original logical paths
      receipt.json               source hashes, manifest hash, importer build hash, recipe version
    .rr-staging-<unique-id>/      incomplete, never used by the game
```

1. Validate manifest paths, hashes, sizes and limits; reject source/destination overlap.
2. Lock the private root and recover abandoned importer-reserved staging/pointer names.
3. Read/copy each required file in 64 KiB chunks, checking cancellation and hashing bytes while writing.
4. Report all missing/size/hash/read failures found in the required set; discard this attempt's stage on error.
5. Flush files and directory metadata; write the provenance receipt and move the complete generation into place.
6. Atomically replace active.json and synchronize the installation root. The active pointer is the commit point. A final filesystem sync failure can make durability uncertain; inspect the active receipt before retrying after storage failure. Power-loss behavior depends on the filesystem honoring atomic rename/fsync semantics.

Failure or cancellation before the commit point preserves the previous active generation. A process killed after generation completion can leave an unreferenced complete generation; it is deliberately retained. A subsequent locked import removes only reserved `.rr-staging-*` directories and `.rr-active-*` pointer temporaries. It never deletes old completed generations or unrelated user directories. Reserved names belong exclusively to the importer. Disk reclamation of old completed generations is a later explicit management operation.

The installation root must be private, user-owned, and not concurrently altered by hostile processes. The advisory lock serializes cooperating importers. The importer rejects observed symlink components; it is not a sandbox against a same-user adversary racing filesystem changes. Imported data is treated as immutable by the game and importer. Manually editing it is unsupported; reimport from verified media.

## Bounds and compatibility

Default limits: source ISO 2 GiB, individual file 512 MiB, required total 1 GiB, directory buffer 16 MiB, 10,000 source records, directory depth 32. Checked arithmetic validates extents before allocating or seeking. ISO primary volume uses 2048-byte logical blocks; multi-extent/interleaved/extended-attribute records are rejected. Rock Ridge and Joliet features are not interpreted. The supported disc imports without those features.

Earlier development schema-1 receipts without a build hash deserialize as `legacy-unrecorded`; new receipts always record a source-build hash. This compatibility path does not invent provenance for older installs.

Initial production target is Linux x86_64. Directory fsync, replacement semantics and signal handling must be ported/tested before claiming Windows/macOS import support. No Python or Wine is needed by the Rust importer or game. Python tools remain research/acceptance helpers.

## Verification

Public tests: `cargo test --locked -p rr-import`. Authored fixtures exercise valid directories and ISO images, wrong/missing/unreadable files, case collisions, executable modifications, unsafe paths, symlinks, limits/overflow, cancellation at every probe, stale recovery, pointer/receipt tampering and concurrent lock rejection.

Private process checks: `python3 tools/private_import.py --iso local/source.iso --binary ../target/release/rr-import --game ../target/release/rr-game --workspace local/new-acceptance-directory`. The workspace must be new. This performs real import, Ctrl+C cancellation, forced process termination, recovery, unknown-revision rejection, read-only folder import, Unicode paths and launch against the retained active install. It writes only under the selected private test workspace.

Public CI explicitly prints SKIP for unconfigured private corpus tests through `tools/private_corpus.py`; `--require` makes absence fail for a private acceptance gate. No skipped private test is counted as a passed synthetic test.
