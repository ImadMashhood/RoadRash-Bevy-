//! Enumerate route contexts and attach each physical terrain's native boundary type.
use rr_simulation::{encounter::Setup, source_roads::Roads};
pub fn import(
    setup: &mut Setup,
    scene: &rr_render::Scene,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut roads = Roads::from_course(&scene.course, &scene.route)?;
    roads.set_finishes(&scene.course, scene.options.level, &setup.topology)?;
    for (&path, ground) in &mut roads.ground {
        let section = scene.course.nodes[&path].link as usize;
        for (local, g) in ground.iter_mut().enumerate() {
            g.kind = ground_kind(&scene.terrain[section][local], g.flags);
        }
    }
    roads.validate(&setup.topology)?;
    setup.rules.source_roads = Some(roads);
    Ok(())
}

pub fn ground_kind(terrain: &rr_render::scene::TerrainSample, flags: u8) -> u8 {
    if flags & 0x40 != 0 {
        return 3;
    }
    match terrain.kind {
        rr_render::scene::TerrainKind::Building => 0,
        rr_render::scene::TerrainKind::Wall => 1,
        rr_render::scene::TerrainKind::Hill => 4,
        // Surface classification distinguishes only native types 0 and 3.
        rr_render::scene::TerrainKind::None => 2,
    }
}

pub fn boundary(
    scene: &rr_render::Scene,
    path: u32,
    local: usize,
    offset: i32,
) -> rr_simulation::building_wall::Boundary {
    use rr_simulation::building_wall::{Boundary, Surface};
    let section = scene.course.nodes[&path].link as usize;
    let sample = &scene.course.sections[section].samples[local];
    let terrain = &scene.terrain[section][local];
    let tunnel = sample.marking_flags & 0x40 != 0;
    let mut kind = ground_kind(terrain, sample.marking_flags);
    if terrain.kind == rr_render::scene::TerrainKind::None && !tunnel {
        kind = 255;
    }
    let left = -sample.width[0];
    let right = sample.width[1];
    let mut edges = [left - sample.shoulder[0], right + sample.shoulder[1]];
    let walls = terrain.source_walls.unwrap_or([[0; 3]; 2]);
    let mut height = [walls[0][2], walls[1][2]];
    let outer = if tunnel {
        let mountain = if height[1] < 1 {
            edges[1] = edges[1] * 2 - right;
            0
        } else {
            if height[0] > 0 {
                height[0] = 0;
            }
            edges[0] = edges[0] * 2 - left;
            1
        };
        height[mountain] = height[mountain].max(800);
        [
            left - sample.shoulder[0] / 2,
            right + sample.shoulder[1] / 2,
        ]
    } else {
        edges
    };
    let base = [
        offset + edges[0] - walls[0][0],
        offset + edges[1] + walls[1][0],
    ];
    let top = [
        offset + edges[0] - walls[0][1],
        offset + edges[1] + walls[1][1],
    ];
    for h in &mut height {
        if *h < 1 {
            *h = -sample.elevation;
        }
    }
    let mut points = [[[0; 2]; 3]; 2];
    for (side, target) in points.iter_mut().enumerate() {
        for (i, point) in target.iter_mut().enumerate() {
            if terrain.kind == rr_render::scene::TerrainKind::Hill
                && let Some(source) = terrain.points[side].get(i)
            {
                *point = [
                    if side == 0 {
                        left - source[0] as i32
                    } else {
                        right + source[0] as i32
                    },
                    source[1] as i32,
                ];
            }
        }
    }
    let drawn_rail_height = (sample.event_type == 5)
        .then(|| {
            let selector = sample.event_parameters[0];
            let family = scene.family_slots[section][local][usize::from(selector >> 6)];
            let family = scene.assets.families.get(&family)?;
            let group = family.resolve(6, usize::from(selector & 63), None)?;
            let group = family
                .definition
                .nodes
                .iter()
                .find(|node| node.offset == group)?;
            let row_shift = u32::from(*group.raw_header.get(27)?) & 31;
            let last_row = u32::from(*group.raw_header.get(25)?) & 31;
            Some(
                1u32.wrapping_shl(row_shift)
                    .wrapping_shl(last_row)
                    .wrapping_mul(4) as i32,
            )
        })
        .flatten();
    let mut selectors = terrain.selectors;
    if kind == 0 {
        for selector in &mut selectors {
            if *selector == 255 {
                continue;
            }
            let family = scene.family_slots[section][local][usize::from(*selector >> 6)];
            let nominal_height = scene.assets.families.get(&family).and_then(|family| {
                let group = family.resolve(1, usize::from(*selector & 63), None)?;
                let node = family
                    .definition
                    .nodes
                    .iter()
                    .find(|node| node.offset == group)?;
                node.raw_header.get(22).copied()
            });
            if nominal_height.is_some_and(|height| u32::from(height) * 16 < 17) {
                *selector = 255;
            }
        }
    }
    Boundary {
        path,
        sample: local as u32,
        building: terrain.kind == rr_render::scene::TerrainKind::Building,
        road: [offset + left, offset + right],
        outer: [offset + outer[0], offset + outer[1]],
        selectors,
        flags: sample.marking_flags,
        surface: Some(Surface {
            kind,
            center: offset,
            base,
            top,
            height,
            side_height: sample.side_height,
            points,
            event_type: sample.event_type,
            rail_offset: sample.event_parameters[1] as i8,
            rail_flags: sample.event_parameters[2],
            drawn_rail_height,
        }),
    }
}
