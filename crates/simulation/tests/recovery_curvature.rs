//! Public regressions for native LIVE recovery counterexamples. No source assets.
use rr_simulation::{
    ActorId, fixtures,
    handling::Bike,
    recovery::{Assists, BodyLaw, Impact, Phase, RecoveryEvent, State},
    topology::{Location, Topology},
};

#[test]
fn live_curvature_changes_physical_heading_without_rotating_the_retained_display_pose() {
    // Native terrain case24/tick1: a low-speed fall moves backward one Q8 unit
    // on bend12. atan(12)=7 gives a +224 physical heading; display yaw stays0.
    for version in [1, 2] {
        let mut c = fixtures::riding();
        c.motion_version = 3;
        for ground in &mut c.ground {
            ground.bend = 12;
        }
        let mut angles = vec![0; 257];
        angles[12] = 7;
        let law = BodyLaw {
            source_angles: angles,
            motion_version: 1,
            recovery_version: version,
            ..Default::default()
        };
        let topology = Topology::straight(100_000);
        let at = Location {
            progress: 12_928,
            ..Default::default()
        };
        let mut bike = Bike::new(c.spec(), &c.tables).unwrap();
        bike.distance_q8 = at.progress;
        bike.contact = 37;
        let mut state = State::mounted(ActorId(1), at);
        if version == 2 {
            state.begin_live_source(
                ActorId(1),
                &mut bike,
                &c,
                at,
                Impact {
                    reason: 4,
                    ..Default::default()
                },
                &topology,
            );
        } else {
            state.begin_source(
                ActorId(1),
                &mut bike,
                &c,
                at,
                Impact {
                    reason: 4,
                    ..Default::default()
                },
                &topology,
            );
        }
        state
            .step_with_random(
                &mut bike,
                &c,
                &topology,
                &law,
                Assists::default(),
                None,
                0,
                || 1,
            )
            .unwrap();
        let body = state.body.as_ref().unwrap();
        assert_eq!(body.location.progress, 12_927);
        assert_eq!(body.location.heading, if version == 2 { 224 } else { 0 });
        assert_eq!(
            body.visual_heading,
            if version == 2 { Some(0) } else { None }
        );
    }
}

#[test]
fn a_new_live_crash_after_remount_retains_native_frame_selection_and_quiet_queue() {
    // Actual remount/static-contact case353: the retained mount descriptor is
    // still visible at the new crash entry, before the new fall's first advance.
    let c = fixtures::riding();
    let topology = Topology::straight(100_000);
    let at = Location {
        progress: 12_928,
        ..Default::default()
    };
    let mut bike = Bike::new(c.spec(), &c.tables).unwrap();
    bike.distance_q8 = at.progress;
    let mut state = State::mounted(ActorId(1), at);
    state.begin_live_source(
        ActorId(1),
        &mut bike,
        &c,
        at,
        Impact {
            reason: 4,
            ..Default::default()
        },
        &topology,
    );
    state.phase = Phase::Mounting;
    let body = state.body.as_mut().unwrap();
    body.animation.code = 2;
    body.animation.selected = Some((41, 0));
    let law = BodyLaw {
        motion_version: 1,
        recovery_version: 2,
        ..Default::default()
    };
    let events = state
        .step_with_random(
            &mut bike,
            &c,
            &topology,
            &law,
            Assists::default(),
            None,
            0,
            || 1,
        )
        .unwrap();
    assert_eq!(events, vec![RecoveryEvent::Remount]);
    assert!(state.body.is_none());
    assert_eq!(state.bike_animation.queued, Some(0));
    assert!(state.bike_animation.selected.is_none());
    bike.previous_above = 0;
    assert!(state.begin_live_source(
        ActorId(1),
        &mut bike,
        &c,
        state.bike_location,
        Impact::default(),
        &topology
    ));
    let body = state.body.as_ref().unwrap();
    assert_eq!(body.animation.selected, Some((41, 0)));
    assert_eq!(body.previous_location.unwrap().height, 0);
    assert_eq!(state.bike_animation.queued, Some(0));
}

#[test]
fn live_quiet_bike_keeps_shifting_gears_during_the_detached_episode() {
    // Native complete-terrain case1/tick1 shifts the riderless bike to gear1.
    // Its retained gear also governs the first mounted physics after pickup.
    for version in [1, 2] {
        let c = fixtures::riding();
        let topology = Topology::straight(100_000);
        let at = Location {
            progress: 12_928,
            ..Default::default()
        };
        let mut bike = Bike::new(c.spec(), &c.tables).unwrap();
        bike.distance_q8 = at.progress;
        bike.speed = 8000;
        let mut state = State::mounted(ActorId(1), at);
        let impact = Impact {
            reason: 2,
            ..Default::default()
        };
        if version == 2 {
            state.begin_live_source(ActorId(1), &mut bike, &c, at, impact, &topology);
        } else {
            state.begin_source(ActorId(1), &mut bike, &c, at, impact, &topology);
        }
        let law = BodyLaw {
            motion_version: 1,
            recovery_version: version,
            ..Default::default()
        };
        state
            .step_with_random(
                &mut bike,
                &c,
                &topology,
                &law,
                Assists::default(),
                None,
                0,
                || 1,
            )
            .unwrap();
        assert_eq!(bike.gear, usize::from(version == 2));
    }
}

#[test]
fn riding_controls_stay_quarantined_through_the_live_remount_frame() {
    // These are raw riding controls with no separate Walk command. The app
    // intentionally maps on-foot directional input to Walk instead.
    use rr_simulation::{
        Command, Simulation,
        animation::Weapon,
        combat::{AttackKind, Request, Side},
        encounter,
    };
    let c = fixtures::riding();
    let mut setup = encounter::Setup::authored(&c).unwrap();
    setup.rules.combat_version = 4;
    setup.rules.body.motion_version = 1;
    setup.rules.body.recovery_version = 2;
    let mut neutral = Simulation::with_encounters(1, c.clone(), setup.clone()).unwrap();
    neutral
        .step(Command {
            impact: Some(Impact {
                reason: 4,
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
    let mut held = Simulation::with_encounters(1, c, setup).unwrap();
    held.step(Command {
        impact: Some(Impact {
            reason: 4,
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    let input = Command {
        throttle: true,
        steer: 1,
        attack: Some(Request {
            kind: AttackKind::Punch,
            side: Side::Left,
        }),
        weapon: Some(Weapon::Fists),
        ..Default::default()
    };
    let mut remounted = false;
    for _ in 0..3000 {
        neutral.step(Command::default()).unwrap();
        held.step(input).unwrap();
        let n = neutral.snapshot();
        let h = held.snapshot();
        assert_eq!(
            n.actors[0].rider.as_ref().unwrap().bike,
            h.actors[0].rider.as_ref().unwrap().bike
        );
        assert_eq!(
            n.actors[0].encounter.as_ref().unwrap().recovery,
            h.actors[0].encounter.as_ref().unwrap().recovery
        );
        if n.actors[0].encounter.as_ref().unwrap().recovery.phase == Phase::Mounted {
            remounted = true;
            break;
        }
    }
    assert!(remounted, "unassisted recovery must complete");
    neutral.step(Command::default()).unwrap();
    held.step(input).unwrap();
    assert!(
        held.snapshot().actors[0].rider.as_ref().unwrap().bike.force
            > neutral.snapshot().actors[0]
                .rider
                .as_ref()
                .unwrap()
                .bike
                .force
    );
}

fn walking_fixture(
    owner: ActorId,
    version: u8,
) -> (
    rr_simulation::riding::Configuration,
    Topology,
    Bike,
    State,
    BodyLaw,
) {
    let c = fixtures::riding();
    let topology = Topology::straight(100_000);
    let at = Location {
        progress: 13_800,
        ..Default::default()
    };
    let mut bike = Bike::new(c.spec(), &c.tables).unwrap();
    bike.distance_q8 = at.progress;
    let mut state = State::mounted(owner, at);
    if version == 2 {
        state.begin_live_source(
            owner,
            &mut bike,
            &c,
            at,
            Impact {
                reason: 4,
                ..Default::default()
            },
            &topology,
        );
    } else {
        state.begin_source(
            owner,
            &mut bike,
            &c,
            at,
            Impact {
                reason: 4,
                ..Default::default()
            },
            &topology,
        );
    }
    bike.height = 0;
    state.bike_location.height = 0;
    state.phase = Phase::Walking;
    state.elapsed = 123;
    let body = state.body.as_mut().unwrap();
    body.location.progress = 12_800;
    body.location.height = 0;
    body.motion.distance_q8 = 12_800;
    body.motion.height = 0;
    body.motion.speed = 400;
    body.motion.vertical_speed = 0;
    body.motion.contact = 74;
    body.animation.set(&c.animation, 0, 38);
    body.animation.queued = None;
    body.animation.deadline = Some(1000);
    let law = BodyLaw {
        recovery_version: version,
        motion_version: 1,
        ..Default::default()
    };
    (c, topology, bike, state, law)
}

#[test]
fn zero_manual_walk_means_source2_auto_return_and_preserves_source1_hold() {
    use rr_simulation::recovery::Walk;
    for version in [1, 2] {
        let (c, topology, mut bike, mut state, law) = walking_fixture(ActorId(1), version);
        let mut auto = state.clone();
        let mut auto_bike = bike.clone();
        state
            .step_with_random(
                &mut bike,
                &c,
                &topology,
                &law,
                Assists::default(),
                Some(Walk::default()),
                0,
                || 1,
            )
            .unwrap();
        auto.step_with_random(
            &mut auto_bike,
            &c,
            &topology,
            &law,
            Assists::default(),
            None,
            0,
            || 1,
        )
        .unwrap();
        if version == 2 {
            assert_eq!(state.body, auto.body);
            assert_eq!(state.elapsed, auto.elapsed);
            assert!(state.body.as_ref().unwrap().motion.speed > 0);
        } else {
            assert_eq!(state.body.as_ref().unwrap().motion.speed, 0);
            assert!(auto.body.as_ref().unwrap().motion.speed > 0);
        }
    }
}

#[test]
fn source2_manual_walk_is_player_only_and_uses_native_diagonal_facing() {
    use rr_simulation::recovery::Walk;
    let (c, topology, mut bike, mut state, law) = walking_fixture(ActorId(2), 2);
    let mut auto = state.clone();
    let mut auto_bike = bike.clone();
    let walk = Some(Walk {
        longitudinal: -1,
        lateral: 1,
    });
    state
        .step_with_random(
            &mut bike,
            &c,
            &topology,
            &law,
            Assists::default(),
            walk,
            0,
            || 1,
        )
        .unwrap();
    auto.step_with_random(
        &mut auto_bike,
        &c,
        &topology,
        &law,
        Assists::default(),
        None,
        0,
        || 1,
    )
    .unwrap();
    assert_eq!(state.body, auto.body);
    assert_eq!(state.elapsed, auto.elapsed);
    let (c, topology, mut bike, mut state, law) = walking_fixture(ActorId(1), 2);
    state
        .step_with_random(
            &mut bike,
            &c,
            &topology,
            &law,
            Assists::default(),
            Some(Walk {
                longitudinal: 1,
                lateral: -1,
            }),
            0,
            || 1,
        )
        .unwrap();
    assert_eq!(state.body.as_ref().unwrap().location.heading, -0x200000);
    assert_eq!(state.body.as_ref().unwrap().visual_heading, Some(-0x200000));
    assert_eq!(state.elapsed, 1);
}

#[test]
fn live_quiet_motion_refreshes_cached_yaw_and_its_contact_slip_byte() {
    // Native quiet-callback extension: prior mounted yaw/slip must not survive
    // a grounded quiet move; gentle quiet contact then sets the source byte1.
    for version in [1, 2] {
        for gentle in [false, true] {
            let c = fixtures::riding();
            let topology = Topology::straight(100_000);
            let at = Location {
                progress: 12_928,
                ..Default::default()
            };
            let mut bike = Bike::new(c.spec(), &c.tables).unwrap();
            bike.distance_q8 = at.progress;
            bike.speed = 5000;
            bike.contact = 37;
            bike.source_motion_flags = Some(0);
            bike.source_yaw = Some(0x180000);
            bike.skid = 10;
            let mut state = State::mounted(ActorId(1), at);
            let impact = Impact {
                reason: 4,
                ..Default::default()
            };
            if version == 2 {
                state.begin_live_source(ActorId(1), &mut bike, &c, at, impact, &topology);
            } else {
                state.begin_source(ActorId(1), &mut bike, &c, at, impact, &topology);
            }
            if gentle {
                bike.height = c.spec().suspension + 1;
                bike.vertical_speed = -1;
                bike.contact = 0;
            }
            let law = BodyLaw {
                motion_version: 1,
                recovery_version: version,
                ..Default::default()
            };
            state
                .step_with_random(
                    &mut bike,
                    &c,
                    &topology,
                    &law,
                    Assists::default(),
                    None,
                    0,
                    || 1,
                )
                .unwrap();
            assert_eq!(
                bike.source_yaw,
                Some(if version == 2 { 0 } else { 0x180000 })
            );
            assert_eq!(bike.skid, if version == 2 { u8::from(gentle) } else { 10 });
        }
    }
}
