use rr_simulation::{
    ActorId, fixtures,
    handling::Bike,
    recovery::{Assists, BodyLaw, Impact, Phase, RecoveryEvent, State},
    topology::{Location, Topology},
};

#[test]
fn live_remount_preserves_signed_controller_force_while_source_one_keeps_its_reset() {
    for force in [-1350, 0, 384] {
        for version in [1, 2] {
            let mut config = fixtures::riding();
            config.motion_version = 3;
            let topology = Topology::straight(100000);
            let at = Location {
                progress: 4096,
                ..Default::default()
            };
            let mut bike = Bike::new(config.spec(), &config.tables).unwrap();
            bike.distance_q8 = at.progress;
            let mut recovery = State::mounted(ActorId(1), at);
            let begin = if version == 2 {
                State::begin_live_source
            } else {
                State::begin_source
            };
            assert!(begin(
                &mut recovery,
                ActorId(1),
                &mut bike,
                &config,
                at,
                Impact {
                    reason: 4,
                    ..Default::default()
                },
                &topology,
            ));
            // The body has completed mount animation 41. The next recovery
            // decision rejoins the bike before its mounted motion callback.
            recovery.phase = Phase::Mounting;
            let body = recovery.body.as_mut().unwrap();
            body.animation.set(&config.animation, 0, 2);
            body.animation.queued = Some(2);
            body.motion.speed = 0;
            bike.force = force;
            bike.drive = 72;
            bike.lateral_force = 55;
            bike.lateral_speed = 44;
            bike.lean = 33;
            let law = BodyLaw {
                motion_version: 1,
                recovery_version: version,
                ..Default::default()
            };
            let events = recovery
                .step_with_random(
                    &mut bike,
                    &config,
                    &topology,
                    &law,
                    Assists::default(),
                    None,
                    1,
                    || 7,
                )
                .unwrap();
            assert_eq!(events, [RecoveryEvent::Remount]);
            assert_eq!(recovery.phase, Phase::Mounted);
            assert!(recovery.body.is_none());
            assert!(!bike.crashed);
            assert_eq!(bike.force, if version == 2 { force } else { 0 });
            assert_eq!(
                (
                    bike.drive,
                    bike.lateral_force,
                    bike.lateral_speed,
                    bike.lean
                ),
                (0, 0, 0, 0)
            );
            assert!(
                recovery
                    .step_with_random(
                        &mut bike,
                        &config,
                        &topology,
                        &law,
                        Assists::default(),
                        None,
                        2,
                        || 7,
                    )
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(bike.force, if version == 2 { force } else { 0 });
        }
    }
}

#[test]
fn live_recovery_keeps_ownership_and_ends_without_assists_when_its_bike_is_unreachable() {
    for missing in [false, true] {
        let config = fixtures::riding();
        let mut topology = Topology::straight(100000);
        let mut disconnected = topology.paths[&0].clone();
        disconnected.branch = 1;
        disconnected.selected_start = None;
        topology.paths.insert(1, disconnected);
        let at = Location {
            progress: 4096,
            ..Default::default()
        };
        let mut bike = Bike::new(config.spec(), &config.tables).unwrap();
        let mut recovery = State::mounted(ActorId(1), at);
        assert!(recovery.begin_live_source(
            ActorId(1),
            &mut bike,
            &config,
            at,
            Impact::default(),
            &topology
        ));
        let identity = recovery.body.as_ref().map(|body| (body.owner, body.entity));
        if missing {
            recovery.bike_available = false;
        } else {
            recovery.body.as_mut().unwrap().location.path = 1;
        }
        let law = BodyLaw {
            motion_version: 1,
            recovery_version: 2,
            ..Default::default()
        };
        let events = recovery
            .step_with_random(
                &mut bike,
                &config,
                &topology,
                &law,
                Assists::default(),
                None,
                1,
                || 7,
            )
            .unwrap();
        assert_eq!(events, [RecoveryEvent::Wrecked]);
        assert_eq!(recovery.phase, Phase::Ended);
        assert!(!recovery.assisted);
        assert_eq!(recovery.incident, 1);
        assert_eq!(
            recovery.body.as_ref().map(|body| (body.owner, body.entity)),
            identity
        );
        assert_eq!(recovery.bike_entity, 2);
    }
}
