//! Actual mounted decision callbacks, not preselected crash reasons.
use super::*;
use serde_json::json;
fn row(sim: &Simulation) -> Vec<i64> {
    let u = &sim.encounter.as_ref().unwrap().units[&ActorId(1)];
    let b = &sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().bike;
    let phase = match u.recovery.phase {
        recovery::Phase::Mounted => 0,
        recovery::Phase::Sliding => 1,
        recovery::Phase::Mounting => 3,
        _ => 2,
    };
    let mut v = vec![
        b.distance_q8,
        i64::from(b.lateral),
        i64::from(b.height),
        i64::from(b.speed),
        i64::from(b.lateral_speed),
        i64::from(b.vertical_speed),
        i64::from(b.previous_ground),
        i64::from(b.drive),
        i64::from(b.lean),
        i64::from(b.condition),
        phase,
        b.gear as i64,
        i64::from(b.skid),
        i64::from(b.skid_ticks),
        i64::from(u.fighter.stats.maximum),
        i64::from(u.fighter.stats.health),
        i64::from(u.recovery.body.is_some()),
    ];
    if let Some(body) = &u.recovery.body {
        let b = &body.motion;
        v.extend([
            b.distance_q8,
            i64::from(b.lateral),
            i64::from(b.height),
            i64::from(b.speed),
            i64::from(b.lateral_speed),
            i64::from(b.vertical_speed),
            i64::from(b.previous_ground),
            i64::from(body.location.heading),
            i64::from(body.visual_heading.unwrap_or(body.location.heading)),
            i64::from(body.animation.code),
        ]);
    } else {
        v.extend([0; 10]);
    }
    let a = &u.recovery.bike_animation;
    v.extend([
        if *a == animation::Player::default() {
            -1
        } else {
            i64::from(a.code)
        },
        a.frame as i64,
        i64::from(a.queued.unwrap_or(-1)),
        i64::from(a.loops),
        i64::from(a.deadline.unwrap_or(if *a == animation::Player::default() {
            0
        } else {
            u32::MAX
        })),
        a.selected.map_or(-1, |v| v.0 as i64),
        a.selected.map_or(-1, |v| v.1 as i64),
    ]);
    v.push(i64::from(b.source_yaw.unwrap_or(0)));
    v
}
#[test]
#[ignore = "requires private actual mounted native crash decisions"]
fn source_recovery_entry_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut base = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    base.start_sample = 0;
    base.motion_version = 3;
    let cases: Vec<serde_json::Value> = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_RECOVERY_ENTRY").unwrap()).unwrap(),
    )
    .unwrap();
    assert!([384, 432, 528, 576].contains(&cases.len()));
    let mut bad = vec![];
    let mut prebad = vec![];
    let mut actual = vec![];
    for case in &cases {
        let kind = case["kind"].as_u64().unwrap() as u8;
        let variation = case["variation"].as_i64().unwrap() as i32;
        let mut c = base.clone();
        c.ground = vec![
            handling::Ground {
                left: -500,
                right: 500,
                left_shoulder: -750,
                right_shoulder: 750,
                kind: if kind == 4 || kind == 9 { 0 } else { 2 },
                ..Default::default()
            };
            512
        ];
        let mut setup = encounter::Setup::authored(&c).unwrap();
        setup.topology = topology::Topology::straight(512 * 256);
        setup.rules.combat_version = 4;
        setup.rules.collision_version = 2;
        setup.rules.contact_walk_version = 2;
        setup.rules.collision = Some(collision::Profile::from_exe(&exe).unwrap());
        setup.rules.body = recovery::BodyLaw::from_exe(&exe).unwrap();
        setup.rules.body.motion_version = 1;
        setup.rules.body.recovery_version = 2;
        setup.rules.finish_progress = None;
        setup.rules.obstacles.clear();
        setup.rules.building_boundaries.clear();
        if kind == 4 || kind == 5 || kind == 9 || kind == 10 {
            let n = |i: usize| case["boundary"][i].as_i64().unwrap() as i32;
            setup.rules.building_boundaries = (0..512)
                .map(|sample| building_wall::Boundary {
                    path: 0,
                    sample,
                    building: n(0) == 0,
                    road: [n(1), n(2)],
                    outer: [n(3), n(4)],
                    flags: n(5) as u8,
                    selectors: [n(6) as u8, n(7) as u8],
                    surface: Some(building_wall::Surface {
                        kind: n(0) as u8,
                        center: 0,
                        base: [n(8), n(9)],
                        top: [n(10), n(11)],
                        height: [n(12), n(13)],
                        side_height: [0; 2],
                        points: [[[0; 2]; 3]; 2],
                        event_type: n(14) as u8,
                        rail_offset: n(15) as i8,
                        rail_flags: n(16) as u8,
                        drawn_rail_height: None,
                    }),
                })
                .collect();
        }

        setup.seeds[0].stats = combat::Stats {
            base: 3000,
            maximum: 3000,
            health: if variation == 0 {
                1
            } else if variation == 1 {
                64
            } else if variation == 2 {
                65
            } else {
                3000
            },
            strength: 7,
        };
        setup.seeds[0].distance = 12800;
        setup.seeds[0].speed = case["speed"].as_i64().unwrap() as i32;
        setup.seeds[0].location = Some(topology::Location {
            path: 0,
            progress: 12800,
            ..Default::default()
        });
        let mut sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
        let pre: Vec<i64> = serde_json::from_value(case["pre"].clone()).unwrap();
        {
            let r = sim
                .actors
                .get_mut(&ActorId(1))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            r.bike.lateral_speed = if kind >= 8 {
                case["lateral_speed"].as_i64().unwrap() as i32 * 16
            } else {
                pre[4] as i32
            };
            r.bike.vertical_speed = pre[5] as i32;
            r.bike.height = if kind >= 8 { 0 } else { pre[2] as i32 };
            r.bike.condition = pre[9] as i32;
            r.bike.drive = if kind == 10 {
                pre[7] as i32
            } else if kind >= 8 {
                (variation - 1) * 32
            } else {
                pre[7] as i32
            };
            r.bike.contact = if kind == 3 {
                1
            } else if kind == 5 || kind == 6 {
                0
            } else {
                c.tables.object[11]
            };
            r.bike.lean = pre[8] as i32;
            r.bike.skid_ticks = pre[13] as i32;
            r.bike.lateral = if kind >= 8 { 0 } else { pre[1] as i32 };
            r.advance_animation(&c, 0).unwrap();
        }
        {
            let u = sim
                .encounter
                .as_mut()
                .unwrap()
                .units
                .get_mut(&ActorId(1))
                .unwrap();
            u.recovery.bike_location.lateral = if kind >= 8 { 0 } else { pre[1] as i32 };
            u.recovery.bike_location.height = if kind >= 8 { 0 } else { pre[2] as i32 };
            u.previous_location = u.recovery.bike_location;
        }
        if kind >= 8 {
            let mut world = sim.encounter.take().unwrap();
            world
                .movement_selected(
                    &mut sim.actors,
                    &c,
                    0,
                    &[],
                    &mut vec![],
                    || 1,
                    Some(ActorId(1)),
                )
                .unwrap();
            sim.encounter = Some(world);
            if kind >= 9 {
                let r = sim
                    .actors
                    .get_mut(&ActorId(1))
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap();
                r.bike.lateral = pre[1] as i32;
                r.bike.lateral_speed = pre[4] as i32;
                r.bike.height = pre[2] as i32;
                if kind == 10 {
                    r.bike.contact = 0;
                }
                let u = sim
                    .encounter
                    .as_mut()
                    .unwrap()
                    .units
                    .get_mut(&ActorId(1))
                    .unwrap();
                u.recovery.bike_location.lateral = pre[1] as i32;
                u.recovery.bike_location.height = pre[2] as i32;
            }
        }
        let mut initial = row(&sim);
        initial.truncate(pre.len());
        if initial != pre {
            prebad.push(json!({"id":case["id"],"native":pre,"rust":initial}));
        }
        let mut w = sim.encounter.take().unwrap();
        if kind <= 2 || kind == 7 || kind == 8 || kind == 11 {
            let reason = if kind == 0 {
                4
            } else if kind == 1 {
                5
            } else if kind == 2 || kind == 8 || kind == 11 {
                2
            } else {
                4
            };
            w.hit(
                &mut sim.actors,
                &c,
                ActorId(1),
                recovery::Impact {
                    reason,
                    stamina_damage: if kind < 2 {
                        if variation == 3 { 47 } else { 1 }
                    } else {
                        0
                    },
                    bike_damage: u16::from(kind == 1),
                    rider_velocity: if kind == 2 || kind == 8 || kind == 11 {
                        Some([0, 0, 512])
                    } else if kind == 7 {
                        Some([0; 3])
                    } else {
                        None
                    },
                    bike_velocity: if kind == 2 || kind == 8 || kind == 11 {
                        Some([0, 0, 256])
                    } else if kind == 7 {
                        Some([0; 3])
                    } else {
                        None
                    },
                },
                0,
                kind >= 2,
                &mut vec![],
            )
            .unwrap();
        } else if kind == 3 || kind == 6 {
            w.movement_selected(
                &mut sim.actors,
                &c,
                0,
                &[],
                &mut vec![],
                || 1,
                Some(ActorId(1)),
            )
            .unwrap();
        } else {
            w.source_building_contact(ActorId(1), &mut sim.actors, &c, 0, &mut vec![])
                .unwrap();
        }
        if kind == 11 {
            if variation == 2 {
                let bike = &mut sim
                    .actors
                    .get_mut(&ActorId(1))
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap()
                    .bike;
                bike.height = c.spec().suspension + 1;
                bike.vertical_speed = -1;
            }
            let mut seed = 1u32;
            let unit = w.units.get_mut(&ActorId(1)).unwrap();
            unit.recovery
                .live_bike_motion(
                    &mut sim
                        .actors
                        .get_mut(&ActorId(1))
                        .unwrap()
                        .view
                        .rider
                        .as_mut()
                        .unwrap()
                        .bike,
                    &c,
                    &w.setup.topology,
                    &w.setup.rules.body,
                    0,
                    || {
                        seed = seed.wrapping_mul(214013).wrapping_add(2531011);
                        (seed >> 16) & 0x7fff
                    },
                )
                .unwrap();
        }
        sim.encounter = Some(w);
        let mut got = row(&sim);
        let want: Vec<i64> = serde_json::from_value(case["post"].clone()).unwrap();
        got.truncate(want.len());
        if got != want {
            bad.push(json!({"id":case["id"],"kind":kind,"native":want,"rust":got}));
        }
        actual.push(got);
    }
    let out =
        std::path::PathBuf::from(std::env::var_os("RR_NATIVE_RECOVERY_ENTRY_OUTPUT").unwrap());
    std::fs::write(
        out.join("recovery-entry-rust.json"),
        serde_json::to_vec(&actual).unwrap(),
    )
    .unwrap();
    std::fs::write(out.join("recovery-entry-comparison.json"),serde_json::to_vec_pretty(&json!({"outcome":if bad.is_empty()&&prebad.is_empty(){"passed"}else{"failed"},"observations":cases.len(),"precondition_mismatches":prebad.len(),"mismatches":bad.len(),"first_precondition_divergences":prebad.iter().take(8).collect::<Vec<_>>(),"first_divergences":bad.iter().take(20).collect::<Vec<_>>()})).unwrap()).unwrap();
    assert!(
        bad.is_empty() && prebad.is_empty(),
        "{} mismatches, {} invalid preconditions",
        bad.len(),
        prebad.len()
    );
}

fn authored_live() -> (Simulation, riding::Configuration) {
    let mut c = fixtures::riding();
    c.motion_version = 3;
    let mut setup = encounter::Setup::authored(&c).unwrap();
    setup.rules.combat_version = 4;
    setup.rules.body.motion_version = 1;
    setup.rules.body.recovery_version = 2;
    setup.seeds[0].distance = 12800;
    setup.seeds[0].speed = 5000;
    (Simulation::with_encounters(1, c.clone(), setup).unwrap(), c)
}
#[test]
fn live_skid_copies_the_body_before_quiet_bike_motion_without_launch_impulses() {
    let (mut sim, mut c) = authored_live();
    c.tables.grip = [0; 4];
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
        b.skid_ticks = 25;
        b.lean = 0x2e00;
        b.contact = c.tables.contact;
    }
    let mut world = sim.encounter.take().unwrap();
    world
        .movement_selected(
            &mut sim.actors,
            &c,
            0,
            &[],
            &mut vec![],
            || 1,
            Some(ActorId(1)),
        )
        .unwrap();
    let body = world.units[&ActorId(1)].recovery.body.as_ref().unwrap();
    assert_eq!(body.location.progress, 12800);
    assert_eq!(body.motion.vertical_speed, 0);
    assert_eq!(body.motion.speed, 5000 - c.tables.skid[0]);
    assert!(
        sim.actors[&ActorId(1)]
            .view
            .rider
            .as_ref()
            .unwrap()
            .bike
            .distance_q8
            > body.motion.distance_q8
    );
}
#[test]
fn live_weapon_depletion_retains_quiet_drive_when_condition_reaches_zero() {
    let (mut sim, c) = authored_live();
    let bike = &mut sim
        .actors
        .get_mut(&ActorId(1))
        .unwrap()
        .view
        .rider
        .as_mut()
        .unwrap()
        .bike;
    bike.condition = 1;
    bike.drive = -32;
    let mut world = sim.encounter.take().unwrap();
    world
        .hit(
            &mut sim.actors,
            &c,
            ActorId(1),
            recovery::Impact {
                reason: 5,
                bike_damage: 1,
                ..Default::default()
            },
            0,
            false,
            &mut vec![],
        )
        .unwrap();
    assert!(world.units[&ActorId(1)].recovery.body.is_some());
    let bike = &sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().bike;
    assert_eq!(bike.condition, 0);
    assert_eq!(bike.drive, -32);
}
#[test]
fn live_contact_entry_keeps_prior_display_yaw_separate_from_physical_heading() {
    let (mut sim, c) = authored_live();
    let yaw = -0x180000;
    sim.actors
        .get_mut(&ActorId(1))
        .unwrap()
        .view
        .rider
        .as_mut()
        .unwrap()
        .bike
        .source_yaw = Some(yaw);
    let mut world = sim.encounter.take().unwrap();
    world
        .hit(
            &mut sim.actors,
            &c,
            ActorId(1),
            recovery::Impact {
                reason: 2,
                rider_velocity: Some([0, 0, 512]),
                bike_velocity: Some([0, 0, 256]),
                ..Default::default()
            },
            0,
            true,
            &mut vec![],
        )
        .unwrap();
    let body = world.units[&ActorId(1)].recovery.body.as_ref().unwrap();
    assert_eq!(body.visual_heading, Some(yaw));
    assert_eq!(body.location.heading, 0);
}
#[test]
fn live_voluntary_action_uses_the_source_quiet_bike_coin_once() {
    let mut c = fixtures::riding();
    c.motion_version = 3;
    c.actions = Some(actions::Config {
        recharge: 17,
        multiplier_q8: 384,
        maximum: 120,
        charges: 3,
        minimum: 30,
        tail: 15,
    });
    let mut setup = encounter::Setup::authored(&c).unwrap();
    setup.rules.combat_version = 4;
    setup.rules.body.recovery_version = 2;
    setup.rules.body.motion_version = 1;
    setup.seeds[0].speed = 299;
    let mut sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
    sim.actors.get_mut(&ActorId(1)).unwrap().input.dismount = true;
    let mut world = sim.encounter.take().unwrap();
    let mut draws = 0;
    world
        .movement_selected(
            &mut sim.actors,
            &c,
            0,
            &[],
            &mut vec![],
            || {
                draws += 1;
                0
            },
            Some(ActorId(1)),
        )
        .unwrap();
    assert!(world.units[&ActorId(1)].recovery.body.is_some());
    assert_eq!(draws, 1);
    assert_eq!(world.units[&ActorId(1)].recovery.bike_animation.code, 47);
}
