//! Population acceptance and long-run trace export. Source-derived output stays private.
mod support;
use rr_simulation::{
    population::*,
    replay::{Playback, Replay, run},
    *,
};
use serde_json::json;
use std::path::Path;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() < 2 || args.len() > 3 {
        return Err("usage: rr-population-check DATA_ROOT OUTPUT_DIR [RECORDED_REPLAY] | --authored OUTPUT_DIR".into());
    }
    if args[0] == "--authored" {
        return authored(Path::new(&args[1]));
    }
    let data = Path::new(&args[0]);
    let out = support::output(data, Path::new(&args[1]))?;
    std::fs::create_dir_all(&out)?;
    let exe = std::fs::read(data.join("ROADRASH.EXE"))?;
    let pe = rr_formats::pe::SupportedExe::read(&exe)?;
    let mut cadence = vec![];
    for slice in [false, true] {
        for position in [-10000, -1024, -512, 0, 768, 1024, 2048, 10000] {
            for speed in [0, 2048, 4096, 6144, 10000] {
                for tick in [0, 299, 300] {
                    for human in [false, true] {
                        cadence.push(live_interval(slice, position, 0, speed, tick, human));
                    }
                }
            }
        }
    }
    let mut timers = vec![];
    let mut timer = Timers::default();
    for density in [-25, 0, 3, 12, -10, 0] {
        timer.density(density);
        timers.push(vec![
            timer.cruise,
            timer.rate,
            timer.reload,
            timer.reload,
            timer.oncoming,
            timer.behind,
            timer.ahead,
            i32::from(timer.density),
        ]);
    }
    let mut arrests = vec![];
    for crash in [0, 1, 2, 3] {
        for speed in [-1800, -300, 0, 299, 300, 1799, 1800] {
            for cop_speed in [-5000, 0, 4999, 5000] {
                for delta in [0, 511, 512, -511, -512] {
                    for found in [false, true] {
                        for crashed in [false, true] {
                            arrests.push(i32::from(arrest(
                                crash,
                                speed,
                                cop_speed,
                                1000,
                                1000 + delta,
                                found,
                                crashed,
                            )));
                        }
                    }
                }
            }
        }
    }
    let mut personalities = vec![];
    for id in 0..10u32 {
        for level in 0..5u32 {
            let base = 0x464558 + 0x204 * id;
            let word = |va| -> std::result::Result<i32, Box<dyn std::error::Error>> {
                Ok(rr_formats::binary::u32le(pe.read_va(va, 4)?, 0)? as i32)
            };
            personalities.push(vec![
                id as i32,
                level as i32,
                word(base + 0x1bc + level * 4)?,
                word(base + 0x1a8 + level * 4)?,
                i32::from(pe.read_va(base + 0x1d0, 1)?[0] as i8),
                i32::from(pe.read_va(base + 0x1d1, 1)?[0] as i8),
                i32::from(pe.read_va(base + 0x165, 1)?[0]),
            ]);
        }
    }
    let mut choices = vec![];
    for (stem, _) in rr_course::export::COURSES {
        let masks = vehicle_masks(&std::fs::read(data.join(format!("DATA/CARS/{stem}.CAR")))?)?;
        for kind in 0..4 {
            for random in 0..32 {
                choices.push(vehicle_choice(&masks, kind, random).map_or(-1, i32::from));
            }
        }
    }
    std::fs::write(
        out.join("primitives-rust.json"),
        serde_json::to_vec(
            &json!({"cadence":cadence,"timers":timers,"arrest":arrests,"personalities":personalities,"vehicle_choices":choices}),
        )?,
    )?;
    let mut setups = vec![];
    for (stem, _) in rr_course::export::COURSES {
        let course = rr_course::Course::read(&std::fs::read(
            data.join(format!("DATA/COURSES/{stem}.CRS")),
        )?)?;
        for choice in [0, 1] {
            let route = course.route(&vec![choice; course.splits().len()])?;
            let config = riding::Configuration::load(data, &route, 1, 50)?;
            let cars = std::fs::read(data.join(format!("DATA/CARS/{stem}.CAR")))?;
            let vehicle_types = rr_formats::resource::Directory::read(&cars)?
                .entries
                .iter()
                .filter(|r| r.tag == "ANIM")
                .count() as u16;
            for level in 0..5 {
                let mut e = encounter::Setup::source(&config, &course, &route, &exe, level, 14)?;
                let mut p = Setup::source(&config, &mut e, &course, &exe, level, vehicle_types)?;
                p.vehicle_masks = vehicle_masks(&cars)?;
                let record = json!({"course":stem,"choice":choice,"level":level+1,"riders":e.seeds.len(),"brains":p.brains.len(),"zones":p.zones.len(),"actor_requests":p.spawns.len(),"vehicle_types":p.vehicle_types});
                let mut sim = Simulation::with_population(7, config.clone(), e, p)?;
                sim.step(Command {
                    throttle: true,
                    ..Default::default()
                })?;
                setups.push(record);
            }
        }
    }
    let course = rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS"))?)?;
    let route = course.route(&vec![0; course.splits().len()])?;
    let config = riding::Configuration::load(data, &route, 1, 50)?;
    let mut e = encounter::Setup::source(&config, &course, &route, &exe, 0, 14)?;
    let cars = std::fs::read(data.join("DATA/CARS/CITY.CAR"))?;
    let masks = vehicle_masks(&cars)?;
    let mut p = Setup::source(&config, &mut e, &course, &exe, 0, masks.len() as u16)?;
    p.vehicle_masks = masks;
    let commands = vec![
        QueuedCommand {
            tick: Tick(0),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                throttle: true,
                ..Default::default()
            },
        },
        QueuedCommand {
            tick: Tick(3000),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                throttle: true,
                steer_q15: Some(4000),
                ..Default::default()
            },
        },
        QueuedCommand {
            tick: Tick(3100),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                throttle: true,
                ..Default::default()
            },
        },
    ];
    let replay = if args.len() == 3 {
        let mut recorded = Replay::read_file(Path::new(&args[2]))?;
        if recorded.schema_version != 5 {
            return Err("population comparison requires a schema-5 recording".into());
        }
        recorded.ticks = 10800;
        recorded.commands.retain(|c| c.tick.0 < 10800);
        recorded.expected_hash = None;
        recorded
    } else {
        Replay {
            schema_version: 5,
            seed: 7,
            ticks: 10800,
            commands,
            route: None,
            expected_hash: None,
            handling: Some(config),
            encounter: Some(e),
            population: Some(p),
        }
    };
    let mut runs = vec![];
    let mut baseline = None;
    for rate in ["30", "60", "144", "irregular"] {
        let r = run(replay.clone(), rate)?;
        if let Some(b) = &baseline {
            if &r.tick_hashes != b {
                return Err("long-run divergence".into());
            }
        } else {
            baseline = Some(r.tick_hashes.clone());
        }
        runs.push(json!({"rate":rate,"ticks":r.ticks,"hash":r.state_sha256}));
        std::fs::write(
            out.join(format!("long-{rate}.json")),
            serde_json::to_vec(&r)?,
        )?;
    }
    let mut showcase = replay.clone();
    showcase.ticks = 600;
    showcase.commands.retain(|c| c.tick.0 < 600);
    showcase.expected_hash = Some(run(showcase.clone(), "60")?.state_sha256);
    std::fs::write(
        out.join("showcase-replay.json"),
        serde_json::to_vec(&showcase)?,
    )?;
    let mut visual = showcase.clone();
    visual.expected_hash = None;
    let at = visual
        .encounter
        .as_ref()
        .unwrap()
        .topology
        .locate(
            visual.encounter.as_ref().unwrap().seeds[0].distance,
            visual.encounter.as_ref().unwrap().seeds[0].lateral,
            0,
        )
        .unwrap();
    let population = visual.population.as_mut().unwrap();
    let pedestrian = population
        .spawns
        .iter()
        .find(|s| s.kind == Kind::Pedestrian)
        .cloned();
    population.spawns.clear();
    for (index, kind, offset, lateral, speed, direction) in [
        (1, Kind::Vehicle, 768, -64000, 4000, -1),
        (2, Kind::Police, 2500, 64000, 6000, -1),
    ] {
        population.spawns.push(Spawn {
            id: index,
            location: topology::Location {
                progress: at.progress + offset,
                lateral,
                ..at
            },
            kind,
            family: None,
            selector: 0,
            subtype: 0,
            speed,
            direction,
        });
    }
    if let Some(mut pedestrian) = pedestrian {
        pedestrian.id = 3;
        pedestrian.location = topology::Location {
            progress: at.progress + 500,
            lateral: 192000,
            ..at
        };
        pedestrian.subtype = 5;
        pedestrian.direction = -1;
        population.spawns.push(pedestrian);
    }
    visual.expected_hash = Some(run(visual.clone(), "60")?.state_sha256);
    std::fs::write(out.join("visual-replay.json"), serde_json::to_vec(&visual)?)?;
    let mut playback = Playback::new(replay)?;
    playback.simulation.set_rng_tracing(true);
    let mut traces = vec![];
    let mut rng = vec![];
    while playback.simulation.tick().0 < 10800 {
        playback.simulation.advance()?;
        let population = playback.simulation.population().unwrap();
        for draw in &population.trace {
            rng.push(json!([draw.tick, draw.actor, draw.category, draw.value]));
        }
        if playback.simulation.tick().0 % 300 == 0 {
            traces.push(json!({"tick":playback.simulation.tick().0,"decisions":population.brains.iter().map(|(id,b)|json!([id.0,b.decisions,b.next_tick,b.target_speed,b.desired_lane,b.target])).collect::<Vec<_>>(),"motion":playback.simulation.snapshot().actors.iter().map(|a|json!([a.id.0,a.position.progress.0,a.position.lateral.0,a.speed.0,a.encounter.as_ref().map(|u|u.recovery.phase)])).collect::<Vec<_>>() }));
        }
    }
    std::fs::write(
        out.join("behavior-trace.json"),
        serde_json::to_vec(&traces)?,
    )?;
    std::fs::write(out.join("rng-trace.json"), serde_json::to_vec(&rng)?)?;
    let report = json!({"outcome":"passed","source_setups":setups,"runs":runs,"long_run_seconds":180,"trace_samples":traces.len(),"rng_draws_traced":rng.len(),"input_provenance":if args.len()==3{"complete supplied replay setup and captured controls, extended to 180 seconds with final controls held; human provenance is not inferred"}else{"authored keyboard-equivalent controls; no human calibration claim"},"reference_revision":"7aecf6044f2dd773b925b325823f30d648d61c89","scope":"source primitives plus intended population policy, not full native scheduler/AI parity"});
    std::fs::write(
        out.join("receipt.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "50 source setups; 15-rider field; four 180-second runs; traced {} decisions samples and {} RNG calls",
        traces.len(),
        rng.len()
    );
    Ok(())
}
fn authored(out: &Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out)?;
    let c = fixtures::riding();
    let mut e = encounter::Setup::authored(&c)?;
    e.topology = topology::Topology::straight(100_000_000);
    e.rules.finish_progress = Some(100_000_000);
    e.rules.bike_contacts = true;
    e.seeds[0].distance = 1000;
    let mut p = Setup {
        traffic: true,
        periodic_police: true,
        ..Default::default()
    };
    for i in 1..15 {
        let mut seed = encounter::Seed::player();
        seed.id = ActorId(i + 1);
        seed.distance = 1000 + i as i64 * 256;
        seed.lateral = if i % 2 == 0 { -64000 } else { 64000 };
        seed.color = i as u8;
        e.seeds.push(seed);
        p.brains.insert(ActorId(i + 1), Traits::default());
    }
    let mut replay = Replay {
        schema_version: 5,
        seed: 7,
        ticks: 600,
        commands: vec![QueuedCommand {
            tick: Tick(0),
            actor: ActorId(1),
            sequence: 0,
            input: Command {
                throttle: true,
                ..Default::default()
            },
        }],
        route: None,
        expected_hash: None,
        handling: Some(c),
        encounter: Some(e),
        population: Some(p),
    };
    let r = run(replay.clone(), "60")?;
    replay.expected_hash = Some(r.state_sha256.clone());
    std::fs::write(
        out.join("population-replay.json"),
        serde_json::to_vec(&replay)?,
    )?;
    println!("{}", r.state_sha256);
    Ok(())
}
