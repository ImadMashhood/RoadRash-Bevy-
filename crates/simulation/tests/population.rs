use rr_simulation::{
    combat::{AttackKind, Request, Side},
    population::*,
    recovery::{Impact, Outcome, Phase},
    topology::{Location, Path, Portal, Topology},
    *,
};
fn setup(count: usize) -> (riding::Configuration, encounter::Setup, Setup) {
    let c = fixtures::riding();
    let mut e = encounter::Setup::authored(&c).unwrap();
    e.rules.bike_contacts = false;
    e.rules.obstacles.clear();
    e.topology = Topology::straight(100_000_000);
    e.rules.finish_progress = Some(100_000_000);
    e.seeds[0].distance = 1000;
    let mut p = Setup {
        traffic: false,
        periodic_police: false,
        ..Default::default()
    };
    for i in 1..count {
        let mut seed = encounter::Seed::player();
        seed.id = ActorId(i as u64 + 1);
        seed.distance = 1000 + i as i64 * 64;
        seed.lateral = if i % 2 == 0 { -64000 } else { 64000 };
        seed.color = i as u8;
        e.seeds.push(seed);
        p.brains.insert(ActorId(i as u64 + 1), Traits::default());
    }
    (c, e, p)
}
fn sim(count: usize) -> Simulation {
    let (c, e, p) = setup(count);
    Simulation::with_population(1, c, e, p).unwrap()
}
fn user(kind: Kind, progress: i64, lateral: i32, speed: i32) -> Spawn {
    Spawn {
        id: 1,
        kind,
        location: Location {
            progress,
            lateral,
            ..Default::default()
        },
        family: None,
        selector: 0,
        subtype: 0,
        speed,
        direction: 1,
    }
}
fn brain(s: &Simulation, id: u64) -> &Brain {
    &s.population().unwrap().brains[&ActorId(id)]
}
fn unit(s: &Simulation, id: u64) -> &encounter::Unit {
    &s.encounters().unwrap().units[&ActorId(id)]
}
fn rider(s: &Simulation, id: u64) -> riding::Rider {
    s.snapshot()
        .actors
        .iter()
        .find(|a| a.id.0 == id)
        .unwrap()
        .rider
        .clone()
        .unwrap()
}
#[test]
fn cadence_source_boundaries() {
    assert_eq!(live_interval(false, 0, 0, 10000, 0, false), 30);
    assert_eq!(live_interval(true, 0, 0, 0, 0, false), 5);
    assert_eq!(live_interval(true, 0, 0, 0, 300, false), 1);
    assert_eq!(live_interval(true, 0, 0, 10000, 0, false), 1);
    assert_eq!(live_interval(true, 0, 0, 0, 0, true), 1);
    assert_eq!(live_interval(true, 100000, 0, 0, 0, false), 8);
}
#[test]
fn first_think_and_exact_due_tick() {
    let mut s = sim(2);
    s.advance().unwrap();
    assert_eq!(brain(&s, 2).last_tick, Some(0));
    let due = brain(&s, 2).next_tick;
    while s.tick().0 < due {
        s.advance().unwrap();
        assert_eq!(brain(&s, 2).decisions, 1);
    }
    s.advance().unwrap();
    assert_eq!(brain(&s, 2).last_tick, Some(due));
}
#[test]
fn stable_field_order_and_observer_insertion_preserve_rng() {
    let mut a = sim(15);
    let mut b = sim(15);
    b.set_rng_tracing(true);
    b.set_effects_enabled(false);
    for _ in 0..300 {
        let mut view = b.snapshot();
        view.road_users.push(
            World::new(Setup::default())
                .agents
                .values()
                .next()
                .cloned()
                .unwrap_or_else(|| {
                    let mut w = World::new(Setup::default());
                    w.spawn(
                        &user(Kind::Vehicle, 3000, 0, 0),
                        0,
                        &Topology::straight(10000),
                    )
                    .unwrap();
                    w.agents.values().next().unwrap().clone()
                }),
        );
        a.advance().unwrap();
        b.advance().unwrap();
        assert_eq!(a.state_hash(), b.state_hash());
    }
}
#[test]
fn known_corner_brakes_before_curve_and_accelerates_on_straight() {
    for corner in [false, true] {
        let (mut c, mut e, mut p) = setup(2);
        c.specs[0].gears.last_mut().unwrap().up = 30000;
        for g in &mut c.ground {
            g.bend = if corner { 12 } else { 0 };
        }
        e.seeds[1].speed = 9000;
        p.brains.get_mut(&ActorId(2)).unwrap().pace = 130;
        p.brains.get_mut(&ActorId(2)).unwrap().corner_pace = 60;
        let mut s = Simulation::with_population(1, c, e, p).unwrap();
        s.advance().unwrap();
        assert_eq!(brain(&s, 2).target_speed < 9000, corner);
        assert_eq!(brain(&s, 2).command.brake, corner);
        assert_eq!(brain(&s, 2).command.throttle, !corner);
    }
}
#[test]
fn overtakes_slow_vehicle_and_reserves_shared_gap() {
    let (c, mut e, mut p) = setup(3);
    e.seeds[1].distance = 1000;
    e.seeds[1].speed = 10000;
    e.seeds[1].lateral = 0;
    e.seeds[2].distance = 1010;
    e.seeds[2].lateral = 0;
    e.seeds[2].speed = 10000;
    p.spawns = vec![
        user(Kind::Vehicle, 1200, 0, 0),
        Spawn {
            id: 2,
            ..user(Kind::Vehicle, 1200, -64000, 0)
        },
    ];
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    let a = brain(&s, 2);
    let b = brain(&s, 3);
    assert!(a.yielding || b.yielding || a.desired_lane != b.desired_lane);
    assert!(a.desired_lane == 64000 || b.desired_lane == 64000);
}
#[test]
fn blocked_lanes_force_yield() {
    let (c, mut e, mut p) = setup(2);
    e.seeds[1].speed = 10000;
    e.seeds[1].lateral = 0;
    p.spawns = [-64000, 0, 64000]
        .into_iter()
        .enumerate()
        .map(|(i, l)| Spawn {
            id: i as u64 + 1,
            ..user(Kind::Vehicle, 1250, l, 1000)
        })
        .collect();
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    assert!(brain(&s, 2).yielding);
    assert!(brain(&s, 2).target_speed <= 500);
}
#[test]
fn combat_target_cooldown_and_crash_eligibility() {
    let (c, mut e, mut p) = setup(2);
    e.seeds[1].distance = 1000;
    e.seeds[1].lateral = 12000;
    p.brains.get_mut(&ActorId(2)).unwrap().aggression = Some(255);
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    assert_eq!(brain(&s, 2).target, Some(ActorId(1)));
    assert!(brain(&s, 2).attack_after > 0);
    let deadline = brain(&s, 2).attack_after;
    for _ in 0..20 {
        s.advance().unwrap();
    }
    assert_eq!(brain(&s, 2).attack_after, deadline);
    s.queue(QueuedCommand {
        tick: s.tick(),
        actor: ActorId(2),
        sequence: 50,
        input: Command {
            impact: Some(Impact::default()),
            ..Default::default()
        },
    })
    .unwrap();
    s.advance().unwrap();
    for _ in 0..10 {
        s.advance().unwrap();
    }
    assert_ne!(unit(&s, 2).recovery.phase, Phase::Mounted);
    assert_eq!(brain(&s, 2).target, None);
    assert!(unit(&s, 2).fighter.swing.is_none());
}
#[test]
fn crashed_target_is_ineligible_and_removed_target_safe() {
    let (c, mut e, mut p) = setup(3);
    for seed in e.seeds.iter_mut().skip(1) {
        seed.distance = 1000;
        seed.lateral = 12000;
    }
    p.brains.get_mut(&ActorId(2)).unwrap().aggression = Some(0);
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.step(Command {
        impact: Some(Impact::default()),
        ..Default::default()
    })
    .unwrap();
    for _ in 0..12 {
        s.advance().unwrap();
    }
    assert_ne!(brain(&s, 2).target, Some(ActorId(1)));
    s.despawn(ActorId(3)).unwrap();
    s.advance().unwrap();
    assert!(!s.population().unwrap().brains.contains_key(&ActorId(3)));
}
#[test]
fn unknown_personality_traits_remain_unknown() {
    let t = Traits::default();
    assert_eq!(t.personality, None);
    assert_eq!(t.aggression, None);
    assert_eq!(t.kick_roll, None);
    assert_eq!(t.overtake_index, None);
}
#[test]
fn pools_are_bounded_and_ids_do_not_reuse() {
    let mut w = World::new(Setup::default());
    let t = Topology::straight(10000);
    for (kind, cap) in [
        (Kind::Vehicle, 16),
        (Kind::Pedestrian, 15),
        (Kind::Police, 1),
    ] {
        let mut ids = vec![];
        for _ in 0..cap {
            ids.push(w.spawn(&user(kind, 1000, 0, 0), 0, &t).unwrap().unwrap());
        }
        assert!(w.spawn(&user(kind, 1000, 0, 0), 0, &t).unwrap().is_none());
        w.remove(ids[0]);
        assert!(w.spawn(&user(kind, 1000, 0, 0), 0, &t).unwrap().unwrap() > *ids.last().unwrap());
    }
    assert_eq!(w.agents.len(), 32);
}
#[test]
fn zone_entry_exit_no_duplicate_controller() {
    let (c, e, mut p) = setup(1);
    p.zones.push(Zone {
        id: 1,
        path: 0,
        start: 900,
        end: 1100,
        density: 3,
        variant: 1,
        lanes: 2,
        flags: 0,
    });
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    for _ in 0..30 {
        s.advance().unwrap();
        assert_eq!(s.population().unwrap().controllers.len(), 1);
        assert_eq!(s.population().unwrap().timers.density, 3);
    }
}
#[test]
fn density_timer_sign_and_reload_semantics() {
    let mut t = Timers::default();
    assert_eq!(t.reload, 3515);
    t.density(-10);
    assert_eq!(t.rate, 1000);
    assert_eq!(t.reload, 3515);
    t.density(0);
    assert_eq!(t.reload, 1406);
    assert_eq!(t.density, 6);
    t.oncoming = 0;
    assert!(t.advance(0)[0]);
}
#[test]
fn crossing_vehicle_moves_laterally_without_changing_path() {
    let (c, e, mut p) = setup(1);
    p.spawns.push(Spawn {
        subtype: 1,
        ..user(Kind::Vehicle, 2000, -50000, 8000)
    });
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    let first = s.snapshot().road_users[0].clone();
    for _ in 0..10 {
        s.advance().unwrap();
    }
    let current = &s.snapshot().road_users[0];
    assert_eq!(current.location.path, first.location.path);
    assert_eq!(current.location.progress, first.location.progress);
    assert!(current.location.lateral > first.location.lateral);
}
#[test]
fn vehicle_rear_headon_and_crossing_impacts_are_distinct() {
    for (direction, crossing, crash) in [(1, false, false), (-1, false, true), (1, true, true)] {
        let (c, mut e, mut p) = setup(1);
        e.seeds[0].speed = 5000;
        p.spawns.push(Spawn {
            direction,
            subtype: u8::from(crossing),
            ..user(Kind::Vehicle, 1010, 0, 5000)
        });
        let mut s = Simulation::with_population(1, c, e, p).unwrap();
        s.advance().unwrap();
        assert_eq!(unit(&s, 1).recovery.phase == Phase::Sliding, crash);
        assert_eq!(unit(&s, 1).fighter.hit_count, 1);
    }
}
#[test]
fn pedestrians_spawn_once_cross_and_hit_once_per_lifetime() {
    let (c, e, mut p) = setup(1);
    p.spawns.push(Spawn {
        subtype: 5,
        ..user(Kind::Pedestrian, 1000, -4000, 1000)
    });
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    for _ in 0..350 {
        s.advance().unwrap();
    }
    let w = s.population().unwrap();
    assert_eq!(w.consumed.len(), 1);
    assert_eq!(w.pedestrian_hits, 1);
    assert_eq!(w.agents.values().next().unwrap().hit_count, 1);
}
#[test]
fn traffic_interrupts_pedestrian_without_racer_scoring_twice() {
    let (c, e, mut p) = setup(1);
    p.spawns = vec![
        Spawn {
            subtype: 5,
            ..user(Kind::Pedestrian, 2000, -2000, 1000)
        },
        Spawn {
            id: 2,
            ..user(Kind::Vehicle, 2000, 0, 0)
        },
    ];
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    let ped = s
        .population()
        .unwrap()
        .agents
        .values()
        .find(|a| a.kind == Kind::Pedestrian)
        .unwrap();
    assert_eq!(ped.state, AgentState::Hit);
    assert_eq!(ped.hit_count, 1);
    for _ in 0..10 {
        s.advance().unwrap();
    }
    assert_eq!(s.population().unwrap().pedestrian_hits, 1);
}
#[test]
fn source_arrest_thresholds_are_strict() {
    assert!(arrest(0, 299, 500, 1000, 1511, true, false));
    assert!(!arrest(0, 300, 500, 1000, 1511, true, false));
    assert!(!arrest(0, 299, 500, 1000, 1512, true, false));
    assert!(arrest(2, 1799, 0, 1000, 1000, true, false));
    assert!(!arrest(2, 1800, 0, 1000, 1000, true, false));
    assert!(!arrest(0, 0, 5000, 0, 0, true, false));
    assert!(!arrest(0, 0, 0, 0, 0, true, true));
}
#[test]
fn pursuit_enters_detection_and_disengages_with_terminal_cleanup() {
    let (c, e, mut p) = setup(1);
    p.spawns.push(user(Kind::Police, 2000, 64000, 6000));
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    assert_eq!(s.population().unwrap().audible_sirens(false).len(), 1);
    assert!(
        s.population_cues()
            .iter()
            .any(|c| matches!(c, Cue::SirenStart(_)))
    );
    s.step(Command {
        terminal: Some(Outcome::Finished),
        ..Default::default()
    })
    .unwrap();
    assert!(s.population().unwrap().audible_sirens(false).is_empty());
    assert!(
        s.population_cues()
            .iter()
            .any(|c| matches!(c, Cue::SirenStop(_)))
    );
}
#[test]
fn siren_pause_resume_remove_restart_reconcile() {
    let mut w = World::new(Setup::default());
    let id = w
        .spawn(
            &user(Kind::Police, 1000, 0, 0),
            0,
            &Topology::straight(10000),
        )
        .unwrap()
        .unwrap();
    w.agents.get_mut(&id).unwrap().siren = true;
    assert_eq!(w.audible_sirens(false), vec![id]);
    assert!(w.audible_sirens(true).is_empty());
    assert_eq!(w.audible_sirens(false), vec![id]);
    w.reset();
    assert_eq!(w.cues, vec![Cue::SirenStop(id)]);
    assert!(w.agents.is_empty());
    let id = w
        .spawn(
            &user(Kind::Police, 1000, 0, 0),
            0,
            &Topology::straight(10000),
        )
        .unwrap()
        .unwrap();
    w.agents.get_mut(&id).unwrap().siren = true;
    w.remove(id);
    assert_eq!(w.cues.last(), Some(&Cue::SirenStop(id)));
}
#[test]
fn arrest_cleans_up_detached_recovery() {
    let (c, e, mut p) = setup(1);
    p.spawns.push(user(Kind::Police, 1010, 0, 0));
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.step(Command {
        impact: Some(Impact {
            rider_velocity: Some([0, 0, 0]),
            bike_velocity: Some([0, 0, 0]),
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(unit(&s, 1).recovery.outcome, Some(Outcome::Arrested));
    assert_eq!(unit(&s, 1).recovery.phase, Phase::Ended);
    assert!(unit(&s, 1).fighter.swing.is_none());
    assert_eq!(unit(&s, 1).recovery.body.as_ref().unwrap().motion.speed, 0);
    assert!(s.population().unwrap().audible_sirens(false).is_empty());
}
#[test]
fn invalid_population_fails_before_start() {
    let (c, e, mut p) = setup(2);
    p.brains.get_mut(&ActorId(2)).unwrap().bike = 999;
    assert!(Simulation::with_population(1, c.clone(), e.clone(), p).is_err());
    let mut p = Setup::default();
    p.spawns.push(user(Kind::Police, 1, 0, 0));
    p.spawns.push(user(Kind::Vehicle, 2, 0, 0));
    assert!(Simulation::with_population(1, c, e, p).is_err());
}
#[test]
fn effects_and_rng_trace_do_not_change_dense_scene_state() {
    let (c, mut e, mut p) = setup(15);
    e.rules.bike_contacts = true;
    p.traffic = true;
    p.periodic_police = true;
    for (i, l) in [-64000, 0, 64000].into_iter().enumerate() {
        p.spawns.push(Spawn {
            id: i as u64 + 1,
            ..user(Kind::Vehicle, 1300, l, 1000)
        });
    }
    let mut a = Simulation::with_population(7, c.clone(), e.clone(), p.clone()).unwrap();
    let mut b = Simulation::with_population(7, c, e, p).unwrap();
    b.set_effects_enabled(false);
    b.set_rng_tracing(true);
    for _ in 0..600 {
        a.advance().unwrap();
        b.advance().unwrap();
        assert_eq!(a.state_hash(), b.state_hash());
        assert!(a.snapshot().road_users.len() <= 32);
    }
}
#[test]
fn population_replay_schema_and_legacy_hashes_stay_separate() {
    let (c, e, p) = setup(15);
    let replay = replay::Replay {
        schema_version: 5,
        seed: 7,
        ticks: 600,
        commands: vec![],
        route: None,
        expected_hash: None,
        handling: Some(c),
        encounter: Some(e),
        population: Some(p),
    };
    let bytes = serde_json::to_vec(&replay).unwrap();
    assert!(replay::Replay::read(&bytes).is_ok());
    let mut expected = None;
    for rate in ["30", "60", "144", "irregular"] {
        let r = replay::run(replay.clone(), rate).unwrap();
        if let Some(e) = &expected {
            assert_eq!(&r.tick_hashes, e);
        } else {
            expected = Some(r.tick_hashes);
        }
    }
    let mut wrong = replay;
    wrong.schema_version = 4;
    assert!(replay::Replay::read(&serde_json::to_vec(&wrong).unwrap()).is_err());
}

fn fork_setup(choice: u8) -> (riding::Configuration, encounter::Setup, Setup) {
    let (c, mut e, mut p) = setup(2);
    let mut t = Topology::straight(2048);
    t.paths.get_mut(&0).unwrap().portals = vec![
        Portal {
            end: true,
            target: 1,
            target_end: false,
        },
        Portal {
            end: true,
            target: 2,
            target_end: false,
        },
    ];
    for path in [1, 2] {
        t.paths.insert(
            path,
            Path {
                length: 10000,
                branch: path,
                selected_start: if path == 1 { Some(2048) } else { None },
                rises: vec![],
                portals: vec![Portal {
                    end: false,
                    target: 0,
                    target_end: true,
                }],
            },
        );
    }
    e.topology = t;
    e.seeds[1].distance = 2040;
    e.seeds[1].speed = 10000;
    p.brains.get_mut(&ActorId(2)).unwrap().branch_choice = Some(choice);
    (c, e, p)
}
#[test]
fn route_choices_follow_real_portals_for_both_branches() {
    for choice in [0, 1] {
        let (c, e, p) = fork_setup(choice);
        let mut s = Simulation::with_population(1, c, e, p).unwrap();
        for _ in 0..3 {
            s.advance().unwrap();
        }
        assert_eq!(
            unit(&s, 2).recovery.bike_location.path,
            u32::from(choice) + 1
        );
        assert!(
            s.encounters()
                .unwrap()
                .setup
                .topology
                .valid(unit(&s, 2).recovery.bike_location)
        );
    }
}
#[test]
fn disconnected_roads_do_not_collide_or_trigger_pursuit() {
    let (c, mut e, mut p) = setup(1);
    e.topology.paths.insert(
        1,
        Path {
            length: 100000,
            branch: 1,
            selected_start: None,
            rises: vec![],
            portals: vec![],
        },
    );
    p.spawns = vec![Spawn {
        location: Location {
            path: 1,
            progress: 1000,
            ..Default::default()
        },
        ..user(Kind::Police, 1000, 0, 0)
    }];
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.spawn_road_user(&s.population().unwrap().setup.spawns[0].clone())
        .unwrap();
    for _ in 0..60 {
        s.advance().unwrap();
    }
    assert!(unit(&s, 1).recovery.outcome.is_none());
    assert_eq!(unit(&s, 1).fighter.hit_count, 0);
    assert!(s.population().unwrap().audible_sirens(false).is_empty());
}
#[test]
fn crossing_contacts_on_each_branch_keep_association() {
    for path in [1, 2] {
        let (c, mut e, mut p) = fork_setup(0);
        e.seeds.truncate(1);
        p.brains.clear();
        e.seeds[0].location = Some(Location {
            path,
            progress: 1000,
            ..Default::default()
        });
        p.spawns.clear();
        let mut s = Simulation::with_population(1, c, e, p).unwrap();
        s.spawn_road_user(&Spawn {
            subtype: 1,
            location: Location {
                path,
                progress: 1000,
                lateral: -1000,
                ..Default::default()
            },
            ..user(Kind::Vehicle, 1000, 0, 8000)
        })
        .unwrap();
        s.advance().unwrap();
        assert_eq!(unit(&s, 1).recovery.phase, Phase::Sliding);
        assert_eq!(unit(&s, 1).recovery.bike_location.path, path);
    }
}
#[test]
fn opponent_completes_recovery_then_resumes_decisions() {
    let mut s = sim(2);
    s.queue(QueuedCommand {
        tick: Tick(0),
        actor: ActorId(2),
        sequence: 50,
        input: Command {
            impact: Some(Impact {
                rider_velocity: Some([5000, 0, 0]),
                bike_velocity: Some([0, 0, 0]),
                ..Default::default()
            }),
            ..Default::default()
        },
    })
    .unwrap();
    s.advance().unwrap();
    assert_eq!(unit(&s, 2).recovery.phase, Phase::Sliding);
    let mut remounted = false;
    for _ in 0..1800 {
        s.advance().unwrap();
        if unit(&s, 2).recovery.phase == Phase::Mounted {
            remounted = true;
            break;
        }
    }
    assert!(remounted);
    let before = brain(&s, 2).decisions;
    for _ in 0..35 {
        s.advance().unwrap();
    }
    assert!(brain(&s, 2).decisions > before);
    assert!(brain(&s, 2).target_speed > 0);
}
#[test]
fn arrest_in_each_on_foot_phase_ends_recovery_once() {
    for desired in [
        Phase::GettingUp,
        Phase::Standing,
        Phase::Walking,
        Phase::Mounting,
    ] {
        let mut s = sim(1);
        s.step(Command {
            impact: Some(Impact {
                rider_velocity: Some([5000, 0, 0]),
                bike_velocity: Some([0, 0, 0]),
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
        let mut found = false;
        for _ in 0..1800 {
            if unit(&s, 1).recovery.phase == desired {
                found = true;
                break;
            }
            s.advance().unwrap();
        }
        assert!(found);
        let at = unit(&s, 1).recovery.body.as_ref().unwrap().location;
        s.spawn_road_user(&Spawn {
            location: at,
            ..user(Kind::Police, 0, 0, 0)
        })
        .unwrap();
        s.advance().unwrap();
        assert_eq!(
            unit(&s, 1).recovery.outcome,
            Some(Outcome::Arrested),
            "{desired:?}"
        );
        assert_eq!(
            s.effects()
                .iter()
                .filter(|e| e.kind == combat::EffectKind::Arrest)
                .count(),
            1
        );
        assert_eq!(
            unit(&s, 1)
                .recovery
                .body
                .as_ref()
                .map_or(0, |b| b.motion.speed),
            0
        );
        s.advance().unwrap();
        assert!(s.effects().is_empty());
    }
}
#[test]
fn trace_categories_identify_first_different_draw_without_rng_changes() {
    let (c, e, mut p) = setup(2);
    p.traffic = true;
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.set_rng_tracing(true);
    let mut draws = vec![];
    for _ in 0..450 {
        s.advance().unwrap();
        draws.extend(s.population().unwrap().trace.clone());
    }
    assert!(draws.iter().any(|d| d.category == "traffic_lane"));
    assert!(draws.iter().any(|d| d.category == "vehicle_identity"));
    let mut changed = draws.clone();
    changed[0].value ^= 1;
    let first = draws
        .iter()
        .zip(&changed)
        .position(|(a, b)| a != b)
        .unwrap();
    assert_eq!(first, 0);
    assert_eq!(draws[first].tick, changed[first].tick);
}
#[test]
fn simultaneous_attack_vehicle_and_police_resolve_single_arrest() {
    let (c, mut e, mut p) = setup(2);
    e.seeds[1].distance = 1000;
    e.seeds[1].lateral = 12000;
    p.brains.get_mut(&ActorId(2)).unwrap().aggression = Some(255);
    p.spawns = vec![
        user(Kind::Police, 1000, 24000, 0),
        Spawn {
            id: 2,
            ..user(Kind::Vehicle, 1000, 0, 10000)
        },
    ];
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.step(Command {
        attack: Some(Request {
            kind: AttackKind::Punch,
            side: Side::Right,
        }),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(unit(&s, 1).recovery.outcome, Some(Outcome::Arrested));
    assert_eq!(
        s.effects()
            .iter()
            .filter(|e| e.kind == combat::EffectKind::Arrest)
            .count(),
        1
    );
    assert!(s.population().unwrap().agents.len() <= 32);
}

#[test]
fn crashed_cop_cannot_arrest_or_keep_siren() {
    let (c, e, mut p) = setup(1);
    p.spawns = vec![
        user(Kind::Police, 1000, 0, 0),
        Spawn {
            id: 2,
            ..user(Kind::Vehicle, 1000, 0, 10000)
        },
    ];
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    assert_ne!(unit(&s, 1).recovery.outcome, Some(Outcome::Arrested));
    let cop = s
        .population()
        .unwrap()
        .agents
        .values()
        .find(|a| a.kind == Kind::Police)
        .unwrap();
    assert_eq!(cop.state, AgentState::Hit);
    assert!(!cop.siren);
    assert!(
        s.population_cues()
            .iter()
            .any(|e| matches!(e, Cue::SirenStop(_)))
    );
}
#[test]
fn siren_bus_emits_exactly_one_start_stop_for_each_lifecycle_change() {
    let mut bus = SirenBus::default();
    assert_eq!(bus.reconcile([1]), vec![Cue::SirenStart(1)]);
    assert!(bus.reconcile([1]).is_empty());
    assert_eq!(bus.reconcile([]), vec![Cue::SirenStop(1)]);
    assert!(bus.reconcile([]).is_empty());
    assert_eq!(bus.reconcile([1]), vec![Cue::SirenStart(1)]);
    assert_eq!(
        bus.reconcile([2]),
        vec![Cue::SirenStop(1), Cue::SirenStart(2)]
    );
}
#[test]
fn live_recording_roundtrips_control_stream_and_hash() {
    let mut s = sim(15);
    let mut replay = s.recording_setup(1).unwrap();
    for tick in 0..300 {
        let command = Command {
            throttle: tick < 200,
            brake: tick >= 200,
            steer_q15: Some(if tick % 120 < 60 { 2000 } else { -2000 }),
            ..Default::default()
        };
        replay.commands.push(QueuedCommand {
            tick: Tick(tick),
            actor: ActorId(1),
            sequence: 0,
            input: command,
        });
        s.step(command).unwrap();
    }
    replay.ticks = 300;
    replay.expected_hash = Some(s.state_hash());
    let bytes = serde_json::to_vec(&replay).unwrap();
    let read = replay::Replay::read(&bytes).unwrap();
    assert_eq!(
        replay::run(read, "irregular").unwrap().state_sha256,
        s.state_hash()
    );
    assert!(s.recording_setup(1).is_err());
}

#[test]
fn each_opponent_uses_its_assigned_bike_specification() {
    let (mut c, e, mut p) = setup(2);
    let mut second = c.specs[0].clone();
    second.id = 2;
    second.condition = 321;
    c.specs.push(second);
    p.brains.get_mut(&ActorId(2)).unwrap().bike = 2;
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    assert_eq!(rider(&s, 1).bike.condition, 100);
    assert_eq!(rider(&s, 2).bike.condition, 321);
    for _ in 0..20 {
        s.advance().unwrap();
    }
    assert_eq!(rider(&s, 2).bike.condition, 321);
}
#[test]
fn recovered_opponent_waits_for_a_clear_gap_before_reentry() {
    let mut s = sim(2);
    s.queue(QueuedCommand {
        tick: Tick(0),
        actor: ActorId(2),
        sequence: 50,
        input: Command {
            impact: Some(Impact {
                rider_velocity: Some([5000, 0, 0]),
                bike_velocity: Some([0, 0, 0]),
                ..Default::default()
            }),
            ..Default::default()
        },
    })
    .unwrap();
    s.advance().unwrap();
    for _ in 0..1800 {
        if unit(&s, 2).recovery.phase == Phase::Mounted {
            break;
        }
        s.advance().unwrap();
    }
    let at = unit(&s, 2).recovery.bike_location;
    for (i, lateral) in [-64000, 0, 64000].into_iter().enumerate() {
        s.spawn_road_user(&Spawn {
            id: i as u64 + 1,
            location: Location {
                progress: at.progress + 200,
                lateral,
                ..at
            },
            ..user(Kind::Vehicle, 0, 0, 0)
        })
        .unwrap();
    }
    for _ in 0..35 {
        s.advance().unwrap();
    }
    assert!(brain(&s, 2).yielding);
    assert!(!brain(&s, 2).command.throttle);
}

#[test]
fn event_zone_exit_tears_down_its_controller() {
    let (c, mut e, mut p) = setup(1);
    e.seeds[0].speed = 10000;
    p.zones.push(Zone {
        id: 1,
        path: 0,
        start: 900,
        end: 1100,
        density: 3,
        variant: 1,
        lanes: 2,
        flags: 0,
    });
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    assert_eq!(s.population().unwrap().controllers.len(), 1);
    for _ in 0..10 {
        s.advance().unwrap();
    }
    assert!(s.population().unwrap().controllers.is_empty());
    assert!(s.population().unwrap().active_zones.is_empty());
}

#[test]
fn full_field_with_full_road_user_pools_and_simultaneous_incidents_is_bounded() {
    let (c, mut e, mut p) = setup(15);
    e.rules.bike_contacts = true;
    p.spawns.push(user(Kind::Police, 1000, 24000, 0));
    for i in 0..16 {
        p.spawns.push(Spawn {
            id: i + 2,
            ..user(Kind::Vehicle, 1000, 0, 10000)
        });
    }
    for i in 0..15 {
        p.spawns.push(Spawn {
            id: i + 18,
            subtype: 5,
            ..user(Kind::Pedestrian, 1000, 0, 1000)
        });
    }
    let mut a = Simulation::with_population(7, c.clone(), e.clone(), p.clone()).unwrap();
    let mut b = Simulation::with_population(7, c, e, p).unwrap();
    b.set_rng_tracing(true);
    b.set_effects_enabled(false);
    let attack = Command {
        attack: Some(Request {
            kind: AttackKind::Punch,
            side: Side::Right,
        }),
        ..Default::default()
    };
    a.step(attack).unwrap();
    b.step(attack).unwrap();
    assert_eq!(a.population().unwrap().agents.len(), 32);
    assert_eq!(unit(&a, 1).recovery.outcome, Some(Outcome::Arrested));
    for _ in 0..120 {
        assert_eq!(a.state_hash(), b.state_hash());
        assert!(a.snapshot().actors.len() <= 15);
        assert!(a.snapshot().road_users.len() <= 32);
        a.advance().unwrap();
        b.advance().unwrap();
    }
}

#[test]
fn source_event_mapping_keeps_lane_density_flags_and_runtime_type_distinct() {
    let event = rr_course::Event {
        source_offset: 0,
        start: 3,
        end: 10,
        raw_type: 0,
        parameters: [2, 6, 1],
    };
    let zone = Zone::from_event(1, 2, 10000, &event).unwrap();
    assert_eq!(
        (zone.variant, zone.lanes, zone.density, zone.flags),
        (1, 2, 6, 1)
    );
    assert_eq!((zone.start, zone.end), (768, 2560));
}

#[test]
fn functional_cross_traffic_stays_crosswise_instead_of_becoming_longitudinal() {
    let (c, e, mut p) = setup(1);
    p.functional_traffic = true;
    p.spawns.push(Spawn {
        subtype: 1,
        ..user(Kind::Vehicle, 2000, -200000, 8000)
    });
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    let a = s.population().unwrap().agents.values().next().unwrap();
    assert_eq!(a.state, AgentState::Crossing);
    assert_eq!(a.location.progress, 2000);
    assert_eq!(a.location.lateral, -198000);
}

#[test]
fn functional_vehicle_yields_at_red_then_restores_its_cruise_speed() {
    let (c, e, mut p) = setup(1);
    p.functional_traffic = true;
    p.zones.push(Zone {
        id: 1,
        path: 0,
        start: 900,
        end: 1800,
        density: 1,
        variant: 1,
        lanes: 2,
        flags: 0,
    });
    p.spawns.push(user(Kind::Vehicle, 1100, 64000, 2560));
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    let a = s
        .population()
        .unwrap()
        .agents
        .values()
        .find(|a| a.kind == Kind::Vehicle)
        .unwrap();
    assert_eq!((a.speed, a.state), (0, AgentState::Waiting));
    assert_eq!(a.cruise_speed, Some(2560));
    for _ in 0..780 {
        s.advance().unwrap();
    }
    let a = s
        .population()
        .unwrap()
        .agents
        .values()
        .find(|a| a.kind == Kind::Vehicle)
        .unwrap();
    assert_eq!(a.state, AgentState::Cruise);
    assert_eq!(a.speed, 2560);
    assert!(a.location.progress > 1100);
}

#[test]
fn functional_vehicle_chooses_clear_lane_and_pedestrian_waits_for_approaching_car() {
    let (c, e, mut p) = setup(1);
    p.functional_traffic = true;
    p.spawns = vec![
        user(Kind::Vehicle, 2000, 0, 2560),
        Spawn {
            id: 2,
            subtype: 5,
            ..user(Kind::Pedestrian, 3000, 250000, 1000)
        },
    ];
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    let w = s.population().unwrap();
    assert_eq!(
        w.agents
            .values()
            .find(|a| a.kind == Kind::Pedestrian)
            .unwrap()
            .state,
        AgentState::Waiting
    );
    // Two blocked lanes force the moving vehicle toward the remaining open one.
    let (c, e, mut p) = setup(1);
    p.functional_traffic = true;
    p.spawns = vec![
        user(Kind::Vehicle, 2000, -64000, 2560),
        Spawn {
            id: 2,
            ..user(Kind::Vehicle, 2250, -64000, 0)
        },
        Spawn {
            id: 3,
            ..user(Kind::Vehicle, 2250, 0, 0)
        },
    ];
    let mut s = Simulation::with_population(1, c, e, p).unwrap();
    s.advance().unwrap();
    let a = s
        .population()
        .unwrap()
        .agents
        .values()
        .find(|a| a.cruise_speed == Some(2560))
        .unwrap();
    assert_eq!(a.location.lateral, -62976);
    assert_eq!(
        a.speed, 0,
        "hold forward motion until the current lane is clear"
    );
    assert_eq!(a.location.progress, 2000);
    for _ in 0..150 {
        s.advance().unwrap();
    }
    let a = s
        .population()
        .unwrap()
        .agents
        .values()
        .find(|a| a.cruise_speed == Some(2560))
        .unwrap();
    assert_eq!(a.speed, 2560, "{:?}", s.population().unwrap().agents);
    assert!(a.location.progress > 2000);
}

#[test]
fn modern_opponent_passes_stationary_traffic_instead_of_remaining_queued_forever() {
    let (c, mut e, mut p) = setup(2);
    p.functional_traffic = true;
    e.seeds[0].lateral = 180000;
    e.seeds[1].distance = 1000;
    e.seeds[1].lateral = 64000;
    for (i, lateral) in [-64000, 0, 64000].into_iter().enumerate() {
        p.spawns.push(Spawn {
            id: i as u64 + 1,
            ..user(Kind::Vehicle, 1800, lateral, 0)
        });
    }
    let mut sim = Simulation::with_population(7, c, e, p).unwrap();
    for _ in 0..3000 {
        sim.advance().unwrap();
    }
    assert!(
        rider(&sim, 2).bike.distance_q8 > 15000,
        "{:?}",
        brain(&sim, 2)
    );
    assert!(brain(&sim, 2).decisions > 20);
}

#[test]
fn unused_native_gear_slots_do_not_cap_modern_opponents_at_walking_speed() {
    let (mut c, e, mut p) = setup(2);
    *c.specs[0].gears.last_mut().unwrap() = rr_simulation::handling::Gear {
        ratio: 0,
        force: 0,
        up: 0,
        down: 0,
    };
    p.functional_traffic = true;
    let mut sim = Simulation::with_population(1, c, e, p).unwrap();
    sim.advance().unwrap();
    assert_eq!(brain(&sim, 2).target_speed, 6000);
}

#[test]
fn modern_tactics_avoid_scenery_and_legacy_policy_remains_available() {
    for version in [0, 1] {
        let (c, mut e, mut p) = setup(2);
        p.functional_traffic = true;
        p.tactics_version = version;
        e.seeds[0].distance = 10000;
        e.seeds[1].lateral = 0;
        e.seeds[1].speed = 9000;
        e.rules.obstacles.push(encounter::Obstacle {
            id: 1,
            path: 0,
            progress: 1500,
            half_length: 128,
            left: -16000,
            right: 16000,
            height: 100000,
            kind: encounter::ObstacleKind::Building,
        });
        let mut sim = Simulation::with_population(1, c, e, p).unwrap();
        sim.advance().unwrap();
        if version == 0 {
            assert_eq!(brain(&sim, 2).desired_lane, 0);
        } else {
            assert!(brain(&sim, 2).desired_lane.abs() >= 48000);
        }
    }
}
#[test]
fn unknown_tactics_versions_are_rejected_and_old_setups_omit_the_field() {
    let (c, e, mut p) = setup(2);
    assert!(
        !serde_json::to_string(&p)
            .unwrap()
            .contains("tactics_version")
    );
    p.tactics_version = 2;
    assert!(Simulation::with_population(1, c, e, p).is_err());
}

#[test]
fn aggressive_rider_closes_to_combat_range_while_passive_rider_keeps_passing_lane() {
    for aggression in [0, 255] {
        let (c, mut e, mut p) = setup(2);
        p.functional_traffic = true;
        p.tactics_version = 1;
        e.seeds[1].distance = e.seeds[0].distance;
        e.seeds[0].speed = 9000;
        e.seeds[1].speed = 9000;
        p.brains.get_mut(&ActorId(2)).unwrap().aggression = Some(aggression);
        let mut sim = Simulation::with_population(1, c, e, p).unwrap();
        sim.advance().unwrap();
        let lane = brain(&sim, 2).desired_lane.abs();
        if aggression == 255 {
            assert!((17000..20480).contains(&lane), "{lane}");
        } else {
            assert!(lane >= 48000, "{lane}");
        }
    }
}

#[test]
fn automatic_swing_faces_a_police_target_and_does_not_hit_during_windup() {
    let (c, mut e, mut p) = setup(1);
    e.rules.combat_version = 2;
    e.seeds[0].speed = 4000;
    p.functional_traffic = true;
    p.spawns.push(user(Kind::Police, 1300, 24000, 0));
    let mut sim = Simulation::with_population(1, c, e, p).unwrap();
    // Spawn the cop before asking for a swing; no rider half is available.
    sim.advance().unwrap();
    sim.queue(QueuedCommand {
        tick: sim.tick(),
        actor: ActorId(1),
        sequence: 0,
        input: Command {
            attack: Some(Request {
                kind: AttackKind::Kick,
                side: Side::Auto,
            }),
            ..Default::default()
        },
    })
    .unwrap();
    sim.advance().unwrap();
    let swing = unit(&sim, 1).fighter.swing.as_ref().unwrap();
    assert_eq!(swing.request.side, Side::Right);
    assert!(!swing.definition.active(sim.tick().0 - 1 - swing.started));
    assert_eq!(
        sim.population()
            .unwrap()
            .agents
            .values()
            .next()
            .unwrap()
            .hit_count,
        0
    );
}
