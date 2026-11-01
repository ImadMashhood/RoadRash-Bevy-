//! Startup-only static decoding; unsupported traits remain unknown.
use super::*;
impl Setup {
    pub fn source(
        c: &riding::Configuration,
        e: &mut encounter::Setup,
        course: &rr_course::Course,
        exe: &[u8],
        level: u8,
        vehicle_types: u16,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let pe = rr_formats::pe::SupportedExe::read(exe)?;
        let word = |va| -> std::result::Result<i32, Box<dyn std::error::Error>> {
            Ok(rr_formats::binary::u32le(pe.read_va(va, 4)?, 0)? as i32)
        };
        let mut out = Self {
            vehicle_types,
            ..Default::default()
        };
        e.inventory.items.clear();
        let player_grid = if level < 4 { 0x464088 } else { 0x464108 };
        e.seeds[0].lateral = word(player_grid)?;
        e.seeds[0].distance = i64::from(word(player_grid + 4)?) - i64::from(c.start_sample) * 256;
        for seed in e.seeds.iter_mut().skip(1) {
            let k = seed.id.0 as u32 - 1;
            let personality = if k < 9 { k } else { 0 };
            let base = 0x464558 + 0x204 * personality;
            let bike = u32::from(pe.read_va(0x4641c8 + k * 5 + u32::from(level), 1)?[0]) + 1;
            let traits = Traits {
                personality: Some(personality as u8),
                pace: word(base + 0x1bc + u32::from(level) * 4)?,
                corner_pace: word(base + 0x1a8 + u32::from(level) * 4)?,
                overtake_index: Some(pe.read_va(base + 0x165, 1)?[0]),
                kick_roll: Some(pe.read_va(base + 0x1d0, 1)?[0] as i8),
                backhand_roll: Some(pe.read_va(base + 0x1d1, 1)?[0] as i8),
                bike,
                ..Default::default()
            };
            let grid = if level < 4 { 0x464088 } else { 0x464108 };
            seed.lateral = word(grid + k * 8)?;
            seed.distance = i64::from(word(grid + k * 8 + 4)?) - i64::from(c.start_sample) * 256;
            seed.drive = false;
            let row = 0x463c78 + k * 0x40;
            seed.stats = combat::Stats {
                strength: word(row + 0x30)?,
                base: word(row + 0x34)?,
                maximum: word(row + 0x34)?,
                health: word(row + 0x34)?,
            };
            let weapon = word(row + (6 + u32::from(level)) * 4)?;
            if (1..=2).contains(&weapon) {
                e.inventory.items.insert(
                    combat::WeaponId(u64::from(k)),
                    combat::Item {
                        kind: if weapon == 1 {
                            animation::Weapon::Club
                        } else {
                            animation::Weapon::Chain
                        },
                        owner: Some(seed.id),
                    },
                );
            }
            out.brains.insert(seed.id, traits);
        }
        for (id, node) in course
            .nodes
            .iter()
            .filter(|(_, n)| n.kind == rr_course::NodeKind::Road)
        {
            let section = &course.sections[node.link as usize];
            for event in &section.events {
                if let Some(zone) = Zone::from_event(
                    out.zones.len() as u64 + 1,
                    *id,
                    i64::from(node.length) * 256,
                    event,
                ) {
                    out.zones.push(zone);
                }
            }
        }
        let placements = rr_course::scenery::place(course, level + 1, 1)?;
        for p in placements
            .instances
            .iter()
            .filter(|p| p.kind == "pedestrian" || p.kind == "police")
        {
            for (id, node) in course.nodes.iter().filter(|(_, n)| {
                n.kind == rr_course::NodeKind::Road && n.link == p.section && p.sample < n.length
            }) {
                let _ = node;
                out.spawns.push(Spawn {
                    id: out.spawns.len() as u64 + 1,
                    location: Location {
                        path: *id,
                        progress: i64::from(p.sample) * 256,
                        lateral: if p.mirror { -192000 } else { 192000 },
                        ..Default::default()
                    },
                    kind: if p.kind == "police" {
                        Kind::Police
                    } else {
                        Kind::Pedestrian
                    },
                    family: p.family,
                    selector: p.selector & 63,
                    subtype: (p.raw[0] & 15) as u8,
                    speed: if p.kind == "police" { 6000 } else { 1000 },
                    direction: if p.mirror { 1 } else { -1 },
                });
            }
        }
        out.validate(c, e)?;
        Ok(out)
    }
}

/// Startup masks and sequential identity rules from the source vehicle wrapper.
pub fn vehicle_masks(cars: &[u8]) -> std::result::Result<Vec<u32>, Box<dyn std::error::Error>> {
    use rr_formats::{
        binary::{range, u32le},
        resource::Directory,
    };
    let directory = Directory::read(cars)?;
    let mut animations: Vec<_> = directory
        .entries
        .iter()
        .filter(|e| e.tag == "ANIM")
        .collect();
    animations.sort_by_key(|e| e.id);
    let mut wrappers: Vec<_> = directory
        .entries
        .iter()
        .filter(|e| e.tag == "CANS")
        .collect();
    wrappers.sort_by_key(|e| e.id);
    let (mut inherited, mut next) = (0, 0);
    let mut masks = vec![];
    for (index, animation) in animations.iter().enumerate().take(15) {
        if animation.id != index as u32 + 1 {
            break;
        }
        if let Some(wrapper) = wrappers.get(next).filter(|e| e.id == animation.id) {
            let bytes = range(cars, wrapper.offset as usize, wrapper.length as usize)?;
            if u32le(bytes, 0)? != 0x43414e53 {
                return Err("vehicle SNAC signature".into());
            }
            let base = if u32le(bytes, 8)? == 1 {
                8
            } else {
                u32le(bytes, 4)? as usize
            };
            let cel = base
                .checked_add(u32le(bytes, base + 4)? as usize)
                .and_then(|p| p.checked_add(12))
                .ok_or("vehicle wrapper overflow")?;
            inherited = u32le(bytes, cel + 4)?;
            next += 1;
        }
        masks.push(inherited);
    }
    if masks.is_empty() {
        return Err("no sequential vehicle resources".into());
    }
    Ok(masks)
}
