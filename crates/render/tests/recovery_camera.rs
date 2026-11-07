use rr_render::{
    CameraSpec,
    camera::{route_camera_spec, route_encounter_snapshot, route_snapshot_view},
    fixture,
};
use rr_simulation::{
    CameraFollow, Command, Simulation, encounter, fixtures, recovery::Impact, topology::Location,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    id: usize,
    base: i32,
    left: i32,
    rows: Vec<[i32; 12]>,
}

fn template() -> rr_simulation::Snapshot {
    let config = fixtures::riding();
    let setup = encounter::Setup::authored(&config).unwrap();
    let mut sim = Simulation::with_encounters(1, config, setup).unwrap();
    sim.step(Command {
        impact: Some(Impact::default()),
        ..Default::default()
    })
    .unwrap();
    let mut snapshot = sim.snapshot();
    let unit = snapshot.actors[0].encounter.as_mut().unwrap();
    unit.source_route = Some(0);
    unit.recovery.bike_location.path = 20;
    snapshot
}

fn observe(case: &Case, row: &[i32; 12], applied: bool) -> [f64; 3] {
    let mut scene = fixture::scene("straight").unwrap();
    for (i, point) in scene.route.points.iter_mut().enumerate() {
        point.offset = case.base + (i as i32 % 5 - 2) * 30;
        let left = case.left + (i as i32 % 3) * 17;
        point.path.width = [left, 1000 - left];
    }
    observe_route(&scene.route, template(), row, applied)
}

fn observe_route(
    route: &rr_course::route::Route,
    mut snapshot: rr_simulation::Snapshot,
    row: &[i32; 12],
    applied: bool,
) -> [f64; 3] {
    let quiet = Location {
        path: snapshot.actors[0]
            .encounter
            .as_ref()
            .unwrap()
            .recovery
            .bike_location
            .path,
        progress: i64::from(row[1]),
        lateral: row[2],
        ..Default::default()
    };
    let actor = &mut snapshot.actors[0];
    let rider = actor.rider.as_mut().unwrap();
    rider.bike.previous_ground = row[3];
    rider.bike.lateral = row[2];
    let unit = actor.encounter.as_mut().unwrap();
    unit.recovery.bike_location = quiet;
    let body = unit.recovery.body.as_mut().unwrap();
    body.location = Location {
        path: 20,
        progress: i64::from(row[4]),
        lateral: row[5],
        ..Default::default()
    };
    body.motion.lateral = row[5];
    body.motion.previous_ground = row[6];
    let mut follow = body.location;
    if applied {
        follow.progress = (follow.progress - i64::from(row[7])).max(0);
    }
    snapshot.camera_follow = Some(CameraFollow {
        location: follow,
        back: row[7],
        back_applied: applied,
        bike_location: None,
        ground_height: None,
        road_center: None,
    });
    let before = serde_json::to_value(&snapshot).unwrap();
    let view = route_encounter_snapshot(route, &snapshot);
    let camera = route_snapshot_view(route, &view, &view, 1., 0.);
    let spec = route_camera_spec(route, &CameraSpec::default(), &view, &view, 1.);
    assert_eq!(
        serde_json::to_value(&snapshot).unwrap(),
        before,
        "presentation must not mutate the authority snapshot"
    );
    [camera.position_q8, camera.lateral, spec.camera_height]
}

#[test]
fn quiet_bike_uses_its_own_dual_road_edges_while_the_body_follows_the_other_half() {
    let mut scene = fixture::scene("straight").unwrap();
    for point in &mut scene.route.points {
        point.offset = 500;
        point.path.width = [500, 500];
        let mut path = point.path.clone();
        path.width = [100, 900];
        point.dual = Some(rr_course::route::Dual {
            half: 1,
            node: 92,
            section: 1,
            local: point.local,
            path,
            offset: -700,
            separation: -1200,
            separation_delta: 0,
        });
    }
    let mut snapshot = template();
    snapshot.actors[0]
        .encounter
        .as_mut()
        .unwrap()
        .recovery
        .bike_location
        .path = 92;
    // Quiet road centre is -700+(900-100)/2=-300; the selected scene centre is500.
    let row = [
        0,
        41 * 256,
        -300 * 256,
        500,
        52 * 256,
        2000 * 256,
        9000,
        0,
        0,
        51 * 256,
        -800,
        500,
    ];
    assert_eq!(
        observe_route(&scene.route, snapshot, &row, false),
        [51. * 256., -800., 500.]
    );
}

#[test]
fn quiet_bike_outside_the_visible_route_keeps_its_captured_road_centre() {
    let mut scene = fixture::scene("straight").unwrap();
    for point in &mut scene.route.points {
        point.offset = 500;
    }
    let mut source = template();
    let actor = &mut source.actors[0];
    actor.rider.as_mut().unwrap().bike.lateral = -300 * 256;
    actor.rider.as_mut().unwrap().bike.previous_ground = 500;
    let unit = actor.encounter.as_mut().unwrap();
    unit.recovery.bike_location.path = 999;
    let body = unit.recovery.body.as_mut().unwrap();
    body.location.path = 20;
    body.location.progress = 52 * 256;
    source.camera_follow = Some(CameraFollow {
        location: body.location,
        back: 0,
        back_applied: false,
        bike_location: None,
        ground_height: None,
        road_center: Some(-300),
    });
    let snapshot = route_encounter_snapshot(&scene.route, &source);
    let camera = route_snapshot_view(&scene.route, &snapshot, &snapshot, 1., 0.);
    assert_eq!(camera.position_q8, 51. * 256.);
    assert_eq!(camera.lateral, -800.);
}

#[test]
fn source_snapshot_exports_the_absolute_quiet_road_centre_with_native_odd_rounding() {
    let scene = fixture::scene("straight").unwrap();
    let mut roads =
        rr_simulation::source_roads::Roads::from_course(&scene.course, &scene.route).unwrap();
    for samples in roads.ground.values_mut() {
        for ground in samples {
            ground.left = -201;
            ground.right = 200;
            ground.left_shoulder = -300;
            ground.right_shoulder = 300;
        }
    }
    for profile in &mut roads.profiles {
        for samples in profile.identities.values_mut() {
            for sample in samples {
                sample[2] = 500;
            }
        }
    }
    let mut config = fixtures::riding();
    config.motion_version = 3;
    config.ground = roads.ground(0).unwrap();
    let mut setup = encounter::Setup::authored(&config).unwrap();
    setup.topology =
        rr_simulation::topology::Topology::from_course(&scene.course, &scene.route, 0).unwrap();
    setup.rules.source_roads = Some(roads);
    setup.rules.contact_walk_version = 1;
    setup.rules.collision_version = 2;
    setup.rules.combat_version = 3;
    setup.rules.collision = Some(rr_simulation::collision::Profile {
        mounted: [10, 4, 10],
        fallen: [10, 4, 4],
        mass: 256,
        elasticity: 192,
        rider: None,
    });
    setup.rules.scenery_version = 1;
    setup.rules.scenery_start_ready = true;
    setup.rules.obstacles.clear();
    setup.seeds[0].distance = 50 * 256;
    setup.seeds[0].lateral = 499 * 256;
    let simulation = Simulation::with_encounters(1, config, setup).unwrap();
    let before = simulation.state_hash();
    assert_eq!(
        simulation.snapshot().camera_follow.unwrap().road_center,
        Some(499)
    );
    assert_eq!(simulation.state_hash(), before);
}

#[test]
fn detached_camera_keeps_quiet_bike_lateral_and_ground_but_follows_the_body() {
    let case = Case {
        id: 0,
        base: 500,
        left: 200,
        rows: vec![],
    };
    let row = [
        0,
        41 * 256,
        900 * 256,
        5000,
        52 * 256,
        -1000 * 256,
        700,
        300,
        100,
        52 * 256 - 300 - 256,
        0,
        5100,
    ];
    // Compare with a mounted-body-equivalent observation whose half still differs longitudinally.
    let expected = observe(&case, &row, false);
    assert_eq!(expected[0], f64::from(row[9]));
    assert_eq!(expected[2], 5100.);
    assert_eq!(observe(&case, &row, true), expected);
    let mut other = row;
    other[5] = 1700 * 256;
    other[6] = 9000;
    assert_eq!(
        observe(&case, &other, false),
        expected,
        "body lateral/ground cannot steer or lift the native camera"
    );
}

#[test]
#[ignore = "requires private pinned recovery camera observations"]
fn native_recovery_camera() {
    let path = std::env::var_os("RR_NATIVE_RECOVERY_CAMERA").unwrap();
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let template = template();
    let mut failures = vec![];
    let mut observations = 0;
    for case in cases {
        let mut scene = fixture::scene("straight").unwrap();
        for (i, point) in scene.route.points.iter_mut().enumerate() {
            point.offset = case.base + (i as i32 % 5 - 2) * 30;
            let left = case.left + (i as i32 % 3) * 17;
            point.path.width = [left, 1000 - left];
        }
        for row in case.rows {
            observations += 1;
            let expected = [f64::from(row[9]), f64::from(row[10]), f64::from(row[11])];
            let actual = observe_route(&scene.route, template.clone(), &row, false);
            let applied = observe_route(&scene.route, template.clone(), &row, true);
            if actual != expected || applied != expected {
                failures.push(serde_json::json!({"case":case.id,"tick":row[0],"native":expected,"rust":actual,"already_shifted":applied}));
            }
        }
    }
    let out =
        std::path::PathBuf::from(std::env::var_os("RR_NATIVE_RECOVERY_CAMERA_OUTPUT").unwrap());
    std::fs::write(out.join("camera-comparison.json"),serde_json::to_vec_pretty(&serde_json::json!({"outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":observations,"mismatches":failures.len(),"first_mismatches":failures.iter().take(16).collect::<Vec<_>>(),"scope":"Controlled actual native camera callback sequences with independent quiet-bike/body poses, terminal/arrest ramp and decay; both shifted and unshifted exports; no whole native scheduler/TrackRing parity"})).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "{} native recovery camera mismatches",
        failures.len()
    );
}
