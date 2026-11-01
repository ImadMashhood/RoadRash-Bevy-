//! Bevy adapter: upload the deterministic drawing surface; no domain writes.
use super::{PresentationStage, PresentedSnapshot, VisualAssets};
use bevy::{
    asset::RenderAssetUsages,
    camera::Viewport,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::PrimaryWindow,
};
use rr_render::{CameraSpec, Scene, camera::snapshot_view};
use std::sync::Arc;
#[derive(Resource)]
pub struct CoursePresentation {
    pub scene: Arc<Scene>,
    pub camera: CameraSpec,
    pub start_q8: f64,
}
#[derive(Resource)]
struct CourseSurface(Handle<Image>);
#[derive(Component)]
struct RecoveryHud;
#[derive(Component)]
struct CourseScreen;
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
    commands.spawn((
        RecoveryHud,
        CourseScreen,
        Text2d::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.),
            ..default()
        },
        TextColor(Color::BLACK),
        Transform::from_xyz(0., spec.height as f32 / 2. - 18., 10.),
    ));
    commands.insert_resource(CourseSurface(handle));
}
fn visibility(
    course: Option<Res<CoursePresentation>>,
    mut screens: Query<&mut Visibility, With<CourseScreen>>,
) {
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
    surface: Res<CourseSurface>,
    visual: Res<VisualAssets>,
    mut images: ResMut<Assets<Image>>,
    mut last_mode: Local<Option<bool>>,
    mut hud: Query<&mut Text2d, With<RecoveryHud>>,
) {
    if let Some(actor) = view.0.actors.iter().find(|a| a.id.0 == 1) {
        let status = actor.encounter.as_ref().map_or(String::new(), |u| {
            format!(
                "Health {} / {}   Bike {}   {:?}{}",
                u.fighter.stats.health,
                u.fighter.stats.maximum,
                actor.rider.as_ref().map_or(0, |r| r.bike.condition),
                u.recovery.phase,
                if u.recovery.phase == rr_simulation::recovery::Phase::Walking {
                    "\nArrows: walk / release: return to bike"
                } else {
                    ""
                }
            )
        });
        let status = if view.0.road_users.iter().any(|a| a.siren) {
            format!("{status}\nPOLICE PURSUIT")
        } else {
            status
        };
        for mut text in &mut hud {
            text.0.clone_from(&status);
        }
    }
    let previous = rr_render::camera::encounter_snapshot(&view.1);
    let current = rr_render::camera::encounter_snapshot(&view.0);
    let mut camera = snapshot_view(&previous, &current, view.2, course.start_q8);
    camera.position_q8 = camera
        .position_q8
        .clamp(0., course.scene.route.len_q8() as f64 - 1.);
    let enhanced = visual.initially_enhanced ^ visual.switch_tick.is_some_and(|t| view.0.tick >= t);
    if last_mode.is_some_and(|mode| mode != enhanced) {
        println!(
            "Texture selection changed at tick {} (simulation unchanged)",
            view.0.tick
        )
    }
    *last_mode = Some(enhanced);
    let snapshot = rr_render::camera::interpolated_snapshot(&previous, &current, view.2);
    match course
        .scene
        .draw(&course.camera, &camera, &snapshot, enhanced)
    {
        Ok((frame, _)) => {
            if let Some(mut image) = images.get_mut(&surface.0) {
                if image.texture_descriptor.size.width != frame.image.width
                    || image.texture_descriptor.size.height != frame.image.height
                {
                    image.resize(Extent3d {
                        width: frame.image.width,
                        height: frame.image.height,
                        depth_or_array_layers: 1,
                    });
                }
                image.data = Some(frame.image.pixels)
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
        let handle = app.world().resource::<CourseSurface>().0.clone();
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
        app.world_mut().remove_resource::<CoursePresentation>();
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<&Visibility, With<CourseScreen>>();
        assert!(query.iter(app.world()).all(|v| *v == Visibility::Hidden));
        let mut cameras = app.world_mut().query::<&Camera2d>();
        assert_eq!(cameras.iter(app.world()).count(), 1);
    }
}
