//! Bevy adapter: upload the deterministic drawing surface; no domain writes.
use super::{PresentationStage, PresentedSnapshot, VisualAssets};
use bevy::{
    asset::RenderAssetUsages,
    camera::Viewport,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::PrimaryWindow,
};
use rr_render::{
    CameraSpec, Scene,
    camera::{route_camera_spec, route_encounter_snapshot, route_snapshot_view},
};
use std::sync::Arc;
#[derive(Resource)]
pub struct CoursePresentation {
    pub scene: Arc<Scene>,
    pub camera: CameraSpec,
    pub start_q8: f64,
}
#[derive(Resource)]
struct CourseSurface(Handle<Image>, FollowingScene);
#[derive(Component)]
struct CourseScreen;
#[derive(Default)]
struct FollowingScene {
    source: Option<Arc<Scene>>,
    selected: Option<Arc<Scene>>,
}
pub fn has_course(course: Option<Res<CoursePresentation>>) -> bool {
    course.is_some()
}
pub fn no_course(course: Option<Res<CoursePresentation>>) -> bool {
    course.is_none()
}
pub fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, visibility.in_set(PresentationStage::Prepare))
        .add_systems(
            Update,
            (
                fit.in_set(PresentationStage::Prepare),
                draw.in_set(PresentationStage::Draw),
            )
                .run_if(has_course),
        );
}
fn setup(
    mut commands: Commands,
    course: Option<Res<CoursePresentation>>,
    mut images: ResMut<Assets<Image>>,
) {
    let fallback = rr_render::CameraSpec::new(rr_render::Framing::Classic, false);
    let spec = course.as_ref().map_or(&fallback, |c| &c.camera);
    let image = Image::new_fill(
        Extent3d {
            width: spec.width,
            height: spec.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);
    commands.spawn((
        CourseScreen,
        Sprite::from_image(handle.clone()),
        Transform::from_xyz(0., 0., 5.),
    ));
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::Fixed {
                width: spec.width as f32,
                height: spec.height as f32,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    commands.insert_resource(CourseSurface(handle, FollowingScene::default()));
}
fn visibility(
    course: Option<Res<CoursePresentation>>,
    mut surface: ResMut<CourseSurface>,
    mut screens: Query<&mut Visibility, With<CourseScreen>>,
) {
    if course.is_none() {
        surface.1 = FollowingScene::default();
    }
    for mut visible in &mut screens {
        *visible = if course.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
fn fit(
    window: Single<&Window, With<PrimaryWindow>>,
    course: Res<CoursePresentation>,
    mut cameras: Query<(&mut Camera, &mut Projection), With<Camera2d>>,
) {
    let physical = window.physical_size();
    if let Some((origin, size)) = course.camera.viewport([physical.x, physical.y]) {
        for (mut camera, mut projection) in &mut cameras {
            if let Projection::Orthographic(ref mut p) = *projection {
                p.scaling_mode = bevy::camera::ScalingMode::Fixed {
                    width: course.camera.width as f32,
                    height: course.camera.height as f32,
                };
            }
            camera.viewport = Some(Viewport {
                physical_position: UVec2::from(origin),
                physical_size: UVec2::from(size),
                ..default()
            })
        }
    }
}
fn draw(
    course: Res<CoursePresentation>,
    view: Res<PresentedSnapshot>,
    (mut surface, mut surfaces): (
        ResMut<CourseSurface>,
        Query<&mut Sprite, With<CourseScreen>>,
    ),
    (visual, accessibility, mut timing): (
        Res<VisualAssets>,
        Option<Res<super::Accessibility>>,
        Option<ResMut<super::RenderTiming>>,
    ),
    mut images: ResMut<Assets<Image>>,
    mut last_mode: Local<Option<bool>>,
) {
    let prepare_started = std::time::Instant::now();
    let selected = &mut surface.1;
    if selected
        .source
        .as_ref()
        .is_none_or(|source| !Arc::ptr_eq(source, &course.scene))
    {
        selected.source = Some(course.scene.clone());
        selected.selected = None;
    }
    let active = selected.selected.as_ref().unwrap_or(&course.scene);
    let player = view
        .0
        .actors
        .iter()
        .find(|actor| actor.id.0 == 1)
        .and_then(|actor| actor.encounter.as_ref());
    let player_node = player.map(|unit| {
        unit.recovery
            .body
            .as_ref()
            .map_or(unit.recovery.bike_location.path, |body| body.location.path)
    });
    let following = if let Some(mask) = player.and_then(|unit| {
        unit.recovery
            .body
            .as_ref()
            .and_then(|body| body.source_route)
            .or(unit.source_route)
    }) {
        Some(active.following_mask(u32::from(mask)))
    } else {
        player_node
            .filter(|node| active.course.nodes.contains_key(node))
            .map(|node| active.following_node(node))
    };
    if let Some(following) = following {
        match following {
            Ok(Some(scene)) => selected.selected = Some(Arc::new(scene)),
            Ok(None) => {}
            Err(error) => {
                error!("course route selection: {error}");
                return;
            }
        }
    }
    let scene = selected.selected.as_ref().unwrap_or(&course.scene);
    let previous = route_encounter_snapshot(&scene.route, &view.1);
    let current = route_encounter_snapshot(&scene.route, &view.0);
    let mut camera =
        route_snapshot_view(&scene.route, &previous, &current, view.2, course.start_q8);
    camera.position_q8 = camera
        .position_q8
        .clamp(0., scene.route.len_q8() as f64 - 1.);
    if accessibility.is_some_and(|a| a.reduced_motion) {
        camera.lateral = 0.;
    }
    let enhanced = visual.initially_enhanced ^ visual.switch_tick.is_some_and(|t| view.0.tick >= t);
    if last_mode.is_some_and(|mode| mode != enhanced) {
        println!(
            "Texture selection changed at tick {} (simulation unchanged)",
            view.0.tick
        )
    }
    *last_mode = Some(enhanced);
    let snapshot = rr_render::camera::interpolated_snapshot(&previous, &current, view.2);
    let camera_spec = route_camera_spec(&scene.route, &course.camera, &previous, &current, view.2);
    let prepare_ms = prepare_started.elapsed().as_secs_f64() * 1000.;
    let start = std::time::Instant::now();
    let workers = std::thread::available_parallelism().map_or(1, |n| n.get().min(6));
    let rendered = scene.draw_parallel(&camera_spec, &camera, &snapshot, enhanced, workers);
    if let Some(timing) = timing.as_deref_mut() {
        timing.cpu_prepare_ms = prepare_ms;
        timing.cpu_upload_ms = 0.;
        timing.cpu_draw_ms = start.elapsed().as_secs_f64() * 1000.;
        timing.draws += 1;
    }
    match rendered {
        Ok((frame, _)) => {
            let upload_started = std::time::Instant::now();
            let replaced = crate::upload_rgba(
                &mut images,
                &mut surface.0,
                frame.image.width,
                frame.image.height,
                frame.image.pixels,
            );
            if let Some(timing) = timing.as_deref_mut() {
                timing.cpu_upload_ms = upload_started.elapsed().as_secs_f64() * 1000.;
            }
            if replaced {
                for mut sprite in &mut surfaces {
                    sprite.image = surface.0.clone();
                }
            }
        }
        Err(e) => error!("classic rendering failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uploaded_course_frame_interpolates_without_mutating_authoritative_snapshots() {
        let scene = rr_render::fixture::scene("bend").unwrap();
        let mut simulation = rr_simulation::Simulation::default();
        let old = simulation.snapshot();
        for _ in 0..60 {
            simulation
                .step(rr_simulation::Command {
                    throttle: true,
                    steer: 1,
                    ..Default::default()
                })
                .unwrap();
        }
        let current = simulation.snapshot();
        let before = simulation.state_hash();
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<VisualAssets>()
            .insert_resource(CoursePresentation {
                scene: Arc::new(scene),
                camera: CameraSpec::default(),
                start_q8: 12800.,
            })
            .insert_resource(PresentedSnapshot(current.clone(), old, 0.))
            .add_systems(Startup, setup)
            .add_systems(Update, draw);
        app.update();
        let handle = app.world().resource::<CourseSurface>().0.clone();
        let first = app
            .world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap()
            .data
            .clone();
        app.world_mut().resource_mut::<PresentedSnapshot>().2 = 1.;
        app.update();
        let last = &app
            .world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap()
            .data;
        assert_ne!(&first, last);
        assert_eq!(app.world().resource::<PresentedSnapshot>().0, current);
        assert_eq!(simulation.state_hash(), before);
    }
}
#[cfg(test)]
mod session_surface_tests {
    use super::*;
    #[test]
    fn framing_changes_reuse_one_surface_and_scene_exit_hides_all_source_visuals() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<VisualAssets>()
            .init_resource::<PresentedSnapshot>()
            .insert_resource(CoursePresentation {
                scene: Arc::new(rr_render::fixture::scene("bend").unwrap()),
                camera: CameraSpec::default(),
                start_q8: 12800.,
            })
            .add_systems(Startup, setup)
            .add_systems(Update, (visibility, draw.run_if(has_course)).chain());
        app.update();
        let first = app.world().resource::<Assets<Image>>().len();
        for framing in [
            rr_render::Framing::Wide,
            rr_render::Framing::Classic,
            rr_render::Framing::Wide,
        ] {
            let camera = CameraSpec::new(framing, false);
            let expected = [camera.width, camera.height];
            app.world_mut().resource_mut::<CoursePresentation>().camera = camera;
            app.update();
            let handle = app.world().resource::<CourseSurface>().0.clone();
            let image = app
                .world()
                .resource::<Assets<Image>>()
                .get(&handle)
                .unwrap();
            assert_eq!(
                [
                    image.texture_descriptor.size.width,
                    image.texture_descriptor.size.height
                ],
                expected
            );
            assert_eq!(
                image.data.as_ref().unwrap().len(),
                expected[0] as usize * expected[1] as usize * 4
            );
            assert_eq!(app.world().resource::<Assets<Image>>().len(), first);
        }
        let scene = Arc::downgrade(&app.world().resource::<CoursePresentation>().scene);
        app.world_mut().remove_resource::<CoursePresentation>();
        app.update();
        assert!(
            scene.upgrade().is_none(),
            "leaving a race must release its cached source scene"
        );
        let mut query = app
            .world_mut()
            .query_filtered::<&Visibility, With<CourseScreen>>();
        assert!(query.iter(app.world()).all(|v| *v == Visibility::Hidden));
        let mut cameras = app.world_mut().query::<&Camera2d>();
        assert_eq!(cameras.iter(app.world()).count(), 1);
    }
}
