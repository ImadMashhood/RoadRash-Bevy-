//! Controlled native sub-slice draw-window queries, separate from collision responses.
use super::*;

#[test]
#[ignore = "requires pinned native recovery rail draw-window observations"]
fn native_recovery_rail_camera_window() {
    let rows: Vec<Vec<i64>> = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_RECOVERY_RAIL").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(rows.len(), 2520);
    let mut simulation = fixture();
    let world = simulation.encounter.as_mut().unwrap();
    let mut failures = vec![];
    for (index, row) in rows.iter().enumerate() {
        for complete in [false, true] {
            let actual = query(world, row, complete);
            if i64::from(actual) != row[6] {
                failures.push(serde_json::json!({"row":index,"complete":complete,"native":row,"rust_flags":actual}));
            }
        }
    }
    let output =
        std::path::PathBuf::from(std::env::var_os("RR_NATIVE_RECOVERY_RAIL_OUTPUT").unwrap());
    std::fs::write(output.join("rail-comparison.json"),serde_json::to_vec_pretty(&serde_json::json!({"outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":rows.len(),"rust_query_forms_per_observation":2,"mismatches":failures.len(),"first_mismatches":failures.iter().take(12).collect::<Vec<_>>(),"scope":"Native rail sub-slice drawn-height byte at camera+31 strict boundary; pending and completed Rust camera forms; controlled query kernel, not full collision/Ring parity"})).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "{} native rail draw-window differences",
        failures.len()
    );
}

fn fixture() -> crate::Simulation {
    let config = crate::fixtures::riding();
    let setup = Setup::authored(&config).unwrap();
    let mut simulation = crate::Simulation::with_encounters(1, config, setup).unwrap();
    let world = simulation.encounter.as_mut().unwrap();
    world.setup.rules.contact_walk_version = 2;
    world.units.get_mut(&ActorId(1)).unwrap().source_route = Some(0);
    world.setup.rules.source_roads = Some(crate::source_roads::Roads {
        version: 1,
        initial: 0,
        lanes_q8: None,
        ground: BTreeMap::new(),
        forks: BTreeMap::new(),
        profiles: vec![crate::source_roads::Profile {
            finish_progress: None,
            choices: vec![],
            selected: BTreeMap::from([(0, 0)]),
            identities: BTreeMap::from([(0, (0..512).map(|i| [i, i, 0, 3]).collect())]),
        }],
    });
    world.setup.rules.building_boundaries = (0..512)
        .map(|sample| crate::building_wall::Boundary {
            path: 0,
            sample,
            building: false,
            road: [-500, 500],
            outer: [-550, 550],
            selectors: [0, 0],
            flags: 0,
            surface: Some(crate::building_wall::Surface {
                kind: 255,
                center: 0,
                base: [0; 2],
                top: [0; 2],
                height: [0; 2],
                side_height: [0; 2],
                points: [[[0; 2]; 3]; 2],
                event_type: 5,
                rail_offset: 0,
                rail_flags: 0,
                drawn_rail_height: Some(128),
            }),
        })
        .collect();
    simulation
}

fn query(world: &mut World, row: &[i64], complete: bool) -> u8 {
    let sample = row[2] as u32;
    let surface = world.setup.rules.building_boundaries[sample as usize]
        .surface
        .as_mut()
        .unwrap();
    surface.rail_flags = row[3] as u8;
    surface.drawn_rail_height = Some(row[4] as i32);
    let follow = Location {
        path: 0,
        progress: row[0],
        ..Default::default()
    };
    world.scenery = Some(crate::scenery::State {
        source_route: Some(0),
        camera_back: row[1] as i32,
        pending_camera: Some(Location {
            progress: (row[0] - row[1]).max(0),
            ..follow
        }),
        frame_follow: (!complete).then_some(follow),
        ..Default::default()
    });
    world
        .source_boundary_at(
            ContactObject {
                owner: ActorId(1),
                detached: false,
            },
            Location {
                path: 0,
                progress: i64::from(sample) * 256,
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap()
        .surface
        .unwrap()
        .rail_flags
}

#[test]
fn completed_recovery_camera_uses_the_same_guardrail_window_as_frame_follow() {
    let mut simulation = fixture();
    let world = simulation.encounter.as_mut().unwrap();
    let inside = [12800, 255, 79, 0, 128, 12289, 128];
    let outside = [12800, 255, 80, 0, 128, 12289, 0];
    for complete in [false, true] {
        assert_eq!(query(world, &inside, complete), 128);
        assert_eq!(query(world, &outside, complete), 0);
    }
    world.setup.rules.contact_walk_version = 1;
    assert_eq!(query(world, &inside, false), 128);
    assert_eq!(
        query(world, &inside, true),
        0,
        "historical rule1 remains frozen"
    );
}
