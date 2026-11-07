use rr_session::{
    career::Career,
    catalog::Catalog,
    flow::Settings,
    persistence::{
        disk::{self, Fault},
        paths::UserPaths,
        preferences::Preferences,
        *,
    },
};
use std::path::PathBuf;
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "rr-persistence-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}
#[test]
fn version_two_roundtrips_stable_ids_and_exact_settlement_ledger() {
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 3).unwrap();
    c.alias = "Test rider".into();
    c.allocate_race().unwrap();
    let active = Requirements::base(&cat);
    let b = encode(&c, &cat, &active).unwrap();
    let loaded = decode(&b, &cat, &active).unwrap();
    assert_eq!(loaded.career, c);
    assert!(loaded.migrations.is_empty());
    let text = String::from_utf8(b).unwrap();
    assert!(!text.contains("Entity"));
    assert!(!text.contains("actor"));
    assert_eq!(
        encode(&loaded.career, &cat, &loaded.requirements).unwrap(),
        text.as_bytes()
    );
}
#[test]
fn legacy_bridge_migrates_once_preserves_bytes_and_ids() {
    let cat = Catalog::authored();
    let c = Career::new(&cat, 2).unwrap();
    let old = c.encode().unwrap();
    let active = Requirements::base(&cat);
    let fixture = include_bytes!("../../../docs/fixtures/career-v1.json");
    let fixture_loaded = decode(fixture, &cat, &active).unwrap();
    assert_eq!(fixture_loaded.career.alias, "Authored rider");
    assert_eq!(fixture_loaded.migrations.len(), 1);
    let value = decode(&old, &cat, &active).unwrap();
    assert_eq!(value.migrations.len(), 1);
    assert_eq!(value.career, c);
    let new = encode(&value.career, &cat, &value.requirements).unwrap();
    assert!(decode(&new, &cat, &active).unwrap().migrations.is_empty());
    assert_eq!(old, c.encode().unwrap());
}
#[test]
fn every_truncated_prefix_and_integrity_mutation_fails_without_live_mutation() {
    let cat = Catalog::authored();
    let live = Career::new(&cat, 1).unwrap();
    let frozen = live.clone();
    let active = Requirements::base(&cat);
    let bytes = encode(&live, &cat, &active).unwrap();
    for n in 0..bytes.len() {
        assert!(decode(&bytes[..n], &cat, &active).is_err());
    }
    let mut raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    raw["body"]["career"]["cash"] = 99999.into();
    assert!(
        decode(&serde_json::to_vec(&raw).unwrap(), &cat, &active)
            .unwrap_err()
            .contains("integrity")
    );
    assert_eq!(live, frozen);
}
#[test]
fn version_money_ids_bounds_and_binary_original_are_rejected() {
    let cat = Catalog::authored();
    let c = Career::new(&cat, 1).unwrap();
    let active = Requirements::base(&cat);
    let b = encode(&c, &cat, &active).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&b).unwrap();
    v["schema_version"] = 99.into();
    assert!(decode(&serde_json::to_vec(&v).unwrap(), &cat, &active).is_err());
    for field in ["character", "level", "bike", "qualified"] {
        let mut v: serde_json::Value = serde_json::from_slice(&c.encode().unwrap()).unwrap();
        v["career"][field] = 255.into();
        assert!(decode(&serde_json::to_vec(&v).unwrap(), &cat, &active).is_err());
    }
    let mut binary = vec![0; 160];
    binary[4..8].copy_from_slice(&[0xad, 0xde, 0xfe, 0xca]);
    assert!(
        decode(&binary, &cat, &active)
            .unwrap_err()
            .contains("Original binary")
    );
    assert!(decode(&vec![0; MAX_BYTES + 1], &cat, &active).is_err());
}
#[test]
fn required_simulation_content_blocks_load_optional_art_falls_back() {
    let cat = Catalog::authored();
    let c = Career::new(&cat, 1).unwrap();
    let active = Requirements::base(&cat);
    let mut modified = active.clone();
    modified.packs.push(ContentPack {
        id: "example:handling".into(),
        version: 1,
        sha256: "a".repeat(64),
        simulation: true,
    });
    let bytes = encode(&c, &cat, &modified).unwrap();
    assert!(
        decode(&bytes, &cat, &active)
            .unwrap_err()
            .contains("example:handling")
    );
    modified.packs[1].simulation = false;
    let bytes = encode(&c, &cat, &modified).unwrap();
    assert_eq!(decode(&bytes, &cat, &active).unwrap().warnings.len(), 1);
    let mut changed = active.clone();
    changed.packs[0].sha256 = "b".repeat(64);
    assert!(decode(&bytes, &cat, &changed).is_err());
}
#[test]
fn backup_contains_previous_valid_generation_after_migration() {
    let dir = root();
    let p = dir.join("career.json");
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 1).unwrap();
    let active = Requirements::base(&cat);
    let old = c.encode().unwrap();
    std::fs::write(&p, &old).unwrap();
    c.cash = 321;
    let new = encode(&c, &cat, &active).unwrap();
    let validate = |b: &[u8]| decode(b, &cat, &active).map(|_| ());
    disk::write(&p, None, &new, validate, Fault::None).unwrap();
    assert_eq!(disk::read(&disk::sidecar(&p, ".bak")).unwrap(), old);
    assert_eq!(
        decode(&disk::read(&p).unwrap(), &cat, &active)
            .unwrap()
            .career,
        c
    );
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn every_injected_write_stage_leaves_old_or_new_valid_and_cleans_owned_temporaries() {
    let dir = root();
    let cat = Catalog::authored();
    let c = Career::new(&cat, 1).unwrap();
    let active = Requirements::base(&cat);
    let old = encode(&c, &cat, &active).unwrap();
    let mut next = c.clone();
    next.cash = 12345;
    let new = encode(&next, &cat, &active).unwrap();
    for f in [
        Fault::AfterCreate,
        Fault::AfterWrite,
        Fault::AfterFileSync,
        Fault::AfterBackup,
        Fault::AfterReplace,
        Fault::AfterDirectorySync,
    ] {
        let p = dir.join(format!("{f:?}.json"));
        std::fs::write(&p, &old).unwrap();
        let validate = |b: &[u8]| decode(b, &cat, &active).map(|_| ());
        assert!(disk::write(&p, None, &new, validate, f).is_err());
        let actual = disk::read(&p).unwrap();
        assert!(actual == old || actual == new);
        decode(&actual, &cat, &active).unwrap();
        assert!(!disk::sidecar(&p, ".lock").exists());
    }
    assert!(
        std::fs::read_dir(&dir).unwrap().all(|p| !p
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp."))
    );
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn explicit_backup_recovery_preserves_bad_primary_and_refuses_silent_overwrite() {
    let dir = root();
    let p = dir.join("career.json");
    let cat = Catalog::authored();
    let c = Career::new(&cat, 1).unwrap();
    let active = Requirements::base(&cat);
    let good = encode(&c, &cat, &active).unwrap();
    std::fs::write(&p, b"broken").unwrap();
    std::fs::write(disk::sidecar(&p, ".bak"), &good).unwrap();
    let validate = |b: &[u8]| decode(b, &cat, &active).map(|_| ());
    assert!(disk::write(&p, None, &good, validate, Fault::None).is_err());
    assert_eq!(disk::read(&p).unwrap(), b"broken");
    let preserved = disk::restore(&p, None, validate).unwrap();
    assert_eq!(disk::read(&preserved).unwrap(), b"broken");
    assert_eq!(disk::read(&p).unwrap(), good);
    assert!(disk::sidecar(&p, ".bak").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn competing_writer_and_source_overlap_do_not_create_or_modify_source() {
    let dir = root();
    let p = dir.join("career.json");
    let cat = Catalog::authored();
    let c = Career::new(&cat, 1).unwrap();
    let active = Requirements::base(&cat);
    let good = encode(&c, &cat, &active).unwrap();
    std::fs::write(disk::sidecar(&p, ".lock"), b"busy").unwrap();
    assert!(disk::write(&p, None, &good, |_| Ok(()), Fault::None).is_err());
    assert!(!p.exists());
    let nested = dir.join("immutable/sub/career.json");
    std::fs::create_dir_all(dir.join("immutable")).unwrap();
    assert!(
        disk::write(
            &nested,
            Some(&dir.join("immutable")),
            &good,
            |_| Ok(()),
            Fault::None
        )
        .is_err()
    );
    assert!(!dir.join("immutable/sub").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn preferences_defaults_legacy_unknown_keys_and_volume_limits() {
    let (empty, w) = Preferences::decode(b"{}").unwrap();
    assert!(w.is_empty());
    assert_eq!(empty.settings, Settings::default());
    let bytes=br#"{"settings":{"music_volume":23},"bindings":{"schema_version":1,"contexts":{"race":{"accelerate":["RemovedOldKey"]}}}}"#;
    let (p, w) = Preferences::decode(bytes).unwrap();
    assert_eq!(p.settings.music_volume, 23);
    assert_eq!(p.settings.master_volume, 100);
    assert_eq!(w.len(), 1);
    p.bindings.validate().unwrap();
    assert!(Preferences::decode(br#"{"settings":{"engine_volume":101}}"#).is_err());
    assert!(Preferences::decode(br#"{"schema_version":99}"#).is_err());
    assert!(Preferences::decode(br#"{"bindings":{"schema_version":99}}"#).is_err());
}
#[test]
fn upscaled_preferences_keep_existing_profiles_compatible() {
    assert!(!Preferences::default().settings.upscaled);
    for key in ["enhanced", "upscaled"] {
        let raw = format!(r#"{{"settings":{{"{key}":true}}}}"#);
        let (mut preferences, warnings) = Preferences::decode(raw.as_bytes()).unwrap();
        assert!(warnings.is_empty());
        assert!(preferences.settings.upscaled);
        let encoded: serde_json::Value =
            serde_json::from_slice(&preferences.encode().unwrap()).unwrap();
        // RC6 and older versions still read the same persisted boolean key.
        assert_eq!(encoded["settings"]["enhanced"], true);
        assert!(encoded["settings"].get("upscaled").is_none());
        preferences.settings.reset_graphics();
        assert!(!preferences.settings.upscaled);
    }
    assert!(Preferences::decode(br#"{"settings":{"enhanced":false,"upscaled":true}}"#).is_err());
    assert!(Preferences::decode(br#"{"settings":{"upscaled":"on"}}"#).is_err());
}
#[test]
fn graphics_reset_and_clean_profile_recovery_leave_career_independent() {
    let dir = root();
    let paths = UserPaths::profile(&dir);
    let cat = Catalog::authored();
    let c = Career::new(&cat, 1).unwrap();
    let active = Requirements::base(&cat);
    let save = encode(&c, &cat, &active).unwrap();
    let p = paths.career(0).unwrap();
    disk::write(
        &p,
        None,
        &save,
        |b| decode(b, &cat, &active).map(|_| ()),
        Fault::None,
    )
    .unwrap();
    let mut prefs = Preferences::default();
    prefs.settings.wide = true;
    prefs.settings.upscaled = true;
    prefs.settings.music_volume = 37;
    let cfg = paths.settings();
    disk::write(
        &cfg,
        None,
        &prefs.encode().unwrap(),
        |b| Preferences::decode(b).map(|_| ()),
        Fault::None,
    )
    .unwrap();
    prefs.settings.reset_graphics();
    disk::write(
        &cfg,
        None,
        &prefs.encode().unwrap(),
        |b| Preferences::decode(b).map(|_| ()),
        Fault::None,
    )
    .unwrap();
    assert_eq!(disk::read(&p).unwrap(), save);
    assert_eq!(
        Preferences::decode(&disk::read(&cfg).unwrap())
            .unwrap()
            .0
            .settings
            .music_volume,
        37
    );
    assert!(paths.career(5).is_err());
    std::fs::remove_file(cfg).unwrap();
    assert_eq!(
        decode(&disk::read(&p).unwrap(), &cat, &active)
            .unwrap()
            .career,
        c
    );
    std::fs::remove_dir_all(dir).unwrap();
}
