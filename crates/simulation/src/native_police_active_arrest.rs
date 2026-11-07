//! Controlled actual LIVE player think arrest timing, with staged descriptor hit.
use super::*;
use serde_json::json;
#[test]
#[ignore = "requires native active player police arrest traces"]
fn source_police_active_arrest_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let rows: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_POLICE_ACTIVE_ARREST").unwrap()).unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let police = population::police::Config::read(&exe).unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut c = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    c.motion_version = 3;
    c.ground = vec![
        handling::Ground {
            left: -500,
            right: 500,
            left_shoulder: -1012,
            right_shoulder: 1012,
            ..handling::Ground::flat()
        };
        2048
    ];
    let mut failures = vec![];
    for row in rows.as_array().unwrap() {
        let speed = row[0].as_i64().unwrap() as i32;
        let gap = row[1].as_i64().unwrap();
        let active = row[2] == 1;
        let phase = row[3].as_u64().unwrap() as u8;
        let mut e = encounter::Setup::authored(&c).unwrap();
        e.rules.combat_version = 4;
        e.rules.body = recovery::BodyLaw::from_exe(&exe).unwrap();
        e.rules.body.motion_version = 1;
        e.rules.body.recovery_version = 2;
        e.seeds[0].distance = 12800;
        e.seeds[0].speed = speed.max(0);
        let mut target = encounter::Seed::player();
        target.id = ActorId(2);
        target.distance = 12863;
        target.lateral = 16000;
        e.seeds.push(target);
        let p = population::Setup {
            police: Some(police.clone()),
            traffic: false,
            periodic_police: false,
            ..Default::default()
        };
        let mut sim = Simulation::with_population(1, c.clone(), e, p).unwrap();
        sim.spawn_road_user(&population::Spawn {
            id: 1,
            location: topology::Location {
                progress: 12800,
                ..Default::default()
            },
            kind: population::Kind::Police,
            family: None,
            selector: 0,
            subtype: 0,
            speed: 6000,
            direction: 1,
        })
        .unwrap();
        sim.police_spawn_sync().unwrap();
        let id = sim
            .population
            .as_ref()
            .unwrap()
            .police
            .as_ref()
            .unwrap()
            .actor;
        sim.actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .distance_q8 = 12800 + gap;
        sim.encounter
            .as_mut()
            .unwrap()
            .units
            .get_mut(&id)
            .unwrap()
            .recovery
            .bike_location
            .progress = 12800 + gap;
        sim.actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .speed = speed;
        if phase != 0 {
            let e = sim.encounter.as_mut().unwrap();
            let u = e.units.get_mut(&ActorId(1)).unwrap();
            let r = sim
                .actors
                .get_mut(&ActorId(1))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            u.recovery.begin_live_source(
                ActorId(1),
                &mut r.bike,
                &c,
                u.recovery.bike_location,
                recovery::Impact {
                    reason: 4,
                    ..Default::default()
                },
                &e.setup.topology,
            );
            u.recovery.phase = match phase {
                1 => recovery::Phase::Sliding,
                3 | 5 => recovery::Phase::Mounting,
                _ => recovery::Phase::Walking,
            };
            let b = u.recovery.body.as_mut().unwrap();
            b.motion.distance_q8 = 12800;
            b.location.progress = 12800;
            b.motion.lateral = 0;
            b.location.lateral = 0;
            b.motion.speed = if phase == 1 { 1800 } else { 0 };
            b.motion.height = 0;
            b.motion.vertical_speed = 0;
            b.animation.code = match phase {
                1 => 35,
                2 | 4 => 38,
                3 => 41,
                _ => 2,
            };
            b.animation.frame = 0;
            b.animation.deadline = Some(200);
            r.bike.distance_q8 = if phase == 2 { 13800 } else { 12800 };
            u.recovery.bike_location.progress = r.bike.distance_q8;
            r.bike.lateral = if phase == 4 { 6000 } else { 0 };
            u.recovery.bike_location.lateral = r.bike.lateral;
            r.bike.speed = speed;
        }
        sim.tick = Tick(100);
        if active {
            let request = combat::Request {
                kind: combat::AttackKind::Backhand,
                side: combat::Side::Right,
            };
            let def = combat::Definition::from_library_version(
                &c.animation,
                request,
                animation::Weapon::Fists,
                4,
            )
            .unwrap();
            let r = sim
                .actors
                .get_mut(&ActorId(1))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            r.animation.set(&c.animation, 0, def.code);
            r.animation.frame = 2;
            r.animation.deadline = Some(200);
            sim.encounter
                .as_mut()
                .unwrap()
                .units
                .get_mut(&ActorId(1))
                .unwrap()
                .fighter
                .swing = Some(combat::Swing {
                repeat: false,
                request,
                target: Some(ActorId(2)),
                weapon: animation::Weapon::Fists,
                started: 99,
                definition: def,
                resolved: false,
                swish: false,
            });
        }
        sim.source_riders_step().unwrap();
        let e = sim.encounter.as_ref().unwrap();
        let u = &e.units[&ActorId(1)];
        let busted = u.terminal_request == Some(recovery::Outcome::Arrested)
            || u.recovery.outcome == Some(recovery::Outcome::Arrested);
        let actual = json!([
            speed,
            gap,
            i32::from(active),
            phase,
            i32::from(busted),
            sim.actors[&ActorId(1)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .bike
                .speed
        ]);
        let expected = json!(row.as_array().unwrap()[..6]);
        if actual != expected {
            failures.push(json!({"expected":expected,"actual":actual}));
        }
    }
    let out = std::path::PathBuf::from(
        std::env::var_os("RR_NATIVE_POLICE_ACTIVE_ARREST_OUTPUT").unwrap(),
    );
    std::fs::write(
        out.join("first-divergences.json"),
        serde_json::to_vec_pretty(&failures).unwrap(),
    )
    .unwrap();
    let receipt = json!({"observations":rows.as_array().unwrap().len(),"mismatches":failures.len(),"scope":"Actual LIVE player callback fight-controls-arrest-animation-motion at dt1; staged frame2 backhand and sliding/walking/mount/remount strict speed/range controls"});
    std::fs::write(
        out.join("police-active-arrest-comparison.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(failures.is_empty(), "{} mismatches", failures.len());
}
