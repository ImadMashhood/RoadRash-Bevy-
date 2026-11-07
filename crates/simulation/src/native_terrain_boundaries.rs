use super::*;
use building_wall::{Boundary, Neighbors, Query, Surface};

fn ground_boundary(row: &serde_json::Value) -> Boundary {
    let n = |i: usize| row[i].as_i64().unwrap() as i32;
    let mut points = [[[0; 2]; 3]; 2];
    for (side, p) in points.iter_mut().enumerate() {
        for (band, p) in p.iter_mut().enumerate() {
            *p = [n(17 + side * 6 + band * 2), n(18 + side * 6 + band * 2)];
        }
    }
    Boundary {
        path: 0,
        sample: 0,
        building: n(0) == 0,
        road: [n(2), n(3)],
        outer: [n(4), n(5)],
        selectors: [n(12) as u8, n(13) as u8],
        flags: n(14) as u8,
        surface: Some(Surface {
            kind: n(0) as u8,
            center: n(1),
            base: [n(6), n(7)],
            top: [n(8), n(9)],
            height: [n(10), n(11)],
            side_height: [n(15), n(16)],
            points,
            event_type: n(29) as u8,
            rail_offset: n(30) as i8,
            rail_flags: n(31) as u8,
            drawn_rail_height: (n(32) > 0).then_some(n(32)),
        }),
    }
}

#[test]
#[ignore = "requires pinned native integer terrain-height queries"]
fn source_ground_native() {
    let legacy = std::env::var_os("RR_GROUND_LEGACY").is_some();
    let native: serde_json::Value =
        serde_json::from_slice(&std::fs::read(std::env::var("RR_NATIVE_GROUND").unwrap()).unwrap())
            .unwrap();
    let rows = native["ground"].as_array().unwrap();
    let mut failures = Vec::new();
    for row in rows {
        let mut current = ground_boundary(&row["current"]);
        let mut next = ground_boundary(&row["next"]);
        if legacy {
            current.surface = None;
            next.surface = None;
        }
        let actual = building_wall::ground_height(
            &current,
            &next,
            row["lateral"].as_i64().unwrap() as i32,
            row["fraction"].as_u64().unwrap() as u8,
        )
        .unwrap();
        if i64::from(actual) != row["height"].as_i64().unwrap() {
            failures
                .push(serde_json::json!({"id":row["id"],"expected":row["height"],"actual":actual}));
        }
    }
    let output = std::path::PathBuf::from(std::env::var("RR_GROUND_OUTPUT").unwrap());
    std::fs::write(output.join("ground-comparison.json"),serde_json::to_vec_pretty(&serde_json::json!({"outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":rows.len(),"legacy_profile":legacy,"mismatches":failures.len(),"first_mismatches":failures.iter().take(12).collect::<Vec<_>>(),"scope":"Pinned native terrain_height integer query forbuilding,mountain,cliff,tunnel,hill,open ground; no trajectoryclaim"})).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "{} native terrain height mismatches",
        failures.len()
    );
}

#[test]
#[ignore = "requires private source data and pinned terrain boundary transactions"]
fn source_boundary_native() {
    let data = std::path::PathBuf::from(std::env::var("RR_NATIVE_DATA").unwrap());
    let native: Vec<serde_json::Value> = serde_json::from_slice(
        &std::fs::read(std::env::var("RR_NATIVE_BOUNDARIES").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let topology = topology::Topology::straight(100000);
    let legacy = std::env::var_os("RR_BOUNDARY_LEGACY").is_some();
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    for case in &native {
        let boundary = |row: &serde_json::Value, sample| {
            let n = |i: usize| row[i].as_i64().unwrap() as i32;
            Boundary {
                path: 0,
                sample,
                building: n(0) == 0,
                road: [n(1), n(2)],
                outer: [n(3), n(4)],
                flags: n(5) as u8,
                selectors: [n(6) as u8, n(7) as u8],
                surface: (!legacy).then_some(Surface {
                    kind: n(0) as u8,
                    center: 0,
                    base: [n(8), n(9)],
                    top: [n(10), n(11)],
                    height: [n(12), n(13)],
                    side_height: [0; 2],
                    points: [[[0; 2]; 3]; 2],
                    event_type: n(14) as u8,
                    rail_offset: n(15) as i8,
                    rail_flags: n(16) as u8,
                    drawn_rail_height: None,
                }),
            }
        };
        let current = boundary(&case["boundaries"][0], 10);
        let next = boundary(&case["boundaries"][1], 11);
        let previous = boundary(&case["boundaries"][2], 9);
        let previous_next = boundary(&case["boundaries"][3], 10);
        let n = |i: usize| case["query"][i].as_i64().unwrap() as i32;
        let query = Query {
            progress: n(0),
            previous_progress: n(1),
            lateral: n(2),
            previous_lateral: n(3),
            half_width: n(4),
            half_height: n(5),
            fallen_height: n(6),
            suspension: n(7),
            height: n(8),
            contact: n(9),
            category: n(10) as u32,
            player: n(11) != 0,
            owner_crashed: n(12) != 0,
        };
        let m = |i: usize| case["motion"][i].as_i64().unwrap() as i32;
        let mut motion = collision::Motion {
            speed: m(0),
            lateral: m(1),
            vertical: m(2),
            lean: m(3),
            crashed: m(4) != 0,
            steering_gain: 0,
            elasticity: 0,
        };
        let contact = building_wall::resolve_surface(
            Neighbors {
                current: &current,
                next: &next,
                previous: &previous,
                previous_next: &previous_next,
            },
            &query,
            &mut motion,
        );
        let mut bike = handling::Bike::new(config.spec(), &config.tables).unwrap();
        bike.distance_q8 = i64::from(if contact.restore_progress {
            query.previous_progress
        } else {
            query.progress
        });
        bike.lateral = query.lateral;
        bike.speed = motion.speed;
        bike.lateral_speed = contact.crash_lateral.unwrap_or(motion.lateral);
        bike.vertical_speed = motion.vertical;
        bike.lean = motion.lean;
        bike.height = query.height;
        let mut state = recovery::State::mounted(ActorId(1), topology::Location::default());
        let reason = if contact.crash {
            if contact.rail_crash { 3 } else { 6 }
        } else {
            0
        };
        if contact.crash {
            state.begin_source(
                ActorId(1),
                &mut bike,
                &config,
                topology::Location {
                    progress: i64::from(query.progress),
                    lateral: query.lateral,
                    height: query.height,
                    ..Default::default()
                },
                recovery::Impact {
                    reason,
                    rider_velocity: Some(if contact.rail_crash {
                        [0; 3]
                    } else {
                        [0, 0, 80]
                    }),
                    bike_velocity: Some(if contact.rail_crash {
                        [0; 3]
                    } else {
                        [0, 0, 16]
                    }),
                    ..Default::default()
                },
                &topology,
            );
        }
        bike.lateral_speed = motion.lateral;
        if contact.hit {
            bike.lateral = contact.lateral;
        }
        let mut result = vec![
            bike.distance_q8 as i32,
            bike.lateral,
            bike.speed,
            bike.lateral_speed,
            bike.lean,
            bike.height,
            bike.vertical_speed,
            i32::from(state.body.is_some()),
            i32::from(reason),
        ];
        if let Some(body) = state.body.as_ref() {
            result.extend([
                body.motion.height,
                body.motion.speed,
                body.motion.lateral_speed,
                body.motion.vertical_speed,
                i32::from(body.animation.code),
            ]);
        } else {
            result.extend([0; 5]);
        }
        result.push(contact.visual_heading.unwrap_or(0));
        let expected: Vec<i32> = case["expected"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap() as i32)
            .collect();
        if result != expected {
            failures.push(serde_json::json!({"id":case["id"],"expected":expected,"actual":result}));
        }
        rows.push(result);
    }
    let output = std::path::PathBuf::from(std::env::var("RR_BOUNDARY_OUTPUT").unwrap());
    std::fs::write(output.join("rust.json"), serde_json::to_vec(&rows).unwrap()).unwrap();
    std::fs::write(output.join("rust-comparison.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":native.len(),"mismatches":failures.len(),
        "first_mismatches":failures.iter().take(12).collect::<Vec<_>>(),"legacy_profile":legacy,
        "scope":"Actual native wall/rail detection and response plus crash entry, frozen actor poses; motion/vertical ordering verified separately",
    })).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "{} terrain boundary mismatches",
        failures.len()
    );
}
