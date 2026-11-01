use rr_content::*;
use rr_formats::pixels::Rgba;
use rr_import::{Manifest, RequiredFile};
use std::{fs, path::Path};
use tempfile::tempdir;
fn required(root: &Path, path: &str) -> RequiredFile {
    let bytes = fs::read(root.join(path)).unwrap();
    RequiredFile {
        path: path.into(),
        sha256: hash(&bytes),
        size: bytes.len() as u64,
    }
}
fn fixture(root: &Path) -> Manifest {
    fs::create_dir_all(root.join("IMAGES")).unwrap();
    fs::create_dir_all(root.join("DATA/BIKERS")).unwrap();
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .encode(
            &[200, 50, 20, 200, 50, 20],
            2,
            1,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    fs::write(root.join("IMAGES/A.RRI"), &jpeg).unwrap();
    fs::write(root.join("IMAGES/B.RRI"), &jpeg).unwrap();
    let palette: Vec<_> = (0..256u32).flat_map(|i| [i as u8, 0, 0, 0]).collect();
    fs::write(root.join("DATA/PALETTE.RAW"), palette).unwrap();
    let mut bob = vec![2, 2, 0, 1, 1, 0];
    for _ in 1..326 {
        bob.extend([0, 0]);
    }
    fs::write(root.join("DATA/BIKERS/LOBOB.DAT"), bob).unwrap();
    fs::write(root.join("DATA/BIKERS/TEMPLATE.MIP"), [1, 2, 3, 4]).unwrap();
    Manifest {
        schema_version: 1,
        upstream_revision: "authored-fixture".into(),
        files: [
            "IMAGES/A.RRI",
            "IMAGES/B.RRI",
            "DATA/PALETTE.RAW",
            "DATA/BIKERS/LOBOB.DAT",
            "DATA/BIKERS/TEMPLATE.MIP",
        ]
        .into_iter()
        .map(|p| required(root, p))
        .collect(),
    }
}
#[test]
fn reproducible_catalog_is_order_independent_and_cache_reuses_sources() {
    let t = tempdir().unwrap();
    let data = t.path().join("data");
    let m = fixture(&data);
    let a = t.path().join("a");
    let b = t.path().join("b");
    let (first, report) = build::build(&data, &m, &a, Recipe::default()).unwrap();
    assert_eq!(report.decoded_sources, 3);
    assert!(report.uncategorized_ids.is_empty());
    assert!(
        first
            .entries
            .iter()
            .filter(|e| e.kind == Kind::Ui)
            .all(|e| e.subcategory == Subcategory::MenuArt)
    );
    let bytes = fs::read(a.join("catalog.json")).unwrap();
    let mut reversed = m.clone();
    reversed.files.reverse();
    build::build(&data, &reversed, &b, Recipe::default()).unwrap();
    assert_eq!(bytes, fs::read(b.join("catalog.json")).unwrap());
    let (_, report) = build::build(&data, &m, &a, Recipe::default()).unwrap();
    assert_eq!(report.reused_sources, 5);
    assert_eq!(bytes, fs::read(a.join("catalog.json")).unwrap());
    assert_eq!(
        first.entries.iter().filter(|e| e.frame.is_some()).count(),
        326
    );
}
#[test]
fn identical_pixels_keep_distinct_ids_and_overrides_are_semantic() {
    let t = tempdir().unwrap();
    let data = t.path().join("data");
    let m = fixture(&data);
    let out = t.path().join("out");
    let (c, _) = build::build(&data, &m, &out, Recipe::default()).unwrap();
    let a = c
        .entries
        .iter()
        .find(|e| e.source == "IMAGES/A.RRI")
        .unwrap();
    let b = c
        .entries
        .iter()
        .find(|e| e.source == "IMAGES/B.RRI")
        .unwrap();
    assert_ne!(a.id, b.id);
    assert_eq!(a.preview_variants, b.preview_variants);
    let image = Rgba {
        width: 1,
        height: 1,
        pixels: vec![0, 255, 0, 255],
    };
    let replacement = put_blob(&out, &image.png().unwrap(), "png").unwrap();
    let mut pack = Pack {
        name: "test-override".into(),
        ..Default::default()
    };
    pack.overrides.insert(a.id.clone(), replacement.clone());
    let resolved = c.resolve(&[pack]).unwrap();
    assert_eq!(resolved[&a.id].preview_variants, [replacement]);
    assert_eq!(resolved[&b.id].preview_variants, b.preview_variants);
    let old_id = a.id.clone();
    let mut renamed = a.clone();
    renamed.source = "packed/renamed-source.dat".into();
    assert_eq!(renamed.id, old_id);
}
#[test]
fn dependency_change_only_rebuilds_dependents_and_recipe_changes_invalidate_cache() {
    let t = tempdir().unwrap();
    let data = t.path().join("data");
    let mut m = fixture(&data);
    let out = t.path().join("out");
    build::build(&data, &m, &out, Recipe::default()).unwrap();
    let mut palette = fs::read(data.join("DATA/PALETTE.RAW")).unwrap();
    palette[4] = 200;
    fs::write(data.join("DATA/PALETTE.RAW"), palette).unwrap();
    *m.files
        .iter_mut()
        .find(|f| f.path == "DATA/PALETTE.RAW")
        .unwrap() = required(&data, "DATA/PALETTE.RAW");
    let (_, report) = build::build(&data, &m, &out, Recipe::default()).unwrap();
    assert_eq!(report.reused_sources, 2);
    let recipe = Recipe {
        sprite_alpha_zero: false,
        ..Default::default()
    };
    let (_, report) = build::build(&data, &m, &out, recipe).unwrap();
    assert_eq!(report.reused_sources, 0);
}
#[test]
fn missing_or_mistyped_references_name_pack_and_referrer() {
    let t = tempdir().unwrap();
    let data = t.path().join("data");
    let m = fixture(&data);
    let (c, _) = build::build(&data, &m, &t.path().join("out"), Recipe::default()).unwrap();
    let source = c
        .entries
        .iter()
        .find(|e| e.kind == Kind::Rider)
        .unwrap()
        .id
        .clone();
    let mut pack = Pack {
        name: "example-pack".into(),
        ..Default::default()
    };
    pack.references.push(Reference {
        from: source.clone(),
        to: AssetId::new("rr:missing").unwrap(),
        expected_kind: Kind::Ui,
    });
    let error = c.resolve(&[pack]).unwrap_err().to_string();
    assert!(
        error.contains("example-pack") && error.contains(&source.0) && error.contains("rr:missing")
    );
    let ui = c
        .entries
        .iter()
        .find(|e| e.kind == Kind::Ui)
        .unwrap()
        .id
        .clone();
    let mut pack = Pack {
        name: "bad-kind".into(),
        ..Default::default()
    };
    pack.references.push(Reference {
        from: source,
        to: ui,
        expected_kind: Kind::BikeSpec,
    });
    assert!(
        c.resolve(&[pack])
            .unwrap_err()
            .to_string()
            .contains("BikeSpec")
    );
}
#[test]
fn schema_and_unsafe_blob_paths_are_rejected() {
    let t = tempdir().unwrap();
    let p = t.path().join("catalog.json");
    fs::write(&p, b"{\"schema_version\":0}").unwrap();
    assert!(
        Catalog::read(&p)
            .unwrap_err()
            .to_string()
            .contains("schema")
    );
    for name in ["../escape.png", "/absolute.png", "not-a-hash.png"] {
        assert!(validate_blob(name).is_err());
    }
    assert!(AssetId::new("rr:../escape").is_err());
}
#[test]
fn trimmed_and_untrimmed_pixels_align_to_the_same_anchor() {
    let t = tempdir().unwrap();
    let data = t.path().join("data");
    let m = fixture(&data);
    let (c, _) = build::build(&data, &m, &t.path().join("out"), Recipe::default()).unwrap();
    let mut entry = c.entries[0].clone();
    let mut rgba = Rgba {
        width: 4,
        height: 4,
        pixels: vec![0; 4 * 4 * 4],
    };
    for y in 1..3 {
        for x in 1..3 {
            let p = (y * 4 + x) * 4;
            rgba.pixels[p..p + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
    }
    entry.native_size = [4, 4];
    entry.logical_size = [4, 4];
    entry.trim = trim(&rgba);
    entry.pivot_twice = [4, 8];
    assert_eq!(
        entry.trim,
        Rect {
            x: 1,
            y: 1,
            width: 2,
            height: 2
        }
    );
    let offset = trim_placement(&entry);
    assert_eq!(offset, [-2, -6]);
    let mut full = vec![0u8; 20 * 20];
    let mut trimmed = full.clone();
    let anchor = [10, 10];
    for y in 0..4 {
        for x in 0..4 {
            if rgba.pixels[(y * 4 + x) * 4 + 3] > 0 {
                full[(anchor[1] - 4 + y) * 20 + anchor[0] - 2 + x] = 255;
            }
        }
    }
    for y in 0..2 {
        for x in 0..2 {
            trimmed[(anchor[1] as i32 + offset[1] / 2 + y) as usize * 20
                + (anchor[0] as i32 + offset[0] / 2 + x) as usize] = 255;
        }
    }
    assert_eq!(full, trimmed);
}
#[test]
fn preview_corruption_is_rebuilt_from_immutable_source() {
    let t = tempdir().unwrap();
    let data = t.path().join("data");
    let m = fixture(&data);
    let out = t.path().join("out");
    let (c, _) = build::build(&data, &m, &out, Recipe::default()).unwrap();
    let entry = c
        .entries
        .iter()
        .find(|e| !e.preview_variants.is_empty())
        .unwrap();
    let name = &entry.preview_variants[0];
    fs::write(out.join("objects").join(name), b"corrupt").unwrap();
    let (_, r) = build::build(&data, &m, &out, Recipe::default()).unwrap();
    assert!(r.reused_sources < 5);
    assert!(blob_valid(&out, name));
}
