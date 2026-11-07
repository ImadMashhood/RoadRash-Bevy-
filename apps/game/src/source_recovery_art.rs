//! Controlled original-art recovery episodes; ordinary commands, no injected outcomes.
use super::*;
use rr_simulation::{
    ActorId, QueuedCommand, Tick,
    animation::Weapon,
    combat::{AttackKind, Item, Request, Side, WeaponId},
    recovery::Phase,
};

#[test]
#[ignore = "requires owned original content and full recovery sequence output"]
fn source_player_recovery_native_art_episodes() {
    let output = PathBuf::from(std::env::var_os("RR_RECOVERY_ART_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let stems =
        std::env::var("RR_RECOVERY_ART_COURSES").unwrap_or("CITY,HIWAY,MEDLY,NAPA,CANYN".into());
    let wreck_mode = std::env::var_os("RR_RECOVERY_ART_WRECK").is_some();
    let mut cases = vec![];
    for stem in stems.split(',') {
        let options = LaunchOptions {
            data_root: Some(std::env::var_os("RR_SESSION_DATA").unwrap().into()),
            catalog: Some(std::env::var_os("RR_SESSION_CATALOG").unwrap().into()),
            course: Some(stem.into()),
            level: 1,
            bike: 1,
            practice_riders: 1,
            condition_percent: if wreck_mode { 1 } else { 100 },
            effects: true,
            ..Default::default()
        };
        let mut presentation = course_view(&options, &VisualAssets::default())
            .unwrap()
            .unwrap();
        let mut record = build_race(&options, Some(&presentation), 3, Some(1))
            .0
            .recording_setup(3)
            .unwrap();
        super::source_combat_art::load_actor_palettes(
            &mut presentation,
            &record,
            options.data_root.as_ref().unwrap(),
        );
        let config = record.handling.as_ref().unwrap();
        let start = {
            (config.start_sample as usize + 1..config.ground.len() - 32)
                .find(|start| {
                    config.ground[*start..*start + 8]
                        .iter()
                        .all(|g| g.rise.abs() <= 20 && g.bend.abs() <= 8 && g.right - g.left >= 300)
                        && config.ground[*start..*start + 32]
                            .windows(2)
                            .any(|g| g[0].rise != g[1].rise || g[0].bend != g[1].bend)
                })
                .expect("a gentle approach followed by changing source terrain")
        };
        let distance = (start as i64 - i64::from(config.start_sample)) * 256;
        let center = (config.ground[start].left + config.ground[start].right) * 128;
        let setup = record.encounter.as_mut().unwrap();
        assert_eq!(setup.rules.body.recovery_version, 2);
        assert_eq!(setup.rules.body.source_angles.len(), 257);
        assert!(!setup.rules.assists.fast_recovery && !setup.rules.assists.rescue_unreachable);
        setup.inventory.items.clear();
        setup.inventory.items.insert(
            WeaponId(100),
            Item {
                kind: Weapon::Chain,
                owner: Some(ActorId(2)),
            },
        );
        for (index, seed) in setup.seeds.iter_mut().enumerate() {
            seed.distance = distance;
            seed.lateral = center + if index == 0 { 0 } else { 16000 };
            seed.speed = 5000;
            if wreck_mode && index == 0 {
                assert_eq!(
                    stem, "CITY",
                    "the controlled corner approach uses source City"
                );
                seed.speed = 12000;
            }
            seed.location = Some(setup.topology.locate(distance, seed.lateral, 0).unwrap());
        }
        setup.seeds[0].stats.health = 1;
        if wreck_mode {
            let population = record.population.as_mut().unwrap();
            population.functional_traffic = true;
            population.tactics_version = 1;
            population.vehicle_types = presentation.scene.assets.vehicles.len() as u16;
            population.vehicle_masks = presentation.scene.assets.vehicle_masks.clone();
            population.spawns = vec![rr_simulation::population::Spawn {
                id: 1,
                kind: rr_simulation::population::Kind::Vehicle,
                location: setup.topology.locate(distance + 400, center, 0).unwrap(),
                family: None,
                selector: 0,
                subtype: 1,
                speed: 0,
                direction: 1,
            }];
        }
        let mut sim = Simulation::with_population(
            record.seed,
            config.clone(),
            setup.clone(),
            record.population.as_ref().unwrap().clone(),
        )
        .unwrap();
        let mut first_crash = None;
        let mut remount = None;
        let mut wrecked = None;
        let mut previous = None;
        let mut phases = std::collections::BTreeSet::new();
        let mut frames = vec![];
        for tick in 0..=1800 {
            let canonical = sim.snapshot();
            let unit = canonical
                .actors
                .iter()
                .find(|a| a.id == ActorId(1))
                .unwrap()
                .encounter
                .as_ref()
                .unwrap();
            let phase = unit.recovery.phase;
            if unit.recovery.outcome == Some(rr_simulation::recovery::Outcome::Wrecked) {
                wrecked.get_or_insert(tick);
            }
            if unit.recovery.body.is_some() {
                first_crash.get_or_insert(tick);
            }
            if first_crash.is_some() && phase == Phase::Mounted {
                remount.get_or_insert(tick);
            }
            let phase_name = format!("{phase:?}");
            phases.insert(phase_name);
            if previous != Some(phase)
                || tick % 30 == 0
                || remount.is_some_and(|at| tick == at + 24)
                || wrecked.is_some_and(|at| tick == at + 330)
            {
                let snapshot = rr_render::camera::route_encounter_snapshot(
                    &presentation.scene.route,
                    &canonical,
                );
                let view = rr_render::camera::route_snapshot_view(
                    &presentation.scene.route,
                    &snapshot,
                    &snapshot,
                    1.,
                    presentation.start_q8,
                );
                let camera = rr_render::camera::route_camera_spec(
                    &presentation.scene.route,
                    &presentation.camera,
                    &snapshot,
                    &snapshot,
                    1.,
                );
                let (frame, receipt) = presentation
                    .scene
                    .draw(&camera, &view, &snapshot, false)
                    .unwrap();
                let image = format!("{stem}-{tick}.png");
                std::fs::write(output.join(&image), frame.image.png().unwrap()).unwrap();
                frames.push(serde_json::json!({"tick":tick,"image":image,"player":canonical.actors.iter().find(|a|a.id==ActorId(1)),"camera":camera,"view":view,"actor_frames":receipt.actor_frames}));
            }
            previous = Some(phase);
            if (!wreck_mode && remount.is_some_and(|at| tick >= at + 24))
                || (wreck_mode && wrecked.is_some_and(|at| tick >= at + 330))
                || tick == 1800
            {
                record.ticks = tick;
                break;
            }
            for actor in [ActorId(1), ActorId(2)] {
                let downed = first_crash.is_some();
                let input = if actor == ActorId(1) {
                    Command {
                        throttle: true,
                        ..Default::default()
                    }
                } else {
                    Command {
                        throttle: !downed,
                        brake: downed,
                        steer_q15: downed.then_some(4000),
                        attack: (!downed && !wreck_mode).then_some(Request {
                            kind: AttackKind::Punch,
                            side: Side::Left,
                        }),
                        ..Default::default()
                    }
                };
                let command = QueuedCommand {
                    tick: Tick(tick),
                    actor,
                    sequence: 0,
                    input,
                };
                record.commands.push(command.clone());
                sim.queue(command).unwrap();
            }
            sim.advance().unwrap();
        }
        record.expected_hash = Some(sim.state_hash());
        let bytes = serde_json::to_vec(&record).unwrap();
        std::fs::write(output.join(format!("{stem}.json")), &bytes).unwrap();
        for rate in ["30", "60", "144", "irregular"] {
            assert_eq!(
                rr_simulation::replay::run(Replay::read(&bytes).unwrap(), rate)
                    .unwrap()
                    .state_sha256,
                sim.state_hash()
            );
        }
        let passed = first_crash.is_some()
            && if wreck_mode {
                wrecked.is_some()
                    && sim
                        .snapshot()
                        .camera_follow
                        .as_ref()
                        .is_some_and(|follow| follow.back == 900)
            } else {
                remount.is_some() && phases.contains("Walking") && phases.contains("Mounting")
            };
        cases.push(serde_json::json!({"course":stem,"passed":passed,"ticks":record.ticks,"first_crash":first_crash,"remount":remount,"wrecked":wrecked,"terminal_mode":wreck_mode,"phases":phases,"frames":frames,"state_sha256":sim.state_hash()}));
        std::fs::write(output.join("comparison.json"), serde_json::to_vec_pretty(&serde_json::json!({"outcome":if cases.iter().all(|c|c["passed"]==true){"passed"}else{"failed"},"cases":cases,"schedules":["30","60","144","irregular"],"scope":"Controlled original-art player recovery: initial stamina1; normal mode rival chain strike, wreck mode condition1 and parked source car; ordinary held throttle, no injected impact/result/assists; not whole-career evidence"})).unwrap()).unwrap();
        assert!(
            passed,
            "{stem} must complete the requested natural recovery/terminal sequence"
        );
    }
}
