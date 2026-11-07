//! Private native-game command driver. No terminal injection, inventory grants or modified AI.
use super::*;
use rr_session::{
    career::Career,
    catalog::Catalog,
    persistence::{self, disk},
    race::{Mode, Phase},
};
use rr_simulation::ActorId;
use rr_simulation::recovery::Phase as Recovery;
use std::{io::Write, path::Path, time::Instant};
pub(super) fn run(
    options: &LaunchOptions,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RR_ACCEPTANCE_WRECK").is_some() {
        return wreck(options, output);
    }
    let data = options
        .data_root
        .as_ref()
        .ok_or("functional check requires imported data")?;
    let path = disk::guarded(&output.join("campaign.json"), Some(data))?;
    if path.exists() {
        return Err("Use a new functional-check output directory".into());
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    let catalog = Catalog::from_exe(&std::fs::read(data.join("ROADRASH.EXE"))?)?;
    let character = std::env::var("RR_ACCEPTANCE_CHARACTER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let mut career = Career::new(&catalog, character)?;
    let requirements = persistence::Requirements::base(&catalog);
    let save = output.join("career.json");
    let mut records = vec![];
    let budget = std::env::var("RR_ACCEPTANCE_ATTEMPTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200)
        .min(200);
    for attempt in 0..budget {
        if career.completed {
            break;
        }
        let preparation_before = career.clone();
        let mut transactions = vec![];
        // Earn upgrades by replaying the current-level Peninsula race; these are
        // ordinary prize transactions, not seeded money or forced qualifications.
        let goal = [1u8, 13, 7, 10, 8][usize::from(career.level - 1)] - 1;
        let upgrade = catalog.bike(goal)?;
        let reserve = u64::from(career.level) * 800;
        if career.bike != goal
            && catalog.bike(career.bike)?.price < upgrade.price
            && career.cash + u64::from(catalog.bike(career.bike)?.price / 2)
                >= u64::from(upgrade.price) + reserve
        {
            let before = career.clone();
            career.purchase(&catalog, goal, career.revision())?;
            transactions.push(serde_json::json!({"kind":"purchase","from_bike":before.bike,"to_bike":goal,"price":upgrade.price,"trade_in":catalog.bike(before.bike)?.price/2,"cash_before":before.cash,"cash_after":career.cash}));
        }
        let needs_upgrade = catalog.bike(career.bike)?.price < upgrade.price;
        let course = if needs_upgrade && career.level < 4 {
            3
        } else {
            (0..5).find(|i| career.qualified & (1 << i) == 0).unwrap() as u8 + 1
        };
        let cost = career.repair_quote(&catalog)?;
        if cost <= career.cash {
            let before = career.clone();
            let paid = career.repair(&catalog, career.revision())?;
            transactions.push(serde_json::json!({"kind":"repair","quote":cost,"paid":paid,"cash_before":before.cash,"cash_after":career.cash,"condition_before":before.condition,"condition_after":career.condition}));
        }
        let mut launch = options.clone();
        launch.level = career.level;
        launch.course = Some(
            rr_session::catalog::COURSES[usize::from(course - 1)]
                .0
                .into(),
        );
        launch.bike = u32::from(career.bike) + 1;
        launch.condition_percent = career.condition;
        if std::env::var_os("RR_ACCEPTANCE_ALTERNATE_ROUTE").is_some() {
            let stem = launch.course.as_ref().unwrap();
            let course = rr_course::Course::read(&std::fs::read(
                data.join(format!("DATA/COURSES/{stem}.CRS")),
            )?)?;
            launch.branch_mask = (1u32 << course.splits().len()) - 1;
        }
        launch.enhanced = false;
        launch.override_image = None;
        let wall = Instant::now();
        let scene = course_view(&launch, &VisualAssets::default())?.ok_or("source course")?;
        let mut sim = build_race(
            &launch,
            Some(&scene),
            attempt as u32 + 1,
            Some(career.character),
        )
        .0;
        let record = sim.recording_setup(attempt as u32 + 1)?;
        let encounter = record
            .encounter
            .as_ref()
            .ok_or("Source encounter required")?;
        if encounter.rules.assists.fast_recovery
            || encounter.rules.assists.rescue_unreachable
            || encounter.seeds.len() != 15
            || record
                .population
                .as_ref()
                .is_none_or(|p| p.police.is_none())
        {
            return Err("Career acceptance requires the full source field, police and disabled recovery assists".into());
        }
        let mut frontend = rr_session::flow::Frontend::default();
        frontend.mode = Mode::BigGame;
        frontend.character = career.character;
        frontend.level = career.level;
        frontend.course = course;
        frontend.bike = career.bike;
        frontend.career = Some(career.clone());
        let id = career.allocate_race()?;
        let mut session = frontend::Shell::make_session(&frontend, &catalog, &sim, id);
        let condition_capacity = session.condition_capacity();
        let mut social = frontend::Shell::new(&launch, &sim, false);
        social.frontend = frontend;
        social.frontend.career = Some(career.clone());
        let mut pilot = Driver::default();
        let mut commands = vec![];
        let mut previous = None;
        let mut crashes = 0;
        let mut tick = 0;
        while session.phase() != Phase::Results && tick < 180_000 {
            tick += 1;
            if !session.permit_tick() {
                continue;
            }
            let snapshot = sim.snapshot();
            let command = pilot.command(
                &snapshot,
                record.handling.as_ref().unwrap(),
                record.encounter.as_ref().unwrap(),
            );
            if previous != Some(command) {
                commands.push(rr_simulation::QueuedCommand {
                    tick: rr_simulation::Tick(snapshot.tick),
                    actor: ActorId(1),
                    sequence: commands.len() as u64,
                    input: command,
                });
                previous = Some(command);
            }
            sim.step(command)?;
            frontend::observe_social(&mut social, &sim);
            crashes += sim
                .effects()
                .iter()
                .filter(|e| {
                    e.actor == ActorId(1) && e.kind == rr_simulation::combat::EffectKind::Crash
                })
                .count();
            session.observe(&sim.snapshot());
            if tick % 6000 == 0 {
                eprintln!(
                    "career attempt {} L{} {}: tick {}, progress {}, speed {}, state {:?}",
                    attempt + 1,
                    career.level,
                    launch.course.as_ref().unwrap(),
                    tick,
                    sim.snapshot().actors[0]
                        .rider
                        .as_ref()
                        .unwrap()
                        .bike
                        .distance_q8,
                    sim.snapshot().actors[0].rider.as_ref().unwrap().bike.speed,
                    sim.snapshot().actors[0]
                        .encounter
                        .as_ref()
                        .map(|u| u.recovery.phase)
                );
            }
        }
        let summary = session
            .summary()
            .ok_or("Driver failed to reach a real terminal outcome within 50 simulation minutes")?;
        let before = (career.cash, career.level, career.qualified);
        let settlement = career
            .settle(&catalog, summary)?
            .ok_or("No career settlement")?;
        assert!(career.settle(&catalog, summary)?.is_none());
        social.frontend.career = Some(career.clone());
        social.frontend.settlement = Some(settlement.clone());
        frontend::settle_social(&mut social);
        career = social
            .frontend
            .career
            .take()
            .ok_or("Social settlement lost career")?;
        let bytes = persistence::encode(&career, &catalog, &requirements)?;
        disk::write(
            &save,
            Some(data),
            &bytes,
            |b| persistence::decode(b, &catalog, &requirements).map(|_| ()),
            disk::Fault::None,
        )?;
        std::fs::write(
            output.join(format!("career-state-{attempt:03}.json")),
            &bytes,
        )?;
        career = persistence::decode(&disk::read(&save)?, &catalog, &requirements)?.career;
        let mut replay = record;
        replay.ticks = sim.tick().0;
        replay.commands = commands;
        replay.expected_hash = Some(sim.state_hash());
        let replay_path = output.join(format!("race-{attempt:03}.json"));
        let replay_bytes = serde_json::to_vec(&replay)?;
        rr_simulation::replay::Replay::read(&replay_bytes)?;
        std::fs::write(&replay_path, replay_bytes)?;
        let report = serde_json::json!({"attempt":attempt+1,"level":launch.level,"course":launch.course,"bike":launch.bike,
            "authority_profile":{"combat":replay.encounter.as_ref().unwrap().rules.combat_version,"recovery":replay.encounter.as_ref().unwrap().rules.body.recovery_version,"contact_walk":replay.encounter.as_ref().unwrap().rules.contact_walk_version,"opponents":14,"police":true,"recovery_assists":false},
            "ticks":sim.tick().0,"outcome":summary.outcome(),"place":summary.place(),"crashes":crashes,
            "settlement":settlement,"preparation_before":preparation_before,"shop_transactions":transactions,"condition_capacity":condition_capacity,"condition_initial_percent":launch.condition_percent,"condition_final_percent":summary.bike_condition(),"money_before":before.0,"money_after":career.cash,"qualified":career.qualified,
            "state_sha256":sim.state_hash(),"wall_ms":wall.elapsed().as_millis(),"replay":replay_path.file_name().and_then(|s|s.to_str())});
        eprintln!("{}", report);
        records.push(report);
        std::fs::write(
            output.join("progress.json"),
            serde_json::to_vec_pretty(&records)?,
        )?;
        if attempt >= 24 && records.iter().rev().take(25).all(|r| r["outcome"] != "Won") {
            return Err(
                "Twenty-five real nonqualifying results; driver/game defect needs investigation"
                    .into(),
            );
        }
    }
    let report = serde_json::json!({"schema_version":1,"native_only":true,"real_commands_only":true,
        "completed":career.completed,"races":records,"career":career});
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(&serde_json::to_vec_pretty(&report)?)?;
    if !career.completed {
        return Err("Career incomplete after bounded attempt budget".into());
    }
    Ok(())
}
/// A separately identified damaged-bike scenario, never substituted for the
/// fresh-character campaign. Only ordinary controls can cause its terminal.
fn wreck(options: &LaunchOptions, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = options.data_root.as_ref().ok_or("import required")?;
    let path = disk::guarded(&output.join("wreck.json"), Some(data))?;
    std::fs::create_dir(output)?;
    let mut launch = options.clone();
    launch.condition_percent = 1;
    launch.course = Some("CITY".into());
    launch.enhanced = false;
    launch.override_image = None;
    let scene = course_view(&launch, &VisualAssets::default())?.ok_or("source scene")?;
    let mut sim = build_race(&launch, Some(&scene), 42, None).0;
    let mut record = sim.recording_setup(42)?;
    let mut outcome = None;
    let mut previous = None;
    for _ in 0..180_000 {
        let snapshot = sim.snapshot();
        let rider = snapshot.actors[0].rider.as_ref().unwrap();
        let unit = snapshot.actors[0].encounter.as_ref().unwrap();
        let mut command = drive(
            &snapshot,
            record.handling.as_ref().unwrap(),
            record.encounter.as_ref().unwrap(),
        );
        if unit.recovery.phase == Recovery::Mounted {
            let config = record.handling.as_ref().unwrap();
            let index = (i64::from(config.start_sample) + (rider.bike.distance_q8 >> 8))
                .clamp(0, config.ground.len() as i64 - 1) as usize;
            let g = config.ground[index];
            let at = unit.recovery.bike_location;
            let target = snapshot
                .road_users
                .iter()
                .filter(|a| {
                    a.location.path == at.path
                        && a.location.progress > at.progress
                        && a.location.progress < at.progress + 2048
                })
                .min_by_key(|a| a.location.progress)
                .map_or((g.left + g.right) * 128, |a| a.location.lateral);
            let wanted = ((target - rider.bike.lateral) / 20 - rider.bike.lateral_speed / 2)
                .clamp(-6000, 6000);
            command = Command {
                throttle: true,
                steer_q15: Some(
                    (i64::from(wanted) * 32767 * 4 / (i64::from(rider.bike.speed.max(1000)) * 3))
                        .clamp(-32767, 32767) as i16,
                ),
                ..Default::default()
            };
        }
        if previous != Some(command) {
            record.commands.push(rr_simulation::QueuedCommand {
                tick: sim.tick(),
                actor: ActorId(1),
                sequence: record.commands.len() as u64,
                input: command,
            });
            previous = Some(command);
        }
        sim.step(command)?;
        outcome = sim.snapshot().actors[0]
            .encounter
            .as_ref()
            .and_then(|u| u.recovery.outcome);
        if outcome.is_some() {
            break;
        }
    }
    if outcome != Some(rr_simulation::recovery::Outcome::Wrecked) {
        return Err(format!("Damaged-bike scenario produced {outcome:?}").into());
    }
    record.ticks = sim.tick().0;
    record.expected_hash = Some(sim.state_hash());
    let bytes = serde_json::to_vec(&record)?;
    rr_simulation::replay::Replay::read(&bytes)?;
    std::fs::write(output.join("replay.json"), &bytes)?;
    std::fs::write(
        path,
        serde_json::to_vec_pretty(
            &serde_json::json!({"outcome":"Wrecked","initial_condition_percent":1,"native_only":true,"state_injection":false,"ticks":record.ticks,"state_sha256":sim.state_hash(),"replay_sha256":rr_content::hash(&bytes)}),
        )?,
    )?;
    Ok(())
}
/// Route-specific planning state. The recorded commands remain the sole race input.
#[derive(Default)]
pub(super) struct Driver {
    routes: std::collections::BTreeMap<
        u8,
        (
            rr_simulation::riding::Configuration,
            rr_simulation::topology::Topology,
        ),
    >,
}
impl Driver {
    pub(super) fn command(
        &mut self,
        snapshot: &rr_simulation::Snapshot,
        config: &rr_simulation::riding::Configuration,
        encounter: &rr_simulation::encounter::Setup,
    ) -> Command {
        if let Some((roads, route)) = encounter.rules.source_roads.as_ref().zip(
            snapshot
                .actors
                .first()
                .and_then(|a| a.encounter.as_ref())
                .and_then(|u| u.source_route),
        ) && route != roads.initial
        {
            let (config, topology) = self.routes.entry(route).or_insert_with(|| {
                let mut current = config.clone();
                current.ground = roads.ground(route).expect("validated source driver ground");
                (
                    current,
                    roads
                        .topology(route, &encounter.topology)
                        .expect("validated source driver route"),
                )
            });
            return drive_on_route(snapshot, config, encounter, topology);
        }
        drive_on_route(snapshot, config, encounter, &encounter.topology)
    }
}

pub(super) fn drive(
    snapshot: &rr_simulation::Snapshot,
    config: &rr_simulation::riding::Configuration,
    encounter: &rr_simulation::encounter::Setup,
) -> Command {
    Driver::default().command(snapshot, config, encounter)
}

fn drive_on_route(
    snapshot: &rr_simulation::Snapshot,
    config: &rr_simulation::riding::Configuration,
    encounter: &rr_simulation::encounter::Setup,
    topology: &rr_simulation::topology::Topology,
) -> Command {
    let hazards = &encounter.rules.obstacles;
    let physical_hazard = |o: &rr_simulation::encounter::Obstacle| {
        o.kind != rr_simulation::encounter::ObstacleKind::Decorative
            && !(o.kind == rr_simulation::encounter::ObstacleKind::Building
                && !encounter.rules.building_boundaries.is_empty())
    };
    let ground = &config.ground;
    let start = config.start_sample;
    let route = snapshot.actors[0]
        .encounter
        .as_ref()
        .and_then(|u| u.source_route);
    let actor = &snapshot.actors[0];
    let bike = &actor.rider.as_ref().unwrap().bike;
    let unit = actor.encounter.as_ref().unwrap();
    if unit.recovery.phase != Recovery::Mounted {
        return Command {
            walk: None,
            ..Default::default()
        }; // Native auto-return and remount, no rescue assist.
    }
    let index =
        (i64::from(start) + (bike.distance_q8 >> 8)).clamp(0, ground.len() as i64 - 1) as usize;
    let g = ground[index];
    let center = (g.left + g.right) / 2;
    let at = unit.recovery.bike_location;
    let mut obstacles: Vec<(i64, i32)> = snapshot
        .road_users
        .iter()
        .filter(|a| a.location.path == at.path)
        .map(|a| (a.location.progress, a.location.lateral))
        .collect();
    obstacles.extend(snapshot.actors.iter().skip(1).filter_map(|a| {
        let u = a.encounter.as_ref()?;
        (u.recovery.bike_location.path == at.path && u.recovery.outcome.is_none()).then_some((
            u.recovery.bike_location.progress,
            u.recovery.bike_location.lateral,
        ))
    }));
    let lanes = [
        -116000, -90000, -60000, -30000, 0, 30000, 60000, 90000, 116000,
    ]
    .map(|l| center * 256 + l);
    let mut target = lanes
        .into_iter()
        .filter(|lane| *lane > (g.left + 20) * 256 && *lane < (g.right - 20) * 256)
        .min_by_key(|lane| {
            let blocked = obstacles
                .iter()
                .filter(|(p, l)| {
                    *p > at.progress - 128 && *p < at.progress + 1200 && (*l - *lane).abs() < 30000
                })
                .count();
            let solid = hazards
                .iter()
                .filter(|o| {
                    o.path == at.path
                        && physical_hazard(o)
                        && o.progress > at.progress - 128
                        && o.progress < at.progress + 2048
                        && *lane > o.left - 12000
                        && *lane < o.right + 12000
                })
                .count();
            (
                solid,
                blocked,
                (*lane - bike.lateral).abs() + (*lane - center * 256).abs() / 4,
            )
        })
        .unwrap_or(center * 256);
    if std::env::var_os("RR_DRIVER_CENTER").is_some() {
        target = center * 256;
    }
    let limit = std::env::var("RR_DRIVER_SPEED")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(15000)
        .clamp(1000, 50000);
    if let Ok(lane) = std::env::var("RR_DRIVER_LANE")
        && let Ok(lane) = lane.parse::<i32>()
    {
        target = (center * 256 + lane).clamp((g.left + 70) * 256, (g.right - 70) * 256);
    }
    let error = target - bike.lateral;
    let curve_drift = -g.bend * (bike.speed.max(0) / 256) * 400 / 256;
    let wanted = ((error / 20) - bike.lateral_speed / 2 - curve_drift).clamp(-6000, 6000);
    let steer = (i64::from(wanted) * 32767 * 4 / (i64::from(bike.speed.max(1000)) * 3))
        .clamp(-32767, 32767) as i16;
    let steer = steer / 2048 * 2048;
    let steer = if std::env::var_os("RR_DRIVER_LEGACY").is_none() {
        let mut best = (
            i128::MAX,
            steer,
            bike.speed < limit,
            bike.speed > limit + 1000,
        );
        for mode in 0..3 {
            if mode > 0 && best.0 < 100_000_000 {
                break;
            }
            for candidate in [-32767i16, -16384, -8192, -4096, 0, 4096, 8192, 16384, 32767] {
                let mut projected = actor.rider.as_ref().unwrap().clone();
                let command = Command {
                    throttle: mode == 0 && bike.speed < limit,
                    brake: mode == 2 || (mode == 0 && bike.speed > limit + 1000),
                    nitro: mode == 0,
                    steer_q15: Some(candidate),
                    ..Default::default()
                };
                let mut score = i128::from(mode) * 30000;
                let mut projected_at = at;
                let mut previous_distance = projected.bike.distance_q8;
                for ahead in 1..=60 {
                    if projected
                        .step(config, command, (snapshot.tick + ahead) as u32)
                        .is_err()
                    {
                        score += 1_000_000_000_000;
                        break;
                    }
                    let delta = projected.bike.distance_q8 - previous_distance;
                    previous_distance = projected.bike.distance_q8;
                    if topology.advance(&mut projected_at, delta, None).is_err() {
                        score += 1_000_000_000;
                        break;
                    }
                    if ahead % 5 != 0 {
                        continue;
                    }
                    let b = &projected.bike;
                    let future_index = (i64::from(start) + (b.distance_q8 >> 8))
                        .clamp(0, ground.len() as i64 - 1)
                        as usize;
                    let future_center =
                        (ground[future_index].left + ground[future_index].right) * 128;
                    score += i128::from(
                        (i64::from(b.lateral)
                            - i64::from(target)
                            - i64::from(future_center - center * 256))
                        .abs(),
                    );
                    if b.crashed || b.failed {
                        score += 1_000_000_000;
                    }
                    let sections = &encounter.rules.building_boundaries;
                    if let Ok(i) = sections.binary_search_by_key(
                        &(projected_at.path, (projected_at.progress >> 8) as u32),
                        |s| (s.path, s.sample),
                    ) {
                        let section = &sections[i];
                        if section.building && section.flags & 0x38 == 0 {
                            let widen = (((projected_at.progress & 255) * 250 / 3) >> 8) as i32;
                            let offset = encounter
                                .rules
                                .source_roads
                                .as_ref()
                                .and_then(|roads| {
                                    route.and_then(|route| roads.profiles.get(usize::from(route)))
                                })
                                .and_then(|profile| {
                                    profile.identity(
                                        projected_at.path,
                                        (projected_at.progress >> 8) as u32,
                                    )
                                })
                                .map_or(0, |identity| identity[2]);
                            let left = offset + section.outer[0]
                                - if section.selectors[0] == 255 {
                                    widen
                                } else {
                                    0
                                };
                            let right = offset
                                + section.outer[1]
                                + if section.selectors[1] == 255 {
                                    widen
                                } else {
                                    0
                                };
                            let width = encounter
                                .rules
                                .collision
                                .as_ref()
                                .map_or(encounter.rules.dimensions[0], |p| p.mounted[0]);
                            if b.lateral - width < left * 256 || b.lateral + width > right * 256 {
                                score += 100_000_000;
                            }
                        }
                    }
                    let world_progress = projected_at.progress;
                    for other in snapshot.actors.iter().skip(1) {
                        let Some(u) = &other.encounter else {
                            continue;
                        };
                        if u.recovery.bike_location.path != at.path || u.recovery.outcome.is_some()
                        {
                            continue;
                        }
                        let other_bike = &other.rider.as_ref().unwrap().bike;
                        let mut other_at = u.recovery.bike_location;
                        let _ = topology.advance(
                            &mut other_at,
                            i64::from(other_bike.speed / 256) * ahead as i64,
                            None,
                        );
                        if other_at.path != projected_at.path {
                            continue;
                        }
                        let p = other_at.progress;
                        if (p - world_progress).abs() < 256
                            && (b.lateral - other_bike.lateral).abs() < 20000
                        {
                            score += 100_000_000;
                        }
                    }
                    for other in &snapshot.road_users {
                        let mut other_at = other.location;
                        let _ = topology.advance(
                            &mut other_at,
                            if other.kind == rr_simulation::population::Kind::Pedestrian
                                || other.state == rr_simulation::population::AgentState::Crossing
                            {
                                0
                            } else {
                                i64::from(other.speed / 256)
                                    * i64::from(other.direction)
                                    * ahead as i64
                            },
                            None,
                        );
                        let Some(gap) = rr_simulation::source_roads::Roads::physical_delta(
                            topology,
                            projected_at,
                            other_at,
                        ) else {
                            continue;
                        };
                        let lateral = other.location.lateral
                            + if other.state == rr_simulation::population::AgentState::Crossing {
                                other.speed * i32::from(other.direction)
                                    / if other.kind == rr_simulation::population::Kind::Pedestrian {
                                        1
                                    } else {
                                        4
                                    }
                                    * ahead as i32
                            } else {
                                0
                            };
                        let (long_clearance, side_clearance) = match other.kind {
                            // Cars can choose a new lane and move laterally by 1024
                            // per tick. Reserve that reachable band, rather than
                            // assuming their current lateral position is fixed.
                            rr_simulation::population::Kind::Vehicle => {
                                (384, 30000 + (ahead as i32 * 1024).min(48000))
                            }
                            rr_simulation::population::Kind::Police => (72, 10000),
                            rr_simulation::population::Kind::Pedestrian => (72, 10000),
                        };
                        if gap.abs() < long_clearance
                            && (b.lateral - lateral).abs() < side_clearance
                        {
                            score += 100_000_000;
                        }
                    }
                    for hazard in hazards
                        .iter()
                        .filter(|o| o.path == projected_at.path && physical_hazard(o))
                    {
                        if (hazard.progress - world_progress).abs() < 128
                            && b.lateral > hazard.left - 10000
                            && b.lateral < hazard.right + 10000
                        {
                            score += 100_000_000;
                        }
                    }
                }
                score += i128::from(projected.bike.lateral_speed.unsigned_abs()) * 8;
                score += i128::from((i32::from(candidate) - i32::from(steer)).abs()) * 10;
                if score < best.0 {
                    best = (score, candidate, command.throttle, command.brake);
                }
            }
        }
        (best.1, best.2, best.3)
    } else {
        (steer, bike.speed < limit, bike.speed > limit + 1000)
    };

    Command {
        throttle: steer.1 || bike.speed < 1800,
        brake: steer.2 && bike.speed >= 1800,
        steer_q15: Some(steer.0),
        nitro: true,
        attack: (unit.fighter.swing.is_none()
            && snapshot.tick >= unit.fighter.stun_until
            && (snapshot.actors.iter().skip(1).any(|other| {
                other.encounter.as_ref().is_some_and(|u| {
                    u.recovery.phase == Recovery::Mounted
                        && u.recovery.outcome.is_none()
                        && u.recovery.bike_location.path == at.path
                        && (u.recovery.bike_location.progress - at.progress).abs() < 64
                        && (u.recovery.bike_location.lateral - at.lateral).abs() < 28672
                })
            }) || snapshot.road_users.iter().any(|a| {
                a.kind == rr_simulation::population::Kind::Police
                    && a.location.path == at.path
                    && (a.location.progress - at.progress).abs() < 48
                    && (a.location.lateral - at.lateral).abs() < 20480
            })))
        .then_some(rr_simulation::combat::Request {
            kind: rr_simulation::combat::AttackKind::Kick,
            side: rr_simulation::combat::Side::Auto,
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod diagnostic {
    use super::*;
    #[test]
    #[ignore = "private real-command incident diagnosis"]
    fn recorded_career_incidents() {
        let path = std::path::PathBuf::from(std::env::var_os("RR_CAREER_REPLAY").unwrap());
        let replay = rr_simulation::replay::Replay::read(&std::fs::read(path).unwrap()).unwrap();
        let limit = replay.ticks;
        let expected = replay.expected_hash.clone();
        let mut playback = rr_simulation::replay::Playback::new(replay).unwrap();
        let mut last = std::collections::VecDeque::new();
        for tick in 0..limit {
            let before = playback.simulation.encounters().unwrap().units[&ActorId(1)]
                .fighter
                .stats;
            playback.simulation.advance().unwrap();
            let effects: Vec<_> = playback
                .simulation
                .effects()
                .iter()
                .filter(|e| e.actor == ActorId(1) || e.other == Some(ActorId(1)))
                .cloned()
                .collect();
            for e in &effects {
                last.push_back((tick, e.clone()));
            }
            while last.len() > 12 {
                last.pop_front();
            }
            if effects.iter().any(|e| {
                e.actor == ActorId(1) && e.kind == rr_simulation::combat::EffectKind::Crash
            }) {
                let snapshot = playback.simulation.snapshot();
                let hero = &snapshot.actors[0];
                let unit = hero.encounter.as_ref().unwrap();
                let at = unit.recovery.bike_location;
                let others:Vec<_>=snapshot.actors.iter().skip(1).filter_map(|a| {
                    let u=a.encounter.as_ref()?;let b=a.rider.as_ref()?;
                    (u.recovery.bike_location.path==at.path&&(u.recovery.bike_location.progress-at.progress).abs()<512).then_some(serde_json::json!({"id":a.id,"police":u.police,"phase":u.recovery.phase,"distance":u.recovery.bike_location.progress-at.progress,"side":b.bike.lateral-hero.rider.as_ref().unwrap().bike.lateral,"weapon":b.weapon}))
                }).collect();
                eprintln!(
                    "{}",
                    serde_json::json!({"tick":tick,"before_stats":before,"hero":hero,"near_riders":others,"near_road_users":snapshot.road_users.iter().filter(|a| rr_simulation::source_roads::Roads::physical_delta(&playback.simulation.encounters().unwrap().setup.topology, at,a.location).is_some_and(|gap| gap.abs()<1024)).collect::<Vec<_>>(),"events":last})
                );
            }
        }
        assert_eq!(Some(playback.simulation.state_hash()), expected);
    }
}
