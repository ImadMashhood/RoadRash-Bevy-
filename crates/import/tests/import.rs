use rr_import::{ImportError, Limits, Manifest, RequiredFile, active_install, import};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use tempfile::tempdir;
fn fixture(path: &Path) -> Manifest {
    fs::create_dir_all(path.join("DATA")).unwrap();
    fs::write(path.join("ROADRASH.EXE"), b"authored exe").unwrap();
    fs::write(path.join("DATA/TEST.BIN"), [1, 2, 3, 4]).unwrap();
    Manifest {
        schema_version: 1,
        upstream_revision: "authored-fixture".into(),
        files: vec![
            req("ROADRASH.EXE", b"authored exe"),
            req("DATA/TEST.BIN", &[1, 2, 3, 4]),
        ],
    }
}
fn req(path: &str, bytes: &[u8]) -> RequiredFile {
    RequiredFile {
        path: path.into(),
        size: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    }
}
fn run(src: &Path, dst: &Path, m: &Manifest) -> rr_import::Result<rr_import::Installation> {
    import(src, dst, m, &Limits::default(), || false)
}
#[test]
fn directory_round_trip_unicode_read_only_and_provenance() {
    let temp = tempdir().unwrap();
    let src = temp.path().join("私の disc source");
    let dst = temp.path().join("install raíz");
    let m = fixture(&src);
    fs::rename(src.join("ROADRASH.EXE"), src.join("roadrash.exe")).unwrap();
    fs::rename(src.join("DATA"), src.join("data")).unwrap();
    let mut permissions = fs::metadata(src.join("roadrash.exe"))
        .unwrap()
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(src.join("roadrash.exe"), permissions).unwrap();
    let installed = run(&src, &dst, &m).unwrap();
    let active = active_install(&dst).unwrap();
    assert_eq!(installed.generation, active.generation);
    assert_eq!(
        fs::read(active.data_root.join("ROADRASH.EXE")).unwrap(),
        b"authored exe"
    );
    let receipt: serde_json::Value = serde_json::from_slice(
        &fs::read(active.data_root.parent().unwrap().join("receipt.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["recipe_version"], 1);
    assert_eq!(receipt["importer_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(receipt["files"].as_array().unwrap().len(), 2);
}
#[test]
fn reports_all_mismatches_and_keeps_old_install_usable() {
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let d = t.path().join("dst");
    let mut m = fixture(&s);
    let old = run(&s, &d, &m).unwrap();
    fs::write(s.join("ROADRASH.EXE"), b"modified exe").unwrap();
    fs::write(s.join("DATA/TEST.BIN"), b"x").unwrap();
    m.files.push(req("MISSING", b"missing"));
    let Err(ImportError::Validation(problems)) = run(&s, &d, &m) else {
        panic!("expected validation errors")
    };
    assert_eq!(
        problems.iter().map(|p| p.kind.as_str()).collect::<Vec<_>>(),
        vec!["wrong-hash", "wrong-size", "missing"]
    );
    assert_eq!(active_install(&d).unwrap().generation, old.generation);
    assert_eq!(
        fs::read(old.data_root.join("DATA/TEST.BIN")).unwrap(),
        [1, 2, 3, 4]
    );
    assert_eq!(fs::read_dir(d.join("installs")).unwrap().count(), 1);
}
#[test]
fn executable_correct_modified_and_same_size_unrelated() {
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    let d = t.path().join("dst");
    run(&s, &d, &m).unwrap();
    for bytes in [b"Authored exe".as_slice(), b"xxxxxxxxxxxx".as_slice()] {
        fs::write(s.join("ROADRASH.EXE"), bytes).unwrap();
        let Err(ImportError::Validation(p)) = run(&s, &d, &m) else {
            panic!("expected executable mismatch")
        };
        assert_eq!(p[0].path, "ROADRASH.EXE");
        assert_eq!(p[0].kind, "wrong-hash");
    }
}
#[test]
fn cancellation_at_multiple_copy_and_activation_points_preserves_pointer() {
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    let d = t.path().join("dst");
    let old = run(&s, &d, &m).unwrap();
    let probes = AtomicUsize::new(0);
    import(&s, &d, &m, &Limits::default(), || {
        probes.fetch_add(1, Ordering::Relaxed);
        false
    })
    .unwrap();
    for stop in 0..probes.load(Ordering::Relaxed) {
        let pointer = fs::read(d.join("active.json")).unwrap();
        let n = AtomicUsize::new(0);
        assert!(matches!(
            import(&s, &d, &m, &Limits::default(), || n
                .fetch_add(1, Ordering::Relaxed)
                >= stop),
            Err(ImportError::Cancelled)
        ));
        assert_eq!(fs::read(d.join("active.json")).unwrap(), pointer);
        assert!(!fs::read_dir(d.join("installs")).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".rr-staging-")
        }));
    }
    assert!(old.data_root.join("ROADRASH.EXE").exists());
}
#[test]
fn rejects_case_collisions_and_parent_absolute_or_backslash_manifest_paths() {
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    let d = t.path().join("dst");
    fs::write(s.join("roadrash.exe"), b"duplicate").unwrap();
    assert!(
        run(&s, &d, &m)
            .unwrap_err()
            .to_string()
            .contains("case-colliding")
    );
    fs::remove_file(s.join("roadrash.exe")).unwrap();
    for path in [
        "../escape",
        "/absolute",
        "DATA/../escape",
        "C:/absolute",
        "DATA\\escape",
        "",
    ] {
        let mut invalid = m.clone();
        invalid.files[0].path = path.into();
        assert!(run(&s, &d, &invalid).is_err());
    }
}
#[cfg(unix)]
#[test]
fn rejects_source_destination_and_active_symlinks() {
    use std::os::unix::fs::symlink;
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    let d = t.path().join("dst");
    let outside = t.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&s, t.path().join("source-link")).unwrap();
    assert!(run(&t.path().join("source-link"), &d, &m).is_err());
    symlink(&outside, &d).unwrap();
    assert!(run(&s, &d, &m).is_err());
    fs::remove_file(&d).unwrap();
    symlink(&outside, s.join("escape")).unwrap();
    assert!(run(&s, &d, &m).is_err());
    fs::remove_file(s.join("escape")).unwrap();
    fs::write(outside.join("sentinel"), b"safe").unwrap();
    symlink(outside.join("sentinel"), d.join("active.json")).unwrap();
    assert!(run(&s, &d, &m).is_err());
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"safe");
}
#[test]
fn recovers_abandoned_stage_but_preserves_unrelated_directories() {
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    let d = t.path().join("dst");
    fs::create_dir_all(d.join("installs/.rr-staging-abandoned")).unwrap();
    fs::write(d.join("installs/.rr-staging-abandoned/partial"), b"partial").unwrap();
    fs::create_dir_all(d.join("installs/user-backup")).unwrap();
    fs::write(d.join(".rr-active-abandoned"), b"incomplete pointer").unwrap();
    run(&s, &d, &m).unwrap();
    assert!(!d.join("installs/.rr-staging-abandoned").exists());
    assert!(d.join("installs/user-backup").exists());
    assert!(!d.join(".rr-active-abandoned").exists());
}
#[test]
fn enforces_manifest_file_total_entry_and_depth_limits() {
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    let d = t.path().join("dst");
    for limits in [
        Limits {
            max_file_bytes: 2,
            ..Limits::default()
        },
        Limits {
            max_total_bytes: 2,
            ..Limits::default()
        },
        Limits {
            max_entries: 2,
            ..Limits::default()
        },
        Limits {
            max_depth: 0,
            ..Limits::default()
        },
    ] {
        assert!(import(&s, &d, &m, &limits, || false).is_err());
    }
    let mut overflow = m.clone();
    overflow.files[0].size = u64::MAX;
    overflow.files[1].size = 1;
    let limits = Limits {
        max_total_bytes: u64::MAX,
        max_file_bytes: u64::MAX,
        ..Limits::default()
    };
    assert!(
        import(&s, &d, &overflow, &limits, || false)
            .unwrap_err()
            .to_string()
            .contains("overflow")
    );
}
#[test]
fn concurrent_import_lock_and_corrupt_receipt_are_rejected() {
    use fs2::FileExt;
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    let d = t.path().join("dst");
    let active = run(&s, &d, &m).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(d.join("import.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    assert!(matches!(run(&s, &d, &m), Err(ImportError::Busy)));
    lock.unlock().unwrap();
    fs::write(
        active.data_root.parent().unwrap().join("receipt.json"),
        b"{}",
    )
    .unwrap();
    assert!(
        active_install(&d)
            .unwrap_err()
            .to_string()
            .contains("hash mismatch")
    );
}
#[cfg(unix)]
#[test]
fn unreadable_source_and_unwritable_destination_return_errors() {
    use std::os::unix::fs::PermissionsExt;
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    let d = t.path().join("dst");
    fs::set_permissions(s.join("ROADRASH.EXE"), fs::Permissions::from_mode(0o0)).unwrap();
    let result = run(&s, &d, &m);
    fs::set_permissions(s.join("ROADRASH.EXE"), fs::Permissions::from_mode(0o600)).unwrap();
    let Err(ImportError::Validation(p)) = result else {
        panic!("expected unreadable source")
    };
    assert_eq!(p[0].kind, "unreadable");
    fs::set_permissions(&d, fs::Permissions::from_mode(0o500)).unwrap();
    let result = run(&s, &d, &m);
    fs::set_permissions(&d, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
}
// An entirely authored tiny ISO builder. No original payload.
fn record(name: &[u8], sector: u32, size: u32, dir: bool) -> Vec<u8> {
    let n = (33 + name.len() + 1) & !1;
    let mut r = vec![0; n];
    r[0] = n as u8;
    r[2..6].copy_from_slice(&sector.to_le_bytes());
    r[6..10].copy_from_slice(&sector.to_be_bytes());
    r[10..14].copy_from_slice(&size.to_le_bytes());
    r[14..18].copy_from_slice(&size.to_be_bytes());
    r[25] = if dir { 2 } else { 0 };
    r[32] = name.len() as u8;
    r[33..33 + name.len()].copy_from_slice(name);
    r
}
fn put(image: &mut [u8], at: usize, bytes: &[u8]) {
    image[at..at + bytes.len()].copy_from_slice(bytes);
}
fn tiny_iso() -> Vec<u8> {
    let mut b = vec![0; 24 * 2048];
    put(&mut b, 16 * 2048, b"\x01CD001\x01");
    put(&mut b, 16 * 2048 + 128, &2048u16.to_le_bytes());
    put(&mut b, 16 * 2048 + 130, &2048u16.to_be_bytes());
    put(&mut b, 16 * 2048 + 156, &record(&[0], 20, 2048, true));
    put(&mut b, 20 * 2048, &record(b"ROADRASH", 21, 2048, true));
    put(&mut b, 21 * 2048, &record(b"ROADRASH.EXE;1", 22, 12, false));
    put(&mut b, 22 * 2048, b"authored exe");
    b
}
#[test]
fn imports_authored_iso_and_rejects_outside_truncated_and_cyclic_records() {
    let t = tempdir().unwrap();
    let src = t.path().join("test.iso");
    let dst = t.path().join("dst");
    let m = Manifest {
        schema_version: 1,
        upstream_revision: "synthetic".into(),
        files: vec![req("ROADRASH.EXE", b"authored exe")],
    };
    fs::write(&src, tiny_iso()).unwrap();
    let active = run(&src, &dst, &m).unwrap();
    assert_eq!(
        fs::read(active.data_root.join("ROADRASH.EXE")).unwrap(),
        b"authored exe"
    );
    for variant in 0..4 {
        let mut b = tiny_iso();
        match variant {
            0 => {
                put(
                    &mut b,
                    21 * 2048,
                    &record(b"ROADRASH.EXE;1", u32::MAX, 12, false),
                );
            }
            1 => {
                b.truncate(2048);
            }
            2 => {
                put(&mut b, 21 * 2048, &record(b"CYCLE", 20, 2048, true));
            }
            _ => {
                b[21 * 2048] = 20;
            }
        }
        fs::write(&src, b).unwrap();
        assert!(run(&src, &dst, &m).is_err());
        assert_eq!(active_install(&dst).unwrap().generation, active.generation);
    }
}
#[test]
fn iso_directory_depth_and_entry_bounds_are_enforced() {
    let t = tempdir().unwrap();
    let s = t.path().join("a.iso");
    fs::write(&s, tiny_iso()).unwrap();
    let m = Manifest {
        schema_version: 1,
        upstream_revision: "synthetic".into(),
        files: vec![req("ROADRASH.EXE", b"authored exe")],
    };
    for limits in [
        Limits {
            max_depth: 0,
            ..Limits::default()
        },
        Limits {
            max_entries: 1,
            ..Limits::default()
        },
        Limits {
            max_directory_bytes: 1024,
            ..Limits::default()
        },
        Limits {
            max_source_bytes: 2048,
            ..Limits::default()
        },
    ] {
        assert!(import(&s, &t.path().join("dst"), &m, &limits, || false).is_err());
    }
}
#[test]
fn source_and_destination_overlap_is_rejected() {
    let t = tempdir().unwrap();
    let s = t.path().join("src");
    let m = fixture(&s);
    assert!(run(&s, &s.join("nested"), &m).is_err());
}

#[test]
fn rejects_oversized_active_pointer_before_parsing() {
    let t = tempdir().unwrap();
    fs::write(t.path().join("active.json"), vec![b' '; 4097]).unwrap();
    assert!(
        active_install(t.path())
            .unwrap_err()
            .to_string()
            .contains("oversized")
    );
}

#[test]
fn provenance_separates_source_revision_from_import_recipe() {
    let t = tempdir().unwrap();
    let src = t.path().join("src");
    let mut manifest = fixture(&src);
    let dst = t.path().join("dst");
    let first = run(&src, &dst, &manifest).unwrap();
    let read_receipt = |root: &Path| -> rr_import::Receipt {
        serde_json::from_slice(&fs::read(root.parent().unwrap().join("receipt.json")).unwrap())
            .unwrap()
    };
    let before = read_receipt(&first.data_root);
    fs::write(src.join("DATA/TEST.BIN"), [4, 3, 2, 1]).unwrap();
    manifest.files[1] = req("DATA/TEST.BIN", &[4, 3, 2, 1]);
    let second = run(&src, &dst, &manifest).unwrap();
    let after = read_receipt(&second.data_root);
    assert_ne!(before.manifest_sha256, after.manifest_sha256);
    assert_ne!(before.files[1].sha256, after.files[1].sha256);
    assert_eq!(before.recipe_version, after.recipe_version);
    assert_eq!(before.importer_build_sha256, after.importer_build_sha256);
    assert_eq!(after.importer_build_sha256.len(), 64);
    assert_eq!(
        fs::read(first.data_root.join("DATA/TEST.BIN")).unwrap(),
        [1, 2, 3, 4]
    );
}
