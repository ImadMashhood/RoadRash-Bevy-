//! Private differential evidence for the actual native police rider constructor
//! and shared, command-controlled mounted physics. Cop AI is a separate family.
use super::*;
use serde_json::json;
#[test]
#[ignore = "requires pinned native police rider traces and owned source data"]
fn source_police_riders_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_POLICE").unwrap()).unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let config = population::police::Config::read(&exe).unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut base = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    base.motion_version = 3;
    base.ground = vec![
        handling::Ground {
            left: -5000,
            right: 5000,
            left_shoulder: -5512,
            right_shoulder: 5512,
            ..handling::Ground::flat()
        };
        2048
    ];
    let mut mismatches = vec![];
    let mut observations = 0;
    let mut record =
        |family: &str, id: usize, expected: serde_json::Value, actual: serde_json::Value| {
            observations += 1;
            if expected != actual {
                mismatches
                    .push(json!({"family":family,"id":id,"expected":expected,"actual":actual}));
            }
        };
    record(
        "config",
        0,
        native["config"].clone(),
        serde_json::to_value(&config).unwrap(),
    );
    for (id, row) in native["pursuits"].as_array().unwrap().iter().enumerate() {
        let (words, relationship) = config
            .pursuit(
                row[0].as_u64().unwrap() as u8,
                row[1].as_u64().unwrap() as u8,
                row[2].as_u64().unwrap() as u8,
            )
            .unwrap();
        record(
            "pursuit",
            id,
            json!([row[3], row[4]]),
            json!([relationship, words]),
        );
    }
    for (id, row) in native["spawns"].as_array().unwrap().iter().enumerate() {
        let level = row[0].as_u64().unwrap() as u8;
        let c = config.configuration(&base, level).unwrap();
        let s = c.spec();
        let rv = s.rider_value.unwrap();
        let mass = s.mass + 75;
        let weight = mass + ((rv - 75) >> 3);
        let throttle = s.throttle * mass * 256 / weight;
        let actual = json!([
            row[0],
            row[1],
            row[2],
            row[3],
            row[2],
            -12345,
            0,
            row[3],
            0,
            s.condition,
            c.tables.object[11],
            throttle * s.throttle_percent.unwrap() / 100,
            throttle / s.throttle_on.min(15),
            throttle / s.throttle_off.min(15),
            s.steer_on * mass / (s.mass + rv / 4),
            s.steer_off * mass / (s.mass + rv / 4),
            s.gears[0].force,
            s.gears[1].force,
            s.gears[2].force,
            s.gears[3].force,
            config.stats[12],
            config.stats[13],
            config.stats[13],
            config.stats[13],
            config.weapon(level) as u8
        ]);
        record("spawn", id, row.clone(), actual);
    }
    for episode in native["motion"].as_array().unwrap() {
        let id = episode["id"].as_u64().unwrap() as usize;
        let level = episode["level"].as_u64().unwrap() as u8;
        let steering = episode["steer"].as_i64().unwrap() as i8;
        let c = config.configuration(&base, level).unwrap();
        let mut rider = riding::Rider::new(&c).unwrap();
        rider.bike.distance_q8 = 12800;
        rider.bike.speed = episode["initial"].as_i64().unwrap() as i32;
        rider.animation.queued = Some(2);
        for row in episode["ticks"].as_array().unwrap() {
            let tick = row[0].as_u64().unwrap();
            let command = Command {
                throttle: !(episode["control_mode"] == 1
                    && ((20..=60).contains(&tick) || (100..=110).contains(&tick))),
                brake: episode["control_mode"] == 1 && (20..=60).contains(&tick),
                steer: if tick <= 40 { steering } else { 0 },
                ..Default::default()
            };
            rider.bike.source_drive_mode(
                c.spec(),
                &c.tables,
                if command.brake {
                    2
                } else if command.throttle {
                    1
                } else {
                    0
                },
            );
            rider
                .bike
                .source_steer(c.spec(), &c.tables, i32::from(command.steer) * 0x200000, 0);
            rider
                .step_with_combat_profile(&c, command, tick as u32, 4)
                .unwrap();
            let b = &rider.bike;
            let actual = json!([
                tick,
                b.distance_q8,
                b.lateral,
                b.height,
                b.speed,
                b.lateral_speed,
                b.vertical_speed,
                b.drive,
                b.lean,
                b.condition,
                b.force,
                b.gear,
                i32::from(b.crashed)
            ]);
            record("motion", id * 1000 + tick as usize, row.clone(), actual);
        }
    }
    let mut law = recovery::BodyLaw::from_exe(&exe).unwrap();
    law.motion_version = 1;
    law.recovery_version = if native["live"] == true { 2 } else { 1 };
    for episode in native["recovery"].as_array().unwrap() {
        let id = episode["id"].as_u64().unwrap() as usize;
        let level = episode["level"].as_u64().unwrap() as u8;
        let c = config.configuration(&base, level).unwrap();
        let mut bike = handling::Bike::new(c.spec(), &c.tables).unwrap();
        bike.contact = c.tables.object[11];
        bike.distance_q8 = if episode["away"] == 1 { 128000 } else { 12800 };
        bike.speed = episode["initial"].as_i64().unwrap() as i32;
        if episode["empty"] == 1 {
            bike.condition = 0;
        }
        let topology = topology::Topology::straight(2048 * 256);
        let at = topology::Location {
            progress: bike.distance_q8,
            ..Default::default()
        };
        let mut state = recovery::State::mounted(ActorId(2), at);
        state.police = true;
        let begin = if native["live"] == true {
            recovery::State::begin_live_source
        } else {
            recovery::State::begin_source
        };
        begin(
            &mut state,
            ActorId(2),
            &mut bike,
            &c,
            at,
            recovery::Impact {
                reason: episode["reason"].as_u64().unwrap() as u8,
                ..Default::default()
            },
            &topology,
        );
        if episode["away"] == 1 {
            let b = state.body.as_mut().unwrap();
            b.motion.distance_q8 -= 20000;
            b.location.progress -= 20000;
        }
        if std::env::var_os("RR_POLICE_INITIAL_BIKE").is_some() {
            state.bike_animation.set(c.recovery_animation(), 0, 46);
            state.bike_animation.queued = Some(44);
        }
        let mut seed = 1u32;
        for row in episode["ticks"].as_array().unwrap() {
            let tick = row[0].as_u64().unwrap();
            state
                .step_with_random(
                    &mut bike,
                    &c,
                    &topology,
                    &law,
                    recovery::Assists::default(),
                    None,
                    tick as u32,
                    || {
                        seed = seed.wrapping_mul(214013).wrapping_add(2531011);
                        (seed >> 16) & 0x7fff
                    },
                )
                .unwrap();
            let phase = match state.phase {
                recovery::Phase::Mounted => 0,
                recovery::Phase::Sliding => 1,
                recovery::Phase::Mounting => 3,
                _ => 2,
            };
            let mut actual = vec![
                tick as i64,
                i64::from(seed),
                bike.distance_q8,
                i64::from(bike.lateral),
                i64::from(bike.height),
                i64::from(bike.speed),
                i64::from(bike.vertical_speed),
                i64::from(bike.contact),
                i64::from(bike.condition),
                phase,
                i64::from(state.body.is_some()),
            ];
            if let Some(b) = &state.body {
                actual.extend([
                    b.motion.distance_q8,
                    i64::from(b.motion.lateral),
                    i64::from(b.motion.height),
                    i64::from(b.motion.speed),
                    i64::from(b.motion.vertical_speed),
                    i64::from(b.motion.contact),
                    i64::from(b.location.heading),
                    i64::from(b.animation.code),
                    b.animation.frame as i64,
                    i64::from(b.animation.loops),
                ]);
            } else {
                actual.extend([0, 0, 0, 0, 0, 0, 0, -1, 0, 0]);
            }
            record(
                "recovery",
                id * 10000 + tick as usize,
                row.clone(),
                json!(actual),
            );
        }
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_POLICE_OUTPUT").unwrap());
    std::fs::write(
        out.join("first-divergences.json"),
        serde_json::to_vec_pretty(&mismatches).unwrap(),
    )
    .unwrap();
    let receipt = json!({"outcome":if mismatches.is_empty(){"passed"}else{"failed"},"observations":observations,"mismatches":mismatches.len(),"families":{"complete_recovery":native["recovery"].as_array().unwrap().iter().map(|e|e["ticks"].as_array().unwrap().len()).sum::<usize>(),"configuration":1,"pursuit":native["pursuits"].as_array().unwrap().len(),"spawn":native["spawns"].as_array().unwrap().len(),"mounted_motion":native["motion"].as_array().unwrap().iter().map(|e|e["ticks"].as_array().unwrap().len()).sum::<usize>()},"scope":"Actual native police constructor/pursuit and controlled mounted and complete detached cop trajectories; AI decisions separate"});
    std::fs::write(
        out.join("police-comparison.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(
        mismatches.is_empty(),
        "{} native police mismatches",
        mismatches.len()
    );
}
