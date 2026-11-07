use rr_simulation::{
    collision::Profile as CollisionProfile,
    encounter::*,
    source_roads::{Fork, Profile, Roads},
    topology::{Path, Portal, Topology},
    *,
};
use std::collections::BTreeMap;
fn fixture(lateral: i32) -> (riding::Configuration, Setup) {
    let mut c = fixtures::riding();
    c.motion_version = 3;
    c.tables.object[11] = c.tables.contact;
    c.tables.object[9] = 0;
    c.tables.object[10] = 0;
    c.tables.drag = [0; 4];
    let ground = handling::Ground {
        left: -100,
        right: 100,
        left_shoulder: -150,
        right_shoulder: 150,
        ..Default::default()
    };
    let mut paths = BTreeMap::new();
    for (id, len, branch, start) in [
        (0, 10, 0, Some(0)),
        (1, 100, 1, Some(2560)),
        (2, 120, 2, None),
        (3, 30, 0, Some(28160)),
    ] {
        paths.insert(
            id,
            Path {
                length: len * 256,
                branch,
                selected_start: start,
                portals: vec![],
                rises: vec![0; len as usize],
            },
        );
    }
    for (a, b) in [(0, 1), (0, 2), (1, 3), (2, 3)] {
        paths.get_mut(&a).unwrap().portals.push(Portal {
            end: true,
            target: b,
            target_end: false,
        });
        paths.get_mut(&b).unwrap().portals.push(Portal {
            end: false,
            target: a,
            target_end: true,
        });
    }
    let mut profiles = vec![];
    for half in 0..2 {
        let branch = 1 + half;
        let start = if half == 0 { 110 } else { 130 };
        let selected = BTreeMap::from([(0, 0), (branch, 10), (3, start)]);
        let mut identities = BTreeMap::new();
        for (&path, &global) in &selected {
            let len = paths[&path].length / 256;
            let offset = match path {
                1 => -100,
                2 => 100,
                _ => 0,
            };
            let mask = if path == 1 {
                1
            } else if path == 2 {
                2
            } else {
                3
            };
            identities.insert(
                path,
                (0..len)
                    .map(|local| [local as i32, global as i32 + local as i32, offset, mask])
                    .collect(),
            );
        }
        profiles.push(Profile {
            finish_progress: None,
            choices: vec![half as u8],
            selected,
            identities,
        });
    }
    let roads = Roads {
        version: 1,
        initial: 0,
        lanes_q8: None,
        ground: paths
            .iter()
            .map(|(id, p)| (*id, vec![ground; p.length as usize / 256]))
            .collect(),
        forks: BTreeMap::from([(
            0,
            Fork {
                bit: 0,
                halves: [1, 2],
            },
        )]),
        profiles,
    };
    c.ground = roads.ground(0).unwrap();
    let mut s = Setup::authored(&c).unwrap();
    s.topology = Topology { paths };
    s.rules.source_roads = Some(roads);
    s.rules.contact_walk_version = 1;
    s.rules.collision_version = 2;
    s.rules.combat_version = 3;
    s.rules.scenery_version = 1;
    s.rules.scenery_start_ready = true;
    s.rules.collision = Some(CollisionProfile {
        mounted: [10, 4, 10],
        fallen: [10, 4, 4],
        mass: 256,
        elasticity: 192,
        rider: None,
    });
    s.rules.obstacles.clear();
    s.rules.finish_progress = Some(1_000_000);
    s.seeds[0].distance = 2558;
    s.seeds[0].speed = 512;
    s.seeds[0].lateral = lateral;
    (c, s)
}
#[test]
fn ordinary_motion_steers_at_inclusive_native_split_edge() {
    for (lateral, path, mask) in [(-1, 1, 0), (0, 2, 1), (1, 2, 1)] {
        let (c, s) = fixture(lateral);
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.advance().unwrap();
        let u = &sim.encounters().unwrap().units[&ActorId(1)];
        assert_eq!(
            (
                u.recovery.bike_location.path,
                u.recovery.bike_location.progress,
                u.source_route
            ),
            (path, 0, Some(mask))
        );
        assert_eq!(
            sim.snapshot().actors[0]
                .rider
                .as_ref()
                .unwrap()
                .bike
                .distance_q8,
            2560
        );
    }
}
#[test]
fn unequal_branch_merge_keeps_accumulated_distance_and_physical_owner() {
    let (c, mut s) = fixture(0);
    s.seeds[0].speed = 30000;
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    let mut previous = 2558;
    let mut merged = false;
    for _ in 0..400 {
        sim.advance().unwrap();
        let u = &sim.encounters().unwrap().units[&ActorId(1)];
        let now = sim.snapshot().actors[0]
            .rider
            .as_ref()
            .unwrap()
            .bike
            .distance_q8;
        assert!((0..=118).contains(&(now - previous)));
        previous = now;
        if u.recovery.bike_location.path == 3 {
            assert_eq!(now, 130 * 256 + u.recovery.bike_location.progress);
            assert_eq!(u.source_route, Some(1));
            merged = true;
            break;
        }
    }
    assert!(merged);
}
#[test]
fn metadata_route_order_edges_and_derived_config_are_bounded() {
    let (c, s) = fixture(0);
    for kind in 0..5 {
        let mut invalid = s.clone();
        let roads = invalid.rules.source_roads.as_mut().unwrap();
        match kind {
            0 => roads.version = 2,
            1 => roads.profiles.swap(0, 1),
            2 => roads.profiles[0].identities.get_mut(&1).unwrap()[0][2] = 1_000_001,
            3 => roads.profiles[0]
                .selected
                .insert(1, 11)
                .map(|_| ())
                .unwrap(),
            _ => roads.forks.get_mut(&0).unwrap().halves = [1, 99],
        };
        assert!(Simulation::with_encounters(1, c.clone(), invalid).is_err());
    }
    let old = Setup::authored(&c).unwrap();
    let bytes = serde_json::to_string(&old).unwrap();
    assert!(!bytes.contains("source_roads"));
    assert!(!bytes.contains("contact_walk_version"));
}
#[test]
fn steered_route_replay_matches_every_schedule_and_effect_setting() {
    use replay::{Playback, Replay, frame_ns};
    let (c, mut s) = fixture(0);
    s.seeds[0].speed = 30000;
    let replay = Replay {
        schema_version: 4,
        seed: 3,
        ticks: 360,
        commands: vec![],
        route: None,
        expected_hash: None,
        handling: Some(c),
        encounter: Some(s),
        population: None,
    };
    let bytes = serde_json::to_vec(&replay).unwrap();
    let mut expected = None;
    for effects in [false, true] {
        for rate in ["30", "60", "144", "irregular"] {
            let mut p = Playback::new(Replay::read(&bytes).unwrap()).unwrap();
            p.simulation.set_effects_enabled(effects);
            while !p.done() {
                p.frame(frame_ns(rate, p.frames).unwrap()).unwrap();
            }
            let actual = (p.simulation.state_hash(), p.checkpoints.clone());
            if let Some(expected) = &expected {
                assert_eq!(&actual, expected);
            } else {
                expected = Some(actual);
            }
        }
    }
}

#[test]
fn held_steering_during_last_sample_does_not_commit_a_fork_early() {
    for (lateral, steer, branch) in [(-50, 1, 2), (50, -1, 1)] {
        let (c, mut s) = fixture(lateral);
        s.seeds[0].distance = 2500;
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        let mut crossed = false;
        for _ in 0..40 {
            sim.step(Command {
                steer,
                ..Default::default()
            })
            .unwrap();
            let u = &sim.encounters().unwrap().units[&ActorId(1)];
            if u.recovery.bike_location.path == 0 {
                assert_eq!(
                    u.source_route,
                    Some(0),
                    "choice changed before the actual boundary crossing"
                );
            } else {
                assert_eq!(u.recovery.bike_location.path, branch);
                crossed = true;
                break;
            }
        }
        assert!(crossed);
    }
}
#[test]
fn a_sliding_body_and_its_bike_keep_independent_forks_and_replay_contexts() {
    use replay::{Playback, Replay, frame_ns};
    let (c, mut s) = fixture(100);
    s.seeds[0].distance = 2358;
    s.seeds[0].location = Some(topology::Location {
        path: 0,
        progress: 2358,
        lateral: 100,
        heading: -0x100000,
        ..Default::default()
    });
    s.seeds[0].speed = 5000;
    s.rules.body.motion_version = 1;
    s.rules.body.recovery_version = 1;
    s.rules.body.drag = 0;
    s.rules.body.contact_drag = 0;
    let impact = Command {
        impact: Some(recovery::Impact {
            reason: 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut sim = Simulation::with_encounters(1, c.clone(), s.clone()).unwrap();
    sim.step(impact).unwrap();
    for _ in 0..20 {
        sim.advance().unwrap();
    }
    let u = &sim.encounters().unwrap().units[&ActorId(1)];
    let body = u.recovery.body.as_ref().unwrap();
    assert_eq!(
        (u.recovery.bike_location.path, u.source_route),
        (2, Some(1))
    );
    assert_eq!((body.location.path, body.source_route), (1, Some(0)));
    assert_eq!(body.owner, ActorId(1));
    assert_eq!(body.entity, 3);
    let replay = Replay {
        schema_version: 4,
        seed: 1,
        ticks: 180,
        commands: vec![QueuedCommand {
            tick: Tick(0),
            actor: ActorId(1),
            sequence: 0,
            input: impact,
        }],
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
    }
}
#[test]
fn different_road_halves_do_not_contact_at_the_split() {
    let (c, mut s) = fixture(-1);
    let mut other = s.seeds[0].clone();
    other.id = ActorId(2);
    other.lateral = 1;
    s.seeds.push(other);
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.advance().unwrap();
    let units = &sim.encounters().unwrap().units;
    assert_eq!(units[&ActorId(1)].recovery.bike_location.path, 1);
    assert_eq!(units[&ActorId(2)].recovery.bike_location.path, 2);
    assert!(
        !sim.effects()
            .iter()
            .any(|e| e.kind == combat::EffectKind::Contact)
    );
}

#[test]
fn shared_merge_contacts_use_physical_progress_after_unequal_routes() {
    let (c, mut s) = fixture(0);
    s.seeds[0].distance = 110 * 256 + 100;
    s.seeds[0].source_route = Some(0);
    s.seeds[0].speed = 0;
    let mut other = s.seeds[0].clone();
    other.id = ActorId(2);
    other.source_route = Some(1);
    other.distance = 130 * 256 + 100;
    s.seeds.push(other);
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.advance().unwrap();
    let u = &sim.encounters().unwrap().units;
    assert_eq!(u[&ActorId(1)].recovery.bike_location.path, 3);
    assert_eq!(u[&ActorId(2)].recovery.bike_location.path, 3);
    assert!(
        sim.effects()
            .iter()
            .any(|e| e.kind == combat::EffectKind::Contact && e.other == Some(ActorId(2)))
    );
    let snapshot = sim.snapshot();
    assert_eq!(
        snapshot.actors[1].rider.as_ref().unwrap().bike.distance_q8
            - snapshot.actors[0].rider.as_ref().unwrap().bike.distance_q8,
        20 * 256
    );
}

#[test]
fn disconnected_physical_roads_do_not_share_contacts_or_resident_scenery() {
    let (c, mut s) = fixture(0);
    let length = 100 * 256;
    s.topology.paths.insert(
        99,
        Path {
            length,
            branch: 99,
            selected_start: None,
            portals: vec![],
            rises: vec![0; 100],
        },
    );
    let roads = s.rules.source_roads.as_mut().unwrap();
    roads.ground.insert(99, vec![handling::Ground::flat(); 100]);
    roads.profiles[1].selected = BTreeMap::from([(99, 0)]);
    roads.profiles[1].identities =
        BTreeMap::from([(99, (0..100).map(|local| [local, local, 0, 3]).collect())]);
    s.seeds[0].distance = 100;
    s.seeds[0].speed = 0;
    let mut other = s.seeds[0].clone();
    other.id = ActorId(2);
    other.source_route = Some(1);
    s.seeds.push(other);
    s.rules.obstacles.push(Obstacle {
        id: 1,
        path: 99,
        progress: 128,
        half_length: 1,
        left: -1024,
        right: 1024,
        height: 4096,
        kind: ObstacleKind::Solid,
        source_box: Some(StaticBox {
            bounds: [-1024, 0, 1024, 4096],
            lateral: 0,
            query_bounds: None,
            scenery: Some(scenery::BoxRules {
                class: 0,
                road_mask: 3,
                sample: 0,
                selector: 0,
                ordinal: 0,
                box_count: 1,
                texture_flags: None,
            }),
        }),
    });
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.advance().unwrap();
    assert!(
        !sim.effects()
            .iter()
            .any(|e| e.kind == combat::EffectKind::Contact)
    );
    assert!(
        !sim.encounters()
            .unwrap()
            .scenery
            .as_ref()
            .unwrap()
            .loaded
            .iter()
            .any(|(path, _)| *path == 99)
    );
}

#[test]
fn new_motion_uses_the_behavior_table_contact_word_without_changing_old_profiles() {
    for version in [0, 1, 2, 3] {
        let mut c = fixtures::riding();
        c.motion_version = version;
        c.tables.object[11] = 37;
        let rider = riding::Rider::new(&c).unwrap();
        assert_eq!(
            rider.bike.contact,
            if version == 3 { 37 } else { c.tables.contact }
        );
    }
}
#[test]
fn current_grounded_tail_latches_skid_after_a_soft_return_below_suspension() {
    for version in [2, 3] {
        let mut c = fixtures::riding();
        c.motion_version = version;
        c.tables.object[11] = c.tables.contact;
        let mut rider = riding::Rider::new(&c).unwrap();
        rider.bike.height = c.spec().suspension + 1;
        rider.bike.vertical_speed = -2;
        rider.bike.gravity = 0;
        rider.step(&c, Command::default(), 0).unwrap();
        assert_eq!(rider.bike.height, 0);
        assert_eq!(rider.bike.contact, c.tables.contact);
        assert_eq!(rider.bike.skid, u8::from(version == 3));
    }
}

#[test]
fn finish_threshold_follows_the_physical_tail_after_each_unequal_branch() {
    for mask in [0u8, 1] {
        for gap in [-1, 0, 1] {
            let (c, mut s) = fixture(0);
            let initial = 120 * 256;
            let own = if mask == 0 { 120 } else { 140 } * 256;
            s.rules.finish_progress = Some(initial);
            for (index, profile) in s
                .rules
                .source_roads
                .as_mut()
                .unwrap()
                .profiles
                .iter_mut()
                .enumerate()
            {
                profile.finish_progress = Some(if index == 0 { 120 } else { 140 } * 256);
            }
            s.seeds[0].source_route = Some(mask);
            s.seeds[0].distance = own + gap;
            s.seeds[0].speed = 0;
            let mut sim = Simulation::with_encounters(1, c, s).unwrap();
            sim.advance().unwrap();
            let u = &sim.encounters().unwrap().units[&ActorId(1)];
            assert_eq!(
                u.recovery.outcome == Some(recovery::Outcome::Finished),
                gap >= 0
            );
            assert_eq!(u.race_progress, Some(initial + gap));
        }
    }
}
#[test]
fn off_launch_branch_vehicle_contacts_use_physical_owner_coordinates() {
    use population::{Kind, Setup as Population, Spawn};
    let (c, mut s) = fixture(100 * 256);
    s.seeds[0].distance = 10 * 256 + 128;
    s.seeds[0].source_route = Some(1);
    s.seeds[0].speed = 0;
    let mut population = Population::default();
    population.spawns.push(Spawn {
        id: 1,
        location: topology::Location {
            path: 2,
            progress: 128,
            lateral: 100 * 256,
            ..Default::default()
        },
        kind: Kind::Vehicle,
        family: None,
        selector: 0,
        subtype: 0,
        speed: 0,
        direction: 1,
    });
    let mut sim = Simulation::with_population(1, c, s, population).unwrap();
    sim.advance().unwrap();
    assert!(
        sim.effects()
            .iter()
            .any(|e| e.kind == combat::EffectKind::Contact)
    );
    let car = sim.population().unwrap().agents.values().next().unwrap();
    assert_eq!(car.location.path, 2);
    assert!((0..=120 * 256).contains(&car.location.progress));
    assert_eq!(
        sim.encounters().unwrap().units[&ActorId(1)].source_route,
        Some(1)
    );
}
#[test]
fn merged_vehicle_contact_does_not_depend_on_the_riders_elapsed_branch_length() {
    use population::{Kind, Setup as Population, Spawn};
    let mut outputs = vec![];
    for mask in [0u8, 1] {
        let (c, mut s) = fixture(0);
        s.seeds[0].source_route = Some(mask);
        s.seeds[0].distance = if mask == 0 { 110 } else { 130 } * 256 + 128;
        s.seeds[0].speed = 0;
        let mut population = Population::default();
        population.spawns.push(Spawn {
            id: 1,
            location: topology::Location {
                path: 3,
                progress: 128,
                ..Default::default()
            },
            kind: Kind::Vehicle,
            family: None,
            selector: 0,
            subtype: 0,
            speed: 0,
            direction: 1,
        });
        let mut sim = Simulation::with_population(1, c, s, population).unwrap();
        sim.advance().unwrap();
        let car = sim.population().unwrap().agents.values().next().unwrap();
        let snapshot = sim.snapshot();
        let bike = &snapshot.actors[0].rider.as_ref().unwrap().bike;
        outputs.push((
            car.location,
            sim.encounters().unwrap().units[&ActorId(1)]
                .recovery
                .bike_location,
            bike.lateral,
            bike.height,
            bike.speed,
            sim.effects().to_vec(),
        ));
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn collision_displacement_selects_the_physical_fork_instead_of_the_loaded_route() {
    let (c, mut s) = fixture(1);
    let profile = s.rules.collision.as_mut().unwrap();
    profile.mounted = [7680, 64, 8960];
    profile.fallen = [7680, 64, 3840];
    s.rules.dimensions = profile.mounted;
    s.seeds[0].distance = 2403;
    s.seeds[0].speed = 30000;
    let mut leader = Seed::player();
    leader.id = ActorId(2);
    leader.distance = 2555;
    leader.lateral = 1;
    s.seeds.push(leader);
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.advance().unwrap();
    let leader = &sim.encounters().unwrap().units[&ActorId(2)];
    assert_eq!(leader.recovery.bike_location.path, 2);
    assert_eq!(leader.source_route, Some(1));
    assert!(leader.recovery.bike_location.progress > 0);
    assert_eq!(
        sim.encounters()
            .unwrap()
            .scenery
            .as_ref()
            .unwrap()
            .object_roads[&4],
        1
    );
}

#[test]
fn dense_four_rider_merge_callbacks_are_invariant_to_elapsed_branch_axes() {
    let mut observations = vec![];
    for mixed_masks in [false, true] {
        let (c, mut setup) = fixture(0);
        let seed = setup.seeds[0].clone();
        setup.seeds.clear();
        let mut masks = vec![];
        for (index, (progress, lateral, speed)) in
            [(100, 0, 256), (104, 2, 512), (108, -2, 128), (112, 0, 384)]
                .into_iter()
                .enumerate()
        {
            let mask = u8::from(mixed_masks && index % 2 == 1);
            let mut actor = seed.clone();
            actor.id = ActorId(index as u64 + 1);
            actor.source_route = Some(mask);
            actor.distance = (if mask == 0 { 110 } else { 130 }) * 256 + progress;
            actor.lateral = lateral;
            actor.speed = speed;
            setup.seeds.push(actor);
            masks.push(mask);
        }
        let mut sim = Simulation::with_encounters(1, c, setup).unwrap();
        sim.advance().unwrap();
        assert!(
            sim.effects()
                .iter()
                .filter(|event| event.kind == combat::EffectKind::Contact)
                .count()
                >= 3
        );
        let snapshot = sim.snapshot();
        let bikes: Vec<_> = snapshot
            .actors
            .iter()
            .zip(masks)
            .map(|(actor, mask)| {
                let mut bike = actor.rider.as_ref().unwrap().bike.clone();
                bike.distance_q8 -= (if mask == 0 { 110 } else { 130 }) * 256;
                (
                    bike,
                    actor.encounter.as_ref().unwrap().recovery.bike_location,
                )
            })
            .collect();
        observations.push((bikes, sim.effects().to_vec()));
    }
    assert_eq!(observations[0], observations[1]);
}

#[test]
fn completed_source_snapshot_marks_the_prepared_camera_pullback() {
    let (config, setup) = fixture(0);
    let mut simulation = Simulation::with_encounters(1, config, setup).unwrap();
    simulation
        .step(Command {
            terminal: Some(recovery::Outcome::Arrested),
            ..Default::default()
        })
        .unwrap();
    let hash = simulation.state_hash();
    let snapshot = simulation.snapshot();
    let follow = snapshot.camera_follow.as_ref().unwrap();
    let state = simulation.encounters().unwrap().scenery.as_ref().unwrap();
    assert_eq!(follow.back, 3);
    assert!(follow.back_applied);
    assert_eq!(Some(follow.location), state.pending_camera);
    assert_eq!(simulation.state_hash(), hash);
}

#[test]
fn terminal_camera_pullback_crosses_the_merge_on_the_players_retained_arm() {
    for mask in [0u8, 1] {
        let (config, mut setup) = fixture(0);
        setup.rules.combat_version = 4;
        setup.rules.contact_walk_version = 2;
        setup.rules.body.recovery_version = 2;
        setup.seeds[0].source_route = Some(mask);
        setup.seeds[0].distance = if mask == 0 { 110 } else { 130 } * 256 + 128;
        setup.seeds[0].speed = 0;
        let mut simulation = Simulation::with_encounters(1, config, setup).unwrap();
        simulation
            .step(Command {
                terminal: Some(recovery::Outcome::Arrested),
                ..Default::default()
            })
            .unwrap();
        for _ in 0..299 {
            simulation.advance().unwrap();
        }
        let snapshot = simulation.snapshot();
        let follow = snapshot.camera_follow.unwrap();
        assert_eq!(follow.back, 900);
        assert_eq!(follow.location.path, u32::from(mask) + 1);
        let length = if mask == 0 { 100 } else { 120 } * 256;
        assert_eq!(follow.location.progress, length + 128 - 900);
    }
}
