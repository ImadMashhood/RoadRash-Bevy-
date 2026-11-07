use rr_simulation::{collision::Profile, encounter::*, recovery::Phase, topology::Topology, *};
fn setup() -> (riding::Configuration, Setup) {
    let mut c = fixtures::riding();
    c.specs[0].contact_steer_gain = 32;
    let mut s = Setup::authored(&c).unwrap();
    s.topology = Topology::straight(1000000);
    s.rules.combat_version = 3;
    s.rules.collision_version = 2;
    s.rules.collision = Some(Profile {
        mounted: [7680, 64, 8960],
        fallen: [7680, 64, 3840],
        mass: 256,
        elasticity: 192,
        rider: None,
    });
    s.seeds[0].distance = 100;
    s.seeds[0].speed = 20000;
    (c, s)
}
#[test]
fn source_rear_end_and_glancing_contacts_have_different_outcomes() {
    for lateral in [0, 10000] {
        let (c, mut s) = setup();
        let mut other = Seed::player();
        other.id = ActorId(2);
        other.distance = 240;
        other.speed = 5000;
        other.lateral = lateral;
        s.seeds.push(other);
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.advance().unwrap();
        let unit = &sim.encounters().unwrap().units[&ActorId(1)];
        assert_eq!(unit.recovery.phase == Phase::Sliding, lateral == 0);
        assert_eq!(
            unit.fighter.stats.health, 2500,
            "bike impacts do not take stamina"
        );
        if lateral == 0 {
            assert!(unit.recovery.body.is_some());
        } else {
            assert!(
                !sim.snapshot().actors[0]
                    .rider
                    .as_ref()
                    .unwrap()
                    .bike
                    .crashed
            );
        }
    }
}
#[test]
fn a_side_brush_preserves_forward_motion_but_a_frontal_wall_blocks_it() {
    for frontal in [false, true] {
        let (c, mut s) = setup();
        s.seeds[0].distance = 1000;
        s.rules.obstacles.push(Obstacle {
            id: 1,
            path: 0,
            progress: 1100,
            half_length: 32,
            left: if frontal { -10000 } else { 1000 },
            right: 20000,
            height: 100000,
            kind: ObstacleKind::Building,
            source_box: None,
        });
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.advance().unwrap();
        let snapshot = sim.snapshot();
        let bike = &snapshot.actors[0].rider.as_ref().unwrap().bike;
        assert_eq!(bike.speed == 0, frontal);
        assert_eq!(bike.crashed, frontal);
    }
}
#[test]
fn physical_constants_are_validated_and_legacy_serialization_remains_unchanged() {
    let c = fixtures::riding();
    let mut s = Setup::authored(&c).unwrap();
    assert!(!serde_json::to_string(&s).unwrap().contains("collision"));
    assert!(
        !serde_json::to_string(c.spec())
            .unwrap()
            .contains("contact_steer_gain")
    );
    s.rules.collision_version = 2;
    assert!(Simulation::with_encounters(1, c.clone(), s.clone()).is_err());
    let (_, mut s) = setup();
    s.rules.collision.as_mut().unwrap().mass = 0;
    assert!(Simulation::with_encounters(1, c, s).is_err());
}

#[test]
fn source_recovery_remounts_without_colliding_with_its_own_stopped_bike() {
    let (mut c, mut s) = setup();
    c.motion_version = 2;
    s.rules.body.motion_version = 1;
    s.rules.body.recovery_version = 1;
    s.seeds[0].speed = 0;
    let mut sim = Simulation::with_encounters(4, c, s).unwrap();
    sim.step(Command {
        impact: Some(recovery::Impact {
            reason: 4,
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    let identity = sim.encounters().unwrap().units[&ActorId(1)]
        .recovery
        .body
        .as_ref()
        .unwrap()
        .entity;
    let mut remounts = 0;
    for _ in 0..1500 {
        sim.advance().unwrap();
        let unit = &sim.encounters().unwrap().units[&ActorId(1)];
        assert_eq!(unit.fighter.stats.health, 2500);
        assert!(unit.recovery.outcome.is_none());
        if let Some(body) = &unit.recovery.body {
            assert_eq!(body.entity, identity);
        }
        for effect in sim.effects() {
            assert_ne!(
                effect.kind,
                combat::EffectKind::Contact,
                "own bike created an incident"
            );
            remounts += usize::from(effect.kind == combat::EffectKind::Remount);
        }
        if unit.recovery.phase == Phase::Mounted {
            break;
        }
    }
    assert_eq!(
        sim.encounters().unwrap().units[&ActorId(1)].recovery.phase,
        Phase::Mounted
    );
    assert_eq!(remounts, 1);
    assert!(
        sim.snapshot().actors[0]
            .rider
            .as_ref()
            .unwrap()
            .bike
            .contact_velocity
            .is_some()
    );
}

#[test]
fn source_contact_and_recovery_replays_match_at_all_schedules_with_effects_disabled() {
    use replay::{Playback, Replay, frame_ns};
    let (mut c, mut s) = setup();
    c.motion_version = 2;
    s.rules.body.motion_version = 1;
    s.rules.body.recovery_version = 1;
    let mut other = Seed::player();
    other.id = ActorId(2);
    other.distance = 240;
    other.speed = 5000;
    s.seeds.push(other);
    let recording = Replay {
        schema_version: 4,
        seed: 4,
        ticks: 1600,
        commands: vec![],
        route: None,
        expected_hash: None,
        handling: Some(c),
        encounter: Some(s),
        population: None,
    };
    let encoded = serde_json::to_vec(&recording).unwrap();
    let mut expected = None;
    for rate in ["30", "60", "144", "irregular"] {
        for effects in [false, true] {
            let mut p = Playback::new(Replay::read(&encoded).unwrap()).unwrap();
            p.simulation.set_effects_enabled(effects);
            while !p.done() {
                p.frame(frame_ns(rate, p.frames).unwrap()).unwrap();
            }
            let result = (p.simulation.state_hash(), p.checkpoints.clone());
            if let Some(expected) = &expected {
                assert_eq!(&result, expected);
            } else {
                expected = Some(result);
            }
        }
    }
}

#[test]
fn disconnected_roads_do_not_collide_even_with_identical_coordinates_and_branch_tags() {
    let (c, mut s) = setup();
    let path = s.topology.paths[&0].clone();
    s.topology.paths.insert(1, path);
    s.seeds[0].location = Some(topology::Location {
        path: 0,
        progress: 100,
        ..Default::default()
    });
    let mut other = Seed::player();
    other.id = ActorId(2);
    other.distance = 100;
    other.speed = 5000;
    other.location = Some(topology::Location {
        path: 1,
        progress: 100,
        ..Default::default()
    });
    s.seeds.push(other);
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.advance().unwrap();
    for unit in sim.encounters().unwrap().units.values() {
        assert_eq!(unit.recovery.phase, Phase::Mounted);
    }
    assert!(
        !sim.effects()
            .iter()
            .any(|e| e.kind == combat::EffectKind::Contact)
    );
}

#[test]
fn a_chained_swing_keeps_the_animation_clock_instead_of_restarting_windup() {
    let (c, mut s) = setup();
    s.rules.bike_contacts = false;
    s.seeds[0].speed = 0;
    s.seeds[0].stats.strength = 1;
    let mut target = Seed::player();
    target.id = ActorId(2);
    target.distance = s.seeds[0].distance;
    target.lateral = 18000;
    s.seeds.push(target);
    let mut sim = Simulation::with_encounters(3, c, s).unwrap();
    sim.step(Command {
        attack: Some(combat::Request {
            kind: combat::AttackKind::Punch,
            side: combat::Side::Right,
        }),
        ..Default::default()
    })
    .unwrap();
    for _ in 0..6 {
        sim.advance().unwrap();
    }
    let snapshot = sim.snapshot();
    let attacker = snapshot.actors[0].rider.as_ref().unwrap();
    assert_eq!((attacker.animation.code, attacker.animation.frame), (11, 0));
    assert_eq!(
        sim.encounters().unwrap().units[&ActorId(2)]
            .fighter
            .hit_count,
        1
    );
    for _ in 0..3 {
        sim.advance().unwrap();
    }
    assert_eq!(
        sim.snapshot().actors[0]
            .rider
            .as_ref()
            .unwrap()
            .animation
            .frame,
        1
    );
}
