//! Actual one-tick mixed static/rider/vehicle callback ordering.
use super::*;
use serde_json::{Value, json};
fn car_position(input: &Value, j: i32) -> i64 {
    (if input[2].as_i64().unwrap() >= 20000 {
        2820
    } else {
        2700
    }) + i64::from(j * 20)
}
fn car_lateral(input: &Value, j: i32) -> i32 {
    let side = input[3].as_i64().unwrap() as i32;
    let sign = if j % 2 != 0 { side } else { -side };
    if input[2].as_i64().unwrap() >= 20000 {
        if j == 0 { 0 } else { sign * 4000 }
    } else {
        sign * (10000 + j * 1000)
    }
}
fn car_height(input: &Value, j: i32) -> i32 {
    if input.get(7).and_then(Value::as_i64) == Some(1) && j > 0 {
        4096
    } else {
        0
    }
}
fn state(sim: &Simulation, input: &Value) -> Value {
    let n = |i: usize| input[i].as_i64().unwrap() as i32;
    let mut rows = vec![];
    let world = sim.encounter.as_ref().unwrap();
    for id in (1..=n(5)).map(|i| ActorId(i as u64)) {
        let r = sim.actors[&id].view.rider.as_ref().unwrap();
        let b = &r.bike;
        let u = &world.units[&id];
        rows.push(json!([
            b.distance_q8,
            b.lateral,
            b.height,
            u.previous_location.progress,
            u.previous_location.lateral,
            u.previous_location.height,
            b.speed,
            b.lateral_speed,
            b.vertical_speed,
            b.contact_velocity.map_or(b.speed, |v| v[0]),
            b.contact_velocity.map_or(b.lateral_speed, |v| v[1]),
            b.contact_velocity.map_or(b.vertical_speed, |v| v[2]),
            b.drive,
            b.lean,
            b.condition,
            i32::from(b.crashed),
            u.fighter.stats.health,
            u.fighter.stats.maximum,
            u.recovery.body.as_ref().map(|body| [
                body.motion.distance_q8,
                i64::from(body.motion.lateral),
                i64::from(body.motion.height),
                i64::from(body.motion.speed),
                i64::from(body.motion.vertical_speed)
            ])
        ]));
    }
    let p = sim.population.as_ref().unwrap();
    for j in 0..n(6) {
        if let Some(car) = p.agents.values().nth(j as usize) {
            rows.push(json!([
                car.location.progress,
                car.location.lateral,
                car.location.height,
                car.speed
            ]));
        } else {
            rows.push(json!([
                car_position(input, j),
                car_lateral(input, j),
                car_height(input, j),
                0
            ]));
        }
    }
    json!(rows)
}
#[test]
#[ignore = "requires actual native mixed static/rider/traffic producer"]
fn source_mixed_traffic_order_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_MIXED_TRAFFIC").unwrap()).unwrap(),
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
        g.left = -2000;
        g.right = 2000;
        g.left_shoulder = -2512;
        g.right_shoulder = 2512;
    }
    let legacy = std::env::var("RR_MIXED_LEGACY").is_ok_and(|s| s == "1");
    let legacy_expected: Option<Value> = std::env::var_os("RR_MIXED_LEGACY_EXPECTED")
        .map(|p| serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap());
    let mut failures = vec![];
    let mut rng_failures = vec![];
    let mut pre_failures = vec![];
    let mut actual = vec![];
    let mut reversals = 0;
    let mut reversed_matches = 0;
    let mut car_outer_matches = 0;
    for (case_index, case) in native.as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let n = |i: usize| input[i].as_i64().unwrap() as i32;
        let mut pre = None;
        let mut pre_rng = 0;
        let mut pre_hash = String::new();
        for control in [true, false] {
            let mut e = encounter::Setup::authored(&c).unwrap();
            e.topology = topology::Topology::straight(512000);
            e.rules.combat_version = if legacy { 3 } else { 4 };
            e.rules.collision_version = 2;
            e.rules.contact_walk_version = u8::from(!control);
            e.rules.bike_contacts = !control;
            e.rules.scenery_version = 1;
            e.rules.scenery_start_ready = true;
            e.rules.collision = Some(collision::Profile::from_exe(&exe).unwrap());
            e.rules.body = recovery::BodyLaw::from_exe(&exe).unwrap();
            e.rules.body.motion_version = 1;
            e.rules.body.recovery_version = 1;
            e.rules.finish_progress = Some(500000);
            e.seeds[0].distance = 2688;
            e.seeds[0].speed = n(2);
            e.seeds[0].stats = combat::Stats {
                strength: 7,
                base: 2500,
                maximum: 2500,
                health: 2500,
            };
            for i in 1..n(5) {
                let mut other = e.seeds[0].clone();
                other.id = ActorId(i as u64 + 1);
                other.distance = if n(4) & 4 != 0 {
                    2688 + i64::from(i * 20)
                } else {
                    128000 + i64::from(i * 1000)
                };
                other.lateral = if i == 1 { n(3) * 10000 } else { -n(3) * 12000 };
                e.seeds.push(other);
            }
            if !control && n(4) & 1 != 0 {
                e.rules.obstacles.push(encounter::Obstacle {
                    id: 1,
                    path: 0,
                    progress: 2688,
                    half_length: 1,
                    left: -1024,
                    right: 1024,
                    height: 4096,
                    kind: encounter::ObstacleKind::Solid,
                    source_box: Some(encounter::StaticBox {
                        bounds: match n(1) {
                            1 => [-1024, -4096, 1024, 4096],
                            2 => [-1024, -12287, 1024, 4096],
                            _ => [-1024, 0, 1024, 4096],
                        },
                        lateral: 0,
                        query_bounds: Some([-1024, 0, 1024, 4096]),
                        scenery: Some(scenery::BoxRules {
                            class: n(1) as u8,
                            road_mask: 3,
                            sample: 10,
                            selector: 7,
                            ordinal: 0,
                            box_count: 1,
                            texture_flags: None,
                        }),
                    }),
                });
            }
            let p = population::Setup {
                traffic: false,
                periodic_police: false,
                functional_traffic: false,
                ..Default::default()
            };
            let mut sim = Simulation::with_population(1, c.clone(), e, p).unwrap();
            if std::env::var("RR_MIXED_MUTED").is_ok_and(|v| v == "1") {
                sim.set_effects_enabled(false);
            }
            if !control && n(4) & 2 != 0 {
                for j in 0..n(6) {
                    sim.spawn_road_user(&population::Spawn {
                        id: 1,
                        location: topology::Location {
                            progress: car_position(input, j),
                            lateral: car_lateral(input, j),
                            height: car_height(input, j),
                            ..Default::default()
                        },
                        kind: population::Kind::Vehicle,
                        family: None,
                        selector: 0,
                        subtype: 0,
                        speed: 0,
                        direction: 1,
                    })
                    .unwrap();
                }
            }
            sim.advance().unwrap();
            let observed = state(&sim, input);
            if control {
                pre_rng = sim.gameplay_rng.0;
                pre_hash = sim.state_hash();
                pre = Some(observed);
            } else {
                reversed_matches += usize::from(observed == case["legacy"]);
                car_outer_matches += usize::from(observed == case["car_outer"]);
                let expected = legacy_expected
                    .as_ref()
                    .map_or(&case[if legacy { "legacy" } else { "output" }], |v| {
                        &v[case_index]["output"]
                    });
                if &observed != expected {
                    failures.push(json!({"input":input,"native":expected,"rust":observed}));
                }
                let post_rng = sim.gameplay_rng.0;
                let post_hash = sim.state_hash();
                if let Some(previous) = legacy_expected.as_ref() {
                    if previous[case_index]["post_hash"] != post_hash
                        || previous[case_index]["pre_hash"] != pre_hash
                    {
                        rng_failures.push(json!({"input":input,"before":previous[case_index]["post_hash"],"after":post_hash}));
                    }
                } else if !legacy
                    && case.get("rng_output").is_some()
                    && (case["rng_pre"] != pre_rng || case["rng_output"] != post_rng)
                {
                    rng_failures.push(json!({"input":input,"native_pre":case["rng_pre"],"rust_pre":pre_rng,"native_post":case["rng_output"],"rust_post":post_rng}));
                }
                actual.push(json!({"input":input,"pre":pre,"output":observed,"pre_rng":pre_rng,"post_rng":post_rng,"pre_hash":pre_hash,"post_hash":post_hash}));
            }
        }
        if pre.as_ref().unwrap() != &case["pre"] {
            pre_failures.push(json!({"input":input,"native":case["pre"],"rust":pre}));
        }
        reversals += usize::from(case["output"] != case["legacy"]);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_MIXED_TRAFFIC_OUTPUT").unwrap());
    std::fs::write(
        out.join("mixed-traffic-rust.json"),
        serde_json::to_vec(&actual).unwrap(),
    )
    .unwrap();
    let receipt = json!({"outcome":if failures.is_empty()&&pre_failures.is_empty()&&rng_failures.is_empty(){"passed"}else{"failed"},"legacy_profile":legacy,"legacy_runtime_compatibility":legacy_expected.is_some(),"rng_or_legacy_hash_mismatches":rng_failures.len(),"first_rng_or_hash_divergence":rng_failures.first(),"rows":actual.len(),"mismatches":failures.len(),"pre_mismatches":pre_failures.len(),"native_order_sensitive_cases":reversals,"rust_matches_reversed_native":reversed_matches,"rust_matches_car_outer_native":car_outer_matches,"first_divergence":failures.first(),"first_pre_divergence":pre_failures.first()});
    std::fs::write(
        out.join("mixed-traffic-comparison.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(
        failures.is_empty() && pre_failures.is_empty() && rng_failures.is_empty(),
        "{receipt}"
    );
}
