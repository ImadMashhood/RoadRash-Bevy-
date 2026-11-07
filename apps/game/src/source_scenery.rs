//! Source physical boxes retain frame geometry, source query order and ring identity.
use rr_simulation::{
    encounter::{Obstacle, ObstacleKind, Setup, StaticBox},
    scenery::BoxRules,
};
use std::collections::BTreeMap;
pub fn import(
    setup: &mut Setup,
    scene: &rr_render::Scene,
    level: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let placements = rr_course::scenery::place(&scene.course, level, 1)?;
    let mut identities = BTreeMap::new();
    let mut insert_identity =
        |key, sample, mask, offset| -> Result<(), Box<dyn std::error::Error>> {
            if let Some((old_sample, old_mask, old_offset)) = identities.get_mut(&key) {
                if *old_sample != sample || *old_offset != offset {
                    return Err("conflicting source scenery cache identity".into());
                }
                *old_mask |= mask;
            } else {
                identities.insert(key, (sample, mask, offset));
            }
            Ok(())
        };
    for (sample, p) in scene.route.points.iter().enumerate() {
        insert_identity(
            (p.node, p.local),
            sample as u32,
            p.dual.as_ref().map_or(3, |d| 1 << d.half),
            p.offset,
        )?;
        if let Some(d) = &p.dual {
            insert_identity(
                (d.node, d.local),
                sample as u32,
                1 << (1 - d.half),
                d.offset,
            )?;
        }
    }
    if setup.rules.source_roads.is_some() {
        identities.clear();
        for (&node_id, node) in &scene.course.nodes {
            if node.kind == rr_course::NodeKind::Road {
                for local in 0..node.length {
                    identities.insert((node_id, local), (local, 3, 0));
                }
            }
        }
    }
    let mut spans: Vec<rr_simulation::scenery::RoadSpan> = vec![];
    for (&(path, local), &(_, road_mask, _)) in &identities {
        if let Some(last) = spans
            .last_mut()
            .filter(|s| s.path == path && s.road_mask == road_mask && s.last + 1 == local)
        {
            last.last = local;
        } else {
            spans.push(rr_simulation::scenery::RoadSpan {
                path,
                first: local,
                last: local,
                road_mask,
            });
        }
    }
    setup.rules.scenery_roads = spans;
    for (ordinal, instance) in placements.instances.iter().enumerate() {
        let class = match instance.kind.as_str() {
            "roadside" => 16,
            "billboard" if instance.raw[0] & 0x80000000 == 0 => (instance.raw[0] & 15) as u8,
            _ => continue,
        };
        let (Some(family), Some(lateral)) = (instance.family, instance.lateral) else {
            continue;
        };
        let Some(bank) = scene.assets.families.get(&family) else {
            continue;
        };
        let category = if class == 16 { 0 } else { 4 };
        let Some(boxes) = bank.collision_boxes(category, usize::from(instance.selector & 63))
        else {
            continue;
        };
        if boxes.is_empty() {
            continue;
        }
        let path =
            &scene.course.sections[instance.section as usize].samples[instance.sample as usize];
        if class == 16 && !(lateral > -path.width[0] && lateral < path.width[1]) {
            continue;
        }
        for (node_id, node) in scene.course.nodes.iter().filter(|(_, n)| {
            n.kind == rr_course::NodeKind::Road
                && n.link == instance.section
                && instance.sample < n.length
        }) {
            // Ring statics only exist for samples in the selected/dual native
            // scene tables. A disconnected or unwired node has no loaded boxes.
            let Some(&(sample, road_mask, offset)) = identities.get(&(*node_id, instance.sample))
            else {
                continue;
            };
            let anchor = (lateral + offset) * 256;
            for (index, source) in boxes.iter().enumerate() {
                let mut bounds = *source;
                bounds[0] += anchor;
                bounds[2] += anchor;
                let query_bounds = (class == 1 || class == 2).then_some(bounds);
                if class == 1 {
                    bounds[1] = bounds[3] - 8192;
                } else if class == 2 {
                    bounds[1] = bounds[3] - 16383;
                }
                if bounds[0] >= bounds[2] || bounds[1] >= bounds[3] {
                    continue;
                }
                setup.rules.obstacles.push(Obstacle {
                    id: setup.rules.obstacles.len() as u64 + 1,
                    path: *node_id,
                    progress: i64::from(instance.sample) * 256 + 128,
                    half_length: 1,
                    left: bounds[0],
                    right: bounds[2],
                    height: bounds[3].max(1),
                    kind: if (3..16).contains(&class) {
                        ObstacleKind::Decorative
                    } else {
                        ObstacleKind::Solid
                    },
                    source_box: Some(StaticBox {
                        bounds,
                        lateral: anchor,
                        query_bounds,
                        scenery: Some(BoxRules {
                            class,
                            road_mask,
                            sample,
                            selector: instance.selector,
                            ordinal: (ordinal * 4096 + index) as u32,
                            box_count: boxes.len() as u16,
                            texture_flags: bank
                                .collision_flags(category, usize::from(instance.selector & 63)),
                        }),
                    }),
                });
            }
            let _ = node;
        }
    }
    Ok(())
}
