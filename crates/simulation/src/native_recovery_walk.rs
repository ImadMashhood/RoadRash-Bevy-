//! Actual native manual walking wrapper and strict near-bike pickup decisions.
use super::*;
use serde_json::json;
#[test]
#[ignore = "requires private native recovery walk-control observations"]
fn source_recovery_walk_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_WALK").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let c = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let topology = topology::Topology::straight(2048 * 256);
    let mut observations = 0;
    let mut controls = 0;
    let mut comparisons = 0;
    let mut mismatches = 0;
    let mut counted_mismatches = 0;
    let mut control_mismatches = 0;
    let mut failures = vec![];
    let mut output = vec![];
    for (id, expected) in expected.as_array().unwrap().iter().enumerate() {
        let n = |i: usize| expected[i].as_i64().unwrap();
        let bits = n(5) as u32;
        let mapped = recovery::Walk {
            longitudinal: if bits & 0x1000 != 0 {
                -1
            } else if bits & 0x800 != 0 {
                1
            } else {
                0
            },
            lateral: if bits & 0x4000 != 0 {
                -1
            } else if bits & 0x2000 != 0 {
                1
            } else {
                0
            },
        };
        let variants = if bits == 0 {
            vec![None, Some(recovery::Walk::default())]
        } else {
            vec![Some(mapped)]
        };
        if bits == 0 {
            controls += 1;
        } else {
            observations += 1;
        }
        for walk in variants {
            let at = topology::Location {
                progress: 12800 + n(2),
                lateral: 6000 + n(3) as i32,
                ..Default::default()
            };
            let mut bike = handling::Bike::new(c.spec(), &c.tables).unwrap();
            bike.distance_q8 = at.progress;
            bike.lateral = at.lateral;
            bike.condition = n(1) as i32;
            let mut state = recovery::State::mounted(ActorId(1), at);
            state.begin_live_source(
                ActorId(1),
                &mut bike,
                &c,
                at,
                recovery::Impact {
                    reason: 4,
                    rider_velocity: Some([0; 3]),
                    bike_velocity: Some([0; 3]),
                    ..Default::default()
                },
                &topology,
            );
            state.phase = recovery::Phase::Walking;
            state.elapsed = 123;
            let body = state.body.as_mut().unwrap();
            body.location.progress = 12800;
            body.location.lateral = 0;
            body.location.height = 0;
            body.location.heading = n(4) as i32;
            body.visual_heading = Some(n(4) as i32);
            body.motion.distance_q8 = 12800;
            body.motion.lateral = 0;
            body.motion.height = 0;
            body.motion.speed = 400;
            body.animation.set(&c.animation, 0, 38);
            body.animation.queued = None;
            state
                .prepare_live_recovery(&mut bike, &c, &topology, walk, n(0) != 0)
                .unwrap();
            let body = state.body.as_ref().unwrap();
            let phase = if state.phase == recovery::Phase::Mounting {
                3
            } else {
                2
            };
            let actual = vec![
                phase,
                i64::from(bike.lateral),
                i64::from(bike.condition),
                body.location.progress,
                i64::from(body.motion.lateral),
                i64::from(body.motion.speed),
                i64::from(body.location.heading),
                i64::from(body.visual_heading.unwrap_or(body.location.heading)),
                i64::from(body.animation.code),
                body.animation.frame as i64,
                i64::from(body.animation.queued.unwrap_or(-1)),
                i64::from(body.animation.loops),
                i64::from(state.elapsed),
                1,
            ];
            comparisons += 1;
            let native = &expected.as_array().unwrap()[6..];
            if json!(actual) != json!(native) {
                mismatches += 1;
                if bits == 0 {
                    control_mismatches += 1;
                } else {
                    counted_mismatches += 1;
                }
                if failures.len() < 80 {
                    failures.push(json!({"id":id,"input":&expected.as_array().unwrap()[..6],"walk":walk,"expected":native,"actual":actual}));
                }
            }
            output.push(json!({"id":id,"walk":walk,"actual":actual}));
        }
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_WALK_OUTPUT").unwrap());
    std::fs::write(
        out.join("recovery-walk-rust.json"),
        serde_json::to_vec(&output).unwrap(),
    )
    .unwrap();
    std::fs::write(out.join("recovery-walk-comparison.json"), serde_json::to_vec_pretty(&json!({"outcome":if mismatches == 0 {"passed"} else {"failed"},"observations":observations,"regression_control_observations":controls,"comparisons":comparisons,"mismatches":mismatches,"counted_mismatches":counted_mismatches,"control_mismatches":control_mismatches,"first_divergences":failures,"scope":"Actual native crash_recovery wrapper, eight nonzero cardinal/diagonal walking control combinations, player/NPC eligibility, strict pickup thresholds and initial facing; native0 controls are regression controls excluded from additive count"})).unwrap()).unwrap();
    assert_eq!(mismatches, 0, "native manual walk decision differs");
}
