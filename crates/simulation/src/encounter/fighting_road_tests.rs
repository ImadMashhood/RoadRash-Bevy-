//! Functional physical-frame combat invariants. The native oracle supplies the
//! straight-road kernel; this authored graph is not a native TrackRing replay.
use super::*;
use crate::{
    source_roads::{Fork, Profile as RoadProfile, Roads},
    topology::{Path, Portal},
};
fn graph() -> (Configuration, Simulation) {
    let mut c = fixtures::riding();
    c.motion_version = 3;
    c.tables.object[11] = c.tables.contact;
    c.tables.object[9] = 0;
    c.tables.object[10] = 0;
    c.tables.drag = [0; 4];
    for spec in &mut c.specs {
        spec.steer_scale = 1;
    }
    let ground = crate::handling::Ground {
        left: -1000,
        right: 1000,
        left_shoulder: -1500,
        right_shoulder: 1500,
        ..Default::default()
    };
    let mut paths = BTreeMap::new();
    for (id, samples, branch, start) in [
        (0, 10, 0, Some(0)),
        (1, 100, 1, Some(2560)),
        (2, 120, 2, None),
        (3, 30, 0, Some(28160)),
    ] {
        paths.insert(
            id,
            Path {
                length: samples * 256,
                branch,
                selected_start: start,
                portals: vec![],
                rises: vec![0; samples as usize],
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
        let selected = BTreeMap::from([
            (0, 0),
            (1 + half, 10),
            (3, if half == 0 { 110 } else { 130 }),
        ]);
        let mut identities = BTreeMap::new();
        for (&path, &start) in &selected {
            let offset = if path == 1 {
                -100
            } else if path == 2 {
                100
            } else {
                0
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
                (0..paths[&path].length / 256)
                    .map(|local| [local as i32, start as i32 + local as i32, offset, mask])
                    .collect(),
            );
        }
        profiles.push(RoadProfile {
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
    let mut setup = Setup::authored(&c).unwrap();
    setup.topology = Topology { paths };
    setup.rules.source_roads = Some(roads);
    setup.rules.combat_version = 4;
    setup.rules.contact_walk_version = 1;
    setup.rules.collision_version = 2;
    setup.rules.scenery_version = 1;
    setup.rules.collision = Some(crate::collision::Profile {
        mounted: [10, 4, 10],
        fallen: [10, 4, 4],
        mass: 256,
        elasticity: 192,
        rider: None,
    });
    setup.rules.finish_progress = Some(1000000);
    setup.seeds.clear();
    for id in 1..=3 {
        let mut s = Seed::player();
        s.id = ActorId(id);
        s.distance = 1000;
        s.stats.strength = 1;
        setup.seeds.push(s);
    }
    (c.clone(), Simulation::with_encounters(1, c, setup).unwrap())
}
fn place(sim: &mut Simulation, id: ActorId, profile: u8, path: u32, progress: i64, lateral: i32) {
    let world = sim.encounter.as_mut().unwrap();
    let roads = world.setup.rules.source_roads.as_ref().unwrap();
    let global = i64::from(roads.profiles[profile as usize].selected[&path]) * 256 + progress;
    let unit = world.units.get_mut(&id).unwrap();
    unit.source_route = Some(profile);
    unit.recovery.bike_location = Location {
        path,
        progress,
        lateral,
        ..Default::default()
    };
    unit.previous_location = unit.recovery.bike_location;
    let bike = &mut sim
        .actors
        .get_mut(&id)
        .unwrap()
        .view
        .rider
        .as_mut()
        .unwrap()
        .bike;
    bike.distance_q8 = global;
    bike.lateral = lateral;
    bike.height = 0;
    bike.speed = 0;
}
fn straight(c: &Configuration, positions: [i64; 3]) -> Simulation {
    let mut setup = Setup::authored(c).unwrap();
    setup.rules.combat_version = 4;
    setup.rules.collision_version = 2;
    setup.rules.collision = Some(crate::collision::Profile {
        mounted: [10, 4, 10],
        fallen: [10, 4, 4],
        mass: 256,
        elasticity: 192,
        rider: None,
    });
    setup.rules.finish_progress = Some(1000000);
    setup.seeds.clear();
    for (id, progress) in positions.into_iter().enumerate() {
        let mut s = Seed::player();
        s.id = ActorId(id as u64 + 1);
        s.distance = progress;
        s.lateral = [1000, 16000, 25000][id];
        s.stats.strength = 1;
        setup.seeds.push(s);
    }
    Simulation::with_encounters(1, c.clone(), setup).unwrap()
}
fn state(sim: &Simulation) -> Vec<i64> {
    let mut out = vec![i64::from(sim.gameplay_rng.0)];
    let world = sim.encounter.as_ref().unwrap();
    for (id, u) in &world.units {
        let r = sim.actors[id].view.rider.as_ref().unwrap();
        out.extend([
            i64::from(u.fighter.stats.maximum),
            i64::from(u.fighter.stats.health),
            i64::from(r.bike.condition),
            i64::from(r.bike.speed),
            i64::from(r.bike.lean),
            u.fighter.hit_until as i64,
            r.animation.code as i64,
            r.animation.frame as i64,
            i64::from(r.animation.queued.unwrap_or(-1)),
        ]);
    }
    out
}
#[test]
fn source4_physical_merge_and_seam_transactions_match_the_equivalent_straight_kernel() {
    let cases = [
        (
            [(0, 3, 1000), (1, 3, 1016), (0, 3, 1400)],
            [1000, 1016, 1400],
        ),
        (
            [(1, 2, 120 * 256 - 16), (0, 3, 0), (1, 3, 400)],
            [1000, 1016, 1416],
        ),
        (
            [(0, 3, 0), (1, 2, 120 * 256 - 16), (1, 3, 400)],
            [1016, 1000, 1416],
        ),
        ([(0, 3, 0), (1, 3, 0), (0, 3, 0)], [1000, 1000, 1000]),
    ];
    for (case, (locations, positions)) in cases.into_iter().enumerate() {
        let (c, mut physical) = graph();
        for (index, (profile, path, progress)) in locations.into_iter().enumerate() {
            place(
                &mut physical,
                ActorId(index as u64 + 1),
                profile,
                path,
                progress,
                [1000, 16000, 25000][index],
            );
        }
        let mut control = straight(&c, positions);
        for sim in [&mut physical, &mut control] {
            for id in 1..=3 {
                sim.queue(QueuedCommand {
                    tick: Tick(0),
                    actor: ActorId(id),
                    sequence: 0,
                    input: Command {
                        attack: Some(Request {
                            kind: AttackKind::Punch,
                            side: Side::Auto,
                        }),
                        ..Default::default()
                    },
                })
                .unwrap();
            }
        }
        for tick in 0..96 {
            physical.advance().unwrap();
            control.advance().unwrap();
            assert_eq!(
                state(&physical),
                state(&control),
                "physical case {case} tick {tick}"
            );
        }
        assert!(
            physical
                .encounter
                .as_ref()
                .unwrap()
                .units
                .values()
                .any(|u| u.fighter.hit_count > 0),
            "case must actually strike"
        );
    }
}
#[test]
fn source4_long_branch_actor_converges_to_a_zero_gap_merge_without_changing_half_history() {
    let (_, mut sim) = graph();
    place(&mut sim, ActorId(1), 1, 2, 120 * 256 - 1, 1000);
    place(&mut sim, ActorId(2), 0, 3, 0, 16000);
    place(&mut sim, ActorId(3), 1, 3, 0, 25000);
    sim.encounter
        .as_mut()
        .unwrap()
        .refresh_source_combat_order(&sim.actors);
    assert_eq!(
        sim.encounter
            .as_ref()
            .unwrap()
            .source_combat_target(ActorId(2), &sim.actors),
        Some(ActorId(3)),
        "zero gap half wins over one-unit-behind half"
    );
    place(&mut sim, ActorId(1), 1, 3, 0, 1000);
    sim.encounter
        .as_mut()
        .unwrap()
        .refresh_source_combat_order(&sim.actors);
    assert_eq!(
        sim.encounter
            .as_ref()
            .unwrap()
            .source_combat_target(ActorId(2), &sim.actors),
        Some(ActorId(1)),
        "physical predecessor stays behind at the exact merge"
    );
    assert_eq!(
        sim.encounter
            .as_ref()
            .unwrap()
            .source_combat_target(ActorId(3), &sim.actors),
        Some(ActorId(2))
    );
}
#[test]
fn source4_opposite_fork_halves_and_nonadjacent_paths_cannot_become_false_combat_neighbors() {
    let (_, mut sim) = graph();
    place(&mut sim, ActorId(1), 0, 1, 1000, 1000);
    place(&mut sim, ActorId(2), 1, 2, 1000, 16000);
    place(&mut sim, ActorId(3), 0, 3, 1000, 25000);
    let world = sim.encounter.as_ref().unwrap();
    assert!(world.setup.topology.connected(1, 2));
    assert!(
        world
            .source_combat_relative_position(ActorId(1), ActorId(2), &sim.actors)
            .is_none()
    );
    assert_eq!(world.source_combat_target(ActorId(1), &sim.actors), None);
    place(&mut sim, ActorId(1), 0, 0, 100, 1000);
    let world = sim.encounter.as_ref().unwrap();
    assert!(
        world
            .source_combat_relative_position(ActorId(1), ActorId(3), &sim.actors)
            .is_none(),
        "graph connectivity cannot replace direct physical seam ownership"
    );
}
#[test]
fn source4_detached_combat_half_uses_its_own_route_independently_of_the_quiet_bike() {
    let (c, mut sim) = graph();
    place(&mut sim, ActorId(1), 1, 2, 1000, 1000);
    place(&mut sim, ActorId(2), 0, 1, 1016, 16000);
    place(&mut sim, ActorId(3), 0, 3, 1000, 25000);
    let world = sim.encounter.as_mut().unwrap();
    world
        .hit(
            &mut sim.actors,
            &c,
            ActorId(2),
            Impact {
                reason: 4,
                ..Default::default()
            },
            0,
            true,
            &mut vec![],
        )
        .unwrap();
    let body = world
        .units
        .get_mut(&ActorId(2))
        .unwrap()
        .recovery
        .body
        .as_mut()
        .unwrap();
    body.source_route = Some(1);
    body.location = Location {
        path: 2,
        progress: 1016,
        lateral: 16000,
        ..Default::default()
    };
    body.motion.distance_q8 = 2560 + 1016;
    body.motion.lateral = 16000;
    body.motion.height = 0;
    body.projected = Some(2560 + 1016);
    assert_eq!(
        world.source_combat_target(ActorId(1), &sim.actors),
        Some(ActorId(2))
    );
    let relative = world
        .source_combat_relative_position(ActorId(1), ActorId(2), &sim.actors)
        .unwrap();
    assert_eq!(
        relative.progress - world.position(ActorId(1), &sim.actors).unwrap().progress,
        16
    );
    let request = Request {
        kind: AttackKind::Punch,
        side: Side::Right,
    };
    let definition =
        Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 4)
            .unwrap();
    let tick = (0..definition.duration)
        .find(|age| definition.active(u64::from(*age)))
        .unwrap() as u64;
    sim.actors
        .get_mut(&ActorId(1))
        .unwrap()
        .view
        .rider
        .as_mut()
        .unwrap()
        .animation
        .set(&c.animation, 0, definition.code);
    world.units.get_mut(&ActorId(1)).unwrap().fighter.swing = Some(Swing {
        repeat: false,
        request,
        target: Some(ActorId(2)),
        weapon: animation::Weapon::Fists,
        started: 0,
        definition,
        resolved: false,
        swish: false,
    });
    world
        .combat(&mut sim.actors, &c, tick, |_, _| 1, &mut vec![])
        .unwrap();
    assert_eq!(world.units[&ActorId(2)].fighter.stats.health, 2436);
    assert_eq!(world.units[&ActorId(2)].recovery.bike_location.path, 1);
    assert_eq!(
        world.units[&ActorId(2)]
            .recovery
            .body
            .as_ref()
            .unwrap()
            .source_route,
        Some(1)
    );
    world
        .units
        .get_mut(&ActorId(2))
        .unwrap()
        .recovery
        .body
        .as_mut()
        .unwrap()
        .source_route = Some(0);
    assert!(
        world
            .source_combat_target(ActorId(1), &sim.actors)
            .is_none(),
        "stale bike context cannot grant ownership of the other physical half"
    );
}
#[test]
fn source4_external_fallback_keeps_unselected_local_paths_and_rebases_the_near_seam() {
    let (c, mut sim) = graph();
    place(&mut sim, ActorId(1), 0, 3, 0, 1000);
    for id in [ActorId(2), ActorId(3)] {
        sim.encounter
            .as_mut()
            .unwrap()
            .units
            .get_mut(&id)
            .unwrap()
            .recovery
            .outcome = Some(Outcome::Finished);
    }
    let topology = &sim.encounter.as_ref().unwrap().setup.topology;
    assert!(topology.paths[&2].selected_start.is_none());
    let mut population = crate::population::World::new(crate::population::Setup::default());
    population
        .spawn(
            &crate::population::Spawn {
                id: 1,
                location: Location {
                    path: 2,
                    progress: 120 * 256 - 16,
                    lateral: -1000,
                    ..Default::default()
                },
                kind: crate::population::Kind::Vehicle,
                family: None,
                selector: 0,
                subtype: 0,
                speed: 0,
                direction: 1,
            },
            0,
            topology,
        )
        .unwrap();
    assert!(population.combat_positions(topology, false).is_empty());
    let targets = population.combat_positions(topology, true);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].progress, 120 * 256 - 16);
    let world = sim.encounter.as_mut().unwrap();
    let at = world
        .source_combat_external_position(ActorId(1), targets[0], &sim.actors)
        .unwrap();
    assert_eq!(
        at.progress - world.position(ActorId(1), &sim.actors).unwrap().progress,
        -16
    );
    world.source_combat_external_targets = targets;
    sim.actors.get_mut(&ActorId(1)).unwrap().input.attack = Some(Request {
        kind: AttackKind::Punch,
        side: Side::Auto,
    });
    world
        .source_combat_probe_tick(&mut sim.actors, &c, 0, |_, _| 0, &mut vec![])
        .unwrap();
    assert_eq!(
        world.units[&ActorId(1)]
            .fighter
            .swing
            .as_ref()
            .unwrap()
            .request
            .side,
        Side::Left
    );
}
#[test]
fn source4_disconnected_authored_paths_do_not_share_a_combat_window() {
    let c = fixtures::riding();
    let mut setup = Setup::authored(&c).unwrap();
    setup.rules.combat_version = 4;
    setup.rules.bike_contacts = false;
    let path = setup.topology.paths[&0].clone();
    setup.topology.paths.insert(1, path);
    setup.seeds[0].distance = 1000;
    setup.seeds[0].location = Some(Location {
        progress: 1000,
        ..Default::default()
    });
    let mut other = Seed::player();
    other.id = ActorId(2);
    other.distance = 1000;
    other.location = Some(Location {
        path: 1,
        progress: 1000,
        lateral: 16000,
        ..Default::default()
    });
    setup.seeds.push(other);
    let sim = Simulation::with_encounters(1, c, setup).unwrap();
    let world = sim.encounter.as_ref().unwrap();
    assert_eq!(world.source_combat_target(ActorId(1), &sim.actors), None);
}
