//! Private controlled source-art episodes; no synthetic race result or career evidence.
use super::*;
use rr_simulation::{
    ActorId, QueuedCommand, Tick,
    animation::Weapon,
    combat::{AttackKind, Item, Request, Side, WeaponId},
};

pub(super) fn load_actor_palettes(
    presentation: &mut CoursePresentation,
    record: &Replay,
    data: &std::path::Path,
) {
    let scene =
        std::sync::Arc::get_mut(&mut presentation.scene).expect("private source-art scene owner");
    for seed in &record.encounter.as_ref().unwrap().seeds {
        if let std::collections::btree_map::Entry::Vacant(entry) =
            scene.assets.palettes.entry(seed.color)
        {
            entry.insert(std::sync::Arc::new(
                rr_render::rider::RiderAssets::load(data, seed.color).unwrap(),
            ));
        }
    }
}

#[test]
#[ignore = "requires private source artwork and a full police factory capture"]
fn source_police_native_art_encounter() {
    let output = PathBuf::from(std::env::var_os("RR_POLICE_ART_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let options = LaunchOptions {
        data_root: Some(std::env::var_os("RR_SESSION_DATA").unwrap().into()),
        catalog: Some(std::env::var_os("RR_SESSION_CATALOG").unwrap().into()),
        course: Some("CITY".into()),
        level: 5,
        bike: 1,
        opponents: 14,
        condition_percent: 100,
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
    load_actor_palettes(
        &mut presentation,
        &record,
        options.data_root.as_ref().unwrap(),
    );
    let config = record.handling.as_ref().unwrap();
    let start = (config.start_sample as usize + 1..config.ground.len() - 16)
        .find(|start| {
            config.ground[*start..*start + 16]
                .iter()
                .all(|g| g.rise.abs() <= 20 && g.bend.abs() <= 8 && g.right - g.left >= 250)
        })
        .unwrap();
    let distance = (start as i64 - i64::from(config.start_sample)) * 256;
    let center = presentation.scene.route.points[start].offset * 256 - 18000;
    let setup = record.encounter.as_mut().unwrap();
    setup.rules.combat_version = 4;
    setup.seeds[0].distance = distance;
    setup.seeds[0].lateral = center;
    setup.seeds[0].speed = 3000;
    setup.seeds[0].location = Some(setup.topology.locate(distance, center, 0).unwrap());
    setup
        .inventory
        .items
        .retain(|_, item| item.owner != Some(ActorId(1)));
    setup.inventory.items.insert(
        WeaponId(100),
        Item {
            kind: Weapon::Chain,
            owner: Some(ActorId(1)),
        },
    );
    let population = record.population.as_mut().unwrap();
    assert!(population.police.is_some());
    population.zones.clear();
    population.traffic = false;
    population.periodic_police = false;
    population.spawns = vec![rr_simulation::population::Spawn {
        id: 1,
        kind: rr_simulation::population::Kind::Police,
        location: setup
            .topology
            .locate(distance + 32, center + 18000, 0)
            .unwrap(),
        family: None,
        selector: 0,
        subtype: 0,
        speed: 6000,
        direction: 1,
    }];
    let mut sim = Simulation::with_population(
        record.seed,
        config.clone(),
        setup.clone(),
        population.clone(),
    )
    .unwrap();
    let mut frames = vec![];
    let mut seen = false;
    let mut hits = 0;
    let mut crash = false;
    let mut remounted = false;
    let mut last_phase = None;
    let ticks = 900;
    for tick in 0..=ticks {
        let canonical = sim.snapshot();
        let cop = canonical
            .actors
            .iter()
            .find(|actor| actor.encounter.as_ref().is_some_and(|unit| unit.police));
        seen |= cop.is_some();
        let phase = cop
            .and_then(|actor| actor.encounter.as_ref())
            .map(|unit| unit.recovery.phase);
        remounted |= crash && phase == Some(rr_simulation::recovery::Phase::Mounted);
        let changed_phase = phase != last_phase;
        last_phase = phase;
        let new_hit = cop
            .and_then(|actor| actor.encounter.as_ref())
            .is_some_and(|unit| unit.fighter.hit_count > hits);
        let new_crash = !crash
            && cop
                .and_then(|actor| actor.encounter.as_ref())
                .is_some_and(|unit| unit.recovery.body.is_some());
        if let Some(unit) = cop.and_then(|actor| actor.encounter.as_ref()) {
            hits = unit.fighter.hit_count;
            crash |= unit.recovery.body.is_some();
        }
        if tick % 60 == 0 || tick == 1 || new_hit || new_crash || changed_phase {
            let snapshot =
                rr_render::camera::route_encounter_snapshot(&presentation.scene.route, &canonical);
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
            let image = format!("police-{tick}.png");
            std::fs::write(output.join(&image), frame.image.png().unwrap()).unwrap();
            frames.push(serde_json::json!({"tick":tick,"cop":cop,"image":image,"road_users":canonical.road_users,"player":canonical.actors.iter().find(|actor|actor.id==ActorId(1)),"actor_frames":receipt.actor_frames}));
        }
        if tick == ticks {
            break;
        }
        let hero = canonical
            .actors
            .iter()
            .find(|actor| actor.id == ActorId(1))
            .unwrap()
            .rider
            .as_ref()
            .unwrap();
        let downed = cop
            .and_then(|actor| actor.encounter.as_ref())
            .is_some_and(|unit| unit.recovery.body.is_some());
        let (mut pace, target) =
            cop.and_then(|actor| actor.rider.as_ref())
                .map_or((3000, center), |cop| {
                    let gap =
                        (cop.bike.distance_q8 - hero.bike.distance_q8).clamp(-256, 256) as i32;
                    (
                        (cop.bike.speed + gap * 16).clamp(1500, 8000),
                        cop.bike.lateral - 24000,
                    )
                });
        if downed {
            pace = 0;
        }
        let steer = ((target - hero.bike.lateral) / 8 - hero.bike.lateral_speed * 16)
            .clamp(-20000, 20000) as i16;
        let input = Command {
            throttle: hero.bike.speed < pace,
            brake: hero.bike.speed > pace + 300,
            steer_q15: Some(steer),
            attack: (!downed).then_some(Request {
                kind: AttackKind::Punch,
                side: Side::Auto,
            }),
            ..Default::default()
        };
        record.commands.push(QueuedCommand {
            tick: Tick(tick),
            actor: ActorId(1),
            sequence: 0,
            input,
        });
        sim.step(input).unwrap();
    }
    record.ticks = ticks;
    record.expected_hash = Some(sim.state_hash());
    let bytes = serde_json::to_vec(&record).unwrap();
    std::fs::write(output.join("police.json"), &bytes).unwrap();
    for rate in ["30", "60", "144", "irregular"] {
        assert_eq!(
            rr_simulation::replay::run(Replay::read(&bytes).unwrap(), rate)
                .unwrap()
                .state_sha256,
            sim.state_hash()
        );
    }
    std::fs::write(output.join("comparison.json"), serde_json::to_vec_pretty(&serde_json::json!({"outcome":if seen && crash && remounted {"passed"}else{"failed"},"ticks":ticks,"cop_hit_count":hits,"cop_crashed":crash,"cop_remounted":remounted,"frames":frames,"state_sha256":sim.state_hash(),"scope":"Controlled source City level5 initial cop encounter, full15racer roster/static contacts, chain-equipped player followscop using ordinary throttle/brake/steering/attacks, isolated traffic; not full police/recovery/career acceptance"})).unwrap()).unwrap();
    assert!(
        seen && crash && remounted,
        "cop must complete a natural crash/recovery cycle"
    );
}

#[test]
#[ignore = "requires private source artwork and controlled combat episode output"]
fn source_combat_native_art_episodes() {
    let output = PathBuf::from(std::env::var_os("RR_COMBAT_ART_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let options = LaunchOptions {
        data_root: Some(std::env::var_os("RR_SESSION_DATA").unwrap().into()),
        catalog: Some(std::env::var_os("RR_SESSION_CATALOG").unwrap().into()),
        course: Some("CITY".into()),
        level: 1,
        bike: 1,
        practice_riders: 1,
        condition_percent: 100,
        effects: true,
        ..Default::default()
    };
    let mut presentation = course_view(&options, &VisualAssets::default())
        .unwrap()
        .unwrap();
    let original = build_race(&options, Some(&presentation), 3, Some(1))
        .0
        .recording_setup(3)
        .unwrap();
    load_actor_palettes(
        &mut presentation,
        &original,
        options.data_root.as_ref().unwrap(),
    );
    assert_eq!(
        original.schema_version, 6,
        "source practice recordings use the current bounded format"
    );
    let config = original.handling.as_ref().unwrap();
    let start = (config.start_sample as usize + 1..config.ground.len() - 16)
        .find(|start| {
            config.ground[*start..*start + 16]
                .iter()
                .all(|g| g.rise.abs() <= 20 && g.bend.abs() <= 8 && g.right - g.left >= 250)
        })
        .expect("source City requires a controlled gentle combat stretch");
    let distance = (start as i64 - i64::from(config.start_sample)) * 256;
    assert!(
        distance + 4096
            < original
                .encounter
                .as_ref()
                .unwrap()
                .rules
                .finish_progress
                .unwrap()
    );
    let offset = (config.ground[start].left + config.ground[start].right) * 128;
    let mut cases = vec![];
    let limit = std::env::var("RR_COMBAT_ART_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(14);
    for (slot, kind) in [
        (0, AttackKind::Punch),
        (0, AttackKind::Backhand),
        (1, AttackKind::Punch),
        (1, AttackKind::Backhand),
        (2, AttackKind::Punch),
        (2, AttackKind::Backhand),
        (3, AttackKind::Kick),
    ] {
        for side in [Side::Left, Side::Right] {
            if cases.len() >= limit {
                break;
            }
            let mut record = original.clone();
            let setup = record.encounter.as_mut().unwrap();
            setup.rules.combat_version = 4;
            setup.inventory.items.clear();
            for (index, seed) in setup.seeds.iter_mut().enumerate() {
                seed.distance = distance;
                seed.lateral = offset
                    + if index == 0 {
                        0
                    } else if side == Side::Right {
                        18000
                    } else {
                        -18000
                    };
                seed.speed = 8000;
                seed.location = Some(setup.topology.locate(distance, seed.lateral, 0).unwrap());
                if index == 1 {
                    seed.stats.health = 1;
                }
            }
            if slot == 1 || slot == 2 {
                setup.inventory.items.insert(
                    WeaponId(1),
                    Item {
                        kind: if slot == 1 {
                            Weapon::Club
                        } else {
                            Weapon::Chain
                        },
                        owner: Some(ActorId(1)),
                    },
                );
            }
            assert!(!setup.rules.assists.fast_recovery && !setup.rules.assists.rescue_unreachable);
            let mut sim = Simulation::with_population(
                record.seed,
                config.clone(),
                setup.clone(),
                record.population.clone().unwrap(),
            )
            .unwrap();
            let name = format!(
                "{slot}-{}-{}",
                if kind == AttackKind::Backhand {
                    "backhand"
                } else {
                    "strike"
                },
                if side == Side::Right { "right" } else { "left" }
            );
            let mut frames = vec![];
            let mut first_hit = None;
            let mut first_crash = None;
            for tick in 0..=120 {
                let canonical = sim.snapshot();
                let target = canonical
                    .actors
                    .iter()
                    .find(|actor| actor.id == ActorId(2))
                    .unwrap()
                    .encounter
                    .as_ref()
                    .unwrap();
                let new_hit = first_hit.is_none() && target.fighter.hit_count > 0;
                let new_crash = first_crash.is_none()
                    && target.recovery.phase != rr_simulation::recovery::Phase::Mounted;
                if new_hit {
                    first_hit = Some(tick);
                }
                if new_crash {
                    first_crash = Some(tick);
                }
                if [0, 6, 12, 16, 18, 24, 36, 60, 90, 120].contains(&tick) || new_hit || new_crash {
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
                    let image = format!("{name}-{tick}.png");
                    std::fs::write(output.join(&image), frame.image.png().unwrap()).unwrap();
                    frames.push(serde_json::json!({"tick":tick,"image":image,"target":target,"rider_bounds":receipt.rider_bounds}));
                }
                if tick == 120 {
                    break;
                }
                for actor in [ActorId(1), ActorId(2)] {
                    let input = Command {
                        throttle: true,
                        attack: (actor == ActorId(1)).then_some(Request { kind, side }),
                        ..Default::default()
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
            record.ticks = 120;
            record.expected_hash = Some(sim.state_hash());
            let bytes = serde_json::to_vec(&record).unwrap();
            std::fs::write(output.join(format!("{name}.json")), &bytes).unwrap();
            for rate in ["30", "60", "144", "irregular"] {
                let result =
                    rr_simulation::replay::run(Replay::read(&bytes).unwrap(), rate).unwrap();
                assert_eq!(result.state_sha256, sim.state_hash());
            }
            cases.push(serde_json::json!({"name":name,"first_hit":first_hit,"first_crash":first_crash,"frames":frames,"state_sha256":sim.state_hash()}));
        }
    }
    let failed = cases
        .iter()
        .filter(|case| case["first_hit"].is_null() || case["first_crash"].is_null())
        .count();
    std::fs::write(output.join("comparison.json"), serde_json::to_vec_pretty(&serde_json::json!({"outcome":if failed==0 {"passed"} else {"failed"},"cases":cases,"partial":limit<14,"source_start_sample":start,"schedules":["30","60","144","irregular"],"scope":"Controlled source City poses, two ordinary-command riders, target initial stamina1, full source static/bike contacts and recovery; no native pixel identity or career claim"})).unwrap()).unwrap();
    assert_eq!(
        failed, 0,
        "each source attack requires actual damage and a knock-off"
    );
}
