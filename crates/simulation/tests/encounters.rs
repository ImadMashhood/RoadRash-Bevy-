use rr_simulation::{
    animation::Weapon,
    combat::*,
    encounter::*,
    recovery::*,
    topology::{Location, Path, Portal, Topology},
    *,
};
fn setup(count: usize) -> (riding::Configuration, Setup) {
    let c = fixtures::riding();
    let mut s = Setup::authored(&c).unwrap();
    s.rules.bike_contacts = false;
    s.rules.obstacles.clear();
    for i in 1..count {
        let mut seed = Seed::player();
        seed.id = ActorId(i as u64 + 1);
        seed.distance = 100;
        seed.lateral = 12000 * i as i32;
        s.seeds.push(seed);
    }
    s.seeds[0].distance = 100;
    (c, s)
}
fn request(kind: AttackKind, side: Side) -> Command {
    Command {
        attack: Some(Request { kind, side }),
        ..Default::default()
    }
}
fn world(count: usize) -> Simulation {
    let (c, s) = setup(count);
    Simulation::with_encounters(1, c, s).unwrap()
}
fn unit(sim: &Simulation, id: u64) -> &Unit {
    &sim.encounters().unwrap().units[&ActorId(id)]
}
fn rider(sim: &Simulation, id: u64) -> riding::Rider {
    sim.snapshot()
        .actors
        .iter()
        .find(|a| a.id == ActorId(id))
        .unwrap()
        .rider
        .clone()
        .unwrap()
}
#[test]
fn imported_clip_windows_use_first_and_last_selected_tick() {
    let (c, _) = setup(1);
    let d = Definition::from_library(
        &c.animation,
        Request {
            kind: AttackKind::Punch,
            side: Side::Left,
        },
        Weapon::Fists,
    )
    .unwrap();
    assert_eq!(d.duration, 7);
    assert!(!d.active(3));
    assert!(d.active(4));
    assert!(d.active(6));
    assert!(!d.active(7));
    assert_eq!(d.frame(3), 0);
    assert_eq!(d.frame(4), 1);
}
#[test]
fn reach_strict_boundaries_side_height_and_relative_position() {
    let a = Position {
        route: 1,
        ..Default::default()
    };
    let reach = [20480, 28672, 32768, 20480];
    let b = Position {
        route: 1,
        lateral: 20479,
        progress: 63,
        height: 12287,
        ..Default::default()
    };
    assert!(in_reach(a, b, Side::Right, 0, reach));
    for b in [
        Position {
            lateral: 20480,
            ..b
        },
        Position { progress: 64, ..b },
        Position { height: 12288, ..b },
        Position { branch: 1, ..b },
        Position { route: 2, ..b },
    ] {
        assert!(!in_reach(a, b, Side::Right, 0, reach));
    }
    assert!(!in_reach(a, b, Side::Left, 0, reach));
    let left = Position { lateral: -1, ..b };
    assert!(in_reach(a, left, Side::Left, 0, reach));
}
#[test]
fn nearest_behind_wins_ties_then_stable_actor_id() {
    let at = Position {
        route: 1,
        progress: 100,
        ..Default::default()
    };
    let a = (
        ActorId(8),
        Position {
            progress: 110,
            ..at
        },
    );
    let b = (ActorId(4), Position { progress: 90, ..at });
    let c = (ActorId(3), Position { progress: 90, ..at });
    for v in [vec![a, b, c], vec![c, a, b], vec![b, c, a]] {
        assert_eq!(target(ActorId(1), at, v), Some(ActorId(3)));
    }
}
#[test]
fn punch_lands_once_and_overlap_does_not_repeat() {
    let mut sim = world(2);
    sim.step(request(AttackKind::Punch, Side::Right)).unwrap();
    for _ in 0..4 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(unit(&sim, 2).fighter.hit_count, 1);
    assert_eq!(unit(&sim, 2).fighter.stats.health, 2500 - 7 * 64);
    for _ in 0..8 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(unit(&sim, 2).fighter.hit_count, 1);
}
#[test]
fn crash_cancels_attack_before_first_active_tick() {
    let mut sim = world(2);
    sim.step(request(AttackKind::Punch, Side::Right)).unwrap();
    sim.step(Command {
        impact: Some(Impact::default()),
        ..Default::default()
    })
    .unwrap();
    for _ in 0..20 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(unit(&sim, 2).fighter.hit_count, 0);
    assert!(unit(&sim, 1).fighter.swing.is_none());
}
#[test]
fn effects_are_observers_and_do_not_consume_gameplay_rng() {
    let mut a = world(2);
    let mut b = world(2);
    b.set_effects_enabled(false);
    let mut observed = 0;
    for tick in 0..20 {
        let command = if tick == 0 {
            request(AttackKind::Kick, Side::Right)
        } else {
            Command::default()
        };
        a.step(command).unwrap();
        b.step(command).unwrap();
        observed += a.effects().len();
        assert_eq!(a.state_hash(), b.state_hash());
        assert!(b.effects().is_empty());
    }
    assert!(observed > 0);
}
#[test]
fn kick_impulse_and_condition_damage_are_symmetric() {
    fn run(right: bool, edge: bool) -> (i32, i32, u32) {
        let (c, mut s) = setup(2);
        s.seeds[0].lateral = if edge { 150000 } else { 0 };
        s.seeds[1].lateral = s.seeds[0].lateral + if right { 12000 } else { -12000 };
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.step(request(
            AttackKind::Kick,
            if right { Side::Right } else { Side::Left },
        ))
        .unwrap();
        for _ in 0..4 {
            sim.step(Command::default()).unwrap();
        }
        (
            rider(&sim, 2).bike.lean,
            rider(&sim, 2).bike.condition,
            unit(&sim, 2).fighter.hit_count,
        )
    }
    let l = run(false, false);
    let r = run(true, false);
    assert_eq!(run(false, true), l);
    assert_eq!(run(true, true), r);
    assert_eq!(l.0, -r.0);
    assert_eq!(l.1, r.1);
    assert_eq!(l.2, 1);
    assert_eq!(r.2, 1);
    assert!(r.0 > 0);
}
#[test]
fn damage_formula_reason_scaling_and_health_zero() {
    let mut stats = Stats {
        base: 64,
        maximum: 64,
        health: 64,
        strength: 7,
    };
    assert_eq!(stats.damage(256), 7);
    assert_eq!(stats.damage(320), 8);
    assert_eq!(stats.damage(384), 10);
    assert_eq!(stats.damage(64), 1);
    stats.hit(1);
    assert_eq!(stats.health, 0);
    assert_eq!(stats.maximum, 60);
    for reason in 0..7 {
        assert_eq!(
            reason_damage(8, reason),
            [0, 8, 14, 1, 8, 8, 0][reason as usize]
        );
    }
    assert_eq!(reason_damage(-3, 2), -5);
}
#[test]
fn health_depletion_enters_linked_crash_state() {
    let (c, mut s) = setup(2);
    s.seeds[1].stats = Stats {
        base: 64,
        maximum: 64,
        health: 64,
        strength: 1,
    };
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.step(request(AttackKind::Punch, Side::Right)).unwrap();
    for _ in 0..4 {
        sim.step(Command::default()).unwrap();
    }
    let u = unit(&sim, 2);
    assert_eq!(u.fighter.stats.health, 0);
    assert_eq!(u.recovery.phase, Phase::Sliding);
    assert_eq!(u.recovery.bike_entity, 4);
    assert_eq!(u.recovery.body.as_ref().unwrap().entity, 5);
    assert_eq!(u.recovery.body.as_ref().unwrap().owner, ActorId(2));
}
#[test]
fn inventory_transfer_has_one_owner_even_with_competing_requests() {
    let mut inv = Inventory::default();
    inv.items.insert(
        WeaponId(1),
        Item {
            kind: Weapon::Club,
            owner: Some(ActorId(3)),
        },
    );
    assert!(inv.transfer(WeaponId(1), ActorId(3), ActorId(1)));
    assert!(!inv.transfer(WeaponId(1), ActorId(3), ActorId(2)));
    assert_eq!(inv.carried(ActorId(1)), Some((WeaponId(1), Weapon::Club)));
    assert!(inv.carried(ActorId(2)).is_none());
    assert!(inv.carried(ActorId(3)).is_none());
    inv.validate().unwrap();
}
#[test]
fn duplicate_weapon_ownership_is_rejected_before_simulation() {
    let (c, mut s) = setup(2);
    for id in 1..=2 {
        s.inventory.items.insert(
            WeaponId(id),
            Item {
                kind: Weapon::Club,
                owner: Some(ActorId(2)),
            },
        );
    }
    assert!(Simulation::with_encounters(1, c, s).is_err());
}
#[test]
fn unowned_weapon_cannot_be_conjured_by_command() {
    let mut sim = world(1);
    let old = sim.state_hash();
    assert!(
        sim.step(Command {
            weapon: Some(Weapon::Chain),
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(sim.state_hash(), old);
}
#[test]
fn dropping_weapon_cancels_armed_swing() {
    let (c, mut s) = setup(2);
    s.inventory.items.insert(
        WeaponId(1),
        Item {
            kind: Weapon::Chain,
            owner: Some(ActorId(1)),
        },
    );
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.step(request(AttackKind::Punch, Side::Right)).unwrap();
    sim.step(Command {
        weapon: Some(Weapon::Fists),
        ..Default::default()
    })
    .unwrap();
    assert!(unit(&sim, 1).fighter.swing.is_none());
    assert_eq!(rider(&sim, 1).weapon, Weapon::Fists);
    assert_eq!(
        sim.encounters().unwrap().inventory.items[&WeaponId(1)].owner,
        None
    );
}
#[test]
fn simultaneous_attacks_are_repeatable_and_low_id_ordered() {
    let run = || {
        let mut sim = world(2);
        for (id, side) in [(1, Side::Right), (2, Side::Left)] {
            sim.queue(QueuedCommand {
                tick: Tick(0),
                actor: ActorId(id),
                sequence: 0,
                input: request(AttackKind::Punch, side),
            })
            .unwrap();
        }
        for _ in 0..8 {
            sim.advance().unwrap();
        }
        assert_eq!(unit(&sim, 2).fighter.hit_count, 1);
        assert_eq!(unit(&sim, 1).fighter.hit_count, 0);
        sim.state_hash()
    };
    assert_eq!(run(), run());
}
#[test]
fn removing_target_mid_swing_is_safe() {
    let mut sim = world(2);
    sim.step(request(AttackKind::Punch, Side::Right)).unwrap();
    sim.despawn(ActorId(2)).unwrap();
    for _ in 0..15 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(sim.snapshot().actors.len(), 1);
}
#[test]
fn collision_and_obstacle_policies_distinguish_decorative_and_solid() {
    for kind in [ObstacleKind::Decorative, ObstacleKind::Solid] {
        let (c, mut s) = setup(1);
        s.seeds[0].speed = 9000;
        s.rules.obstacles.push(Obstacle {
            source_box: None,
            id: 1,
            path: 0,
            progress: 116,
            half_length: 16,
            left: -1024,
            right: 1024,
            height: 8960,
            kind,
        });
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.step(Command::default()).unwrap();
        assert_eq!(
            unit(&sim, 1).recovery.phase == Phase::Sliding,
            kind == ObstacleKind::Solid
        );
    }
}
#[test]
fn sustained_obstacle_overlap_does_not_repeat_same_incident() {
    let (c, mut s) = setup(1);
    s.rules.obstacles.push(Obstacle {
        source_box: None,
        id: 1,
        path: 0,
        progress: 100,
        half_length: 16,
        left: -1024,
        right: 1024,
        height: 8960,
        kind: ObstacleKind::Solid,
    });
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    for _ in 0..5 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(unit(&sim, 1).fighter.hit_count, 1);
}
#[test]
fn empty_road_edge_is_not_an_invisible_solid_wall() {
    let (c, mut s) = setup(1);
    s.seeds[0].lateral = 200000;
    s.seeds[0].speed = 5000;
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.step(Command::default()).unwrap();
    assert_eq!(unit(&sim, 1).recovery.phase, Phase::Mounted);
    assert!(rider(&sim, 1).bike.speed < 5000);
}
#[test]
fn source_contact_speed_threshold_and_separation() {
    for speed in [1000, 9000] {
        let (c, mut s) = setup(2);
        s.rules.bike_contacts = true;
        s.seeds[0].speed = speed;
        s.seeds[1].speed = 0;
        s.seeds[1].distance = 105;
        s.seeds[1].lateral = 1000;
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.step(Command::default()).unwrap();
        assert!((rider(&sim, 1).bike.lateral - rider(&sim, 2).bike.lateral).abs() > 1000);
        assert_eq!(
            unit(&sim, 1).recovery.phase == Phase::Sliding,
            speed == 9000
        );
    }
}
#[test]
fn rider_and_bike_move_independently_after_separation() {
    let mut sim = world(1);
    sim.step(Command {
        impact: Some(Impact {
            rider_velocity: Some([5000, 0, 0]),
            bike_velocity: Some([0, 0, 0]),
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    let bike = rider(&sim, 1).bike.distance_q8;
    for _ in 0..5 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(rider(&sim, 1).bike.distance_q8, bike);
    assert!(
        unit(&sim, 1)
            .recovery
            .body
            .as_ref()
            .unwrap()
            .motion
            .distance_q8
            > bike
    );
}
#[test]
fn culling_or_modifying_copied_snapshot_cannot_delete_recovery_target() {
    let mut sim = world(1);
    sim.step(Command {
        impact: Some(Impact::default()),
        ..Default::default()
    })
    .unwrap();
    let hash = sim.state_hash();
    let mut view = sim.snapshot();
    view.actors.clear();
    assert_eq!(sim.state_hash(), hash);
    assert!(unit(&sim, 1).recovery.bike_available);
    assert!(unit(&sim, 1).recovery.body.is_some());
}
#[test]
fn repeated_impact_does_not_replace_body_identity_or_snap_position() {
    let mut sim = world(1);
    sim.step(Command {
        impact: Some(Impact {
            rider_velocity: Some([5000, 0, 0]),
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    for _ in 0..3 {
        sim.step(Command::default()).unwrap();
    }
    let before = unit(&sim, 1).recovery.body.as_ref().unwrap().clone();
    sim.step(Command {
        impact: Some(Impact {
            rider_velocity: Some([0, 0, 0]),
            bike_velocity: Some([0, 0, 0]),
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    let after = unit(&sim, 1).recovery.body.as_ref().unwrap();
    assert_eq!(before.entity, after.entity);
    assert!(after.location.progress >= before.location.progress);
    assert_eq!(unit(&sim, 1).recovery.incident, 2);
}
#[test]
fn terminal_outcomes_prioritize_arrest_and_latch_once() {
    let mut sim = world(1);
    sim.step(Command {
        impact: Some(Impact::default()),
        terminal: Some(Outcome::Arrested),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(unit(&sim, 1).recovery.outcome, Some(Outcome::Arrested));
    assert_eq!(unit(&sim, 1).recovery.phase, Phase::Ended);
    assert!(unit(&sim, 1).fighter.swing.is_none());
    let count = sim
        .effects()
        .iter()
        .filter(|e| e.kind == EffectKind::Arrest)
        .count();
    assert_eq!(count, 1);
    sim.step(Command {
        terminal: Some(Outcome::Finished),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(unit(&sim, 1).recovery.outcome, Some(Outcome::Arrested));
    assert!(sim.effects().is_empty());
}
#[test]
fn removing_bike_produces_explicit_terminal_fallback() {
    let mut sim = world(1);
    sim.step(Command {
        impact: Some(Impact::default()),
        ..Default::default()
    })
    .unwrap();
    sim.remove_recovery_bike(ActorId(1)).unwrap();
    sim.step(Command::default()).unwrap();
    assert_eq!(unit(&sim, 1).recovery.outcome, Some(Outcome::Wrecked));
    assert_eq!(unit(&sim, 1).recovery.phase, Phase::Ended);
}
#[test]
fn assist_setting_changes_replay_identity_but_effect_toggle_does_not() {
    let (c, a) = setup(1);
    let mut b = a.clone();
    b.rules.assists.fast_recovery = true;
    let a = Simulation::with_encounters(1, c.clone(), a).unwrap();
    let b = Simulation::with_encounters(1, c, b).unwrap();
    assert_ne!(a.state_hash(), b.state_hash());
}
#[test]
fn topology_uses_portals_instead_of_lateral_shortcuts() {
    let mut t = Topology::straight(100);
    t.paths.insert(
        1,
        Path {
            length: 100,
            branch: 1,
            selected_start: None,
            portals: vec![Portal {
                end: false,
                target: 0,
                target_end: true,
            }],
            rises: vec![],
        },
    );
    t.paths.get_mut(&0).unwrap().portals.push(Portal {
        end: true,
        target: 1,
        target_end: false,
    });
    t.validate().unwrap();
    let mut l = Location {
        path: 0,
        progress: 99,
        ..Default::default()
    };
    let leg = t.leg(0, 1).unwrap();
    t.advance(&mut l, 5, Some(leg)).unwrap();
    assert_eq!((l.path, l.progress), (1, 4));
    assert!(t.connected(1, 0));
    t.paths.insert(
        2,
        Path {
            length: 100,
            branch: 2,
            selected_start: None,
            portals: vec![],
            rises: vec![],
        },
    );
    assert!(!t.connected(0, 2));
}
#[test]
fn remount_distance_and_orientation_edges_are_explicit() {
    let t = Topology::straight(1000);
    let c = fixtures::riding();
    let mut bike = handling::Bike::new(c.spec(), &c.tables).unwrap();
    let mut state = State::mounted(
        ActorId(1),
        Location {
            progress: 100,
            ..Default::default()
        },
    );
    state.begin(
        ActorId(1),
        &mut bike,
        &c,
        state.bike_location,
        Impact::default(),
        &t,
    );
    state.phase = Phase::Walking;
    let body = state.body.as_mut().unwrap();
    body.location = Location {
        progress: 91,
        lateral: -6000,
        heading: 0,
        ..Default::default()
    };
    assert!(state.can_mount(&t));
    state.body.as_mut().unwrap().location.progress = 90;
    assert!(!state.can_mount(&t));
    state.body.as_mut().unwrap().location.progress = 100;
    state.body.as_mut().unwrap().location.lateral = -12001;
    assert!(!state.can_mount(&t));
    state.body.as_mut().unwrap().location.lateral = -6000;
    state.body.as_mut().unwrap().location.heading = 0x800000;
    assert!(!state.can_mount(&t));
}
#[test]
fn invalid_topology_and_commands_fail_before_mutation() {
    let mut sim = world(1);
    let old = sim.state_hash();
    assert!(
        sim.step(Command {
            walk: Some(Walk {
                longitudinal: 2,
                lateral: 0
            }),
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(old, sim.state_hash());
    let (c, mut s) = setup(1);
    s.topology.paths.get_mut(&0).unwrap().portals.push(Portal {
        end: true,
        target: 99,
        target_end: false,
    });
    assert!(Simulation::with_encounters(1, c, s).is_err());
}
#[test]
fn schema_three_rejects_schema_four_commands() {
    let c = fixtures::riding();
    let mut s = Simulation::with_handling(1, c).unwrap();
    assert!(s.step(request(AttackKind::Punch, Side::Auto)).is_err());
}

#[test]
fn all_attack_weapon_side_scenarios_have_tick_range_expectations() {
    for kind in [AttackKind::Punch, AttackKind::Kick, AttackKind::Backhand] {
        for side in [Side::Left, Side::Right] {
            for weapon in [Weapon::Fists, Weapon::Club, Weapon::Chain] {
                let (c, mut s) = setup(2);
                s.seeds[1].lateral = if side == Side::Left { -12000 } else { 12000 };
                if weapon != Weapon::Fists {
                    s.inventory.items.insert(
                        WeaponId(1),
                        Item {
                            kind: weapon,
                            owner: Some(ActorId(1)),
                        },
                    );
                }
                let definition =
                    Definition::from_library(&c.animation, Request { kind, side }, weapon).unwrap();
                let first = definition.windows[0].start;
                let mut sim = Simulation::with_encounters(1, c, s).unwrap();
                sim.step(request(kind, side)).unwrap();
                for _ in 0..first - 1 {
                    sim.step(Command::default()).unwrap();
                }
                assert_eq!(
                    unit(&sim, 2).fighter.hit_count,
                    0,
                    "{kind:?}/{side:?}/{weapon:?}"
                );
                sim.step(Command::default()).unwrap();
                let strength =
                    Stats::default().damage([256, 320, 384, 64][weapon_slot(kind, weapon)]);
                assert_eq!(unit(&sim, 2).fighter.stats.health, 2500 - 64 * strength);
                assert_eq!(unit(&sim, 2).fighter.stats.maximum, 2500 - 4 * strength);
                assert_eq!(unit(&sim, 2).fighter.hit_count, 1);
                assert_eq!(unit(&sim, 2).recovery.phase, Phase::Mounted);
                assert_eq!(
                    sim.encounters()
                        .unwrap()
                        .inventory
                        .carried(ActorId(1))
                        .map(|(_, k)| k)
                        .unwrap_or(Weapon::Fists),
                    weapon
                );
                let kick = kind == AttackKind::Kick;
                assert_eq!(
                    rider(&sim, 2).bike.lean.signum(),
                    if side == Side::Left { -1 } else { 1 }
                );
                assert_eq!(
                    rider(&sim, 2).bike.lean.abs(),
                    if kick { 11000 } else { 3000 }
                );
                for _ in 0..15 {
                    sim.step(Command::default()).unwrap();
                }
                assert_eq!(unit(&sim, 2).fighter.hit_count, 1);
            }
        }
    }
}
#[test]
fn successful_theft_cancels_both_old_swings_and_keeps_one_owner() {
    let (c, mut s) = setup(3);
    s.seeds[1].lateral = 12000;
    s.seeds[2].lateral = 0;
    s.seeds[2].distance = 101;
    s.inventory.items.insert(
        WeaponId(7),
        Item {
            kind: Weapon::Club,
            owner: Some(ActorId(2)),
        },
    );
    let seed = (0u32..100)
        .find(|n| (((n.wrapping_mul(214013).wrapping_add(2531011)) >> 16) & 0x7fff) & 3 == 3)
        .unwrap();
    let mut sim = Simulation::with_encounters(seed, c, s).unwrap();
    sim.queue(QueuedCommand {
        sequence: 100,
        tick: Tick(0),
        actor: ActorId(3),
        input: request(AttackKind::Punch, Side::Right),
    })
    .unwrap();
    sim.step(request(AttackKind::Punch, Side::Right)).unwrap();
    sim.step(Command::default()).unwrap();
    sim.step(Command::default()).unwrap();
    sim.queue(QueuedCommand {
        sequence: 100,
        tick: sim.tick(),
        actor: ActorId(2),
        input: request(AttackKind::Punch, Side::Left),
    })
    .unwrap();
    sim.step(Command::default()).unwrap();
    sim.step(Command::default()).unwrap();
    assert_eq!(
        sim.encounters().unwrap().inventory.items[&WeaponId(7)].owner,
        Some(ActorId(1))
    );
    assert_eq!(rider(&sim, 1).weapon, Weapon::Club);
    assert_eq!(rider(&sim, 2).weapon, Weapon::Fists);
    assert!(unit(&sim, 1).fighter.swing.is_none());
    assert_eq!(unit(&sim, 2).fighter.hit_count, 1); // second thief hits after first atomically transfers
    sim.encounters().unwrap().inventory.validate().unwrap();
}
#[test]
fn new_hit_at_stun_expiry_cancels_the_new_attack() {
    let (c, mut s) = setup(3);
    s.rules.stun = 1;
    s.rules.immunity = 0;
    s.seeds[2].lateral = 24000;
    s.seeds[1].distance = 101;
    s.seeds[2].distance = 102;
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.queue(QueuedCommand {
        sequence: 100,
        tick: Tick(1),
        actor: ActorId(3),
        input: request(AttackKind::Punch, Side::Left),
    })
    .unwrap();
    sim.queue(QueuedCommand {
        sequence: 100,
        tick: Tick(5),
        actor: ActorId(2),
        input: request(AttackKind::Punch, Side::Left),
    })
    .unwrap();
    sim.step(request(AttackKind::Punch, Side::Right)).unwrap();
    for _ in 0..5 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(unit(&sim, 2).fighter.hit_count, 2);
    assert_eq!(unit(&sim, 2).fighter.stun_until, 6);
    assert!(unit(&sim, 2).fighter.swing.is_none());
}
fn crash_command() -> Command {
    Command {
        impact: Some(Impact {
            rider_velocity: Some([5000, 0, 0]),
            bike_velocity: Some([0, 0, 0]),
            ..Default::default()
        }),
        ..Default::default()
    }
}
#[test]
fn full_recovery_visits_every_phase_without_teleporting() {
    let mut sim = world(1);
    sim.step(crash_command()).unwrap();
    let mut phases = std::collections::BTreeSet::new();
    phases.insert(format!("{:?}", unit(&sim, 1).recovery.phase));
    let mut recovered = false;
    for _ in 0..1800 {
        sim.step(Command::default()).unwrap();
        let state = &unit(&sim, 1).recovery;
        phases.insert(format!("{:?}", state.phase));
        assert!(state.outcome.is_none(), "unexpected terminal: {state:?}");
        if state.phase == Phase::Mounted {
            recovered = true;
            break;
        }
    }
    assert!(recovered, "{phases:?}");
    for phase in [
        "Sliding",
        "GettingUp",
        "Standing",
        "Walking",
        "Mounting",
        "Mounted",
    ] {
        assert!(phases.contains(phase), "{phases:?}");
    }
    assert!(unit(&sim, 1).recovery.body.is_none());
    assert!(!rider(&sim, 1).bike.crashed);
}
#[test]
fn impact_interrupts_getup_and_mount_without_emitting_remount() {
    for target in [Phase::GettingUp, Phase::Mounting] {
        let mut sim = world(1);
        sim.step(crash_command()).unwrap();
        let mut found = false;
        for _ in 0..1800 {
            if unit(&sim, 1).recovery.phase == target {
                found = true;
                break;
            }
            sim.step(Command::default()).unwrap();
        }
        assert!(found, "never reached {target:?}");
        let identity = unit(&sim, 1).recovery.body.as_ref().unwrap().entity;
        sim.step(crash_command()).unwrap();
        assert_eq!(unit(&sim, 1).recovery.phase, Phase::Sliding);
        assert_eq!(
            unit(&sim, 1).recovery.body.as_ref().unwrap().entity,
            identity
        );
        assert!(!sim.effects().iter().any(|e| e.kind == EffectKind::Remount));
    }
}
#[test]
fn grade_integration_crosses_exact_portal_endpoint() {
    let mut t = Topology::straight(256);
    t.paths.get_mut(&0).unwrap().rises = vec![12];
    t.paths.get_mut(&0).unwrap().portals = vec![Portal {
        end: true,
        target: 1,
        target_end: false,
    }];
    t.paths.insert(
        1,
        Path {
            length: 256,
            branch: 1,
            selected_start: None,
            rises: vec![-8],
            portals: vec![Portal {
                end: false,
                target: 0,
                target_end: true,
            }],
        },
    );
    t.validate().unwrap();
    let mut at = Location {
        path: 0,
        progress: 250,
        ..Default::default()
    };
    assert_eq!(t.advance_with_rise(&mut at, 6, None).unwrap(), 72);
    assert_eq!((at.path, at.progress), (1, 0));
    assert_eq!(t.advance_with_rise(&mut at, 5, None).unwrap(), -40);
    assert_eq!(t.advance_with_rise(&mut at, -8, None).unwrap(), 4);
    assert_eq!((at.path, at.progress), (0, 253));
}
#[test]
fn replay_schedules_and_effect_switch_preserve_checkpoints() {
    use replay::{Playback, Replay, frame_ns};
    let (c, s) = setup(2);
    let replay = Replay {
        schema_version: 4,
        seed: 1,
        ticks: 500,
        commands: vec![
            QueuedCommand {
                sequence: 100,
                tick: Tick(0),
                actor: ActorId(1),
                input: request(AttackKind::Kick, Side::Right),
            },
            QueuedCommand {
                sequence: 100,
                tick: Tick(25),
                actor: ActorId(1),
                input: crash_command(),
            },
        ],
        route: None,
        expected_hash: None,
        handling: Some(c),
        encounter: Some(s),
        population: None,
    };
    let encoded = serde_json::to_vec(&replay).unwrap();
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

fn walking_state(
    c: &riding::Configuration,
    t: &Topology,
    bike_at: Location,
    body_at: Location,
) -> (State, handling::Bike) {
    let mut bike = handling::Bike::new(c.spec(), &c.tables).unwrap();
    bike.distance_q8 = t.projected(bike_at).unwrap_or(0);
    bike.lateral = bike_at.lateral;
    let mut state = State::mounted(ActorId(1), bike_at);
    state.begin(ActorId(1), &mut bike, c, bike_at, Impact::default(), t);
    state.phase = Phase::Walking;
    state.elapsed = 0;
    let body = state.body.as_mut().unwrap();
    body.location = body_at;
    body.motion.speed = 0;
    body.motion.height = 0;
    body.motion.vertical_speed = 0;
    body.animation.set(&c.animation, 0, 38);
    bike.speed = 0;
    bike.height = 0;
    bike.vertical_speed = 0;
    (state, bike)
}
#[test]
fn walking_routes_backwards_and_to_neighboring_branch_without_timeout() {
    let c = fixtures::riding();
    for neighbor in [false, true] {
        let mut t = Topology::straight(256);
        if neighbor {
            t.paths.get_mut(&0).unwrap().portals = vec![Portal {
                end: true,
                target: 1,
                target_end: false,
            }];
            t.paths.insert(
                1,
                Path {
                    length: 256,
                    branch: 1,
                    selected_start: None,
                    rises: vec![8],
                    portals: vec![Portal {
                        end: false,
                        target: 0,
                        target_end: true,
                    }],
                },
            );
        }
        let bike_at = Location {
            path: u32::from(neighbor),
            progress: if neighbor { 100 } else { 40 },
            lateral: 6000,
            ..Default::default()
        };
        let body_at = Location {
            path: 0,
            progress: 200,
            ..Default::default()
        };
        let (mut state, mut bike) = walking_state(&c, &t, bike_at, body_at);
        let identity = state.body.as_ref().unwrap().entity;
        let mut mounted = false;
        for tick in 0..300 {
            state
                .step(
                    &mut bike,
                    &c,
                    &t,
                    &BodyLaw::default(),
                    Assists::default(),
                    None,
                    tick,
                )
                .unwrap();
            if state.phase == Phase::Mounting {
                mounted = true;
                break;
            }
            let body = state.body.as_ref().unwrap();
            assert_eq!(body.entity, identity);
            assert!(t.valid(body.location));
        }
        assert!(mounted, "{neighbor}: {state:?}");
        assert!(state.elapsed < 300);
        assert_eq!(state.body.as_ref().unwrap().location.path, bike_at.path);
    }
}
#[test]
fn disconnected_branch_requires_explicit_rescue_setting() {
    let c = fixtures::riding();
    let mut t = Topology::straight(256);
    t.paths.insert(
        1,
        Path {
            length: 256,
            branch: 1,
            selected_start: None,
            rises: vec![],
            portals: vec![],
        },
    );
    let (state, bike) = walking_state(
        &c,
        &t,
        Location {
            path: 1,
            progress: 80,
            lateral: 6000,
            ..Default::default()
        },
        Location {
            path: 0,
            progress: 80,
            ..Default::default()
        },
    );
    for rescue in [false, true] {
        let mut state = state.clone();
        let mut bike = bike.clone();
        let events = state
            .step(
                &mut bike,
                &c,
                &t,
                &BodyLaw::default(),
                Assists {
                    rescue_unreachable: rescue,
                    ..Default::default()
                },
                None,
                0,
            )
            .unwrap();
        if rescue {
            assert!(state.assisted);
            assert!(events.contains(&RecoveryEvent::Rescue));
            assert_eq!(state.phase, Phase::Mounting);
        } else {
            assert_eq!(state.outcome, Some(Outcome::Wrecked));
        }
    }
}
#[test]
fn finish_wins_over_wreck_and_arrest_wins_over_finish_on_same_tick() {
    for result in [Outcome::Finished, Outcome::Arrested] {
        let mut sim = world(1);
        sim.step(Command {
            impact: Some(Impact {
                bike_damage: u16::MAX,
                ..Default::default()
            }),
            terminal: Some(result),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(unit(&sim, 1).recovery.outcome, Some(result));
        assert_eq!(sim.effects().len(), 1);
        let completed = unit(&sim, 1).completed_at;
        sim.step(Command {
            terminal: Some(Outcome::Arrested),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(unit(&sim, 1).recovery.outcome, Some(result));
        assert_eq!(unit(&sim, 1).completed_at, completed);
    }
}

#[test]
fn collision_on_final_getup_or_mount_tick_preserves_body_and_cancels_transition() {
    for interrupted in [Phase::GettingUp, Phase::Mounting] {
        let mut baseline = world(1);
        baseline.step(crash_command()).unwrap();
        let mut boundary = None;
        for _ in 0..1800 {
            let previous = unit(&baseline, 1).recovery.phase;
            baseline.step(Command::default()).unwrap();
            if previous == interrupted && unit(&baseline, 1).recovery.phase != interrupted {
                boundary = Some(baseline.tick().0 - 1);
                break;
            }
        }
        let boundary = boundary.expect("baseline completes phase");
        let mut sim = world(1);
        sim.step(crash_command()).unwrap();
        while sim.tick().0 < boundary {
            sim.step(Command::default()).unwrap();
        }
        let previous = unit(&sim, 1).recovery.body.as_ref().unwrap().clone();
        sim.step(crash_command()).unwrap();
        let next = unit(&sim, 1).recovery.body.as_ref().unwrap();
        assert_eq!(next.entity, previous.entity);
        assert_eq!(next.location, previous.location);
        assert_eq!(unit(&sim, 1).recovery.phase, Phase::Sliding);
        assert!(
            !sim.effects()
                .iter()
                .any(|e| matches!(e.kind, EffectKind::Remount | EffectKind::GetUp))
        );
    }
}
#[test]
fn slide_travel_and_stop_timing_increase_with_entry_speed() {
    let mut last_distance = 0;
    let mut last_ticks = 0;
    for speed in [1000, 5000, 10000] {
        let mut sim = world(1);
        sim.step(Command {
            impact: Some(Impact {
                rider_velocity: Some([speed, 0, 0]),
                bike_velocity: Some([0, 0, 0]),
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
        let start = unit(&sim, 1)
            .recovery
            .body
            .as_ref()
            .unwrap()
            .location
            .progress;
        let mut stopped = None;
        for tick in 1..1200 {
            sim.step(Command::default()).unwrap();
            if unit(&sim, 1).recovery.phase == Phase::GettingUp {
                stopped = Some(tick);
                break;
            }
        }
        let ticks = stopped.expect("bounded slide");
        let distance = unit(&sim, 1)
            .recovery
            .body
            .as_ref()
            .unwrap()
            .location
            .progress
            - start;
        assert!(
            distance > last_distance && ticks >= last_ticks,
            "{speed}: {distance}/{ticks}"
        );
        println!("entry_speed={speed}, slide_distance_q8={distance}, ticks_until_getup={ticks}");
        last_distance = distance;
        last_ticks = ticks;
    }
}

#[test]
fn fast_recovery_is_faster_and_default_retains_unassisted_timeline() {
    let mut times = vec![];
    for fast in [false, true] {
        let (c, mut s) = setup(1);
        s.rules.assists.fast_recovery = fast;
        let mut sim = Simulation::with_encounters(1, c, s).unwrap();
        sim.step(crash_command()).unwrap();
        for _ in 0..1800 {
            sim.step(Command::default()).unwrap();
            if unit(&sim, 1).recovery.phase == Phase::Mounted {
                break;
            }
        }
        assert_eq!(unit(&sim, 1).recovery.phase, Phase::Mounted);
        assert_eq!(unit(&sim, 1).recovery.assisted, fast);
        times.push(sim.tick().0);
    }
    assert!(times[1] < times[0], "{times:?}");
}
#[test]
fn configured_finish_is_crossed_by_detached_rider_and_latches_once() {
    let (c, mut s) = setup(1);
    s.rules.finish_progress = Some(105);
    let mut sim = Simulation::with_encounters(1, c, s).unwrap();
    sim.step(crash_command()).unwrap();
    let stationary = rider(&sim, 1).bike.distance_q8;
    sim.step(Command::default()).unwrap();
    assert_eq!(rider(&sim, 1).bike.distance_q8, stationary);
    assert_eq!(unit(&sim, 1).recovery.outcome, Some(Outcome::Finished));
    assert_eq!(
        sim.effects()
            .iter()
            .filter(|e| e.kind == EffectKind::Finish)
            .count(),
        1
    );
}

#[test]
fn live_attack_rechecks_exact_reach_after_different_relative_speeds() {
    for relative in [-1000, 0, 1000] {
        let (c, mut setup) = setup(2);
        setup.seeds[0].speed = 4000;
        setup.seeds[1].speed = 4000 + relative;
        let mut pilot = Simulation::with_encounters(1, c.clone(), setup.clone()).unwrap();
        for _ in 0..5 {
            pilot.step(Command::default()).unwrap();
        }
        let moved_gap = rider(&pilot, 2).bike.distance_q8 - rider(&pilot, 1).bike.distance_q8;
        for boundary in [63, 64] {
            let mut setup = setup.clone();
            setup.seeds[1].distance += boundary - moved_gap;
            let mut sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
            sim.step(request(AttackKind::Punch, Side::Right)).unwrap();
            for _ in 0..4 {
                sim.step(Command::default()).unwrap();
            }
            assert_eq!(
                rider(&sim, 2).bike.distance_q8 - rider(&sim, 1).bike.distance_q8,
                boundary
            );
            assert_eq!(
                unit(&sim, 2).fighter.hit_count,
                u32::from(boundary == 63),
                "relative={relative} boundary={boundary}"
            );
        }
    }
}

#[test]
fn building_front_stops_bikes_without_repeated_damage_and_open_lane_stays_open() {
    for speed in [1000, 9000, 30000] {
        let (c, mut e) = setup(1);
        e.seeds[0].speed = speed;
        e.rules.obstacles.push(Obstacle {
            source_box: None,
            id: 1,
            path: 0,
            progress: 180,
            half_length: 16,
            left: -10000,
            right: 10000,
            height: 100000,
            kind: ObstacleKind::Building,
        });
        let length = i64::from(e.rules.dimensions[1]);
        let mut sim = Simulation::with_encounters(1, c, e).unwrap();
        for _ in 0..12 {
            sim.step(Command {
                throttle: true,
                ..Default::default()
            })
            .unwrap();
        }
        assert!(
            unit(&sim, 1).recovery.bike_location.progress < 180 - 16 - length,
            "speed {speed}: {:?}",
            unit(&sim, 1).recovery.bike_location
        );
        assert_eq!(unit(&sim, 1).fighter.hit_count, 1);
    }
    let (c, mut e) = setup(1);
    e.seeds[0].speed = 9000;
    e.rules.obstacles.push(Obstacle {
        source_box: None,
        id: 1,
        path: 0,
        progress: 180,
        half_length: 16,
        left: 20000,
        right: 40000,
        height: 100000,
        kind: ObstacleKind::Building,
    });
    let mut sim = Simulation::with_encounters(1, c, e).unwrap();
    for _ in 0..12 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(unit(&sim, 1).fighter.hit_count, 0);
    assert!(unit(&sim, 1).recovery.bike_location.progress > 180);
}

#[test]
fn adjacent_building_panels_do_not_restart_an_existing_crash() {
    let (c, mut e) = setup(1);
    e.seeds[0].speed = 30000;
    for (id, progress) in [(1, 116), (2, 132), (3, 148)] {
        e.rules.obstacles.push(Obstacle {
            source_box: None,
            id,
            path: 0,
            progress,
            half_length: 16,
            left: -10000,
            right: 10000,
            height: 100000,
            kind: ObstacleKind::Building,
        });
    }
    let mut sim = Simulation::with_encounters(1, c, e).unwrap();
    for _ in 0..20 {
        sim.step(Command {
            throttle: true,
            ..Default::default()
        })
        .unwrap();
    }
    assert_eq!(unit(&sim, 1).fighter.hit_count, 1);
    assert_eq!(unit(&sim, 1).recovery.phase, Phase::Sliding);
}

#[test]
fn source_attack_counter_opens_and_closes_the_window_on_native_ticks() {
    let (mut c, mut e) = setup(2);
    let code = c
        .animation
        .pose_code(animation::Pose::PunchRight, animation::Weapon::Fists) as usize;
    let original = c.animation.banks[0][code].as_ref().unwrap()[0].clone();
    c.animation.banks[0][code] = Some(
        [2, 3, 4]
            .into_iter()
            .enumerate()
            .map(|(i, duration)| animation::Frame {
                duration,
                control: if i == 2 { 2 } else { 0 },
                flags: 0,
                ..original.clone()
            })
            .collect(),
    );
    let request = Request {
        kind: AttackKind::Punch,
        side: Side::Right,
    };
    let old = Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 0)
        .unwrap();
    let source =
        Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 1)
            .unwrap();
    for tick in 0..11 {
        assert_eq!(old.active(tick), (6..10).contains(&tick));
        assert_eq!(source.active(tick), (5..9).contains(&tick));
    }
    assert_eq!(source.frame(4), 1);
    assert_eq!(
        source.frame(5),
        2,
        "weapon theft uses the advanced counter too"
    );
    e.rules.combat_version = 1;
    let mut sim = Simulation::with_encounters(1, c, e).unwrap();
    sim.step(Command {
        attack: Some(request),
        ..Default::default()
    })
    .unwrap();
    for _ in 0..4 {
        sim.step(Command::default()).unwrap();
    }
    assert_eq!(unit(&sim, 2).fighter.hit_count, 0);
    sim.step(Command::default()).unwrap();
    assert_eq!(unit(&sim, 2).fighter.hit_count, 1);
}

#[test]
fn source_crash_entry_preserves_reverse_impulse_and_ignores_repeat_entry() {
    for reason in [1, 2, 4, 5] {
        let c = fixtures::riding();
        let mut bike = handling::Bike::new(c.spec(), &c.tables).unwrap();
        let topology = Topology::straight(100000);
        let at = Location::default();
        let mut state = State::mounted(ActorId(1), at);
        let impact = Impact {
            reason,
            ..Default::default()
        };
        assert!(state.begin_source(ActorId(1), &mut bike, &c, at, impact, &topology));
        assert_eq!(
            state.body.as_ref().unwrap().motion.speed,
            if reason == 4 { -80 } else { -256 }
        );
        let before = serde_json::to_value((&state, &bike)).unwrap();
        assert!(!state.begin_source(ActorId(1), &mut bike, &c, at, impact, &topology));
        assert_eq!(serde_json::to_value((&state, &bike)).unwrap(), before);
    }
}

#[test]
fn source_contact_does_not_emit_duplicate_crash_or_restart_recovery() {
    let (c, mut e) = setup(1);
    e.rules.combat_version = 1;
    let mut sim = Simulation::with_encounters(1, c, e).unwrap();
    sim.step(crash_command()).unwrap();
    for _ in 0..5 {
        sim.step(Command::default()).unwrap();
    }
    let before = unit(&sim, 1).recovery.clone();
    sim.step(crash_command()).unwrap();
    let after = &unit(&sim, 1).recovery;
    assert_eq!(after.incident, before.incident);
    assert_eq!(after.elapsed, before.elapsed);
    assert_eq!(after.body, before.body);
    assert!(
        !sim.effects()
            .iter()
            .any(|event| event.kind == EffectKind::Crash)
    );
}

#[test]
fn combat_rules_version_is_explicit_and_legacy_encoding_is_unchanged() {
    let (c, mut e) = setup(1);
    assert!(
        !serde_json::to_string(&e)
            .unwrap()
            .contains("combat_version")
    );
    for version in 0..=3 {
        e.rules.combat_version = version;
        assert!(Simulation::with_encounters(1, c.clone(), e.clone()).is_ok());
    }
    e.rules.combat_version = 4;
    assert!(Simulation::with_encounters(1, c, e).is_err());
}

#[test]
fn recovery_motion_profile_preserves_backwards_travel_and_legacy_hash_inputs() {
    let c = fixtures::riding();
    for version in [0, 1] {
        let mut bike = handling::Bike::new(c.spec(), &c.tables).unwrap();
        bike.speed = -256;
        bike.height = 20000;
        let topology = Topology::straight(100000);
        let mut at = Location {
            progress: 1000,
            height: 20000,
            ..Default::default()
        };
        let law = BodyLaw {
            motion_version: version,
            drag: 0,
            contact_drag: 0,
            ..Default::default()
        };
        motion_step(&mut bike, &mut at, 0, &c, &topology, &law, None).unwrap();
        assert_eq!(
            (at.progress, bike.speed),
            if version == 0 { (1000, 0) } else { (999, -256) }
        );
    }
    assert!(
        !serde_json::to_string(&BodyLaw::default())
            .unwrap()
            .contains("motion_version")
    );
    assert!(
        BodyLaw {
            motion_version: 2,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn shallow_building_corner_slides_out_instead_of_trapping_a_slow_bike() {
    let (c, mut e) = setup(1);
    e.rules.collision_version = 1;
    e.seeds[0].speed = 1000;
    e.seeds[0].lateral = 17000;
    e.rules.obstacles.push(Obstacle {
        source_box: None,
        id: 1,
        path: 0,
        progress: 220,
        half_length: 64,
        left: 20000,
        right: 60000,
        height: 100000,
        kind: ObstacleKind::Building,
    });
    let mut sim = Simulation::with_encounters(1, c, e).unwrap();
    for _ in 0..120 {
        sim.step(Command {
            throttle: true,
            steer_q15: Some(-10000),
            ..Default::default()
        })
        .unwrap();
    }
    assert!(unit(&sim, 1).recovery.bike_location.progress > 300);
    assert!(rider(&sim, 1).bike.lateral < 20000 - 7680);
    assert_eq!(unit(&sim, 1).recovery.phase, Phase::Mounted);
}

#[test]
fn published_profile_two_replay_hash_is_unchanged() {
    use replay::{Playback, Replay, frame_ns};
    let (c, mut s) = setup(2);
    s.rules.combat_version = 2;
    s.rules.collision_version = 1;
    let recording = Replay {
        schema_version: 4,
        seed: 3,
        ticks: 1600,
        commands: vec![
            QueuedCommand {
                tick: Tick(0),
                actor: ActorId(1),
                sequence: 0,
                input: request(AttackKind::Kick, Side::Right),
            },
            QueuedCommand {
                tick: Tick(25),
                actor: ActorId(1),
                sequence: 1,
                input: crash_command(),
            },
        ],
        route: None,
        expected_hash: None,
        handling: Some(c),
        encounter: Some(s),
        population: None,
    };
    let encoded = serde_json::to_vec(&recording).unwrap();
    if let Some(path) = std::env::var_os("RR_LEGACY_FIXTURE") {
        std::fs::write(path, &encoded).unwrap();
    }
    let mut p = Playback::new(Replay::read(&encoded).unwrap()).unwrap();
    while !p.done() {
        p.frame(frame_ns("60", p.frames).unwrap()).unwrap();
    }
    // Pinned to the published dba66c6 executable, rather than this implementation.
    assert_eq!(
        p.simulation.state_hash(),
        "e30f2c04243754c538b4380cb835ec79d92a62625b725e47f2fdb7666e3a5b6f"
    );
}
