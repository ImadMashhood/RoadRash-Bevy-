//! Opt-in private native matrix. Outputs hashes/counts, never source pixels.
use rr_render::{
    camera::{CameraSpec, View},
    scene::{Scene, SceneOptions},
    source::SourceAssets,
};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() != 2 {
        return Err("usage: rr-native-verify DATA_ROOT CATALOG_ROOT".into());
    }
    let data = Path::new(&a[0]);
    let catalog = Path::new(&a[1]);
    let mut rows = vec![];
    for (stem, _) in rr_course::export::COURSES {
        let course = rr_course::Course::read(&std::fs::read(
            data.join(format!("DATA/COURSES/{stem}.CRS")),
        )?)?;
        let assets = SourceAssets::load(data, catalog, stem, &course)?;
        let mut poses = 0usize;
        for appearance in &assets.pedestrian_appearances {
            appearance.animation.validate()?;
            for code in 0..7 {
                let resolved = appearance
                    .animation
                    .resolve(0, code)
                    .ok_or("Unresolved pedestrian state")?;
                for frame in appearance.animation.banks[0][resolved].as_ref().unwrap() {
                    for facing in [-32768, -16384, 0, 16384, 32767] {
                        let (cell, _) = appearance.animation.select(0, frame, facing, 0)?;
                        let sprite = assets.families[&appearance.family].pedestrian_frames
                            [&appearance.selector]
                            .get(cell.frame)
                            .and_then(Option::as_ref)
                            .ok_or("Invisible animated pedestrian")?;
                        if !sprite
                            .texture
                            .image
                            .pixels
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .any(|p| p[3] != 0)
                        {
                            return Err("Transparent pedestrian".into());
                        }
                        poses += 1;
                    }
                }
            }
        }
        for level in 1..=5 {
            let scene = Scene::new(
                course.clone(),
                assets.clone(),
                SceneOptions {
                    course: stem.into(),
                    level,
                    ..Default::default()
                },
            )?;
            let mut captures = vec![];
            for sample in [50., 75., 120.] {
                let view = View {
                    position_q8: sample * 256.,
                    ..Default::default()
                };
                let snapshot = rr_simulation::Simulation::default().snapshot();
                let (serial, receipt) =
                    scene.draw(&CameraSpec::default(), &view, &snapshot, true)?;
                let (parallel, other) =
                    scene.draw_parallel(&CameraSpec::default(), &view, &snapshot, true, 6)?;
                if serial.image.pixels != parallel.image.pixels
                    || serial.depth != parallel.depth
                    || serde_json::to_value(&receipt)? != serde_json::to_value(&other)?
                {
                    return Err(format!(
                        "{stem} L{level} sample{sample}: parallel raster divergence"
                    )
                    .into());
                }
                let depth_bytes: Vec<_> = serial
                    .depth
                    .iter()
                    .flat_map(|value| value.to_bits().to_le_bytes())
                    .collect();
                captures.push(serde_json::json!({"sample":sample,"rgba_sha256":rr_content::hash(&serial.image.pixels),"depth_sha256":rr_content::hash(&depth_bytes),"render_receipt_sha256":rr_content::hash(&serde_json::to_vec(&receipt)?),"different_pixels":0,"depth_equal":true}));
            }
            rows.push(serde_json::json!({"course":stem,"level":level,"status":"passed","animated_pedestrian_appearances":assets.pedestrian_appearances.len(),"pedestrian_voice_references":assets.pedestrian_voices.values().flatten().filter(|v|v.is_some()).count(),"animated_pose_cases":poses,"fixed_camera_cases":captures}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"outcome":"passed","native_only":true,"matrix":rows,"masks":[],"pixel_tolerance_8bit":0,"depth_tolerance":0})
        )?
    );
    Ok(())
}
