//! Private fixed-camera checkpoints, actual pixel diffs and metadata receipts.
use crate::{camera::*, scene::*, source::*, *};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn output_path(install: &Path, out: &Path) -> Result<PathBuf> {
    let input = install.canonicalize()?;
    if out
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("render output parent traversal".into());
    }
    let absolute = std::path::absolute(out)?;
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("output ancestor")?
    }
    let resolved = ancestor
        .canonicalize()?
        .join(absolute.strip_prefix(ancestor)?);
    if resolved.starts_with(&input) || input.starts_with(&resolved) {
        return Err("render output overlaps immutable install".into());
    }
    Ok(resolved)
}
pub fn image_diff(
    expected: &rr_formats::pixels::Rgba,
    actual: &rr_formats::pixels::Rgba,
) -> Result<(usize, rr_formats::pixels::Rgba)> {
    expected.validate()?;
    actual.validate()?;
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Err("diff image dimensions differ".into());
    }
    let mut diff = expected.clone();
    let mut count = 0;
    for ((a, b), out) in expected
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(actual.pixels.as_chunks::<4>().0.iter())
        .zip(diff.pixels.as_chunks_mut::<4>().0)
    {
        if a != b {
            count += 1
        }
        *out = [
            a[0].abs_diff(b[0]),
            a[1].abs_diff(b[1]),
            a[2].abs_diff(b[2]),
            255,
        ];
        if a[3] != b[3] && a[..3] == b[..3] {
            *out = [255; 4];
        }
    }
    Ok((count, diff))
}
pub fn run(install: &Path, catalog: &Path, out: &Path) -> Result<()> {
    let out = output_path(install, out)?;
    let active = rr_import::active_install(install)?;
    fs::create_dir_all(&out)?;
    let mut reports = vec![];
    let mut source_notes = BTreeMap::<String, BTreeSet<String>>::new();
    for (stem, _) in rr_course::export::COURSES {
        let course = rr_course::Course::read(&fs::read(
            active.data_root.join(format!("DATA/COURSES/{stem}.CRS")),
        )?)?;
        let assets = SourceAssets::load(&active.data_root, catalog, stem, &course)?;
        for mask in 0..1u32 << course.splits().len() {
            let base = Scene::new(
                course.clone(),
                assets.clone(),
                SceneOptions {
                    course: stem.into(),
                    branch_mask: mask,
                    ..Default::default()
                },
            )?;
            let mut checkpoints = vec![(50., "start".to_string(), 1u8)];
            for (ti, t) in base.route.transitions.iter().enumerate() {
                for delta in [-1, 0, 1] {
                    checkpoints.push((
                        t.index as f64 + f64::from(delta),
                        format!("seam-{ti}-{delta}"),
                        1,
                    ))
                }
            }
            for level in 1..=5 {
                checkpoints.push((
                    base.route.finish(&course, level)? as f64,
                    format!("finish-{level}"),
                    level,
                ))
            }
            let mut kinds = BTreeSet::new();
            for (i, p) in base.route.points.iter().enumerate() {
                let kind = base.terrain[p.section as usize][p.local as usize].kind;
                let name = format!("{kind:?}");
                if kinds.insert(name.clone()) {
                    checkpoints.push((
                        (i + 8).min(base.route.points.len() - 33) as f64,
                        format!("terrain-{name}"),
                        1,
                    ));
                }
            }
            for (sample, name, level) in checkpoints {
                let alternate;
                let scene = if level == 1 {
                    &base
                } else {
                    alternate = Scene::new(
                        course.clone(),
                        assets.clone(),
                        SceneOptions {
                            course: stem.into(),
                            branch_mask: mask,
                            level,
                            ..Default::default()
                        },
                    )?;
                    &alternate
                };
                source_notes
                    .entry(stem.into())
                    .or_default()
                    .extend(scene.notes.iter().cloned());
                let view = View {
                    position_q8: sample * 256.,
                    ..Default::default()
                };
                let spec = CameraSpec::default();
                let snapshot = rr_simulation::Simulation::default().snapshot();
                let (frame, receipt) = scene.draw(&spec, &view, &snapshot, false)?;
                let (repeat, _) = scene.draw(&spec, &view, &snapshot, false)?;
                let (different, diff) = image_diff(&frame.image, &repeat.image)?;
                let file = format!("{stem}-{mask}-{name}.png");
                let png = frame.image.png()?;
                fs::write(out.join(&file), &png)?;
                fs::write(out.join(file.replace(".png", "-diff.png")), diff.png()?)?;
                if different != 0 {
                    return Err(format!(
                        "checkpoint {file}: {different} differing pixels (diff retained)"
                    )
                    .into());
                }
                reports.push(serde_json::json!({"course":stem,"level":level,"branch_mask":mask,"sample":sample,"tick":snapshot.tick,"camera":spec,"file":file,"rgba_sha256":hash(&frame.image.pixels),"png_sha256":hash(&png),"different_pixels":different,"mask":"none","render":receipt}));
            }
        }
    }
    fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"schema_version":1,"outcome":"passed","source_notes":source_notes,"checkpoints":reports}),
        )?,
    )?;
    println!("{} deterministic checkpoint images", reports.len());
    Ok(())
}
