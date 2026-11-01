use crate::*;
use rr_formats::{
    decode_bob,
    family::Family,
    pe::SupportedExe,
    pixels::{Cel, Palette, Rgba},
    resource::Directory,
};
use rr_import::Manifest;
use serde::Serialize;
use std::{fs, path::Path};
#[derive(Default, Serialize)]
pub struct BuildReport {
    pub total_sources: usize,
    pub decoded_sources: usize,
    pub reused_sources: usize,
    pub entries: usize,
    pub errors: Vec<String>,
    pub unsupported: Vec<String>,
    pub categories: BTreeMap<String, usize>,
    pub subcategories: BTreeMap<String, usize>,
    pub uncategorized_ids: Vec<AssetId>,
}
pub fn classify(path: &str) -> Kind {
    if path.starts_with("AUDIO/") {
        Kind::Audio
    } else if path.ends_with(".AVI") {
        Kind::Video
    } else if path.starts_with("IMAGES/HORIZONS/") || path.contains("CLOUD.BOB") {
        Kind::Background
    } else if path.ends_with(".BOB") || path.ends_with(".RRI") {
        Kind::Ui
    } else if path.ends_with(".MIP") {
        Kind::Resource
    } else if path.contains("BIKERS/") {
        if path.contains("SHAD") {
            Kind::Shadow
        } else if path.contains("COP") {
            Kind::Police
        } else {
            Kind::Rider
        }
    } else if path.ends_with("BIKESPEC.RSC") {
        Kind::BikeSpec
    } else if path.ends_with(".CRS") {
        Kind::Course
    } else if path.ends_with(".RSC") || path.ends_with(".CAR") {
        Kind::Resource
    } else if path.ends_with(".EXE") {
        Kind::Executable
    } else if path.ends_with(".RAW") {
        Kind::Palette
    } else if path.starts_with("TEXT/") {
        Kind::Text
    } else {
        Kind::Other
    }
}
fn subcategory(entry: &Entry) -> (Subcategory, ClassificationConfidence) {
    use ClassificationConfidence::{Inferred, SourceDerived, Unreviewed};
    use Subcategory::*;
    let category = match entry.kind {
        Kind::Rider | Kind::Police => Actor,
        Kind::Shadow => Shadow,
        Kind::Audio => {
            if entry.source.contains("/EFFECTS/") {
                AudioEffect
            } else {
                Music
            }
        }
        Kind::Video => Video,
        Kind::Background => {
            if entry.source.contains("CLOUD") {
                Cloud
            } else {
                Horizon
            }
        }
        Kind::Ui => {
            if entry.source.contains("/CHARS/") {
                CharacterArt
            } else if entry.source.ends_with(".BOB") {
                Hud
            } else if entry.source.contains("/BIKES/") || entry.source.contains("/SHOWROOM/") {
                BikeArt
            } else {
                MenuArt
            }
        }
        Kind::SceneryTexture | Kind::CourseTexture => {
            let name = entry.metadata["texture_name"]
                .as_str()
                .unwrap_or("")
                .to_uppercase();
            let rules: [(&[&str], Subcategory); 4] = [
                (&["RR", "SCT", "SECT", "ROAD", "PLAIN"], Road),
                (
                    &[
                        "HILL", "CLIFF", "GRAS", "TERR", "ROCK", "GROUND", "SAND", "DIRT",
                    ],
                    Terrain,
                ),
                (
                    &[
                        "WRHS", "WHSE", "OFFC", "VICT", "BUSS", "CLST", "CHINA", "RUNDWN", "SHOP",
                        "HOUSE", "BLDG",
                    ],
                    Building,
                ),
                (&["TREE", "BUSH", "SIGN", "RAIL", "POLE", "FENCE"], Prop),
            ];
            for (prefixes, kind) in rules {
                if prefixes.iter().any(|prefix| name.starts_with(prefix)) {
                    return (kind, Inferred);
                }
            }
            return (Unclassified, Unreviewed);
        }
        Kind::Other => return (Unclassified, Unreviewed),
        _ => SourceMetadata,
    };
    (category, SourceDerived)
}

fn id(path: &str, suffix: &str) -> AssetId {
    AssetId(format!(
        "rr:{}{}",
        path.to_ascii_lowercase().replace('.', "_"),
        suffix
    ))
}
fn base(path: &str, source_hash: &str, kind: Kind, suffix: &str) -> Entry {
    Entry {
        id: id(path, suffix),
        kind,
        subcategory: Subcategory::Unclassified,
        classification_confidence: ClassificationConfidence::Unreviewed,
        source: path.into(),
        source_sha256: source_hash.into(),
        resource: None,
        frame: None,
        locator: None,
        native_size: [0, 0],
        trim: Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
        logical_size: [0, 0],
        pivot_twice: [0, 0],
        pivot_policy: "none".into(),
        indexed_blob: None,
        preview_variants: vec![],
        dependencies: vec![path.into()],
        metadata: serde_json::Value::Null,
    }
}
fn image(entry: &mut Entry, rgba: &Rgba, out: &Path) -> Result<()> {
    if entry.native_size == [0, 0] {
        entry.native_size = [rgba.width, rgba.height];
        entry.logical_size = entry.native_size;
        entry.trim = trim(rgba);
        entry.pivot_twice = [rgba.width as i32, rgba.height as i32 * 2];
        entry.pivot_policy =
            "authored bottom-center; per-animation hotspots may override later".into();
    }
    let png = rgba.png()?;
    // Check generated bytes independently through the lossless decoder before accepting a preview.
    if Rgba::from_png(&png)? != *rgba {
        return Err("PNG round-trip mismatch".into());
    }
    entry.preview_variants.push(put_blob(out, &png, "png")?);
    Ok(())
}
fn opaque(path: &str, hash: &str, kind: Kind, resource: Option<(String, u32)>) -> Entry {
    let suffix = resource.as_ref().map_or(String::new(), |(tag, n)| {
        format!("/resource/{}/{n}", tag.trim().to_lowercase())
    });
    let mut e = base(path, hash, kind, &suffix);
    e.resource = resource;
    e
}

pub fn build(
    data: &Path,
    manifest: &Manifest,
    out: &Path,
    recipe: Recipe,
) -> Result<(Catalog, BuildReport)> {
    if recipe.schema_version != 1
        || recipe.decoder != DECODER_VERSION
        || recipe.converter_version != CONVERTER_VERSION
        || recipe.rgba_policy != "standard-jpeg-native-rgbx"
    {
        return Err("unsupported build recipe/decoder".into());
    }
    let absolute_data = data.canonicalize()?;
    let absolute_out = std::path::absolute(out)?;
    if absolute_out.starts_with(&absolute_data) {
        return Err("catalog output cannot be inside source".into());
    }
    fs::create_dir_all(out)?;
    let canonical_out = out.canonicalize()?;
    if canonical_out.starts_with(&absolute_data) || absolute_data.starts_with(&canonical_out) {
        return Err("catalog output and source must be disjoint".into());
    }
    for file in &manifest.files {
        if file.path.contains('\\')
            || file.path.contains(':')
            || Path::new(&file.path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("unsafe catalog source path".into());
        }
    }
    if out.starts_with(data) || data.starts_with(out) {
        return Err("catalog output and source must be disjoint".into());
    }
    fs::create_dir_all(out.join("objects"))?;
    let previous = if out.join("catalog.json").exists() {
        Some(Catalog::read(&out.join("catalog.json"))?)
    } else {
        None
    };
    let recipe_hash = hash(&serde_json::to_vec(&recipe)?);
    let palette_bytes = if data.join("DATA/PALETTE.RAW").exists() {
        fs::read(data.join("DATA/PALETTE.RAW"))?
    } else {
        vec![0; 1024]
    };
    let palette = Palette::read(&palette_bytes)?;
    let palette_hash = hash(&palette_bytes);
    let exe_bytes = if data.join("ROADRASH.EXE").exists() {
        Some(fs::read(data.join("ROADRASH.EXE"))?)
    } else {
        None
    };
    let remaps = if let Some(bytes) = &exe_bytes {
        SupportedExe::read(bytes)?.rider_remaps()?
    } else {
        vec![std::array::from_fn(|i| i as u8)]
    };
    let remap_hash = hash(&remaps.iter().flatten().copied().collect::<Vec<_>>());
    let mut catalog = Catalog {
        schema_version: SCHEMA,
        recipe,
        sources: vec![],
        entries: vec![],
        palettes: vec![put_blob(out, &palette_bytes, "bin")?],
        metadata_blob: String::new(),
    };
    for remap in &remaps {
        catalog.palettes.push(put_blob(out, remap, "bin")?);
    }
    let mut report = BuildReport {
        total_sources: manifest.files.len(),
        ..Default::default()
    };
    let mut files = manifest.files.clone();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut inspections: BTreeMap<String, serde_json::Value> = previous
        .as_ref()
        .filter(|c| blob_valid(out, &c.metadata_blob))
        .and_then(|c| fs::read(out.join("objects").join(&c.metadata_blob)).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    for required in &files {
        let path = &required.path;
        let bytes = fs::read(data.join(path)).map_err(|e| format!("{path}: {e}"))?;
        let source_hash = hash(&bytes);
        if source_hash != required.sha256 || bytes.len() as u64 != required.size {
            return Err(format!("source hash/size mismatch before decode: {path}").into());
        }
        let kind = classify(path);
        let indexed = path.ends_with(".BOB")
            || (path.starts_with("DATA/")
                && matches!(
                    kind,
                    Kind::Rider | Kind::Police | Kind::Shadow | Kind::Resource | Kind::Course
                ));
        let cache_key = hash(
            format!(
                "{source_hash}:{recipe_hash}:{}:{}",
                if indexed { &palette_hash } else { "" },
                if indexed { &remap_hash } else { "" }
            )
            .as_bytes(),
        );
        let master_blob = put_blob(out, &bytes, "bin")?;
        let mut source = Source {
            path: path.clone(),
            logical_id: id(path, "").0,
            sha256: source_hash.clone(),
            size: required.size,
            kind: kind.clone(),
            status: "retained opaque for later subsystem".into(),
            cache_key: cache_key.clone(),
            master_blob,
        };
        if let Some(old) = &previous
            && let Some(old_source) = old.sources.iter().find(|s| {
                s.path == *path
                    && s.cache_key == cache_key
                    && (!(path.ends_with(".RSC")
                        || path.ends_with(".CRS")
                        || path.ends_with(".CAR"))
                        || inspections.contains_key(path))
            })
        {
            let cached: Vec<_> = old
                .entries
                .iter()
                .filter(|e| e.source == *path)
                .cloned()
                .collect();
            if cached
                .iter()
                .flat_map(|e| e.preview_variants.iter().chain(e.indexed_blob.iter()))
                .all(|name| blob_valid(out, name))
            {
                source.status = old_source.status.clone();
                catalog.sources.push(source);
                catalog.entries.extend(cached);
                report.reused_sources += 1;
                continue;
            }
        }
        let mut entries = vec![];
        let decoded = (|| -> Result<()> {
            if path.ends_with(".RRI") {
                let rgba = rr_formats::pixels::rri(&bytes)?;
                let mut entry = base(path, &source_hash, Kind::Ui, "");
                image(&mut entry, &rgba, out)?;
                entries.push(entry);
                source.status = "decoded RRI/JPEG".into();
            } else if path.ends_with(".BOB") && !path.contains("CLOUD") {
                let width = if path.starts_with("IMAGES/HORIZONS/") {
                    if path.rsplit('/').next().unwrap().starts_with('L') {
                        360
                    } else {
                        720
                    }
                } else if path.ends_with("BIGMETER.BOB") {
                    190
                } else if path.ends_with("LILMETER.BOB") {
                    95
                } else if path.contains("LODASH") {
                    320
                } else {
                    640
                };
                if bytes.is_empty() || bytes.len() % width != 0 {
                    return Err(format!("raw BOB stride mismatch: {path}").into());
                }
                let height = bytes.len() / width;
                let mut entry = base(path, &source_hash, kind.clone(), "");
                entry.indexed_blob = Some(put_blob(out, &bytes, "bin")?);
                entry.dependencies.push("DATA/PALETTE.RAW".into());
                entry.metadata = serde_json::json!({"format":"headerless indexed BOB","alpha_policy":"opaque preview; semantic mask composition belongs to presentation","stride_basis":"pinned hud.cpp and course_scene.cpp layout constants"});
                if let Some(rgba) =
                    palette.rgba(width as u32, height as u32, &bytes, false, None)?
                {
                    image(&mut entry, &rgba, out)?;
                }
                entries.push(entry);
                source.status = "decoded headerless indexed BOB".into();
            } else if path.starts_with("DATA/BIKERS/") && path.ends_with(".DAT") {
                let count = if path.contains("COP") || path.contains("SHADOC") {
                    45
                } else {
                    326
                };
                for (index, frame) in decode_bob(&bytes, count)?.into_iter().enumerate() {
                    let mut entry = base(
                        path,
                        &source_hash,
                        kind.clone(),
                        &format!("/frame/{index:03}"),
                    );
                    entry.frame = Some(index as u32);
                    entry.native_size = [u32::from(frame.width), u32::from(frame.height)];
                    entry.logical_size = entry.native_size;
                    entry.pivot_twice = [i32::from(frame.width), i32::from(frame.height) * 2];
                    entry.pivot_policy = "authored bottom-center; empty slots retained".into();
                    entry.indexed_blob = Some(put_blob(out, frame.pixels, "bin")?);
                    entry.dependencies.push("DATA/PALETTE.RAW".into());
                    if kind == Kind::Rider {
                        entry
                            .dependencies
                            .push("ROADRASH.EXE:0x46e158/remaps".into());
                        for remap in &remaps {
                            if let Some(rgba) = palette.rgba(
                                u32::from(frame.width),
                                u32::from(frame.height),
                                frame.pixels,
                                catalog.recipe.sprite_alpha_zero,
                                Some(remap),
                            )? {
                                image(&mut entry, &rgba, out)?;
                            }
                        }
                    } else if let Some(rgba) = palette.rgba(
                        u32::from(frame.width),
                        u32::from(frame.height),
                        frame.pixels,
                        true,
                        None,
                    )? {
                        image(&mut entry, &rgba, out)?;
                    }
                    // Dimensions were assigned before variants, so compute trim from variant zero explicitly.
                    if let Some(name) = entry.preview_variants.first() {
                        let rgba = Rgba::from_png(&fs::read(out.join("objects").join(name))?)?;
                        entry.trim = trim(&rgba);
                    }
                    entries.push(entry);
                }
                source.status = format!("decoded {count} frames, including empty slots");
            } else if path.ends_with(".RSC") || path.ends_with(".CRS") || path.ends_with(".CAR") {
                let directory = Directory::read(&bytes)?;
                for resource in &directory.entries {
                    let payload = resource.payload(&bytes)?;
                    let identity = Some((resource.tag.clone(), resource.id));
                    if resource.tag == "CEL " {
                        let cel = Cel::read(payload)?;
                        let mut entry = opaque(
                            path,
                            &source_hash,
                            if path.ends_with(".CRS") {
                                Kind::CourseTexture
                            } else {
                                Kind::SceneryTexture
                            },
                            identity,
                        );
                        entry.metadata = serde_json::to_value(&cel)?;
                        entry.dependencies.push("DATA/PALETTE.RAW".into());
                        if let Some(texture) = &cel.texture {
                            let mip = &texture.mips[0];
                            entry.indexed_blob = Some(put_blob(out, &mip.indices, "bin")?);
                            if let Some(rgba) =
                                palette.rgba(mip.width, mip.height, &mip.indices, false, None)?
                            {
                                image(&mut entry, &rgba, out)?;
                            }
                        }
                        entries.push(entry);
                    } else if resource.tag == "FAM " {
                        let family = Family::read(payload)?;
                        let mut raw = opaque(path, &source_hash, Kind::Resource, identity.clone());
                        raw.metadata = serde_json::json!({"nodes":family.nodes,"unpacked_length":family.unpacked_length});
                        entries.push(raw);
                        for texture in family.textures {
                            let mut entry = base(
                                path,
                                &source_hash,
                                Kind::SceneryTexture,
                                &format!(
                                    "/family/{}/group/{}/cell/{}-{}",
                                    resource.id, texture.group_offset, texture.column, texture.row
                                ),
                            );
                            entry.resource = identity.clone();
                            entry.locator = Some(texture.texture_offset);
                            entry.dependencies.push("DATA/PALETTE.RAW".into());
                            let mip = &texture.texture.mips[0];
                            entry.indexed_blob = Some(put_blob(out, &mip.indices, "bin")?);
                            if let Some(rgba) =
                                palette.rgba(mip.width, mip.height, &mip.indices, false, None)?
                            {
                                image(&mut entry, &rgba, out)?;
                            }
                            entry.metadata = serde_json::json!({"group":texture.group_offset,"texture_name":texture.texture.name,"logical_group_size":[texture.logical_width,texture.logical_height],"bounds":texture.texture.bounds,"flags":texture.texture.flags});
                            entries.push(entry);
                        }
                    } else {
                        let mut entry = opaque(path, &source_hash, kind.clone(), identity);
                        entry.metadata = serde_json::json!({"raw_words":resource.raw,"interpretation":"retained for later course/actor subsystem"});
                        entries.push(entry);
                    }
                }
                inspections.insert(path.clone(), serde_json::to_value(directory)?);
                source.status =
                    "decoded container; typed image payloads; remaining payloads retained".into();
            } else {
                entries.push(base(path, &source_hash, kind.clone(), ""));
            }
            Ok(())
        })();
        match decoded {
            Ok(()) => {
                if source.status.starts_with("decoded") {
                    report.decoded_sources += 1;
                } else {
                    report
                        .unsupported
                        .push(format!("{path}: {}", source.status));
                }
            }
            Err(e) => {
                report.errors.push(format!("{path}: {e}"));
                source.status = format!("decode failed: {e}");
            }
        }
        for entry in &mut entries {
            (entry.subcategory, entry.classification_confidence) = subcategory(entry);
            if !entry.metadata.is_object() {
                entry.metadata = serde_json::json!({});
            }
            entry.metadata["source_group"] =
                serde_json::json!(path.rsplit_once('/').map_or("root", |(p, _)| p));
            entry.metadata["category_policy"] = serde_json::json!(
                "typed source domain; fine scenery semantics assigned during course interpretation"
            );
        }
        catalog.entries.extend(entries);
        catalog.sources.push(source);
    }
    if !report.errors.is_empty() {
        fs::write(
            out.join("build-report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        return Err(format!(
            "{} source decode failures; inspect build-report.json",
            report.errors.len()
        )
        .into());
    }
    catalog.sources.sort_by(|a, b| a.path.cmp(&b.path));
    catalog.entries.sort_by(|a, b| a.id.cmp(&b.id));
    catalog.metadata_blob = if inspections.is_empty() {
        previous
            .as_ref()
            .map(|c| c.metadata_blob.clone())
            .unwrap_or_default()
    } else {
        put_blob(out, &serde_json::to_vec_pretty(&inspections)?, "json")?
    };
    catalog.validate()?;
    for entry in &catalog.entries {
        *report
            .subcategories
            .entry(format!("{:?}", entry.subcategory))
            .or_default() += 1;
        if entry.subcategory == Subcategory::Unclassified {
            report.uncategorized_ids.push(entry.id.clone());
        }
    }
    report.entries = catalog.entries.len();
    report.unsupported = catalog
        .sources
        .iter()
        .filter(|s| s.status.starts_with("retained"))
        .map(|s| format!("{}: {}", s.path, s.status))
        .collect();
    for source in &catalog.sources {
        *report
            .categories
            .entry(format!("{:?}", source.kind))
            .or_default() += 1;
    }
    fs::write(
        out.join("catalog.json"),
        serde_json::to_vec_pretty(&catalog)?,
    )?;
    fs::write(
        out.join("build-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    let json = serde_json::to_string(&catalog)?.replace('<', "\\u003c");
    fs::write(
        out.join("inspector.html"),
        include_str!("inspector.html").replace("__CATALOG__", &json),
    )?;
    Ok((catalog, report))
}
