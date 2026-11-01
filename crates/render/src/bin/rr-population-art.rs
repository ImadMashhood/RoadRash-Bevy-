//! Private startup asset census: no source pixels are included in the receipt.
use serde_json::json;
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() != 2 {
        return Err("usage: rr-population-art DATA_ROOT CATALOG_ROOT".into());
    }
    let data = Path::new(&a[0]);
    let catalog = Path::new(&a[1]);
    let mut rows = vec![];
    for (stem, _) in rr_course::export::COURSES {
        let course = rr_course::Course::read(&std::fs::read(
            data.join(format!("DATA/COURSES/{stem}.CRS")),
        )?)?;
        let assets = rr_render::source::SourceAssets::load(data, catalog, stem, &course)?;
        let cars:Vec<_>=assets.vehicles.iter().map(|views|views.iter().map(|s|s.as_ref().map(|s|json!({"logical":s.logical,"pivot":s.pivot,"opaque_pixels":s.texture.image.pixels.as_chunks::<4>().0.iter().filter(|p|p[3]!=0).count()}))).collect::<Vec<_>>()).collect();
        for views in &assets.vehicles {
            if views.iter().any(|s| {
                s.as_ref().is_none_or(|s| {
                    !s.texture
                        .image
                        .pixels
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|p| p[3] != 0)
                })
            }) {
                return Err(format!("{stem}: missing/transparent vehicle view").into());
            }
        }
        let placements = rr_course::scenery::place(&course, 1, 1)?;
        let pedestrians: Vec<_> = placements
            .instances
            .iter()
            .filter(|p| p.kind == "pedestrian")
            .collect();
        let resolved = pedestrians
            .iter()
            .filter(|p| {
                p.family
                    .and_then(|id| assets.families.get(&id))
                    .and_then(|f| {
                        f.resolve(2, usize::from(p.selector & 63), Some(0))
                            .and_then(|at| f.sprites.get(&at))
                    })
                    .is_some()
            })
            .count();
        let missing: Vec<_> = pedestrians
            .iter()
            .filter(|p| {
                p.family
                    .and_then(|id| assets.families.get(&id))
                    .and_then(|f| {
                        f.resolve(2, usize::from(p.selector & 63), Some(0))
                            .and_then(|at| f.sprites.get(&at))
                    })
                    .is_none()
            })
            .map(|p| json!([stem, p.section, p.sample, p.selector]))
            .collect();
        rows.push(json!({"course":stem,"vehicles":cars,"police_slots":assets.police.len(),"police_nonempty":assets.police.iter().flatten().count(),"pedestrian_requests":pedestrians.len(),"drawable_pedestrian_requests":resolved,"missing_queries":missing}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"outcome":"passed","courses":rows}))?
    );
    Ok(())
}
