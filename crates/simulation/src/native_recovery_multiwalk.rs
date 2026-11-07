//! Full fixed-dt LIVE body/bike motion interleaved with crowded contact walks.
fn state(sim: &Simulation) -> serde_json::Value {
    let mut result = super::native_recovery_contacts::state(sim);
    for ((id, unit), row) in sim
        .encounter
        .as_ref()
        .unwrap()
        .units
        .iter()
        .zip(result[1].as_array_mut().unwrap())
    {
        let clock = unit.recovery.body.as_ref().map_or(
            &sim.actors[id].view.rider.as_ref().unwrap().animation,
            |body| &body.animation,
        );
        row.as_array_mut()
            .unwrap()
            .push(super::native_recovery_contacts::animation(clock));
    }
    result
}
use super::*;
use serde_json::{Value, json};

#[test]
#[ignore = "requires private full native crowded recovery episodes"]
fn source_recovery_multiwalk_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_RECOVERY_MULTIWALK").unwrap()).unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut c = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    c.motion_version = 3;
    c.start_sample = 0;
    c.specs
        .iter_mut()
        .find(|s| s.id == c.selected)
        .unwrap()
        .condition = 150;
    c.ground = vec![handling::Ground::flat(); 2000];
    for g in &mut c.ground {
        g.left = -5000;
        g.right = 5000;
        g.left_shoulder = -5512;
        g.right_shoulder = 5512;
    }
    let mut failures = vec![];
    let mut pre_failures = vec![];
    let mut actual = vec![];
    let mut rows = 0;
    let mut complete = 0;
    for case in native.as_array().unwrap() {
        let input = &case["input"];
        let n = |i: usize| input[i].as_i64().unwrap() as i32;
        let mut e = encounter::Setup::authored(&c).unwrap();
        e.topology = topology::Topology::straight(512000);
        e.rules.combat_version = 4;
        e.rules.level = 0;
        e.rules.immunity = 30;
        e.rules.collision_version = 2;
        e.rules.contact_walk_version = 2;
        e.rules.scenery_version = 1;
        e.rules.scenery_start_ready = true;
        e.rules.collision = Some(collision::Profile::from_exe(&exe).unwrap());
        e.rules.body = recovery::BodyLaw::from_exe(&exe).unwrap();
        e.rules.body.motion_version = 1;
        e.rules.body.recovery_version = 2;
        e.rules.finish_progress = Some(500000);
        e.rules.obstacles.clear();
        let mut seed = e.seeds.remove(0);
        seed.stats = combat::Stats {
            strength: 7,
            base: 2500,
            maximum: 2500,
            health: 2500,
        };
        for i in 0..n(1) {
            let mut s = seed.clone();
            s.id = ActorId(i as u64 + 1);
            s.distance = i64::from(2688 + i * n(2));
            s.lateral = (i - 1) * n(3);
            s.speed = n(4);
            e.seeds.push(s);
        }
        let mut sim = Simulation::with_encounters(1, c.clone(), e).unwrap();
        for i in 0..n(1) {
            let id = ActorId(i as u64 + 1);
            let w = sim.encounter.as_mut().unwrap();
            let u = w.units.get_mut(&id).unwrap();
            let b = &mut sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            b.contact_velocity = Some([n(4), 0, 0]);
            assert!(u.recovery.begin_live_source(
                id,
                b,
                &c,
                u.recovery.bike_location,
                recovery::Impact {
                    reason: n(5) as u8,
                    rider_velocity: Some(if n(5) == 4 {
                        [-80, 0, 0]
                    } else {
                        [-256, 0, 1792]
                    }),
                    bike_velocity: Some(if n(5) == 4 { [0, 0, 0] } else { [0, 0, 1024] }),
                    ..Default::default()
                },
                &w.setup.topology
            ));
        }
        let pre = state(&sim);
        if pre != case["pre"] {
            pre_failures.push(json!({"input":input,"native":case["pre"],"rust":pre}));
        }
        let mut observed = vec![];
        let mut remounted = std::collections::BTreeSet::new();
        for (tick, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
            let held = input.get(6).and_then(Value::as_i64).unwrap_or(0);
            sim.step(Command {
                throttle: held != 0,
                steer: if held == 2 { -1 } else { 0 },
                attack: (held == 2).then_some(combat::Request {
                    kind: combat::AttackKind::Punch,
                    side: combat::Side::Auto,
                }),
                ..Default::default()
            })
            .unwrap();
            let got = state(&sim);
            rows += 1;
            if &got != expected {
                failures.push(json!({"input":input,"tick":tick+1,"native":expected,"rust":got}));
            }
            for (id, u) in &sim.encounter.as_ref().unwrap().units {
                if u.recovery.phase == recovery::Phase::Mounted {
                    remounted.insert(*id);
                }
                if let Some(body) = &u.recovery.body {
                    assert_eq!(body.owner, *id);
                }
            }
            observed.push(got);
        }
        if remounted.len() == n(1) as usize {
            complete += 1;
        }
        actual.push(
            json!({"input":input,"pre":pre,"steps":observed,"remounted_owners":remounted.len()}),
        );
    }
    let out =
        std::path::PathBuf::from(std::env::var_os("RR_NATIVE_RECOVERY_MULTIWALK_OUTPUT").unwrap());
    std::fs::write(out.join("rust.json"), serde_json::to_vec(&actual).unwrap()).unwrap();
    std::fs::write(
        out.join("failures.json"),
        serde_json::to_vec(&failures).unwrap(),
    )
    .unwrap();
    std::fs::write(
        out.join("pre-failures.json"),
        serde_json::to_vec(&pre_failures).unwrap(),
    )
    .unwrap();
    std::fs::write(out.join("recovery-multiwalk-comparison.json"),serde_json::to_vec_pretty(&json!({"outcome":if failures.is_empty()&&pre_failures.is_empty()&&complete==native.as_array().unwrap().len(){"passed"}else{"failed"},"cases":native.as_array().unwrap().len(),"rows":rows,"mismatches":failures.len(),"pre_mismatches":pre_failures.len(),"episodes_all_owners_remounted":complete,"first_divergence":failures.first(),"first_pre_divergence":pre_failures.first(),"scope":"Actual fixed-dt LIVE per-actor half/bike movement then sorted crowded contact walk, from controlled initial falls through every owner's remount plus30ticks"})).unwrap()).unwrap();
    assert!(
        pre_failures.is_empty(),
        "{}initialdifferences",
        pre_failures.len()
    );
    assert!(failures.is_empty(), "{}differences", failures.len());
    assert_eq!(
        complete,
        native.as_array().unwrap().len(),
        "every owner must remount"
    );
}
