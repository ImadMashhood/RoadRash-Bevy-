use rr_simulation::{
    collision::Profile as CollisionProfile,
    encounter::*,
    source_roads::{Fork, Profile, Roads},
    topology::{Path, Portal, Topology},
    *,
};
use std::collections::BTreeMap;
fn graph_fixture(lateral: i32) -> (riding::Configuration, Setup) {
    let mut c = fixtures::riding();
    c.motion_version = 3;
    c.specs[0].brake = -200;
    c.tables.object[11] = c.tables.contact;
    c.tables.object[9] = 0;
    c.tables.object[10] = 0;
    c.tables.drag = [0; 4];
    let ground = handling::Ground {
        left: -500,
        right: 500,
        left_shoulder: -750,
        right_shoulder: 750,
        ..Default::default()
    };
    let mut paths = BTreeMap::new();
    for (id, len, branch, start) in [
        (0, 10, 0, Some(0)),
        (1, 100, 1, Some(2560)),
        (2, 120, 2, None),
        (3, 300, 0, Some(28160)),
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
                1 => -500,
                2 => 500,
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
        let other = if branch == 1 { 2 } else { 1 };
        let other_start = 10;
        let other_offset = if other == 1 { -500 } else { 500 };
        identities.insert(
            other,
            (0..paths[&other].length / 256)
                .map(|local| {
                    [
                        local as i32,
                        other_start + local as i32,
                        other_offset,
                        if other == 1 { 1 } else { 2 },
                    ]
                })
                .collect(),
        );
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
        lanes_q8: Some(
            paths
                .iter()
                .map(|(id, p)| (*id, vec![[512; 2]; p.length as usize / 256]))
                .collect(),
        ),
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
    s.rules.combat_version = 4;
    s.rules.body.motion_version = 1;
    s.rules.body.recovery_version = 2;
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

fn population(c: &riding::Configuration) -> population::Setup {
    let mut bike = c.tables.object;
    bike[1] = 4;
    let mut body = bike;
    body[1] = 8;
    let mut words = vec![0; 129];
    words[0x164 / 4] = 9 << 8;
    words[0x1d0 / 4] = 0x0202;
    words[0x1a4 / 4] = 120;
    for i in 0..5 {
        words[0x1bc / 4 + i] = 120;
        words[0x1a8 / 4 + i] = 80;
    }
    words[0x1d4 / 4] = 4;
    let personalities = (0..10)
        .map(|index| {
            let mut w = words.clone();
            w[0x164 / 4] = index << 8;
            w
        })
        .collect();
    let mut stats = [0; 16];
    stats[3] = 171;
    stats[6..11].fill(1);
    stats[12] = 6;
    stats[13] = 2200;
    population::Setup {
        police: Some(population::police::Config {
            bike_template: bike,
            rider_template: body,
            atan: vec![0; 257],
            personality: words,
            personalities,
            overtake_eagerness: [16; 5],
            stats,
            bikes: [0; 5],
            throttle_percent: [116; 5],
            messages: [0; 8],
        }),
        traffic: false,
        periodic_police: false,
        ..Default::default()
    }
}
fn independent_owner() -> Simulation {
    let (mut c, mut setup) = graph_fixture(100);
    c.tables.object[9] = 0;
    c.tables.object[10] = 0;
    setup.seeds[0].distance = 2358;
    setup.seeds[0].location = Some(topology::Location {
        path: 0,
        progress: 2358,
        lateral: 100,
        heading: -0x100000,
        ..Default::default()
    });
    setup.seeds[0].speed = 5000;
    setup.rules.body.motion_version = 1;
    setup.rules.body.recovery_version = 2;
    setup.rules.body.drag = 0;
    setup.rules.body.contact_drag = 0;
    let pop = population(&c);
    let mut sim = Simulation::with_population(1, c, setup, pop).unwrap();
    sim.step(Command {
        impact: Some(recovery::Impact {
            reason: 1,
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    for _ in 0..20 {
        sim.advance().unwrap();
    }
    let player = &sim.encounters().unwrap().units[&ActorId(1)];
    assert_eq!(
        (player.recovery.bike_location.path, player.source_route),
        (2, Some(1))
    );
    let body = player.recovery.body.as_ref().unwrap();
    assert_eq!((body.location.path, body.source_route), (1, Some(0)));
    sim
}
#[test]
fn cop_spawn_inherits_the_chased_body_route_and_half_instead_of_the_quiet_bike() {
    let mut sim = independent_owner();
    let player = &sim.encounters().unwrap().units[&ActorId(1)];
    let body = player.recovery.body.as_ref().unwrap();
    let mut location = body.location;
    location.progress += 512;
    let body_half = sim
        .encounters()
        .unwrap()
        .scenery
        .as_ref()
        .unwrap()
        .object_roads[&3];
    let bike_half = sim
        .encounters()
        .unwrap()
        .scenery
        .as_ref()
        .unwrap()
        .object_roads[&2];
    assert_ne!(body_half, bike_half);
    sim.spawn_road_user(&population::Spawn {
        id: 100,
        location,
        kind: population::Kind::Police,
        family: None,
        selector: 0,
        subtype: 0,
        speed: 0,
        direction: 1,
    })
    .unwrap()
    .unwrap();
    sim.advance().unwrap();
    let live = sim
        .population()
        .unwrap()
        .police
        .as_ref()
        .expect("A cop on the chased body's physical arm must instantiate");
    let unit = &sim.encounters().unwrap().units[&live.actor];
    assert_eq!(unit.source_route, Some(0));
    assert_eq!(unit.recovery.bike_location.path, 1);
    assert_eq!(
        sim.encounters()
            .unwrap()
            .scenery
            .as_ref()
            .unwrap()
            .object_roads[&(live.actor.0 * 2)],
        body_half
    );
}

fn cop_spawn(location: topology::Location) -> population::Spawn {
    population::Spawn {
        id: 100,
        location,
        kind: population::Kind::Police,
        family: None,
        selector: 0,
        subtype: 0,
        speed: 0,
        direction: 1,
    }
}
#[test]
fn an_interior_zero_local_coordinate_does_not_unload_a_live_cop() {
    let (c, mut setup) = graph_fixture(-250 * 256);
    setup.seeds[0].distance = 10 * 256 + 2000;
    setup.seeds[0].speed = 5000;
    let pop = population(&c);
    let mut sim = Simulation::with_population(1, c, setup, pop).unwrap();
    sim.spawn_road_user(&cop_spawn(topology::Location {
        path: 1,
        progress: 0,
        ..Default::default()
    }))
    .unwrap()
    .unwrap();
    sim.advance().unwrap();
    let live = sim.population().unwrap().police.as_ref().unwrap();
    assert!(
        !live.removal_pending,
        "Interior section entry is a valid physical slice, not an unload boundary"
    );
    sim.advance().unwrap();
    assert!(sim.population().unwrap().police.is_some());
}
#[test]
fn pursuit_joins_an_unequal_shared_tail_without_comparing_elapsed_route_axes() {
    let (c, mut setup) = graph_fixture(585 * 256);
    setup.seeds[0].distance = 1000;
    setup.seeds[0].speed = 5000;
    let mut pop = population(&c);
    pop.disengage = 120 * 256;
    let mut sim = Simulation::with_population(1, c, setup, pop).unwrap();
    sim.spawn_road_user(&cop_spawn(topology::Location {
        path: 1,
        progress: 512,
        ..Default::default()
    }))
    .unwrap()
    .unwrap();
    let (mut saw_cop_arm, mut saw_both_tail, mut saw_near_follow) = (false, false, false);
    for _ in 0..5000 {
        sim.advance().unwrap();
        let Some(live) = sim.population().unwrap().police.as_ref() else {
            continue;
        };
        let units = &sim.encounters().unwrap().units;
        let cop = &units[&live.actor];
        let pc = &units[&ActorId(1)];
        if cop.recovery.bike_location.path == 1 {
            assert_eq!(cop.source_route, Some(0));
            saw_cop_arm = true;
        }
        if cop.recovery.bike_location.path == 3
            && pc.recovery.bike_location.path == 3
            && pc.source_route == Some(1)
        {
            let snap = sim.snapshot();
            let cb = &snap
                .actors
                .iter()
                .find(|a| a.id == live.actor)
                .unwrap()
                .rider
                .as_ref()
                .unwrap()
                .bike;
            let pb = &snap
                .actors
                .iter()
                .find(|a| a.id == ActorId(1))
                .unwrap()
                .rider
                .as_ref()
                .unwrap()
                .bike;
            let physical = pc.recovery.bike_location.progress - cop.recovery.bike_location.progress;
            assert_eq!((pb.distance_q8 - cb.distance_q8) - physical, 20 * 256);
            saw_both_tail = true;
            let ai = cop.police_ai.as_ref().unwrap();
            if physical.abs() < 256 && ai.words[0x160 / 4] == 4 && ai.target == Some(ActorId(1)) {
                // One physical sample apart despite twenty samples of elapsed-axis difference.
                // Native follow matches the nearby player speed; using raw elapsed positions
                // here would instead select the distant-rider acceleration/deceleration gate.
                assert!((40..=55).contains(&ai.words[0x19c / 4]));
                saw_near_follow = true;
                break;
            }
        }
    }
    assert!(
        saw_cop_arm && saw_both_tail && saw_near_follow,
        "Cop must follow its own short arm, then meet the long-arm player on the shared physical tail: {saw_cop_arm}/{saw_both_tail}/{saw_near_follow}"
    );
}

#[test]
fn a_far_cop_unloads_on_a_connected_road_and_releases_its_owned_state() {
    let (c, mut setup) = graph_fixture(-250 * 256);
    setup.seeds[0].distance = 10 * 256 + 95 * 256;
    setup.seeds[0].speed = 5000;
    let pop = population(&c);
    let mut sim = Simulation::with_population(1, c, setup, pop).unwrap();
    sim.spawn_road_user(&cop_spawn(topology::Location {
        path: 1,
        progress: 1000,
        ..Default::default()
    }))
    .unwrap()
    .unwrap();
    sim.advance().unwrap();
    let live = sim.population().unwrap().police.as_ref().unwrap().clone();
    assert!(
        live.removal_pending,
        "A connected path does not make an out-of-window cop resident forever"
    );
    sim.advance().unwrap();
    assert!(sim.population().unwrap().police.is_none());
    assert!(!sim.encounters().unwrap().units.contains_key(&live.actor));
    assert!(!sim.population().unwrap().agents.contains_key(&live.agent));
    assert!(
        !sim.population()
            .unwrap()
            .navigation
            .contains_key(&live.actor)
    );
    let e = sim.encounters().unwrap();
    assert!(
        !e.inventory
            .items
            .values()
            .any(|item| item.owner == Some(live.actor))
    );
    assert!(
        !e.scenery
            .as_ref()
            .unwrap()
            .object_roads
            .contains_key(&(live.actor.0 * 2))
    );
    assert!(
        !e.scenery
            .as_ref()
            .unwrap()
            .object_roads
            .contains_key(&(live.actor.0 * 2 + 1))
    );
}
#[test]
fn disconnected_spawns_are_rejected_and_disconnected_riders_do_not_become_targets() {
    let (c, mut setup) = graph_fixture(-250 * 256);
    setup.seeds[0].distance = 10 * 256 + 2000;
    setup.seeds[0].speed = 5000;
    setup.topology.paths.insert(
        99,
        Path {
            length: 100 * 256,
            branch: 99,
            selected_start: None,
            portals: vec![],
            rises: vec![0; 100],
        },
    );
    let roads = setup.rules.source_roads.as_mut().unwrap();
    roads.ground.insert(99, vec![handling::Ground::flat(); 100]);
    roads
        .lanes_q8
        .as_mut()
        .unwrap()
        .insert(99, vec![[512; 2]; 100]);
    roads.profiles[1].selected = BTreeMap::from([(99, 0)]);
    roads.profiles[1].identities =
        BTreeMap::from([(99, (0..100).map(|local| [local, local, 0, 3]).collect())]);
    let mut other = setup.seeds[0].clone();
    other.id = ActorId(2);
    other.source_route = Some(1);
    other.distance = 1280;
    other.speed = 0;
    setup.seeds.push(other);
    let pop = population(&c);
    let mut sim = Simulation::with_population(1, c, setup, pop).unwrap();
    sim.spawn_road_user(&cop_spawn(topology::Location {
        path: 99,
        progress: 1280,
        ..Default::default()
    }))
    .unwrap()
    .unwrap();
    sim.advance().unwrap();
    assert!(sim.population().unwrap().police.is_none());
    sim.spawn_road_user(&cop_spawn(topology::Location {
        path: 1,
        progress: 1000,
        ..Default::default()
    }))
    .unwrap()
    .unwrap();
    for _ in 0..80 {
        sim.advance().unwrap();
        let live = sim.population().unwrap().police.as_ref().unwrap();
        let ai = sim.encounters().unwrap().units[&live.actor]
            .police_ai
            .as_ref()
            .unwrap();
        assert_ne!(ai.target, Some(ActorId(2)));
    }
}
#[test]
fn a_cop_crashes_recovers_and_keeps_its_physical_route_and_half() {
    let (c, mut setup) = graph_fixture(250 * 256);
    setup.seeds[0].source_route = Some(1);
    setup.seeds[0].distance = 10 * 256 + 6000;
    setup.seeds[0].speed = 5000;
    let mut pop = population(&c);
    pop.disengage = 120 * 256;
    let mut sim = Simulation::with_population(1, c, setup, pop).unwrap();
    sim.spawn_road_user(&cop_spawn(topology::Location {
        path: 2,
        progress: 4000,
        ..Default::default()
    }))
    .unwrap()
    .unwrap();
    sim.advance().unwrap();
    let id = sim.population().unwrap().police.as_ref().unwrap().actor;
    let half = sim
        .encounters()
        .unwrap()
        .scenery
        .as_ref()
        .unwrap()
        .object_roads[&(id.0 * 2)];
    sim.queue(QueuedCommand {
        tick: sim.tick(),
        actor: id,
        sequence: 0,
        input: Command {
            impact: Some(recovery::Impact {
                reason: 4,
                rider_velocity: Some([0; 3]),
                bike_velocity: Some([0; 3]),
                ..Default::default()
            }),
            ..Default::default()
        },
    })
    .unwrap();
    sim.advance().unwrap();
    assert_ne!(
        sim.encounters().unwrap().units[&id].recovery.phase,
        recovery::Phase::Mounted
    );
    let body = sim.encounters().unwrap().units[&id]
        .recovery
        .body
        .as_ref()
        .unwrap();
    assert_eq!(body.source_route, Some(1));
    // The dormant body half is prepared at the next contact walk.
    sim.advance().unwrap();
    assert_eq!(
        sim.encounters()
            .unwrap()
            .scenery
            .as_ref()
            .unwrap()
            .object_roads[&(id.0 * 2 + 1)],
        half
    );
    let mut remounted = false;
    for _ in 0..600 {
        sim.advance().unwrap();
        let Some(unit) = sim.encounters().unwrap().units.get(&id) else {
            break;
        };
        assert_eq!(unit.source_route, Some(1));
        if unit.recovery.phase == recovery::Phase::Mounted {
            assert!(unit.recovery.body.is_none());
            assert!(
                !sim.encounters()
                    .unwrap()
                    .scenery
                    .as_ref()
                    .unwrap()
                    .object_roads
                    .contains_key(&(id.0 * 2 + 1))
            );
            remounted = true;
            break;
        }
    }
    assert!(
        remounted,
        "A cop must complete its ordinary authored recovery on the steered physical arm"
    );
}
#[test]
fn independent_body_route_police_replay_matches_all_frame_schedules() {
    use replay::{Playback, Replay, frame_ns};
    let (c, mut setup) = graph_fixture(100);
    setup.seeds[0].distance = 2358;
    setup.seeds[0].location = Some(topology::Location {
        path: 0,
        progress: 2358,
        lateral: 100,
        heading: -0x100000,
        ..Default::default()
    });
    setup.seeds[0].speed = 5000;
    setup.rules.body.drag = 0;
    setup.rules.body.contact_drag = 0;
    let mut pop = population(&c);
    pop.spawns.push(cop_spawn(topology::Location {
        path: 1,
        progress: 512,
        ..Default::default()
    }));
    let replay = Replay {
        schema_version: 5,
        seed: 1,
        ticks: 600,
        commands: vec![QueuedCommand {
            tick: Tick(0),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                impact: Some(recovery::Impact {
                    reason: 1,
                    ..Default::default()
                }),
                ..Default::default()
            },
        }],
        route: None,
        expected_hash: None,
        handling: Some(c),
        encounter: Some(setup),
        population: Some(pop),
    };
    let bytes = serde_json::to_vec(&replay).unwrap();
    let mut expected = None;
    for rate in ["30", "60", "144", "irregular"] {
        let mut play = Playback::new(Replay::read(&bytes).unwrap()).unwrap();
        while !play.done() {
            play.frame(frame_ns(rate, play.frames).unwrap()).unwrap();
        }
        let actual = (play.simulation.state_hash(), play.checkpoints.clone());
        if let Some(expected) = &expected {
            assert_eq!(&actual, expected);
        } else {
            expected = Some(actual);
        }
        let e = play.simulation.encounters().unwrap();
        let player = &e.units[&ActorId(1)];
        assert_ne!(
            player.source_route,
            player.recovery.body.as_ref().unwrap().source_route
        );
        assert!(play.simulation.population().unwrap().police.is_some());
    }
}

#[test]
fn a_cop_on_the_players_selected_arm_can_complete_an_arrest() {
    let (c, mut setup) = graph_fixture(250 * 256);
    setup.seeds[0].source_route = Some(1);
    setup.seeds[0].distance = 10 * 256 + 2000;
    setup.seeds[0].speed = 0;
    let pop = population(&c);
    let mut sim = Simulation::with_population(1, c, setup, pop).unwrap();
    sim.spawn_road_user(&cop_spawn(topology::Location {
        path: 2,
        progress: 2000,
        ..Default::default()
    }))
    .unwrap()
    .unwrap();
    for _ in 0..3 {
        sim.advance().unwrap();
    }
    assert_eq!(
        sim.encounters().unwrap().units[&ActorId(1)]
            .recovery
            .outcome,
        Some(recovery::Outcome::Arrested)
    );
    let live = sim.population().unwrap().police.as_ref().unwrap();
    let cop = &sim.encounters().unwrap().units[&live.actor];
    assert_eq!(cop.source_route, Some(1));
    assert_eq!(cop.recovery.bike_location.path, 2);
}
