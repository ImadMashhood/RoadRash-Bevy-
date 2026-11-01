use rr_simulation::{arithmetic::*, replay::*, *};
use std::collections::BTreeMap;
fn throttle() -> Command {
    Command {
        throttle: true,
        ..Default::default()
    }
}
fn fork() -> Route {
    Route {
        id: RouteId(7),
        root: SectionId(0),
        sections: BTreeMap::from([
            (
                SectionId(0),
                Section {
                    id: SectionId(0),
                    length: Millimetres(40),
                    next: vec![
                        Edge {
                            to: SectionId(1),
                            branch: BranchId(1),
                        },
                        Edge {
                            to: SectionId(2),
                            branch: BranchId(2),
                        },
                    ],
                },
            ),
            (
                SectionId(1),
                Section {
                    id: SectionId(1),
                    length: Millimetres(60),
                    next: vec![Edge {
                        to: SectionId(3),
                        branch: BranchId(0),
                    }],
                },
            ),
            (
                SectionId(2),
                Section {
                    id: SectionId(2),
                    length: Millimetres(100),
                    next: vec![Edge {
                        to: SectionId(3),
                        branch: BranchId(0),
                    }],
                },
            ),
            (
                SectionId(3),
                Section {
                    id: SectionId(3),
                    length: Millimetres(40),
                    next: vec![],
                },
            ),
        ]),
    }
}
#[test]
fn units_and_signed_arithmetic_have_explicit_boundary_semantics() {
    assert_eq!(Millimetres::from_metres(-5).unwrap(), Millimetres(-5000));
    assert_eq!(Millimetres(-1999).whole_metres(), -1);
    assert!(Millimetres::from_metres(i64::MAX).is_err());
    assert_eq!(wrapping_add(i32::MAX, 1), i32::MIN);
    assert_eq!(wrapping_mul(i32::MAX, 2), -2);
    assert_eq!(q16_mul(-32768, 65536), -32768);
    assert_eq!(floor_q16(-1), -1);
    assert_eq!(truncate_shift(-3, 1).unwrap(), -1);
    assert_eq!(truncate_shift(i32::MIN, 31).unwrap(), -1);
    assert!(truncate_shift(1, 32).is_err());
    assert!(divide(i32::MIN, -1).is_err());
    assert!(divide(5, 0).is_err());
    assert_eq!(divide(-7, 3).unwrap(), -2);
}
#[test]
fn actor_ids_survive_despawn_without_reuse() {
    let mut s = Simulation::default();
    let two = s.spawn().unwrap();
    assert_eq!(two, ActorId(2));
    s.despawn(two).unwrap();
    assert_eq!(s.spawn().unwrap(), ActorId(3));
    assert_eq!(
        s.snapshot().actors.iter().map(|a| a.id).collect::<Vec<_>>(),
        [ActorId(1), ActorId(3)]
    );
    assert!(s.despawn(two).is_err());
}
#[test]
fn both_branches_merge_and_finish_without_flat_z_aliasing() {
    let mut left = Simulation::new(1, fork()).unwrap();
    let mut right = Simulation::new(1, fork()).unwrap();
    for _ in 0..3 {
        left.step(Command {
            fork: Some(0),
            ..throttle()
        })
        .unwrap();
        right
            .step(Command {
                fork: Some(1),
                ..throttle()
            })
            .unwrap();
    }
    assert_eq!(left.snapshot().actors[0].position.branch, BranchId(1));
    assert_eq!(right.snapshot().actors[0].position.branch, BranchId(2));
    assert_ne!(
        left.snapshot().actors[0].position.section,
        right.snapshot().actors[0].position.section
    );
    for _ in 0..20 {
        left.step(throttle()).unwrap();
        right
            .step(Command {
                fork: Some(1),
                ..throttle()
            })
            .unwrap();
    }
    for s in [&left, &right] {
        let p = &s.snapshot().actors[0];
        assert_eq!(p.position.section, SectionId(3));
        assert_eq!(p.position.branch, BranchId(0));
        assert!(p.finished);
    }
    assert_eq!(left.snapshot().distance_mm, 140);
    assert_eq!(right.snapshot().distance_mm, 180);
}
#[test]
fn invalid_route_cycles_unreachable_and_dangling_edges_fail() {
    let mut r = fork();
    r.sections.get_mut(&SectionId(3)).unwrap().next.push(Edge {
        to: SectionId(0),
        branch: BranchId(0),
    });
    assert!(r.validate().is_err());
    let mut r = fork();
    r.sections.get_mut(&SectionId(3)).unwrap().next.push(Edge {
        to: SectionId(99),
        branch: BranchId(0),
    });
    assert!(r.validate().is_err());
    let mut r = fork();
    r.sections.insert(
        SectionId(99),
        Section {
            id: SectionId(99),
            length: Millimetres(1),
            next: vec![],
        },
    );
    assert!(r.validate().is_err());
}
#[test]
fn commands_are_tick_targeted_and_sequence_ordered() {
    let mut a = Simulation::default();
    let mut b = Simulation::default();
    let one = QueuedCommand {
        tick: Tick(0),
        actor: ActorId(1),
        sequence: 1,
        input: throttle(),
    };
    let two = QueuedCommand {
        tick: Tick(0),
        actor: ActorId(1),
        sequence: 2,
        input: Command {
            brake: true,
            ..Default::default()
        },
    };
    a.queue(two.clone()).unwrap();
    a.queue(one.clone()).unwrap();
    b.queue(one.clone()).unwrap();
    b.queue(two.clone()).unwrap();
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.queue(one.clone()), Err(Error::DuplicateCommand));
    a.advance().unwrap();
    b.advance().unwrap();
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.snapshot().speed_mm_per_tick, 0);
    assert_eq!(a.queue(one), Err(Error::PastCommand));
}
#[test]
fn invalid_input_does_not_mutate_a_queued_world() {
    let mut s = Simulation::default();
    let old = s.state_hash();
    assert!(
        s.queue(QueuedCommand {
            tick: Tick(0),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                steer: 127,
                ..Default::default()
            }
        })
        .is_err()
    );
    assert_eq!(s.state_hash(), old);
    assert!(
        s.queue(QueuedCommand {
            tick: Tick(0),
            actor: ActorId(99),
            sequence: 0,
            input: throttle()
        })
        .is_err()
    );
}
#[test]
fn presentation_snapshot_changes_cannot_mutate_simulation() {
    let mut s = Simulation::default();
    s.step(throttle()).unwrap();
    let before = s.state_hash();
    let mut view = s.snapshot();
    view.actors[0].position.lateral = Millimetres(900000);
    view.distance_mm = u64::MAX;
    assert_eq!(s.state_hash(), before);
    assert_ne!(view, s.snapshot());
}
#[test]
fn stage_order_is_independent_of_registration_order() {
    let mut reverse = STAGES.to_vec();
    reverse.reverse();
    assert_eq!(ordered_stages(reverse), STAGES);
    for offset in 0..STAGES.len() {
        let mut registration = STAGES.to_vec();
        registration.rotate_left(offset);
        assert_eq!(ordered_stages(registration), STAGES);
    }
    let mut s = Simulation::default();
    s.advance().unwrap();
    assert_eq!(s.stages(), STAGES);
}
#[test]
fn seeded_streams_are_reproducible_and_spawn_draws_are_isolated() {
    let mut a = Simulation::new(1, Route::straight()).unwrap();
    let mut b = Simulation::new(1, Route::straight()).unwrap();
    a.spawn().unwrap();
    let mut output = vec![];
    for _ in 0..5 {
        let av = a.draw_gameplay_random();
        assert_eq!(av, b.draw_gameplay_random());
        output.push(av);
    }
    assert_eq!(output, [41, 18467, 6334, 26500, 19169]);
}
#[test]
fn rendering_frequency_and_local_effect_rng_do_not_change_hashes() {
    let replay = Replay::read(include_bytes!("../../../docs/fixtures/replay.json")).unwrap();
    let reports: Vec<_> = ["30", "60", "144", "irregular"]
        .into_iter()
        .map(|rate| run(replay.clone(), rate).unwrap())
        .collect();
    assert!(
        reports
            .windows(2)
            .all(|r| r[0].state_sha256 == r[1].state_sha256)
    );
    assert_ne!(
        reports[0].presentation_frames,
        reports[2].presentation_frames
    );
    let mut a = Simulation::default();
    let mut b = Simulation::default();
    let mut decorative_rng = 7u64;
    for i in 0..120 {
        a.step(throttle()).unwrap();
        b.step(throttle()).unwrap();
        if i % 3 == 0 {
            let _ = a.snapshot();
            decorative_rng = decorative_rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
        }
    }
    assert_eq!(a.state_hash(), b.state_hash());
    assert_ne!(decorative_rng, 7);
}
#[test]
fn canonical_encoding_excludes_addresses_and_tracks_future_input() {
    let mut a = Simulation::default();
    let b = Simulation::default();
    assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    assert_eq!(&a.canonical_bytes()[..4], &2u32.to_le_bytes());
    let prior = a.state_hash();
    a.queue(QueuedCommand {
        tick: Tick(10),
        actor: ActorId(1),
        sequence: 2,
        input: throttle(),
    })
    .unwrap();
    assert_ne!(a.state_hash(), prior);
}
#[test]
fn replay_schema_duration_and_hash_mismatch_fail() {
    assert!(
        Replay::read(
            br#"{"schema_version":999,"seed":1,"ticks":1,"route":null,"expected_hash":null}"#
        )
        .is_err()
    );
    let mut r = Replay::read(include_bytes!("../../../docs/fixtures/replay.json")).unwrap();
    r.expected_hash = Some("bad".into());
    assert!(run(r, "60").is_err());
    let r = Replay::read(include_bytes!("../../../docs/fixtures/replay.json")).unwrap();
    assert!(run(r, "bogus").is_err());
}
#[test]
fn braking_and_lateral_bounds_are_integer_stable() {
    let mut s = Simulation::default();
    for _ in 0..10000 {
        s.step(Command {
            steer: 1,
            ..throttle()
        })
        .unwrap();
    }
    assert_eq!(s.snapshot().speed_mm_per_tick, 1000);
    assert_eq!(s.snapshot().lateral_mm, 4000);
    for _ in 0..100 {
        s.step(Command {
            throttle: true,
            brake: true,
            steer: -1,
            ..Default::default()
        })
        .unwrap();
    }
    assert_eq!(s.snapshot().speed_mm_per_tick, 0);
}

#[test]
fn live_held_input_and_recorded_changes_have_equal_final_state() {
    let mut live = Simulation::default();
    let mut recorded = Simulation::default();
    recorded
        .queue(QueuedCommand {
            tick: Tick(0),
            actor: ActorId(1),
            sequence: 0,
            input: throttle(),
        })
        .unwrap();
    for _ in 0..120 {
        live.step(throttle()).unwrap();
        recorded.advance().unwrap();
    }
    assert_eq!(live.state_hash(), recorded.state_hash());
    let before = live.state_hash();
    assert!(
        live.step(Command {
            steer: 127,
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(live.state_hash(), before);
}

#[test]
fn schema_one_replay_retains_its_original_bytes_hash_and_report_version() {
    let replay = Replay::read(include_bytes!("../../../docs/fixtures/replay.json")).unwrap();
    let report = run(replay, "irregular").unwrap();
    assert_eq!(report.simulation_schema, 1);
    assert_eq!(
        report.state_sha256,
        "f70c202279f360fc89e95d503443a91a523c816788d9e62102a6d3572fc7f143"
    );
    let mut old = Simulation::with_schema(1, Route::straight(), 1).unwrap();
    assert!(
        old.step(Command {
            steer_q15: Some(0),
            ..Default::default()
        })
        .is_err()
    );
}
#[test]
fn analog_steering_is_proportional_symmetric_and_full_scale_matches_digital() {
    fn trace(axis: i16) -> Snapshot {
        let mut sim = Simulation::default();
        for _ in 0..30 {
            sim.step(Command {
                throttle: true,
                steer_q15: Some(axis),
                ..Default::default()
            })
            .unwrap();
        }
        sim.snapshot()
    }
    let full = trace(32767);
    let half = trace(16384);
    let left = trace(-16384);
    assert!(half.lateral_mm > 0 && half.lateral_mm < full.lateral_mm);
    assert_eq!(half.lateral_mm, -left.lateral_mm);
    let mut digital = Simulation::default();
    for _ in 0..30 {
        digital
            .step(Command {
                throttle: true,
                steer: 1,
                ..Default::default()
            })
            .unwrap();
    }
    assert_eq!(full, digital.snapshot());
}
#[test]
fn invalid_or_ambiguous_analog_commands_leave_state_unchanged_and_axes_are_hashed() {
    let mut a = Simulation::default();
    let before = a.state_hash();
    for command in [
        Command {
            steer_q15: Some(i16::MIN),
            ..Default::default()
        },
        Command {
            steer: 1,
            steer_q15: Some(1),
            ..Default::default()
        },
    ] {
        assert!(a.step(command).is_err());
        assert_eq!(a.state_hash(), before);
    }
    let mut b = Simulation::default();
    for (sim, axis) in [(&mut a, 100), (&mut b, 200)] {
        sim.queue(QueuedCommand {
            tick: Tick(10),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                steer_q15: Some(axis),
                ..Default::default()
            },
        })
        .unwrap();
    }
    assert_ne!(a.state_hash(), b.state_hash());
    assert_eq!(&a.canonical_bytes()[..4], &2u32.to_le_bytes());
}

#[test]
fn schema_two_analog_fixture_has_identical_checkpoints_at_all_schedules() {
    let bytes = include_bytes!("../../../docs/fixtures/replay-analog.json");
    let mut baseline = None;
    for rate in ["30", "60", "144", "irregular"] {
        let report = run(Replay::read(bytes).unwrap(), rate).unwrap();
        assert_eq!(report.simulation_schema, 2);
        assert_eq!(
            (
                report.final_snapshot.distance_mm,
                report.final_snapshot.speed_mm_per_tick,
                report.final_snapshot.lateral_mm
            ),
            (51150, 300, 7)
        );
        if let Some(ref expected) = baseline {
            assert_eq!(&report.tick_hashes, expected);
        } else {
            baseline = Some(report.tick_hashes);
        }
    }
    let mut invalid: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    invalid["schema_version"] = serde_json::json!(1);
    assert!(Replay::read(&serde_json::to_vec(&invalid).unwrap()).is_err());
}
#[test]
fn forensic_channels_are_schedule_invariant_and_do_not_change_authority() {
    use rr_simulation::replay::{Replay, run_with_tick_hashes};
    let replay = Replay::read(include_bytes!("../../../docs/fixtures/replay.json")).unwrap();
    let ordinary = run_with_tick_hashes(replay.clone(), "60", false).unwrap();
    let a = run_with_tick_hashes(replay.clone(), "30", true).unwrap();
    let b = run_with_tick_hashes(replay.clone(), "144", true).unwrap();
    assert!(ordinary.forensic.is_empty());
    assert_eq!(ordinary.state_sha256, a.state_sha256);
    assert_eq!(a.tick_hashes, b.tick_hashes);
    assert_eq!(a.forensic.len() as u64, replay.ticks);
    assert_eq!(
        serde_json::to_value(a.forensic).unwrap(),
        serde_json::to_value(b.forensic).unwrap()
    );
    let mut oversized = replay;
    oversized.ticks = 100_001;
    assert!(run_with_tick_hashes(oversized, "60", true).is_err());
}
