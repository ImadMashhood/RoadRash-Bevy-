//! Redistributable authored visual scenarios, without commercial game bytes.
use crate::{Result, raster::Texture, scene::*, source::*};
use rr_course::{Course, FamilyEvent, Hill, Node, NodeKind, Sample, Section, Terrain, Wall};
use rr_formats::family::{Family, FamilyNode};
use std::{collections::BTreeMap, sync::Arc};
pub const CASES: [&str; 7] = [
    "straight", "bend", "crest", "valley", "hill", "wall", "building",
];
pub fn scene(name: &str) -> Result<Scene> {
    if !CASES.contains(&name) {
        return Err("unknown authored scene".into());
    }
    let mut samples = vec![Sample::road(); 128];
    for (i, p) in samples.iter_mut().enumerate() {
        p.bend = if name == "bend" {
            if i % 30 < 15 { 8 } else { -8 }
        } else {
            0
        };
        p.rise = if name == "crest" {
            if i % 40 < 20 { 20 } else { -20 }
        } else if name == "valley" {
            if i % 40 < 20 { -20 } else { 20 }
        } else {
            0
        };
        p.shoulder = [80, 80];
    }
    let mut terrain = Terrain {
        tag: "DLBR".into(),
        source_offset: 0,
        raw: vec![],
        selectors: vec![[0; 2]; 128],
        endpoint_selectors: None,
        wall_records: [vec![], vec![]],
        hills: vec![],
    };
    match name {
        "hill" => {
            terrain.tag = "LIHR".into();
            terrain.hills = vec![
                Hill {
                    start: 0,
                    end: 80,
                    selectors: [0, 0],
                    points: [[4, 20, 2, 0, 15, 1, 5, 15]; 8],
                },
                Hill {
                    start: 80,
                    end: 128,
                    selectors: [0, 0],
                    points: [[4, 20, 2, 0, 15, 1, 5, 15]; 8],
                },
            ]
        }
        "wall" => {
            terrain.tag = "NTMR".into();
            terrain.wall_records = [
                vec![Wall {
                    start: 0,
                    parameters: [0, 1, 30, 1, 50, 2, 1, 10],
                    base_height: 400,
                }],
                vec![Wall {
                    start: 0,
                    parameters: [0, 1, 30, 1, 50, 2, 1, 10],
                    base_height: -500,
                }],
            ];
            for p in &mut samples {
                p.elevation = 300;
            }
        }
        _ => {}
    }
    let section = Section {
        source_offset: 0,
        raw: [0; 13],
        backward_endpoint: samples.last().unwrap().clone(),
        samples,
        lane_intervals: vec![],
        side_intervals: vec![],
        events: vec![],
        family_events: vec![FamilyEvent {
            source_offset: 0,
            sample: 0,
            kind: 1,
            slot: 0,
            family: 1,
            raw: [0; 3],
        }],
        family_bias: 0,
        roadside: vec![],
        scenery: vec![],
        scenery_following: None,
        terrain: if ["hill", "wall", "building"].contains(&name) {
            Some(terrain)
        } else {
            None
        },
    };
    let node = |offset, kind, next, previous, link, length| Node {
        offset,
        file_offset: offset,
        kind,
        next,
        previous,
        link,
        length,
        primary: 0,
        raw: vec![0; 8],
    };
    let course = Course {
        root: 20,
        sections: vec![section],
        nodes: BTreeMap::from([
            (20, node(20, NodeKind::Road, 60, 52, 0, 128)),
            (52, node(52, NodeKind::Start, 0, 0, 0, 0)),
            (60, node(60, NodeKind::End, 0, 0, 0, 0)),
        ]),
        source_sha256: "authored-render-fixture-v1".into(),
    };
    let mut assets = SourceAssets::authored();
    let mut pixels = vec![];
    for y in 0..16 {
        for x in 0..16 {
            pixels.extend(if (x / 4 + y / 4) % 2 == 0 {
                [150, 90, 55, 255]
            } else {
                [90, 125, 160, 255]
            })
        }
    }
    let material = Material {
        texture: Texture::new(
            rr_formats::pixels::Rgba {
                width: 16,
                height: 16,
                pixels,
            },
            true,
        )?,
        height: 300.,
    };
    let leaf = FamilyNode {
        offset: 200,
        tag: 0x434c4750,
        children: vec![],
        raw_header: vec![0; 32],
    };
    let mut root = FamilyNode {
        offset: 0,
        tag: 6,
        children: vec![100; 6],
        raw_header: vec![],
    };
    root.children[1] = 100;
    let family = FamilyAssets {
        definition: Arc::new(Family {
            nodes: vec![
                root,
                FamilyNode {
                    offset: 100,
                    tag: 1,
                    children: vec![200],
                    raw_header: vec![],
                },
                leaf,
            ],
            textures: vec![],
            unpacked_length: 0,
        }),
        groups: BTreeMap::from([(200, material)]),
        sprites: BTreeMap::new(),
    };
    assets.families.insert(1, family);
    Scene::new(course, assets, SceneOptions::default())
}
