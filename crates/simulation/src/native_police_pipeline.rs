//! Full controlled cop think/fight/motion/contact integration, until first crash.
use super::*;
use serde_json::json;
#[test]
#[ignore = "requires actual native coupled cop pipeline traces"]
fn source_police_pipeline_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_POLICE_PIPELINE").unwrap()).unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let police = population::police::Config::read(&exe).unwrap();
    let pe = rr_formats::pe::SupportedExe::read(&exe).unwrap();
    let steal = std::array::from_fn(|i| {
        rr_formats::binary::u32le(pe.read_va(0x464538 + i as u32 * 4, 4).unwrap(), 0).unwrap() as i8
    });
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
    let mut observations = 0;
    for case in native.as_array().unwrap() {
        let level = case["level"].as_u64().unwrap() as u8;
        let side = case["side"].as_i64().unwrap() as i32;
        let mut e = encounter::Setup::authored(&c).unwrap();
        e.rules.combat_version = 4;
        e.rules.steal_codes = Some(steal);
        e.rules.collision_version = 2;
        e.rules.contact_walk_version = 1;
        e.rules.collision = Some(collision::Profile::from_exe(&exe).unwrap());
        e.rules.body = recovery::BodyLaw::from_exe(&exe).unwrap();
        e.rules.body.motion_version = 1;
        e.rules.body.recovery_version = if case["live"] == true { 2 } else { 1 };
        e.rules.level = level;
        e.rules.immunity = u16::from(5 - level) * 6;
        e.rules.finish_progress = Some(500000);
        e.seeds[0].distance = 4864;
        for n in 2..=3 {
            let mut seed = encounter::Seed::player();
            seed.id = ActorId(n);
            seed.distance = if n == 2 {
                12800 + case["gap"].as_i64().unwrap()
            } else {
                128000
            };
            seed.lateral = if n == 2 {
                (if case["attack"] == 1 { 64000 } else { 0 }) + side * 16000
            } else {
                0
            };
            e.seeds.push(seed);
        }
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
        assert_eq!(id, ActorId(4));
        sim.actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .speed = case["initial"].as_i64().unwrap() as i32;
        let mut saw_crash = false;
        for row in case["rows"].as_array().unwrap() {
            sim.actors.get_mut(&ActorId(2)).unwrap().input.attack = (case["attack"] == 1
                && (case["recovery"] != true || !saw_crash))
                .then_some(combat::Request {
                    kind: combat::AttackKind::Punch,
                    side: combat::Side::Auto,
                });
            sim.advance().unwrap();
            let e = sim.encounter.as_ref().unwrap();
            saw_crash |= e.units[&id].recovery.phase != recovery::Phase::Mounted;
            let mut actual = vec![row[0].as_i64().unwrap(), i64::from(sim.gameplay_rng.0)];
            for id in [ActorId(2), ActorId(4)] {
                let u = &e.units[&id];
                let r = sim.actors[&id].view.rider.as_ref().unwrap();
                let b = &r.bike;
                let anim = u
                    .recovery
                    .body
                    .as_ref()
                    .map_or(&r.animation, |body| &body.animation);
                let phase = match u.recovery.phase {
                    recovery::Phase::Mounted => 0,
                    recovery::Phase::Sliding => 1,
                    recovery::Phase::Mounting => 3,
                    _ => 2,
                };
                actual.extend([
                    b.distance_q8,
                    i64::from(b.lateral),
                    i64::from(b.height),
                    i64::from(b.speed),
                    i64::from(b.lateral_speed),
                    i64::from(b.vertical_speed),
                    i64::from(b.drive),
                    i64::from(b.lean),
                    i64::from(b.condition),
                    phase,
                    i64::from(u.fighter.stats.health),
                    i64::from(u.fighter.stats.maximum),
                    u.fighter.hit_until as i64,
                    u.fighter.steal_until as i64,
                    r.weapon as i64,
                    i64::from(anim.code),
                    anim.frame as i64,
                    i64::from(anim.queued.unwrap_or(-1)),
                ]);
            }
            let ai = e.units[&id].police_ai.as_ref().unwrap();
            actual.extend([
                i64::from(ai.words[0x160 / 4]),
                ai.target.map_or(-1, |id| id.0 as i64 - 1),
                i64::from(ai.words[0x174 / 4]),
                i64::from(ai.words[0x198 / 4]),
                i64::from(ai.words[0x194 / 4]),
                i64::from(ai.words[0x19c / 4]),
                i64::from(ai.words[0x188 / 4]),
            ]);
            observations += 1;
            if json!(actual) != *row {
                failures
                    .push(json!({"case":case["id"],"tick":row[0],"expected":row,"actual":actual}));
            }
        }
    }
    let out =
        std::path::PathBuf::from(std::env::var_os("RR_NATIVE_POLICE_PIPELINE_OUTPUT").unwrap());
    std::fs::write(
        out.join("first-divergences.json"),
        serde_json::to_vec_pretty(&failures).unwrap(),
    )
    .unwrap();
    let full_recovery = native[0]["recovery"] == true;
    let receipt = json!({"outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":observations,"mismatches":failures.len(),"episodes":native.as_array().unwrap().len(),"full_recovery":full_recovery,"scope":if full_recovery {"Actual LIVE cop AI/combat/animation/half/bike callback composition through natural depletion, recovery, remount and30 postmount ticks; explicit dt1/fixed host actor order, not native variable-dt scheduler parity"} else {"Actual LIVE native half/bike callback composition at explicit dt1 and fixed host actor order, through first crash; no native variable-dt scheduler parity"}});
    std::fs::write(
        out.join("police-pipeline-comparison.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(
        failures.is_empty(),
        "{} coupled police mismatches",
        failures.len()
    );
}
