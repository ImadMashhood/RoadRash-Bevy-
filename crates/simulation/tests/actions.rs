use rr_simulation::{
    actions::*, encounter::Setup, fixtures, population, recovery::Phase, replay::Replay, *,
};
fn config() -> Config {
    Config {
        recharge: 17,
        multiplier_q8: 384,
        maximum: 120,
        charges: 3,
        minimum: 30,
        tail: 15,
    }
}
fn simulation() -> Simulation {
    let mut c = fixtures::riding();
    c.actions = Some(config());
    let e = Setup::authored(&c).unwrap();
    Simulation::with_population(1, c, e, population::Setup::default()).unwrap()
}
#[test]
fn activation_consumes_one_charge_hold_release_and_depletion_follow_source_policy() {
    let c = config();
    let mut s = State::new(c);
    let gas = Command {
        throttle: true,
        nitro: true,
        ..Default::default()
    };
    assert_eq!(s.tick(c, gas, true), 384);
    assert_eq!(s.charges, 2);
    for _ in 0..30 {
        s.tick(c, gas, true);
    }
    assert_eq!(s.charges, 2);
    assert_eq!(
        s.tick(
            c,
            Command {
                nitro: false,
                ..gas
            },
            true
        ),
        384
    );
    assert_eq!(s.tick(c, Command::default(), true), 256);
    assert!(!s.active);
    s.charge_q8 = 1;
    s.active = true;
    s.charges = 0;
    assert_eq!(s.tick(c, gas, true), 256);
    assert!(!s.active);
    s.charges = 2;
    s.charge_q8 = 29 * 256;
    assert_eq!(s.tick(c, gas, true), 256);
    s.charge_q8 = 120 * 256;
    s.charges = 0;
    assert_eq!(s.tick(c, gas, true), 256);
}
#[test]
fn brake_failure_and_ineligible_states_stop_boost_without_negative_inventory() {
    let c = config();
    let mut s = State::new(c);
    let gas = Command {
        throttle: true,
        nitro: true,
        ..Default::default()
    };
    s.tick(c, gas, true);
    assert_eq!(s.tick(c, gas, false), 256);
    assert_eq!(s.tick(c, Command { brake: true, ..gas }, true), 256);
    assert!(s.charges >= 0);
    let absent = Config {
        recharge: 0,
        multiplier_q8: 0,
        maximum: 0,
        charges: 0,
        minimum: 0,
        tail: 0,
    };
    assert_eq!(State::new(absent).tick(absent, gas, true), 256);
}
#[test]
fn schema_six_actions_replay_at_all_render_rates_and_reject_on_old_schema() {
    let sim = simulation();
    assert_eq!(sim.schema_version(), 6);
    let mut replay = sim.recording_setup(1).unwrap();
    replay.ticks = 600;
    replay.commands = vec![
        QueuedCommand {
            tick: Tick(0),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                throttle: true,
                nitro: true,
                lean: 1,
                ..Default::default()
            },
        },
        QueuedCommand {
            tick: Tick(100),
            actor: ActorId(1),
            sequence: 1,
            input: Command::default(),
        },
    ];
    let bytes = serde_json::to_vec(&replay).unwrap();
    let replay = Replay::read(&bytes).unwrap();
    let mut hash = None;
    for rate in ["30", "60", "144", "irregular"] {
        let report = replay::run(replay.clone(), rate).unwrap();
        if let Some(ref h) = hash {
            assert_eq!(&report.state_sha256, h);
        } else {
            hash = Some(report.state_sha256);
        }
    }
    let mut old = Simulation::with_handling(1, fixtures::riding()).unwrap();
    assert!(
        old.step(Command {
            nitro: true,
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        simulation()
            .step(Command {
                lean: 2,
                ..Default::default()
            })
            .is_err()
    );
}
#[test]
fn stationary_dismount_is_edge_triggered_and_retains_body_until_remount() {
    let mut sim = simulation();
    let before = sim.snapshot().actors[0]
        .rider
        .as_ref()
        .unwrap()
        .bike
        .condition;
    sim.step(Command {
        dismount: true,
        ..Default::default()
    })
    .unwrap();
    let s = sim.snapshot();
    let u = s.actors[0].encounter.as_ref().unwrap();
    assert_ne!(u.recovery.phase, Phase::Mounted);
    assert!(u.recovery.body.is_some());
    let incident = u.recovery.incident;
    for _ in 0..60 {
        sim.step(Command {
            dismount: true,
            ..Default::default()
        })
        .unwrap();
    }
    let s = sim.snapshot();
    let u = s.actors[0].encounter.as_ref().unwrap();
    assert_eq!(u.recovery.incident, incident);
    assert_eq!(s.actors[0].rider.as_ref().unwrap().bike.condition, before);
}
#[test]
fn moving_dismount_is_ignored_and_boost_changes_drive_without_presentation_input() {
    let mut boosted = simulation();
    let mut regular = simulation();
    for _ in 0..30 {
        boosted
            .step(Command {
                throttle: true,
                nitro: true,
                dismount: true,
                ..Default::default()
            })
            .unwrap();
        regular.step(fixtures::gas()).unwrap();
    }
    // Start the speed eligibility check in a separate continuously mounted run.
    regular
        .step(Command {
            throttle: true,
            ..Default::default()
        })
        .unwrap();
    let speed = regular.snapshot().actors[0]
        .rider
        .as_ref()
        .unwrap()
        .bike
        .speed;
    assert!(speed >= 300);
    regular
        .step(Command {
            throttle: true,
            dismount: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        regular.snapshot().actors[0]
            .encounter
            .as_ref()
            .unwrap()
            .recovery
            .phase,
        Phase::Mounted
    );
    let mut boosted = simulation();
    let mut normal = simulation();
    for _ in 0..20 {
        boosted
            .step(Command {
                throttle: true,
                nitro: true,
                ..Default::default()
            })
            .unwrap();
        normal.step(fixtures::gas()).unwrap();
    }
    assert!(
        boosted.snapshot().actors[0]
            .rider
            .as_ref()
            .unwrap()
            .bike
            .speed
            > normal.snapshot().actors[0]
                .rider
                .as_ref()
                .unwrap()
                .bike
                .speed
    );
}

#[test]
fn static_obstacle_overlap_survives_recovery_without_restarting_crashes() {
    let mut c = fixtures::riding();
    c.actions = Some(config());
    let mut e = Setup::authored(&c).unwrap();
    e.rules.bike_contacts = false;
    e.seeds[0].distance = 1000;
    e.seeds[0].speed = 7000;
    e.rules.obstacles = vec![encounter::Obstacle {
        source_box: None,
        id: 1,
        path: 0,
        progress: 1020,
        half_length: 64,
        left: -16000,
        right: 16000,
        height: 20000,
        kind: encounter::ObstacleKind::Solid,
    }];
    let mut sim = Simulation::with_population(1, c, e, population::Setup::default()).unwrap();
    let mut crashes = 0;
    let mut remounted = false;
    for _ in 0..2400 {
        sim.step(Command {
            throttle: true,
            ..Default::default()
        })
        .unwrap();
        crashes += sim
            .effects()
            .iter()
            .filter(|e| e.actor == ActorId(1) && e.kind == combat::EffectKind::Crash)
            .count();
        remounted |= sim
            .effects()
            .iter()
            .any(|e| e.kind == combat::EffectKind::Remount);
    }
    assert!(remounted);
    assert_eq!(crashes, 1);
    assert!(
        sim.snapshot().actors[0]
            .rider
            .as_ref()
            .unwrap()
            .bike
            .distance_q8
            > 1500
    );
}

#[test]
fn full_native_replay_budget_grows_only_for_schema_six_and_remains_bounded() {
    let simulation = simulation();
    let mut replay = simulation.recording_setup(1).unwrap();
    replay.ticks = 1;
    let mut bytes = serde_json::to_vec(&replay).unwrap();
    bytes.resize(4 * 1024 * 1024 + 1, b' ');
    assert!(Replay::read(&bytes).is_ok());
    replay.schema_version = 5;
    replay.handling.as_mut().unwrap().actions = None;
    let mut legacy = serde_json::to_vec(&replay).unwrap();
    legacy.resize(4 * 1024 * 1024 + 1, b' ');
    assert!(Replay::read(&legacy).is_err());
    bytes.resize(rr_simulation::replay::MAX_REPLAY_BYTES + 1, b' ');
    assert!(Replay::read(&bytes).is_err());
}
