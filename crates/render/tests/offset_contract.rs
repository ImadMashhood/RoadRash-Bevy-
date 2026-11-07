use rr_render::{
    CameraSpec,
    camera::{View, route_snapshot_view},
    fixture,
};
use rr_simulation::{ActorId, Simulation, encounter, fixtures};

fn fork() -> rr_render::Scene {
    use rr_course::{Node, NodeKind, Sample};
    use std::collections::BTreeMap;
    let base = fixture::scene("straight").unwrap();
    let mut course = base.course;
    let section = course.sections[0].clone();
    course.sections = [128, 200, 220, 128]
        .into_iter()
        .map(|length| {
            let mut section = section.clone();
            section.samples = vec![Sample::road(); length];
            section.terrain = None;
            section
        })
        .collect();
    let node = |offset, kind, next, previous, link, length| Node {
        offset,
        file_offset: offset,
        kind,
        next,
        previous,
        link,
        length,
        primary: 0,
        raw: vec![0; 8],
    };
    course.nodes = BTreeMap::from([
        (20, node(20, NodeKind::Road, 60, 52, 0, 128)),
        (52, node(52, NodeKind::Start, 0, 0, 0, 0)),
        (60, node(60, NodeKind::Split, 92, 124, 20, 15360)),
        (92, node(92, NodeKind::Road, 156, 60, 1, 200)),
        (124, node(124, NodeKind::Road, 156, 60, 2, 220)),
        (156, node(156, NodeKind::Merge, 92, 124, 188, 15360)),
        (188, node(188, NodeKind::Road, 220, 156, 3, 128)),
        (220, node(220, NodeKind::End, 0, 0, 0, 0)),
    ]);
    rr_render::Scene::new(course, base.assets, Default::default()).unwrap()
}

fn picture(offset: i32, traffic: bool) -> Vec<u8> {
    let mut scene = fixture::scene("straight").unwrap();
    for point in &mut scene.route.points {
        point.offset += offset;
    }
    let config = fixtures::riding();
    let mut setup = encounter::Setup::authored(&config).unwrap();
    setup.seeds[0].lateral = offset * 256;
    let mut other = encounter::Seed::player();
    other.id = ActorId(2);
    other.distance = 256;
    other.lateral = (offset + 100) * 256;
    setup.seeds.push(other);
    let topology = setup.topology.clone();
    let mut snapshot = Simulation::with_encounters(1, config, setup)
        .unwrap()
        .snapshot();
    if traffic {
        use rr_simulation::{
            population::{Kind, Setup, Spawn, World},
            topology::Location,
        };
        let mut population = World::new(Setup::default());
        population
            .spawn(
                &Spawn {
                    id: 1,
                    location: Location {
                        progress: 512,
                        lateral: (offset - 100) * 256,
                        ..Default::default()
                    },
                    kind: Kind::Vehicle,
                    family: None,
                    selector: 0,
                    subtype: 0,
                    speed: 0,
                    direction: 1,
                },
                0,
                &topology,
            )
            .unwrap()
            .unwrap();
        snapshot.road_users = population.agents.into_values().collect();
        let sprite = rr_render::source::SpriteAsset {
            texture: rr_render::raster::Texture::solid([0, 200, 255, 255]),
            logical: [30., 20.],
            pivot: [0.5, 1.],
        };
        scene.assets.vehicles = vec![vec![Some(sprite); 3]];
    }
    scene
        .draw(
            &CameraSpec::default(),
            &View {
                position_q8: 50. * 256.,
                lateral: 0.,
                heading: 0,
            },
            &snapshot,
            false,
        )
        .unwrap()
        .0
        .image
        .pixels
}

#[test]
fn source_riders_keep_their_screen_pose_when_road_and_world_translate_together() {
    let centered = picture(0, false);
    let translated = picture(500, false);
    assert_eq!(centered.len(), translated.len());
    let changed = centered
        .iter()
        .zip(&translated)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        changed, 0,
        "uniform world/road offset changed {changed} image bytes"
    );
}

#[test]
fn source_traffic_keeps_its_screen_pose_when_road_and_world_translate_together() {
    let centered = picture(0, true);
    assert_ne!(
        centered,
        picture(0, false),
        "traffic fixture must actually draw"
    );
    let translated = picture(500, true);
    assert_eq!(
        centered
            .iter()
            .zip(&translated)
            .filter(|(a, b)| a != b)
            .count(),
        0
    );
}

#[test]
fn source_camera_converts_absolute_lateral_to_the_sampled_road_center() {
    let mut scene = fixture::scene("straight").unwrap();
    for point in &mut scene.route.points {
        point.offset = 500;
    }
    let config = fixtures::riding();
    let mut setup = encounter::Setup::authored(&config).unwrap();
    setup.seeds[0].lateral = 520 * 256;
    let old = Simulation::with_encounters(1, config, setup)
        .unwrap()
        .snapshot();
    let mut new = old.clone();
    new.actors[0].rider.as_mut().unwrap().bike.lateral = 540 * 256;
    for (alpha, expected) in [(0., 20.), (0.5, 30.), (1., 40.)] {
        let view = route_snapshot_view(&scene.route, &old, &new, alpha, 50. * 256.);
        assert_eq!(view.lateral, expected);
    }
    let legacy = Simulation::default().snapshot();
    assert_eq!(
        route_snapshot_view(&scene.route, &legacy, &legacy, 1., 12800.).lateral,
        0.
    );
}

#[test]
fn steered_branch_rebuilds_route_and_keeps_physical_positions_after_unequal_merge() {
    use rr_render::camera::{route_encounter_snapshot, route_position};
    let scene = fork();
    assert!(scene.following_node(92).unwrap().is_none());
    let right = scene.following_node(124).unwrap().unwrap();
    assert_eq!(right.route.choices, [1]);
    assert_eq!(
        route_position(&right.route, 124, 100 * 256 + 17),
        Some(228 * 256 + 17)
    );
    assert_eq!(route_position(&scene.route, 188, 5 * 256), Some(333 * 256));
    assert_eq!(route_position(&right.route, 188, 5 * 256), Some(353 * 256));
    assert!(route_position(&right.route, 92, 100 * 256).is_none());
    assert!(right.following_node(188).unwrap().is_none());
    let restored = scene.following_mask(1).unwrap().unwrap();
    assert_eq!(restored.route.choices, [1]);
    assert!(restored.following_mask(1).unwrap().is_none());
    assert!(restored.following_mask(2).is_err());
    assert!(right.following_node(999).is_err());
    let config = fixtures::riding();
    let setup = encounter::Setup::authored(&config).unwrap();
    let mut snapshot = Simulation::with_encounters(1, config, setup)
        .unwrap()
        .snapshot();
    let location = &mut snapshot.actors[0]
        .encounter
        .as_mut()
        .unwrap()
        .recovery
        .bike_location;
    location.path = 188;
    location.progress = 5 * 256;
    let hash = snapshot.clone();
    let projected = route_encounter_snapshot(&right.route, &snapshot);
    assert_eq!(projected.actors[0].position.progress.0, 353 * 256 * 8);
    let camera = route_snapshot_view(&right.route, &projected, &projected, 1., 0.);
    assert_eq!(camera.position_q8, 352. * 256.);
    assert_eq!(
        snapshot, hash,
        "presentation must not mutate simulation state"
    );
}

#[test]
#[ignore = "requires pinned native camera producer and private physics assets"]
fn native_camera_pose() {
    use rr_simulation::topology::Location;
    let native = std::env::var("RR_NATIVE_CAMERA_POSE").unwrap();
    let rows: Vec<Vec<i64>> = serde_json::from_slice(&std::fs::read(native).unwrap()).unwrap();
    let mut failures = Vec::new();
    let legacy_height = std::env::var_os("RR_CAMERA_HEIGHT_LEGACY").is_some();
    for row in &rows {
        let mut scene = fixture::scene("straight").unwrap();
        for point in &mut scene.route.points {
            point.offset = row[1] as i32;
            point.path.width = [row[2] as i32, row[3] as i32];
        }
        let config = fixtures::riding();
        let mut setup = encounter::Setup::authored(&config).unwrap();
        let path = setup.topology.paths.remove(&0).unwrap();
        setup.topology.paths.insert(20, path);
        setup.seeds[0].location = Some(Location {
            path: 20,
            progress: 51 * 256,
            lateral: row[4] as i32,
            ..Default::default()
        });
        setup.seeds[0].distance = 51 * 256;
        setup.seeds[0].lateral = row[4] as i32;
        let mut snapshot = Simulation::with_encounters(1, config, setup)
            .unwrap()
            .snapshot();
        if row.len() > 7 {
            snapshot.actors[0]
                .rider
                .as_mut()
                .unwrap()
                .bike
                .previous_ground = row[7] as i32;
            snapshot.actors[0].encounter.as_mut().unwrap().source_route = Some(0);
        }
        let camera = route_snapshot_view(&scene.route, &snapshot, &snapshot, 1., 0.);
        let spec = if legacy_height {
            CameraSpec::default()
        } else {
            rr_render::camera::route_camera_spec(
                &scene.route,
                &CameraSpec::default(),
                &snapshot,
                &snapshot,
                1.,
            )
        };
        spec.validate().unwrap();
        if camera.position_q8 != row[5] as f64
            || camera.lateral != row[6] as f64
            || (row.len() > 8 && spec.camera_height != row[8] as f64)
        {
            failures.push(serde_json::json!({"id":row[0],"native":row,"rust":[camera.position_q8,camera.lateral,spec.camera_height]}));
        }
    }
    let output = std::path::PathBuf::from(std::env::var("RR_NATIVE_CAMERA_OUTPUT").unwrap());
    std::fs::write(output.join("camera-comparison.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "outcome":if failures.is_empty() {"passed"} else {"failed"},
        "observations":rows.len(), "mismatches":failures.len(),
        "legacy_height":legacy_height,
        "first_mismatches":failures.iter().take(12).collect::<Vec<_>>(),
        "scope":"mounted no-contact camera lateral, ground-height following and one-sample trailing position; no recovery, lift/pullback or contact lag",
    })).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "{} native camera mismatches",
        failures.len()
    );
}

#[test]
fn native_ground_height_drives_the_sprite_foot_and_camera_without_touching_legacy_views() {
    use rr_render::camera::{interpolated_snapshot, route_camera_spec};
    let scene = fixture::scene("straight").unwrap();
    let config = fixtures::riding();
    let setup = encounter::Setup::authored(&config).unwrap();
    let mut low = Simulation::with_encounters(1, config, setup)
        .unwrap()
        .snapshot();
    low.actors[0].encounter.as_mut().unwrap().source_route = Some(0);
    let mut high = low.clone();
    high.actors[0].rider.as_mut().unwrap().bike.previous_ground = 20;
    let spec = CameraSpec::default();
    let view = View {
        position_q8: 50. * 256.,
        lateral: 0.,
        heading: 0,
    };
    let (_, a) = scene.draw(&spec, &view, &low, false).unwrap();
    let (_, b) = scene.draw(&spec, &view, &high, false).unwrap();
    let shift = a.rider_bounds.unwrap()[3] - b.rider_bounds.unwrap()[3];
    assert!((shift - 20. * scene.assets.depths.scale(spec.player_depth_q8)).abs() < 1e-8);
    let middle = interpolated_snapshot(&low, &high, 0.5);
    assert_eq!(
        middle.actors[0]
            .rider
            .as_ref()
            .unwrap()
            .bike
            .previous_ground,
        10
    );
    high.actors[0].rider.as_mut().unwrap().bike.previous_ground = 5000;
    let camera = route_camera_spec(&scene.route, &spec, &high, &high, 1.);
    assert_eq!(camera.camera_height, 5000.);
    camera.validate().unwrap();
    high.actors[0].encounter.as_mut().unwrap().source_route = None;
    assert_eq!(
        route_camera_spec(&scene.route, &spec, &high, &high, 1.).camera_height,
        150.
    );
}

#[test]
fn a_contact_shove_moves_the_actor_without_moving_the_frozen_camera_follow_pose() {
    use rr_render::camera::route_camera_spec;
    use rr_simulation::{CameraFollow, topology::Location};
    let scene = fixture::scene("straight").unwrap();
    let config = fixtures::riding();
    let setup = encounter::Setup::authored(&config).unwrap();
    let mut snapshot = Simulation::with_encounters(1, config, setup)
        .unwrap()
        .snapshot();
    let unit = snapshot.actors[0].encounter.as_mut().unwrap();
    unit.source_route = Some(0);
    unit.recovery.bike_location = Location {
        path: 20,
        progress: 51 * 256 + 7,
        ..Default::default()
    };
    snapshot.camera_follow = Some(CameraFollow {
        location: Location {
            path: 20,
            progress: 51 * 256,
            ..Default::default()
        },
        back: 0,
        back_applied: false,
        bike_location: None,
        ground_height: None,
        road_center: None,
    });
    let camera = route_snapshot_view(&scene.route, &snapshot, &snapshot, 1., 0.);
    let spec = route_camera_spec(
        &scene.route,
        &CameraSpec::default(),
        &snapshot,
        &snapshot,
        1.,
    );
    assert_eq!(camera.position_q8, 50. * 256.);
    assert_eq!(spec.player_depth_q8, 519.);
    let before = snapshot.clone();
    scene.draw(&spec, &camera, &snapshot, false).unwrap();
    assert_eq!(snapshot, before);
    snapshot.camera_follow.as_mut().unwrap().back = 900;
    let spec = route_camera_spec(
        &scene.route,
        &CameraSpec::default(),
        &snapshot,
        &snapshot,
        1.,
    );
    assert_eq!(spec.player_depth_q8, 1419.);
    spec.validate().unwrap();
    let expected_view = route_snapshot_view(&scene.route, &snapshot, &snapshot, 1., 0.);
    let follow = snapshot.camera_follow.as_mut().unwrap();
    follow.location.progress -= i64::from(follow.back);
    follow.back_applied = true;
    let prepared_view = route_snapshot_view(&scene.route, &snapshot, &snapshot, 1., 0.);
    let prepared_spec = route_camera_spec(
        &scene.route,
        &CameraSpec::default(),
        &snapshot,
        &snapshot,
        1.,
    );
    assert_eq!(
        prepared_view.position_q8, expected_view.position_q8,
        "completed scenery pullback must not be subtracted twice"
    );
    assert_eq!(prepared_spec.player_depth_q8, 1419.);
}
