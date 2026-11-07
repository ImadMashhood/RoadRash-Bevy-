use rr_simulation::{
    collision::Profile, combat::EffectKind, encounter::*, scenery::BoxRules, topology::Topology, *,
};
fn setup(class: u8) -> (riding::Configuration, Setup) {
    let mut c = fixtures::riding();
    c.motion_version = 2;
    c.tables.object[9] = 0;
    c.tables.object[10] = 0;
    c.tables.drag = [0; 4];
    for g in &mut c.ground {
        g.left = -2000;
        g.right = 2000;
        g.left_shoulder = -2200;
        g.right_shoulder = 2200;
    }
    let mut s = Setup::authored(&c).unwrap();
    s.topology = Topology::straight(1000000);
    s.seeds[0].distance = 2688;
    s.rules.combat_version = 3;
    s.rules.collision_version = 2;
    s.rules.scenery_version = 1;
    s.rules.scenery_start_ready = true;
    s.rules.bike_contacts = false;
    s.rules.collision = Some(Profile {
        mounted: [7680, 64, 8960],
        fallen: [7680, 64, 2048],
        mass: 256,
        elasticity: 192,
        rider: None,
    });
    s.rules.obstacles = vec![Obstacle {
        id: 1,
        path: 0,
        progress: 2688,
        half_length: 1,
        left: -1024,
        right: 1024,
        height: 4096,
        kind: if (3..16).contains(&class) {
            ObstacleKind::Decorative
        } else {
            ObstacleKind::Solid
        },
        source_box: Some(StaticBox {
            bounds: [-1024, 0, 1024, 4096],
            lateral: 0,
            query_bounds: None,
            scenery: Some(BoxRules {
                class,
                road_mask: 3,
                sample: 10,
                selector: 7,
                ordinal: 0,
                box_count: 1,
                texture_flags: Some(0),
            }),
        }),
    }];
    (c, s)
}
fn contact(s: &Simulation) -> bool {
    s.effects().iter().any(|e| e.kind == EffectKind::Contact)
}
#[test]
fn loading_is_one_tick_delayed_unless_the_recording_starts_after_countdown() {
    for ready in [false, true] {
        let (c, mut s) = setup(16);
        s.rules.scenery_start_ready = ready;
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.advance().unwrap();
        assert_eq!(contact(&sim), ready);
        if !ready {
            sim.advance().unwrap();
            assert!(contact(&sim));
        }
    }
}
#[test]
fn borr_widening_uses_frame_relative_extents_and_respects_flags_and_box_count() {
    for flags in [0, 0x4000] {
        for count in [1, 2] {
            let (c, mut s) = setup(16);
            let anchor = 1024 * 256;
            s.seeds[0].lateral = anchor + 7680 + 2500;
            let o = &mut s.rules.obstacles[0];
            o.left += anchor;
            o.right += anchor;
            let b = o.source_box.as_mut().unwrap();
            b.lateral = anchor;
            b.bounds[0] += anchor;
            b.bounds[2] += anchor;
            let meta = b.scenery.as_mut().unwrap();
            meta.box_count = count;
            meta.texture_flags = Some(flags);
            if count == 2 {
                let mut extra = o.clone();
                extra.id = 2;
                let b = extra.source_box.as_mut().unwrap();
                b.bounds[0] = anchor - 6144;
                b.bounds[2] = anchor - 4096;
                b.scenery.as_mut().unwrap().ordinal = 1;
                s.rules.obstacles.push(extra);
            }
            let mut sim = Simulation::with_encounters(1, c, s).unwrap();
            sim.advance().unwrap();
            assert_eq!(contact(&sim), flags == 0 && count == 1);
        }
    }
}
#[test]
fn borr_queries_precede_billboards_even_when_obstacles_are_serialized_in_reverse_order() {
    let (c, mut s) = setup(16);
    let mut marking = s.rules.obstacles[0].clone();
    marking.id = 2;
    marking
        .source_box
        .as_mut()
        .unwrap()
        .scenery
        .as_mut()
        .unwrap()
        .class = 3;
    s.rules.obstacles.insert(0, marking);
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.advance().unwrap();
    assert!(contact(&sim));
    let snap = sim.snapshot();
    let bike = &snap.actors[0].rider.as_ref().unwrap().bike;
    assert!(bike.scenery_mark.is_none());
    assert_eq!(bike.scenery_timer, 0);
}
#[test]
fn marking_queries_are_not_solid_impacts_and_latch_only_above_the_native_speed_threshold() {
    for speed in [1000, 1001] {
        let (c, mut s) = setup(3);
        s.seeds[0].speed = speed;
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.advance().unwrap();
        let snap = sim.snapshot();
        let bike = &snap.actors[0].rider.as_ref().unwrap().bike;
        assert_eq!(bike.scenery_timer, 60);
        assert_eq!(bike.scenery_mark.is_some(), speed > 1000);
        assert!(!bike.crashed);
        assert!(!contact(&sim));
    }
    let (c, mut s) = setup(5);
    s.seeds[0].speed = 5000;
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.step(Command {
        throttle: true,
        steer: 1,
        ..Default::default()
    })
    .unwrap();
    let snap = sim.snapshot();
    let bike = &snap.actors[0].rider.as_ref().unwrap().bike;
    assert_eq!(bike.scenery_timer, 180);
    assert_eq!(bike.lateral_speed, 0);
    assert!(
        bike.drive > 0,
        "markings clear lateral motion, not throttle force"
    );
}
#[test]
fn marking_frames_expire_without_forgetting_the_same_sample_latch() {
    let mut bike = handling::Bike {
        scenery_mark: Some(scenery::Mark {
            sample: 10,
            selector: 7,
            tick: 100,
            frame: 1,
            display_frame: 1,
        }),
        ..Default::default()
    };
    for (age, frame) in [
        (0, 1),
        (3, 1),
        (4, 2),
        (7, 2),
        (8, 3),
        (11, 3),
        (12, 0),
        (20, 0),
    ] {
        bike.refresh_scenery(10, 100 + age);
        let mark = bike.scenery_mark.as_ref().unwrap();
        assert_eq!(mark.frame, frame);
        assert_eq!(mark.display_frame, frame);
        assert_eq!(mark.sample, 10);
    }
    bike.refresh_scenery(10, 101);
    assert_eq!(
        bike.scenery_mark.as_ref().unwrap().frame,
        0,
        "expired flashes must not spontaneously restart"
    );
}
#[test]
fn physical_scenery_loading_never_crosses_a_disconnected_road() {
    let (c, mut s) = setup(16);
    s.topology.paths.insert(1, s.topology.paths[&0].clone());
    s.rules.obstacles[0].path = 1;
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    for _ in 0..3 {
        sim.advance().unwrap();
        assert!(!contact(&sim));
    }
    assert!(
        sim.encounters()
            .unwrap()
            .scenery
            .as_ref()
            .unwrap()
            .loaded
            .is_empty()
    );
}
#[test]
fn source_cache_and_markings_replay_identically_with_all_frame_schedules() {
    use replay::{Playback, Replay, frame_ns};
    let (c, mut s) = setup(3);
    s.seeds[0].speed = 5000;
    let replay = Replay {
        schema_version: 4,
        seed: 3,
        ticks: 180,
        commands: vec![],
        route: None,
        expected_hash: None,
        handling: Some(c),
        encounter: Some(s),
        population: None,
    };
    let bytes = serde_json::to_vec(&replay).unwrap();
    let mut expected = None;
    for rate in ["30", "60", "144", "irregular"] {
        let mut p = Playback::new(Replay::read(&bytes).unwrap()).unwrap();
        while !p.done() {
            p.frame(frame_ns(rate, p.frames).unwrap()).unwrap();
        }
        let actual = (p.simulation.state_hash(), p.checkpoints.clone());
        if let Some(expected) = &expected {
            assert_eq!(&actual, expected);
        } else {
            expected = Some(actual);
        }
        let state = serde_json::to_string(p.simulation.encounters().unwrap()).unwrap();
        let decoded: World = serde_json::from_str(&state).unwrap();
        assert_eq!(serde_json::to_string(&decoded).unwrap(), state);
    }
}
#[test]
fn scenery_metadata_and_version_are_validated() {
    let (c, base) = setup(16);
    for version in [0, 2] {
        let mut s = base.clone();
        s.rules.scenery_version = version;
        assert!(Simulation::with_encounters(1, c.clone(), s).is_err());
    }
    for (class, mask, count, sample) in [
        (17, 1, 1, 10),
        (16, 0, 1, 10),
        (16, 4, 1, 10),
        (16, 1, 0, 10),
        (16, 1, 4097, 10),
        (16, 1, 1, 1000001),
    ] {
        let mut s = base.clone();
        let m = s.rules.obstacles[0]
            .source_box
            .as_mut()
            .unwrap()
            .scenery
            .as_mut()
            .unwrap();
        m.class = class;
        m.road_mask = mask;
        m.box_count = count;
        m.sample = sample;
        assert!(Simulation::with_encounters(1, c.clone(), s).is_err());
    }
    let mut s = base;
    s.rules.obstacles[0].source_box.as_mut().unwrap().scenery = None;
    assert!(Simulation::with_encounters(1, c, s).is_err());
}
