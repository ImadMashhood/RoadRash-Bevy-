//! Private complete mounted-terrain transactions; source assets never ship with this probe.
use super::*;
use building_wall::{Boundary, Surface};
fn boundary(row: &serde_json::Value, sample: u32, legacy: bool) -> Boundary {
    let n = |i: usize| row[i].as_i64().unwrap() as i32;
    let offset = n(1);
    let mut points = [[[0; 2]; 3]; 2];
    for (side, p) in points.iter_mut().enumerate() {
        for (band, p) in p.iter_mut().enumerate() {
            *p = [n(17 + side * 6 + band * 2), n(18 + side * 6 + band * 2)];
        }
    }
    Boundary {
        path: 0,
        sample,
        building: n(0) == 0,
        road: [n(2) - offset, n(3) - offset],
        outer: [n(4) - offset, n(5) - offset],
        selectors: [n(12) as u8, n(13) as u8],
        flags: n(14) as u8,
        surface: (!legacy).then_some(Surface {
            kind: n(0) as u8,
            center: 0,
            base: [n(6) - offset, n(7) - offset],
            top: [n(8) - offset, n(9) - offset],
            height: [n(10), n(11)],
            side_height: [n(15), n(16)],
            points,
            event_type: n(29) as u8,
            rail_offset: n(30) as i8,
            rail_flags: n(31) as u8,
            drawn_rail_height: None,
        }),
    }
}
#[test]
#[ignore = "requires complete pinned native mounted terrain trajectories"]
fn source_terrain_motion_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_TERRAIN_MOTION").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut base = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let legacy = std::env::var_os("RR_TERRAIN_MOTION_LEGACY").is_some();
    base.motion_version = if legacy { 2 } else { 3 };
    let collision =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut bases = vec![];
    for fixture in native["fixtures"].as_array().unwrap() {
        let mut config = base.clone();
        let kind = fixture["kind"].as_u64().unwrap() as u8;
        let mode = fixture["mode"].as_u64().unwrap() as u8;
        let offset = fixture["offset"].as_i64().unwrap() as i32;
        let ground = fixture["ground"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let n = |i: usize| row[i].as_i64().unwrap() as i32;
                handling::Ground {
                    left: n(0),
                    right: n(1),
                    left_shoulder: n(2),
                    right_shoulder: n(3),
                    kind: if legacy {
                        match kind {
                            0 => 4,
                            1 | 3 => 1,
                            _ => 0,
                        }
                    } else {
                        if kind == 255 { 2 } else { kind }
                    },
                    flags: n(5) as u8,
                    rise: n(6),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        config.ground = ground
            .iter()
            .map(|g| handling::Ground {
                left: g.left + offset,
                right: g.right + offset,
                left_shoulder: g.left_shoulder + offset,
                right_shoulder: g.right_shoulder + offset,
                ..*g
            })
            .collect();
        let mut setup = encounter::Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(128 * 256);
        setup.topology.paths.get_mut(&0).unwrap().rises =
            ground.iter().map(|g| g.rise as i8).collect();
        setup.rules.combat_version = 3;
        setup.rules.collision_version = 2;
        setup.rules.contact_walk_version = u8::from(!legacy);
        setup.rules.scenery_version = 1;
        setup.rules.collision = Some(collision.clone());
        setup.rules.finish_progress = Some(1_000_000);
        setup.rules.obstacles.clear();
        setup.rules.building_boundaries = fixture["boundaries"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(sample, row)| {
                let mut boundary = boundary(row, sample as u32, legacy);
                if legacy {
                    boundary.translate(offset);
                }
                boundary
            })
            .collect();
        if !legacy {
            setup.rules.source_roads = Some(source_roads::Roads {
                version: 1,
                initial: 0,
                lanes_q8: None,
                ground: std::collections::BTreeMap::from([(0, ground)]),
                forks: Default::default(),
                profiles: vec![source_roads::Profile {
                    finish_progress: None,
                    choices: vec![],
                    selected: std::collections::BTreeMap::from([(0, 0)]),
                    identities: std::collections::BTreeMap::from([(
                        0,
                        (0..128).map(|sample| [sample, sample, offset, 3]).collect(),
                    )]),
                }],
            });
        }
        bases.push((config, setup, kind, mode, offset));
    }
    let mut actual = vec![];
    let mut failures = vec![];
    let mut observations = 0usize;
    let mut crashes = 0usize;
    for case in native["cases"].as_array().unwrap() {
        let index = case["fixture"].as_u64().unwrap() as usize;
        let (config, base, _, _, _) = &bases[index];
        let n = |i: usize| case["initial"][i].as_i64().unwrap() as i32;
        let mut setup = base.clone();
        setup.seeds[0].distance = i64::from(n(0));
        setup.seeds[0].lateral = n(1);
        setup.seeds[0].speed = n(3);
        setup.seeds[0].location = Some(topology::Location {
            progress: i64::from(n(0)),
            lateral: n(1),
            height: n(2),
            ..Default::default()
        });
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        {
            let bike = &mut sim
                .actors
                .get_mut(&ActorId(1))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            bike.condition = 150;
            bike.previous_ground = n(4);
            bike.contact = if n(2) > 0 {
                0
            } else {
                config.tables.object[11]
            };
            bike.previous_rise = config.ground[(n(0) >> 8) as usize].rise;
        }
        let mut rows = vec![];
        for expected in case["rows"].as_array().unwrap() {
            let m = |i: usize| expected[i].as_i64().unwrap() as i32;
            let command = Command {
                steer: m(1) as i8,
                ..Default::default()
            };
            let mut moved = sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().clone();
            moved
                .step_with_combat_profile_motion(config, command, sim.tick.0 as u32, 3)
                .unwrap();
            sim.step(command).unwrap();
            let bike = &sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().bike;
            let unit = &sim.encounter.as_ref().unwrap().units[&ActorId(1)];
            let down = unit.recovery.phase != recovery::Phase::Mounted;
            let mut output = vec![
                m(0),
                m(1),
                bike.distance_q8 as i32,
                bike.lateral,
                bike.height,
                bike.speed,
                bike.lateral_speed,
                bike.vertical_speed,
                bike.drive,
                bike.lean,
                bike.condition,
                bike.previous_ground,
                bike.contact,
                i32::from(bike.skid),
                bike.gravity,
                bike.pitch,
                unit.fighter.stats.maximum,
                unit.fighter.stats.health,
                i32::from(down),
            ];
            if let Some(body) = &unit.recovery.body {
                let b = &body.motion;
                output.extend([
                    b.distance_q8 as i32,
                    b.lateral,
                    b.height,
                    b.speed,
                    b.lateral_speed,
                    b.vertical_speed,
                    i32::from(body.animation.code),
                ]);
            } else {
                output.extend([0; 7]);
            }
            let response = [
                bike.distance_q8 as i32,
                bike.lateral,
                bike.speed,
                bike.lateral_speed,
                i32::from(bike.crashed),
            ] != [
                moved.bike.distance_q8 as i32,
                moved.bike.lateral,
                moved.bike.speed,
                moved.bike.lateral_speed,
                i32::from(moved.bike.crashed),
            ];
            output.push(i32::from(response));
            let expected = expected
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as i32)
                .collect::<Vec<_>>();
            if output != expected {
                failures.push(serde_json::json!({"case":case["id"],"fixture":index,"tick":m(0),"native":expected,"rust":output}));
            }
            observations += 1;
            rows.push(output);
            if down {
                crashes += 1;
            }
        }
        actual.push(serde_json::json!({"id":case["id"],"rows":rows}));
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_TERRAIN_MOTION_OUTPUT").unwrap());
    std::fs::write(
        out.join("terrain-motion-rust.json"),
        serde_json::to_vec(&actual).unwrap(),
    )
    .unwrap();
    std::fs::write(out.join("rust-comparison.json"),serde_json::to_vec_pretty(&serde_json::json!({"outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":observations,"episodes":native["cases"].as_array().unwrap().len(),"fixtures":bases.len(),"legacy_profile":legacy,"mismatches":failures.len(),"first_mismatches":failures.iter().take(12).collect::<Vec<_>>(),"first_crash_observations":crashes,"scope":"Complete controlled mounted motion through source terrain, boundary response and vertical integration until first crash; no detached recovery episode claim"})).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "{} mounted terrain trajectory mismatches",
        failures.len()
    );
}
