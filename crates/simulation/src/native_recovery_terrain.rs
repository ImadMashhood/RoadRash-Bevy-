//! Native LIVE complete changing-terrain recovery trajectories.
use super::*;
use building_wall::{Boundary, Surface};
use serde_json::json;
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
fn clock(p: &animation::Player) -> Vec<i64> {
    vec![
        if *p == animation::Player::default() {
            -1
        } else {
            i64::from(p.code)
        },
        p.frame as i64,
        i64::from(p.queued.unwrap_or(-1)),
        i64::from(p.loops),
        i64::from(p.deadline.unwrap_or(if *p == animation::Player::default() {
            0
        } else {
            u32::MAX
        })),
        p.selected.map_or(-1, |v| v.0 as i64),
        p.selected.map_or(-1, |v| v.1 as i64),
    ]
}
fn row(sim: &Simulation) -> Vec<i64> {
    let w = sim.encounter.as_ref().unwrap();
    let u = &w.units[&ActorId(1)];
    let b = &sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().bike;
    let phase = match u.recovery.phase {
        recovery::Phase::Mounted => 0,
        recovery::Phase::Sliding => 1,
        recovery::Phase::Mounting => 3,
        _ => 2,
    };
    let mut v = vec![
        sim.tick.0 as i64,
        i64::from(sim.gameplay_rng.0),
        b.distance_q8,
        i64::from(b.lateral),
        i64::from(b.height),
        i64::from(b.speed),
        i64::from(b.lateral_speed),
        i64::from(b.vertical_speed),
        i64::from(b.contact),
        b.gear as i64,
        i64::from(b.previous_ground),
        i64::from(b.gravity),
        i64::from(b.pitch),
        i64::from(b.drive),
        i64::from(b.lean),
        i64::from(b.condition),
        phase,
        i64::from(u.fighter.stats.maximum),
        i64::from(u.fighter.stats.health),
        i64::from(u.recovery.elapsed),
        i64::from(u.recovery.body.is_some()),
    ];
    v.extend(clock(&u.recovery.bike_animation));
    if let Some(body) = &u.recovery.body {
        let b = &body.motion;
        v.extend([
            b.distance_q8,
            i64::from(b.lateral),
            i64::from(b.height),
            i64::from(b.speed),
            i64::from(b.lateral_speed),
            i64::from(b.vertical_speed),
            i64::from(b.contact),
            i64::from(b.previous_ground),
            i64::from(b.gravity),
            i64::from(b.pitch),
            i64::from(body.location.heading),
            i64::from(body.visual_heading.unwrap_or(body.location.heading)),
        ]);
        v.extend(clock(&body.animation));
    } else {
        v.extend([0; 12]);
        v.extend([-1, 0, -1, 0, 0, -1, -1]);
    }
    v
}
#[test]
#[ignore = "requires private native LIVE changing-terrain recovery traces"]
fn source_recovery_terrain_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_RECOVERY_TERRAIN").unwrap()).unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut base = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    base.motion_version = 3;
    let mut bases = vec![];
    for f in native["fixtures"].as_array().unwrap() {
        let offset = f["offset"].as_i64().unwrap() as i32;
        let mut c = base.clone();
        let ground = f["ground"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let n = |i: usize| r[i].as_i64().unwrap() as i32;
                handling::Ground {
                    left: n(0),
                    right: n(1),
                    left_shoulder: n(2),
                    right_shoulder: n(3),
                    kind: if n(4) == 255 { 2 } else { n(4) as u8 },
                    flags: n(5) as u8,
                    rise: n(6),
                    bend: n(7),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        c.ground = ground
            .iter()
            .map(|g| handling::Ground {
                left: g.left + offset,
                right: g.right + offset,
                left_shoulder: g.left_shoulder + offset,
                right_shoulder: g.right_shoulder + offset,
                ..*g
            })
            .collect();
        let mut s = encounter::Setup::authored(&c).unwrap();
        s.topology = topology::Topology::straight(512 * 256);
        s.topology.paths.get_mut(&0).unwrap().rises = ground.iter().map(|g| g.rise as i8).collect();
        s.rules.combat_version = 4;
        s.rules.collision_version = 2;
        s.rules.contact_walk_version = 2;
        s.rules.scenery_version = 1;
        s.rules.bike_contacts = true;
        s.rules.collision = Some(collision::Profile::from_exe(&exe).unwrap());
        s.rules.body = recovery::BodyLaw::from_exe(&exe).unwrap();
        s.rules.body.motion_version = 1;
        s.rules.body.recovery_version = 2;
        if std::env::var_os("RR_RECOVERY_NO_ANGLES").is_none() {
            s.rules.body.enable_source_curvature(&exe).unwrap();
        }
        s.rules.finish_progress = Some(1000000);
        s.rules.obstacles.clear();
        s.rules.building_boundaries = f["boundaries"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(sample, r)| boundary(r, sample as u32, false))
            .collect();
        s.rules.source_roads = Some(source_roads::Roads {
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
                    (0..512).map(|n| [n, n, offset, 3]).collect(),
                )]),
            }],
        });
        bases.push((c, s));
    }
    let mut failures = vec![];
    let mut all = vec![];
    let mut observations = 0;
    let mut mismatches = 0;
    for case in native["cases"].as_array().unwrap() {
        let id = case["id"].as_u64().unwrap();
        let fi = case["fixture"].as_u64().unwrap() as usize;
        let (c, base) = &bases[fi];
        let mut s = base.clone();
        let n = |i: usize| case["initial"][i].as_i64().unwrap() as i32;
        s.seeds[0].distance = i64::from(n(0));
        s.seeds[0].lateral = n(1);
        s.seeds[0].speed = n(3).max(0);
        s.seeds[0].stats = combat::Stats {
            base: 2500,
            maximum: 2500,
            health: 2500,
            strength: 7,
        };
        s.seeds[0].location = Some(topology::Location {
            progress: i64::from(n(0)),
            lateral: n(1),
            height: n(2),
            ..Default::default()
        });
        let mut sim = Simulation::with_encounters(1, c.clone(), s).unwrap();
        {
            let b = &mut sim
                .actors
                .get_mut(&ActorId(1))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            b.condition = n(5);
            b.failed = n(5) == 0;
            b.speed = n(3);
            b.height = n(2);
            b.vertical_speed = n(6);
            b.previous_ground = n(4);
            b.previous_above = n(2);
            b.contact = if n(2) > 0 { 0 } else { c.tables.object[11] };
            b.previous_rise = c.ground[(n(0) >> 8) as usize].rise;
        }
        let mut w = sim.encounter.take().unwrap();
        w.prepare_source_roads(&sim.actors, c, 0).unwrap();
        w.hit(
            &mut sim.actors,
            c,
            ActorId(1),
            recovery::Impact {
                reason: n(7) as u8,
                rider_velocity: Some([n(8), n(9), n(10)]),
                bike_velocity: Some([n(11), n(12), n(13)]),
                ..Default::default()
            },
            0,
            true,
            &mut vec![],
        )
        .unwrap();
        sim.encounter = Some(w);
        let mut rows = vec![];
        for expected in case["rows"].as_array().unwrap() {
            let tick = expected[0].as_u64().unwrap();
            if tick > 0 {
                sim.step(Command::default()).unwrap();
            }
            let actual = row(&sim);
            observations += 1;
            let value = json!(actual);
            if value != *expected {
                mismatches += 1;
            }
            if value != *expected && failures.len() < 100 {
                let diffs = actual
                    .iter()
                    .zip(expected.as_array().unwrap())
                    .enumerate()
                    .filter_map(|(i, (a, e))| {
                        if json!(a) != *e {
                            Some(json!([i, e, a]))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                failures.push(json!({"case":id,"fixture":fi,"scenario":case["scenario"],"tick":tick,"differences":diffs,"expected":expected,"actual":actual}));
            }
            rows.push(value);
        }
        all.push(json!({"id":id,"rows":rows}));
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_RECOVERY_OUTPUT").unwrap());
    std::fs::write(
        out.join("recovery-terrain-rust.json"),
        serde_json::to_vec(&all).unwrap(),
    )
    .unwrap();
    std::fs::write(out.join("recovery-terrain-comparison.json"),serde_json::to_vec_pretty(&json!({"outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":observations,"cases":all.len(),"mismatches":mismatches,"failure_records_capped_at":100,"first_divergences":failures,"scope":"Actual LIVE dt1 half then bike callbacks through native controlled changing-terrain crash recovery/remount+30ticks"})).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "native recovery terrain diverged; see receipt"
    );
}
