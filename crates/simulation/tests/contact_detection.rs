//! Source contact detection is an endpoint predicate, unlike authored wall sweeps.
//! The private differential probe exercises the native and Rust world scans;
//! these authored vectors keep its measured edge policy publicly reproducible.
use rr_simulation::collision::Body;

fn bike(height: i32) -> Body {
    Body {
        position: [2560, 0, 0],
        previous: [2560, 0, 0],
        half: [64, 7680, height],
        velocity: [30000, 0, 0],
        speed: 30000,
        mass: 256,
        category: 4,
        flags: 0,
        impulse: [0; 3],
    }
}

#[test]
fn native_contact_edges_are_inclusive_on_every_axis_for_mounted_and_fallen_bikes() {
    for height in [8960, 2048] {
        let target = bike(height);
        for axis in 0..3 {
            for side in [-1, 1] {
                for edge in [-1, 0, 1] {
                    let mut moving = target;
                    moving.position[axis] += side * (target.half[axis] * 2 + edge);
                    moving.previous[axis] = moving.position[axis] + side * 10000;
                    assert_eq!(moving.overlaps(&target), edge <= 0);
                }
            }
        }
    }
}

#[test]
fn endpoint_detection_does_not_claim_continuous_collision_coverage() {
    let target = bike(8960);
    for axis in [0, 1] {
        let mut moving = target;
        let span = target.half[axis] * 2 + 2;
        moving.previous[axis] -= span;
        moving.position[axis] += span;
        assert!(!moving.overlaps(&target));
        // This controlled path crosses the target in between both missed
        // endpoints. Native/Rust source scans miss it; it is not CCD proof.
        moving.position[axis] = target.position[axis];
        assert!(moving.overlaps(&target));
    }
}

fn source_scenery_setup() -> (
    rr_simulation::riding::Configuration,
    rr_simulation::encounter::Setup,
) {
    use rr_simulation::{collision::Profile, encounter::*, fixtures, topology::Topology};
    let mut config = fixtures::riding();
    config.motion_version = 2;
    config.tables.object[9] = 0;
    config.tables.object[10] = 0;
    config.tables.drag = [0; 4];
    let mut setup = Setup::authored(&config).unwrap();
    setup.topology = Topology::straight(1000000);
    setup.rules.combat_version = 3;
    setup.rules.collision_version = 2;
    setup.rules.collision = Some(Profile {
        mounted: [7680, 64, 8960],
        fallen: [7680, 64, 2048],
        mass: 256,
        elasticity: 192,
        rider: None,
    });
    setup.rules.bike_contacts = false;
    setup.rules.obstacles = vec![Obstacle {
        id: 1,
        path: 0,
        progress: 2688,
        half_length: 1,
        left: -1024,
        right: 1024,
        height: 4096,
        kind: ObstacleKind::Solid,
        source_box: Some(StaticBox {
            bounds: [-1024, 0, 1024, 4096],
            lateral: 0,
            query_bounds: None,
            scenery: None,
        }),
    }];
    setup.seeds[0].speed = 30000;
    (config, setup)
}

#[test]
fn source_scenery_midpoint_window_is_strict_at_all_native_max_speed_tick_phases() {
    use rr_simulation::{Simulation, combat::EffectKind};
    for phase in 0..256 {
        let (config, mut setup) = source_scenery_setup();
        // Zero acceleration at speed 30000 advances exactly117 Q8 units.
        // Tick through the real handling/world pipeline, not a copied predicate.
        setup.seeds[0].distance = 2560 + phase - 117;
        let mut control_setup = setup.clone();
        control_setup.rules.obstacles.clear();
        let mut control = Simulation::with_encounters(1, config.clone(), control_setup).unwrap();
        control.advance().unwrap();
        assert_eq!(
            control.snapshot().actors[0]
                .rider
                .as_ref()
                .unwrap()
                .bike
                .distance_q8,
            2560 + phase
        );
        let mut simulation = Simulation::with_encounters(1, config, setup).unwrap();
        simulation.advance().unwrap();
        let contact = simulation
            .effects()
            .iter()
            .any(|effect| effect.kind == EffectKind::Contact);
        assert_eq!(contact, (65..=191).contains(&phase), "phase {phase}");
    }
}

#[test]
fn source_scenery_lateral_box_edges_remain_inclusive_after_a_max_speed_tick() {
    use rr_simulation::{Simulation, combat::EffectKind};
    for side in [-1, 1] {
        for edge in [-1, 0, 1] {
            let (config, mut setup) = source_scenery_setup();
            setup.seeds[0].distance = 2688 - 117;
            setup.seeds[0].lateral = side * (7680 + 1024 + edge);
            let mut simulation = Simulation::with_encounters(1, config, setup).unwrap();
            simulation.advance().unwrap();
            let contact = simulation
                .effects()
                .iter()
                .any(|effect| effect.kind == EffectKind::Contact);
            assert_eq!(contact, edge <= 0);
        }
    }
}

#[test]
fn legacy_source_box_recordings_retain_their_bytes_and_state_hash() {
    use rr_simulation::replay::{Replay, run};
    use sha2::{Digest, Sha256};
    let (config, setup) = source_scenery_setup();
    let recording = Replay {
        schema_version: 4,
        seed: 11,
        ticks: 180,
        commands: vec![],
        route: None,
        expected_hash: None,
        handling: Some(config),
        encounter: Some(setup),
        population: None,
    };
    let encoded = serde_json::to_vec(&recording).unwrap();
    let bytes_hash = format!("{:x}", Sha256::digest(&encoded));
    for rate in ["30", "60", "144", "irregular"] {
        let replay = Replay::read(&encoded).unwrap();
        assert_eq!(serde_json::to_vec(&replay).unwrap(), encoded);
        let report = run(replay, rate).unwrap();
        assert_eq!(
            bytes_hash,
            "f434690b138adff9fa33e8640b2a7c47acde88a2b00f66d5068c26f9cec5ae3d"
        );
        assert_eq!(
            report.state_sha256,
            "fe031e380ce44adccf91a046c8b572e0e8d184fe0478b5199b9c590436b8cdeb"
        );
    }
}

#[test]
fn elevated_billboards_query_original_boxes_before_lowered_response_geometry() {
    use rr_simulation::{Simulation, combat::EffectKind};
    for lowered_bottom in [8192, 1] {
        for query in [false, true] {
            let (config, mut setup) = source_scenery_setup();
            setup.seeds[0].distance = 2688 - 117;
            let obstacle = &mut setup.rules.obstacles[0];
            obstacle.height = 16384;
            let source = obstacle.source_box.as_mut().unwrap();
            source.bounds = [-1024, lowered_bottom, 1024, 16384];
            source.query_bounds = query.then_some([-1024, 12288, 1024, 16384]);
            let mut simulation = Simulation::with_encounters(1, config, setup).unwrap();
            simulation.advance().unwrap();
            let contact = simulation
                .effects()
                .iter()
                .any(|effect| effect.kind == EffectKind::Contact);
            assert_eq!(
                contact, !query,
                "grounded rider must clear the original elevated box"
            );
        }
    }
}

#[test]
fn source_query_boxes_are_validated_and_only_explicit_geometry_changes_replay_identity() {
    use rr_simulation::Simulation;
    let (config, mut setup) = source_scenery_setup();
    let old = serde_json::to_string(&setup).unwrap();
    assert!(!old.contains("query_bounds"));
    let decoded: rr_simulation::encounter::Setup = serde_json::from_str(&old).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), old);
    let old_hash = Simulation::with_encounters(1, config.clone(), decoded)
        .unwrap()
        .state_hash();
    setup.rules.obstacles[0]
        .source_box
        .as_mut()
        .unwrap()
        .query_bounds = Some([-1024, 0, 1024, 4096]);
    let new_hash = Simulation::with_encounters(1, config.clone(), setup.clone())
        .unwrap()
        .state_hash();
    assert_ne!(
        old_hash, new_hash,
        "query geometry is authoritative replay data"
    );
    for invalid in [
        [1, 0, 1, 100],
        [-1, 5, 1, 5],
        [100, 0, -100, 10],
        [-1, 100, 1, 0],
        [0, 0, 1048577, 1],
        [0, 0, 1, 1048577],
        [256000001, 0, 256000002, 1],
        [i32::MIN, 0, i32::MAX, 1],
    ] {
        setup.rules.obstacles[0]
            .source_box
            .as_mut()
            .unwrap()
            .query_bounds = Some(invalid);
        assert!(Simulation::with_encounters(1, config.clone(), setup.clone()).is_err());
    }
}
