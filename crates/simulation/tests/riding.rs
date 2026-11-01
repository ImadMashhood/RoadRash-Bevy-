use rr_simulation::{animation::*, handling::*, riding::*, *};
fn spec() -> BikeSpec {
    BikeSpec {
        id: 1,
        mass: 100,
        gears: (0..7)
            .map(|i| Gear {
                ratio: 256,
                force: 256 - i * 20,
                up: 1000 + i * 1000,
                down: i * 1000 - 200,
            })
            .collect(),
        throttle: 20,
        throttle_on: 5,
        throttle_off: 12,
        brake: -20,
        brake_on: 5,
        brake_off: 5,
        brake_steer: 128,
        steer_on: 50,
        steer_off: 25,
        steer_scale: 100,
        yaw_gain: 128,
        skid_yaw_gain: 256,
        lean_on: 256,
        lean_off: 128,
        grip_scale: 320,
        skid_threshold: 30,
        landing: -500,
        suspension: 1000,
        rebound: 128,
        gravity: 100,
        launch_cap: 2,
        condition: 100,
    }
}
fn tables() -> Tables {
    let mut object = [0; 26];
    object[7] = 10000;
    object[8] = 8000;
    object[9] = 256;
    object[10] = 1;
    let sine = (0..1024)
        .map(|i| ((i as f64 * std::f64::consts::FRAC_PI_2 / 1024.).sin() * 32768.) as i32)
        .collect();
    Tables {
        contact: 74,
        drag: [0, 2, 20, 30],
        grip: [256, 240, 128, 100],
        gain: [230, 230, 128, 100],
        damp: [6, 5, 1, 1],
        skid: [12, 12, 0, 0],
        curve: [256; 32],
        sine,
        object,
    }
}
fn library() -> Library {
    let frame = Frame {
        render: [0; 12],
        duration: 3,
        control: 1,
        flags: 0,
    };
    let mut lib = Library {
        banks: [
            vec![Some(vec![frame.clone()]); 52],
            vec![Some(vec![frame.clone()]); 52],
        ],
        views: [
            vec![vec![Cell { frame: 0, flags: 0 }]],
            vec![vec![Cell { frame: 0, flags: 0 }]],
        ],
        fallback: vec![2; 52],
        attacks: [
            [[10, 12, 14, 16], [18, 20, 22, 16]],
            [[11, 13, 15, 17], [19, 21, 23, 17]],
        ],
    };
    for bank in &mut lib.banks {
        for sequence in bank.iter_mut().take(24).skip(10) {
            let mut first = frame.clone();
            first.control = 0;
            let mut hit = frame.clone();
            hit.flags = 0x600000;
            hit.control = 2;
            *sequence = Some(vec![first, hit]);
        }
    }
    lib
}
fn config() -> Configuration {
    Configuration {
        specs: vec![spec()],
        selected: 1,
        tables: tables(),
        animation: library(),
        ground: vec![Ground::flat(); 10000],
        start_sample: 0,
    }
}
fn gas() -> Command {
    Command {
        throttle: true,
        ..Default::default()
    }
}
#[test]
fn authored_bike_ramps_shifts_and_settles() {
    let c = config();
    let mut b = Bike::new(c.spec(), &c.tables).unwrap();
    let mut previous = 0;
    let mut shifted = false;
    for _ in 0..2400 {
        b.step(c.spec(), &c.tables, gas(), Ground::flat());
        assert!(b.speed >= previous);
        previous = b.speed;
        shifted |= b.gear > 0;
    }
    assert!(shifted);
    assert!(b.speed > 2000);
    let last = b.speed;
    for _ in 0..1000 {
        b.step(c.spec(), &c.tables, gas(), Ground::flat());
    }
    assert_eq!(last, b.speed);
}
#[test]
fn brake_stops_and_never_reverses() {
    let c = config();
    let mut b = Bike::new(c.spec(), &c.tables).unwrap();
    b.speed = 2000;
    let mut distance = 0;
    for _ in 0..1000 {
        b.step(
            c.spec(),
            &c.tables,
            Command {
                brake: true,
                ..Default::default()
            },
            Ground::flat(),
        );
        assert!(b.speed >= 0);
        distance += b.speed;
    }
    assert_eq!(b.speed, 0);
    assert!(distance > 0 && distance < 2_000_000);
}
#[test]
fn coast_releases_force_and_stops() {
    let c = config();
    let mut b = Bike::new(c.spec(), &c.tables).unwrap();
    for _ in 0..200 {
        b.step(c.spec(), &c.tables, gas(), Ground::flat());
    }
    let peak = b.speed;
    for _ in 0..20000 {
        b.step(c.spec(), &c.tables, Command::default(), Ground::flat());
    }
    assert_eq!((b.speed, b.force), (0, 0));
    assert!(peak > 0);
}
#[test]
fn steering_is_symmetric_at_three_speeds_and_durations() {
    let c = config();
    for speed in [100, 4000, 10000] {
        for duration in [1, 8, 90] {
            let mut l = Bike::new(c.spec(), &c.tables).unwrap();
            l.speed = speed;
            let mut r = l.clone();
            for _ in 0..duration {
                l.lateral = 0;
                r.lateral = 0;
                l.step(
                    c.spec(),
                    &c.tables,
                    Command { steer: -1, ..gas() },
                    Ground::flat(),
                );
                r.step(
                    c.spec(),
                    &c.tables,
                    Command { steer: 1, ..gas() },
                    Ground::flat(),
                );
            }
            assert!((l.speed - r.speed).abs() <= 256);
            assert!((l.lean + r.lean).abs() <= 128);
            assert!((l.lateral + r.lateral).abs() <= duration * 200);
        }
    }
}
#[test]
fn takeoff_and_landing_are_fixed_tick_transitions() {
    let c = config();
    let mut b = Bike::new(c.spec(), &c.tables).unwrap();
    b.speed = 8000;
    b.vertical_speed = 2000;
    let mut flying = false;
    let mut landed = false;
    for _ in 0..200 {
        b.step(c.spec(), &c.tables, gas(), Ground::flat());
        flying |= b.airborne;
        landed |= b.landed;
    }
    assert!(flying && landed);
}
#[test]
fn crest_removes_contact_and_grade_changes_vertical_state() {
    let c = config();
    let mut level = Bike::new(c.spec(), &c.tables).unwrap();
    level.speed = 10000;
    let mut hill = level.clone();
    for _ in 0..20 {
        level.step(c.spec(), &c.tables, gas(), Ground::flat());
        hill.step(
            c.spec(),
            &c.tables,
            gas(),
            Ground {
                rise: 50,
                ..Ground::flat()
            },
        );
    }
    assert_ne!(level.vertical_speed, hill.vertical_speed);
    hill.step(
        c.spec(),
        &c.tables,
        gas(),
        Ground {
            rise: -100,
            ..Ground::flat()
        },
    );
    assert!(hill.airborne);
}
#[test]
fn surface_boundaries_and_overrides_have_known_answers() {
    let g = Ground::flat();
    for (x, want) in [
        (-751, 3),
        (-750, 3),
        (-749, 1),
        (-500, 1),
        (-499, 0),
        (0, 0),
        (499, 0),
        (500, 1),
        (749, 1),
        (750, 3),
    ] {
        assert_eq!(g.surface(x * 256), want, "x={x}");
    }
    assert_eq!(Ground { kind: 1, ..g }.surface(600 * 256), 2);
    assert_eq!(Ground { flags: 0x20, ..g }.surface(1000 * 256), 0);
    assert_eq!(
        Ground {
            flags: 0x18,
            kind: 1,
            ..g
        }
        .surface(600 * 256),
        2
    );
}
#[test]
fn offroad_surface_has_more_drag() {
    let c = config();
    let mut road = Bike::new(c.spec(), &c.tables).unwrap();
    road.speed = 20000;
    let mut terrain = road.clone();
    terrain.lateral = 1000 * 256;
    road.step(c.spec(), &c.tables, Command::default(), Ground::flat());
    terrain.step(c.spec(), &c.tables, Command::default(), Ground::flat());
    assert!(terrain.speed < road.speed);
}
#[test]
fn damage_equality_failure_and_repetition_are_idempotent() {
    let c = config();
    let mut b = Bike::new(c.spec(), &c.tables).unwrap();
    b.damage(99);
    assert_eq!(b.condition, 1);
    assert!(!b.failed);
    b.damage(1);
    assert!(b.failed);
    let failed = b.clone();
    b.damage(u32::MAX);
    assert_eq!(failed, b);
    for _ in 0..30 {
        b.step(c.spec(), &c.tables, gas(), Ground::flat());
    }
    assert_eq!(b.speed, 0);
}
#[test]
fn missing_spec_fields_and_bad_gears_fail_before_race() {
    let c = config();
    let mut v = serde_json::to_value(c.spec()).unwrap();
    v.as_object_mut().unwrap().remove("mass");
    assert!(serde_json::from_value::<BikeSpec>(v).is_err());
    let mut s = spec();
    s.gears.clear();
    assert!(s.validate().is_err());
    let mut c = config();
    c.selected = 2;
    assert!(c.validate().is_err());
    let mut c = config();
    c.tables.drag[1] = i32::MAX;
    assert!(c.validate().is_err());
}
#[test]
fn source_animation_loops_match_all_render_schedules() {
    use rr_simulation::replay::*;
    let mut baseline = None;
    for rate in ["30", "60", "144", "irregular"] {
        let r = Replay {
            schema_version: 3,
            seed: 3,
            ticks: 600,
            commands: vec![QueuedCommand {
                tick: Tick(0),
                actor: ActorId(1),
                sequence: 0,
                input: gas(),
            }],
            route: None,
            expected_hash: None,
            handling: Some(config()),
            encounter: None,
            population: None,
        };
        let report = run(r, rate).unwrap();
        if let Some(b) = &baseline {
            assert_eq!(&report.tick_hashes, b);
        } else {
            baseline = Some(report.tick_hashes);
        }
        assert!(report.final_snapshot.actors[0].rider.is_some());
    }
}
#[test]
fn timing_selects_before_advancing_and_contact_matches_frame_flags() {
    let lib = library();
    let mut p = Player::default();
    assert!(p.set(&lib, 0, 10));
    for tick in 0..4 {
        let f = p.advance(&lib, 0, tick).unwrap().unwrap();
        assert_eq!(f.flags, 0);
    }
    let f = p.advance(&lib, 0, 4).unwrap().unwrap();
    assert_eq!(f.flags & 0x600000, 0x600000);
    assert_eq!(p.selected, Some((10, 1)));
}
#[test]
fn missing_animation_states_are_reported_before_start() {
    let mut lib = library();
    lib.banks[0][36] = None;
    lib.fallback[36] = 51;
    lib.banks[0][51] = None;
    assert!(lib.validate().is_err());
    let mut lib = library();
    lib.banks[0][2].as_mut().unwrap()[0].duration = 0;
    assert!(lib.validate().is_err());
}
#[test]
fn unavailable_selection_preserves_player_and_uses_one_fallback() {
    let mut lib = library();
    let mut p = Player::default();
    p.set(&lib, 0, 2);
    let old = p.clone();
    assert!(!p.set(&lib, 0, 127));
    assert_eq!(p, old);
    lib.banks[0][10] = None;
    assert!(p.set(&lib, 0, 10));
    assert_eq!(p.code, 2);
}
#[test]
fn source_mirroring_is_explicit_not_implicit_left_right() {
    let mut lib = library();
    let f = lib.banks[0][2].as_ref().unwrap()[0].clone();
    assert_eq!(lib.select(0, &f, -10000, 0).unwrap().0.flags, 0);
    lib.views[0][0][0].flags = 1;
    assert_eq!(lib.select(0, &f, 10000, 0).unwrap().0.flags, 1);
}
#[test]
fn weapon_changes_and_drops_switch_attack_descriptors() {
    let c = config();
    let mut r = Rider::new(&c).unwrap();
    r.step(
        &c,
        Command {
            pose: Some(Pose::PunchLeft),
            weapon: Some(Weapon::Club),
            ..gas()
        },
        0,
    )
    .unwrap();
    assert_eq!(r.animation.code, 12);
    r.step(
        &c,
        Command {
            pose: Some(Pose::PunchLeft),
            weapon: Some(Weapon::Fists),
            ..gas()
        },
        1,
    )
    .unwrap();
    assert_eq!(r.animation.code, 10);
    assert_eq!(r.weapon, Weapon::Fists);
}
#[test]
fn state_transition_resets_timing_and_preserves_world_anchor() {
    let c = config();
    let mut r = Rider::new(&c).unwrap();
    r.step(
        &c,
        Command {
            pose: Some(Pose::PunchLeft),
            ..Default::default()
        },
        0,
    )
    .unwrap();
    r.step(
        &c,
        Command {
            pose: Some(Pose::Crash),
            ..Default::default()
        },
        1,
    )
    .unwrap();
    assert_eq!(r.animation.code, 33);
    assert_eq!(r.animation.frame, 0);
    assert_eq!(r.bike.lateral, 0);
    r.step(
        &c,
        Command {
            pose: Some(Pose::GetUp),
            ..Default::default()
        },
        2,
    )
    .unwrap();
    r.step(
        &c,
        Command {
            pose: Some(Pose::Walk),
            ..Default::default()
        },
        3,
    )
    .unwrap();
    assert_eq!(r.animation.code, 38);
    assert_eq!(r.bike.lateral, 0);
    r.step(
        &c,
        Command {
            pose: Some(Pose::Remount),
            ..Default::default()
        },
        4,
    )
    .unwrap();
    for tick in 5..15 {
        r.step(&c, Command::default(), tick).unwrap();
    }
    assert!(!matches!(r.pose, Pose::Remount));
    assert_ne!(r.animation.code, 41);
}
#[test]
fn configuration_and_hidden_state_change_schema_three_hash() {
    let mut a = Simulation::with_handling(1, config()).unwrap();
    let mut c = config();
    c.specs[0].mass += 1;
    let b = Simulation::with_handling(1, c).unwrap();
    assert_ne!(a.state_hash(), b.state_hash());
    let old = a.state_hash();
    a.step(Command {
        weapon: Some(Weapon::Chain),
        ..Default::default()
    })
    .unwrap();
    assert_ne!(old, a.state_hash());
    assert_eq!(&a.canonical_bytes()[..4], &3u32.to_le_bytes());
}
#[test]
fn old_schema_rejects_new_commands() {
    let mut sim = Simulation::default();
    assert!(
        sim.step(Command {
            pose: Some(Pose::PunchLeft),
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        sim.step(Command {
            damage: 1,
            ..Default::default()
        })
        .is_err()
    );
}
#[test]
fn damage_events_are_consumed_once() {
    let mut sim = Simulation::with_handling(1, config()).unwrap();
    sim.queue(QueuedCommand {
        tick: Tick(0),
        actor: ActorId(1),
        sequence: 0,
        input: Command {
            damage: 1,
            ..Default::default()
        },
    })
    .unwrap();
    for _ in 0..30 {
        sim.advance().unwrap();
    }
    assert_eq!(
        sim.snapshot().actors[0]
            .rider
            .as_ref()
            .unwrap()
            .bike
            .condition,
        99
    );
}
#[test]
fn versioned_hand_calculated_acceleration_and_braking_vectors_match() {
    let v: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../docs/fixtures/riding-regression.json"
    ))
    .unwrap();
    let c = config();
    let mut b = Bike::new(c.spec(), &c.tables).unwrap();
    let mut actual = vec![];
    for _ in 0..5 {
        b.step(c.spec(), &c.tables, gas(), Ground::flat());
        actual.push(b.speed);
    }
    assert_eq!(serde_json::json!(actual), v["throttle_speed_first_five"]);
    let mut b = Bike::new(c.spec(), &c.tables).unwrap();
    b.speed = 2000;
    let mut actual = vec![];
    for _ in 0..5 {
        b.step(
            c.spec(),
            &c.tables,
            Command {
                brake: true,
                ..Default::default()
            },
            Ground::flat(),
        );
        actual.push(b.speed);
    }
    assert_eq!(serde_json::json!(actual), v["brake_speed_first_five"]);
}
#[test]
fn crash_visuals_distinguish_airborne_slide_and_stopped_recovery() {
    assert_eq!(crash_pose(12000, 4096), Pose::Crash);
    assert_eq!(crash_pose(12000, 4095), Pose::Slide);
    assert_eq!(crash_pose(2500, 0), Pose::Slide);
    assert_eq!(crash_pose(2499, 0), Pose::GetUp);
    assert_eq!(crash_pose(0, 0), Pose::GetUp);
    let c = config();
    let mut rider = Rider::new(&c).unwrap();
    rider.bike.crashed = true;
    rider.bike.speed = 10000;
    rider.step(&c, Command::default(), 0).unwrap();
    assert_eq!(rider.pose, Pose::Slide);
    let mut rider = Rider::new(&c).unwrap();
    rider.bike.crashed = true;
    rider.step(&c, Command::default(), 0).unwrap();
    assert_eq!(rider.pose, Pose::GetUp);
    assert_eq!(rider.animation.queued, Some(37));
}
#[test]
fn swept_grade_and_bend_split_exactly_at_sample_boundaries() {
    let mut c = config();
    c.ground[0].rise = 10;
    c.ground[1].rise = -20;
    c.ground[0].bend = 3;
    c.ground[1].bend = -4;
    let g = c.swept_ground(250, 266);
    assert_eq!(g.movement, 6 * 10 - 10 * 20);
    assert_eq!(g.bend_movement, -(6 * 3 - 10 * 4));
    assert_eq!(g.rise, -20);
}
#[test]
fn road_curvature_pushes_laterally_and_recovery_reaches_standing() {
    let c = config();
    let mut straight = Bike::new(c.spec(), &c.tables).unwrap();
    straight.speed = 8000;
    let mut bend = straight.clone();
    straight.step(c.spec(), &c.tables, gas(), Ground::flat());
    bend.step(
        c.spec(),
        &c.tables,
        gas(),
        Ground {
            bend: 4,
            ..Ground::flat()
        },
    );
    assert!(bend.lateral < straight.lateral);
    let mut r = Rider::new(&c).unwrap();
    r.bike.crashed = true;
    for tick in 0..10 {
        r.step(&c, Command::default(), tick).unwrap();
    }
    assert_eq!(r.pose, Pose::Stand);
}
