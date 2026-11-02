//! Private source-data acceptance driver. Published receipts contain metadata only.
mod support;
use rr_simulation::{combat::*, encounter::*, recovery::*, replay::*, riding::Configuration, *};
use serde_json::json;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err(
            "usage: rr-encounter-check DATA_ROOT OUTPUT_DIR | --authored OUTPUT_DIR".into(),
        );
    }
    if args[0] == "--authored" {
        return authored(std::path::Path::new(&args[1]));
    }
    let data = std::path::Path::new(&args[0]);
    let out = support::output(data, std::path::Path::new(&args[1]))?;
    std::fs::create_dir_all(&out)?;
    let exe = std::fs::read(data.join("ROADRASH.EXE"))?;
    let mut sources = vec![];
    for (stem, _) in rr_course::export::COURSES {
        let course = rr_course::Course::read(&std::fs::read(
            data.join(format!("DATA/COURSES/{stem}.CRS")),
        )?)?;
        for choice in [0, 1] {
            let route = course.route(&vec![choice; course.splits().len()])?;
            let config = Configuration::load(data, &route, 1, 50)?;
            for level in 0..5 {
                let setup = Setup::source(&config, &course, &route, &exe, level, 14)?;
                sources.push(json!({"course":stem,"choice":choice,"level":level+1,"paths":setup.topology.paths.len(),"obstacles":setup.rules.obstacles.len(),"actors":setup.seeds.len()}));
            }
        }
    }
    let course = rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS"))?)?;
    let route = course.route(&vec![0; course.splits().len()])?;
    let config = Configuration::load(data, &route, 1, 50)?;
    let mut setup = Setup::source(&config, &course, &route, &exe, 0, 2)?;
    setup.rules.combat_version = 1;
    setup.rules.bike_contacts = false;
    setup.rules.obstacles.clear();
    let command = |tick, input| QueuedCommand {
        tick: Tick(tick),
        actor: ActorId(1),
        sequence: 0,
        input,
    };
    let replay = Replay {
        schema_version: 4,
        seed: 1,
        ticks: 1800,
        commands: vec![
            command(
                0,
                Command {
                    attack: Some(Request {
                        kind: AttackKind::Punch,
                        side: Side::Auto,
                    }),
                    ..Default::default()
                },
            ),
            command(
                45,
                Command {
                    impact: Some(Impact {
                        rider_velocity: Some([5000, 0, 0]),
                        bike_velocity: Some([0, 0, 0]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ),
        ],
        route: None,
        expected_hash: None,
        handling: Some(config.clone()),
        encounter: Some(setup),
        population: None,
    };
    let mut baseline = None;
    let mut runs = vec![];
    for rate in ["30", "60", "144", "irregular"] {
        let r = run(replay.clone(), rate)?;
        if let Some(b) = &baseline {
            if b != &r.tick_hashes {
                return Err("schedule divergence".into());
            }
        } else {
            baseline = Some(r.tick_hashes.clone());
        }
        runs.push(json!({"rate":rate,"hash":r.state_sha256,"ticks":r.ticks}));
        std::fs::write(
            out.join(format!("replay-{rate}.json")),
            serde_json::to_vec_pretty(&r)?,
        )?;
    }
    std::fs::write(
        out.join("showcase-replay.json"),
        serde_json::to_vec(&replay)?,
    )?;
    let mut p = Playback::new(replay)?;
    let mut trace = String::from(
        "tick,health,maximum,bike_condition,bike_progress,body_progress,phase,owner\n",
    );
    let mut phases = std::collections::BTreeMap::new();
    while !p.done() {
        p.frame(frame_ns("60", p.frames)?)?;
        let w = p.simulation.encounters().unwrap();
        let u = &w.units[&ActorId(1)];
        let s = p.simulation.snapshot();
        let b = &s.actors[0].rider.as_ref().unwrap().bike;
        phases
            .entry(format!("{:?}", u.recovery.phase))
            .or_insert(p.simulation.tick().0);
        trace += &format!(
            "{},{},{},{},{},{:?},{:?},{:?}\n",
            p.simulation.tick().0,
            u.fighter.stats.health,
            u.fighter.stats.maximum,
            b.condition,
            b.distance_q8,
            u.recovery.body.as_ref().map(|b| b.motion.distance_q8),
            u.recovery.phase,
            w.inventory.carried(ActorId(1))
        );
    }
    for phase in [
        "Sliding",
        "GettingUp",
        "Standing",
        "Walking",
        "Mounting",
        "Mounted",
    ] {
        if !phases.contains_key(phase) {
            return Err(format!("source recovery missing {phase}: {phases:?}").into());
        }
    }
    std::fs::write(out.join("trace.csv"), trace)?;
    let mut motion = vec![];
    for speed in [-10000, -1, 0, 1200, 5000, 1000000] {
        for acceleration in [-500, 0, 300] {
            for heading in [0, 0x100000, 0x400000, 0x800000, 0xc00000, -0x100000] {
                let (forward, side) = actor_delta(speed, acceleration, heading, &config.tables);
                motion.push(json!([speed, acceleration, heading, forward, side]));
            }
        }
    }
    std::fs::write(out.join("motion-rust.json"), serde_json::to_vec(&motion)?)?;
    let mut damage = vec![];
    for reason in 0..7 {
        for amount in [0, 1, 7, 32, 100] {
            let mut stats = Stats::default();
            let p = reason_damage(amount, reason);
            stats.hit(p);
            damage.push(json!([reason, amount, stats.maximum, stats.health]));
        }
    }
    std::fs::write(out.join("damage-rust.json"), serde_json::to_vec(&damage)?)?;
    let mut timing = vec![];
    for side in 0..2 {
        for kind in 0..2 {
            for slot in 0..4 {
                let request = Request {
                    side: if side == 0 { Side::Left } else { Side::Right },
                    kind: if slot == 3 {
                        AttackKind::Kick
                    } else if kind == 1 {
                        AttackKind::Backhand
                    } else {
                        AttackKind::Punch
                    },
                };
                let weapon = match slot {
                    1 => animation::Weapon::Club,
                    2 => animation::Weapon::Chain,
                    _ => animation::Weapon::Fists,
                };
                let definition =
                    Definition::from_library_version(&config.animation, request, weapon, 1)?;
                for age in 0..definition.duration {
                    timing.push(json!([
                        side,
                        kind,
                        slot,
                        age,
                        i32::from(definition.active(u64::from(age)))
                    ]));
                }
            }
        }
    }
    std::fs::write(
        out.join("attack_timing-rust.json"),
        serde_json::to_vec(&timing)?,
    )?;
    let mut entries = vec![];
    for reason in [1, 2, 4, 5] {
        for speed in [0, 5000, 20000] {
            let mut c = config.clone();
            let selected = c.selected;
            c.specs
                .iter_mut()
                .find(|s| s.id == selected)
                .unwrap()
                .suspension = 1000;
            let mut bike = handling::Bike::new(c.spec(), &c.tables)?;
            bike.speed = speed;
            bike.height = 100;
            bike.lateral_speed = 123;
            bike.vertical_speed = 50;
            let at = topology::Location {
                height: 100,
                ..Default::default()
            };
            let topology = topology::Topology::straight(1000000);
            let mut state = State::mounted(ActorId(1), at);
            for repeat in 0..2 {
                let entered = state.begin_source(
                    ActorId(1),
                    &mut bike,
                    &c,
                    at,
                    Impact {
                        reason,
                        ..Default::default()
                    },
                    &topology,
                );
                let body = state.body.as_ref().unwrap();
                entries.push(json!([
                    reason,
                    speed,
                    repeat,
                    i32::from(entered),
                    bike.speed,
                    bike.height,
                    bike.vertical_speed,
                    body.motion.speed,
                    body.motion.height,
                    body.motion.vertical_speed,
                    body.animation.code
                ]));
            }
        }
    }
    std::fs::write(
        out.join("crash_entry-rust.json"),
        serde_json::to_vec(&entries)?,
    )?;
    let mut transactions = vec![];
    for reason in 0..7 {
        for amount in [0, 1, 7, 32, 100] {
            for condition in [1, 10, 100] {
                for health in [1, 2500] {
                    let mut stats = Stats {
                        health,
                        ..Stats::default()
                    };
                    let mut bike = handling::Bike::new(config.spec(), &config.tables)?;
                    bike.condition = condition;
                    stats.hit(reason_damage(amount, reason));
                    bike.damage(reason_damage(amount + 3, reason).max(0) as u32);
                    transactions.push(json!([
                        reason,
                        amount,
                        condition,
                        health,
                        stats.maximum,
                        stats.health,
                        bike.condition,
                        i32::from(stats.health == 0 || bike.condition == 0)
                    ]));
                }
            }
        }
    }
    std::fs::write(
        out.join("damage_transaction-rust.json"),
        serde_json::to_vec(&transactions)?,
    )?;
    let mut recovery_motion = vec![];
    for speed in [-256, -80, 5000] {
        let mut c = config.clone();
        c.ground.fill(handling::Ground::flat());
        c.start_sample = 0;
        c.tables.drag = [0; 4];
        let topology = topology::Topology::straight(1000000);
        let mut location = topology::Location {
            progress: 1000,
            height: 20000,
            ..Default::default()
        };
        let mut body = handling::Bike::new(c.spec(), &c.tables)?;
        body.speed = speed;
        body.height = 20000;
        body.distance_q8 = 1000;
        let law = BodyLaw {
            drag: 0,
            contact_drag: 0,
            ..BodyLaw::default()
        };
        for tick in 1..=3 {
            motion_step(&mut body, &mut location, 0, &c, &topology, &law, None)?;
            recovery_motion.push(json!([speed, tick, location.progress, body.speed]));
        }
    }
    std::fs::write(
        out.join("recovery_motion-rust.json"),
        serde_json::to_vec(&recovery_motion)?,
    )?;
    let receipt = json!({"outcome":"passed","source_setups":sources,"schedules":runs,"phases_first_observed":phases,"motion_vectors":motion.len(),"damage_vectors":damage.len(),"attack_timing_vectors":timing.len(),"crash_entry_vectors":entries.len(),"scope":"Pinned native public API comparison for motion, damage, attack timing and crash entry; full contact classifier/body-motion parity remains open"});
    std::fs::write(
        out.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!(
        "50 source setups; four equal schedules; six recovery phases; {} motion and {} damage vectors",
        motion.len(),
        damage.len()
    );
    Ok(())
}

fn authored(out: &std::path::Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out)?;
    let config = fixtures::riding();
    let mut setup = Setup::authored(&config)?;
    setup.rules.bike_contacts = false;
    setup.seeds[0].distance = 100;
    let mut other = Seed::player();
    other.id = ActorId(2);
    other.distance = 100;
    other.lateral = 12000;
    setup.seeds.push(other);
    let mut replay = Replay {
        schema_version: 4,
        seed: 1,
        ticks: 500,
        commands: vec![
            QueuedCommand {
                tick: Tick(0),
                actor: ActorId(1),
                sequence: 0,
                input: Command {
                    attack: Some(Request {
                        kind: AttackKind::Kick,
                        side: Side::Right,
                    }),
                    ..Default::default()
                },
            },
            QueuedCommand {
                tick: Tick(25),
                actor: ActorId(1),
                sequence: 0,
                input: Command {
                    impact: Some(Impact {
                        rider_velocity: Some([5000, 0, 0]),
                        bike_velocity: Some([0, 0, 0]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            },
        ],
        route: None,
        expected_hash: None,
        handling: Some(config),
        encounter: Some(setup),
        population: None,
    };
    let report = run(replay.clone(), "60")?;
    replay.expected_hash = Some(report.state_sha256.clone());
    std::fs::write(
        out.join("encounter-replay.json"),
        serde_json::to_vec(&replay)?,
    )?;
    let mut cases = vec![];
    for kind in [AttackKind::Punch, AttackKind::Kick, AttackKind::Backhand] {
        for side in [Side::Left, Side::Right] {
            for weapon in [
                animation::Weapon::Fists,
                animation::Weapon::Club,
                animation::Weapon::Chain,
            ] {
                let slot = weapon_slot(kind, weapon);
                let damage = Stats::default().damage([256, 320, 384, 64][slot]);
                cases.push(json!({"attack":kind,"side":side,"weapon":weapon,"ticks_0_through_3":{"victim_health":2500,"contacts":0},"tick_4":{"victim_health":2500-64*damage,"victim_maximum":2500-4*damage,"victim_lean":if side==Side::Left{-1}else{1}*if slot==3{11000}else{3000},"contacts":1,"recovery":"mounted","weapon_owner":if weapon==animation::Weapon::Fists{None}else{Some(1)}},"ticks_5_through_19":{"contacts":1,"health_regenerates_per_mounted_tick":1}}));
            }
        }
    }
    std::fs::write(
        out.join("combat-scenarios.json"),
        serde_json::to_vec_pretty(
            &json!({"fixture":"rr_simulation::fixtures (authored)","test":"all_attack_weapon_side_scenarios_have_tick_range_expectations","cases":cases}),
        )?,
    )?;
    println!("authored schema-4 replay: {}", report.state_sha256);
    Ok(())
}
