//! Classic presentation from immutable snapshots; authored fallback without local data.
mod course_view;
use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
pub use course_view::CoursePresentation;
use rr_simulation::Snapshot;

#[derive(Resource, Default)]
pub struct PresentedSnapshot(pub Snapshot, pub Snapshot, pub f32);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PresentationStage {
    Prepare,
    Draw,
}
#[derive(Resource, Default)]
pub struct VisualAssets {
    pub native: Option<String>,
    pub enhanced: Option<String>,
    pub size: [u32; 2],
    pub initially_enhanced: bool,
    pub switch_tick: Option<u64>,
}
#[derive(Component)]
struct PlayerTextures {
    native: Handle<Image>,
    enhanced: Handle<Image>,
}
pub struct ClassicPresentationPlugin;
impl Plugin for ClassicPresentationPlugin {
    fn build(&self, app: &mut App) {
        course_view::install(app);
        app.init_resource::<PresentedSnapshot>()
            .init_resource::<VisualAssets>()
            .configure_sets(
                Update,
                (PresentationStage::Prepare, PresentationStage::Draw).chain(),
            )
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Startup, setup.run_if(course_view::no_course))
            .add_systems(
                Update,
                fit_viewport
                    .in_set(PresentationStage::Prepare)
                    .run_if(course_view::no_course),
            )
            .add_systems(
                Update,
                present
                    .in_set(PresentationStage::Draw)
                    .run_if(course_view::no_course),
            );
    }
}

#[derive(Component)]
struct RoadRow(u32);
#[derive(Component)]
struct Player;

fn setup(mut commands: Commands, assets: Res<AssetServer>, visual: Res<VisualAssets>) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.35, 0.65, 0.85), Vec2::new(640.0, 480.0)),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.18, 0.42, 0.15), Vec2::new(640.0, 340.0)),
        Transform::from_xyz(0.0, -70.0, 0.0),
    ));
    for row in 0..160 {
        let depth = row as f32 / 159.0;
        commands.spawn((
            Sprite::from_color(
                Color::srgb(0.18, 0.18, 0.2),
                Vec2::new(20.0 + depth * 560.0, 2.1),
            ),
            Transform::from_xyz(0.0, 80.0 - row as f32 * 2.0, 1.0),
            RoadRow(row),
        ));
    }
    if let Some(native) = &visual.native {
        let enhanced_name = visual.enhanced.as_ref().unwrap_or(native).clone();
        let native = assets.load(native.clone());
        let enhanced = assets.load(enhanced_name);
        let mut sprite = Sprite::from_image(native.clone());
        sprite.custom_size = Some(Vec2::new(visual.size[0] as f32, visual.size[1] as f32));
        commands.spawn((
            sprite,
            Transform::from_xyz(0.0, -190.0 + visual.size[1] as f32 / 2.0, 3.0),
            Player,
            PlayerTextures { native, enhanced },
        ));
    } else {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.8, 0.1, 0.15), Vec2::new(20.0, 42.0)),
            Transform::from_xyz(0.0, -170.0, 3.0),
            Player,
        ));
    }
}

fn present(
    snapshot: Res<PresentedSnapshot>,
    mut rows: Query<(&RoadRow, &mut Sprite), Without<Player>>,
    visual: Res<VisualAssets>,
    mut player: Query<(&mut Transform, &mut Sprite, Option<&PlayerTextures>), With<Player>>,
    mut last_mode: Local<Option<bool>>,
) {
    let distance = snapshot.1.distance_mm as f64
        + (snapshot.0.distance_mm as f64 - snapshot.1.distance_mm as f64) * f64::from(snapshot.2);
    let phase = distance.max(0.0) as u64 / 250;
    for (row, mut sprite) in &mut rows {
        let shade = if (u64::from(row.0) + phase) % 16 < 8 {
            0.20
        } else {
            0.24
        };
        sprite.color = Color::srgb(shade, shade, shade);
    }
    let enhanced = visual.initially_enhanced
        ^ visual
            .switch_tick
            .is_some_and(|tick| snapshot.0.tick >= tick);
    if last_mode.is_some_and(|mode| mode != enhanced) {
        println!(
            "Texture selection changed at tick {} (simulation unchanged)",
            snapshot.0.tick
        );
    }
    *last_mode = Some(enhanced);
    for (mut transform, mut sprite, textures) in &mut player {
        if let Some(textures) = textures {
            sprite.image = if enhanced {
                textures.enhanced.clone()
            } else {
                textures.native.clone()
            };
        }
        transform.translation.x = (snapshot.1.lateral_mm as f32
            + (snapshot.0.lateral_mm - snapshot.1.lateral_mm) as f32 * snapshot.2)
            / 20.0;
    }
}

fn fit_viewport(
    window: Single<&Window, With<PrimaryWindow>>,
    mut cameras: Query<(&mut Camera, &mut Projection), With<Camera2d>>,
) {
    let size = window.physical_size();
    if size.x == 0 || size.y == 0 {
        return;
    }
    let scale = (size.x as f32 / 640.0).min(size.y as f32 / 480.0);
    let extent = UVec2::new(
        (640.0 * scale).floor().max(1.0) as u32,
        (480.0 * scale).floor().max(1.0) as u32,
    );
    for (mut camera, mut projection) in &mut cameras {
        if let Projection::Orthographic(ref mut p) = *projection {
            p.scaling_mode = bevy::camera::ScalingMode::Fixed {
                width: 640.,
                height: 480.,
            };
        }
        camera.viewport = Some(Viewport {
            physical_position: (size - extent) / 2,
            physical_size: extent,
            ..default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn road_and_player_sprite_queries_are_disjoint_without_a_gpu() {
        let mut app = App::new();
        app.init_resource::<PresentedSnapshot>()
            .init_resource::<VisualAssets>()
            .add_systems(Update, present);
        app.world_mut().spawn((RoadRow(0), Sprite::default()));
        let player = app
            .world_mut()
            .spawn((Player, Sprite::default(), Transform::default()))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(player).unwrap().translation.x,
            0.0
        );
    }
}

#[cfg(test)]
mod interpolation_tests {
    use super::*;
    use rr_simulation::replay::{Playback, Replay, frame_ns};
    #[test]
    fn actual_presentation_system_has_smaller_steps_at_high_refresh() {
        fn positions(rate: &str) -> (f32, f32, String) {
            let replay =
                Replay::read(include_bytes!("../../../docs/fixtures/replay.json")).unwrap();
            let mut playback = Playback::new(replay).unwrap();
            let mut app = App::new();
            app.init_resource::<PresentedSnapshot>()
                .init_resource::<VisualAssets>()
                .add_systems(Update, present);
            let player = app
                .world_mut()
                .spawn((Player, Sprite::default(), Transform::default()))
                .id();
            let mut last = 0.0f32;
            let mut largest = 0.0f32;
            while !playback.done() {
                playback
                    .frame(frame_ns(rate, playback.frames).unwrap())
                    .unwrap();
                {
                    let mut view = app.world_mut().resource_mut::<PresentedSnapshot>();
                    view.0 = playback.current.clone();
                    view.1 = playback.previous.clone();
                    view.2 = playback.alpha();
                }
                app.update();
                let x = app.world().get::<Transform>(player).unwrap().translation.x;
                assert!(x.abs() <= 200.0);
                largest = largest.max((x - last).abs());
                last = x;
            }
            (largest, last, playback.simulation.state_hash())
        }
        let low = positions("30");
        let high = positions("144");
        assert!(high.0 < low.0);
        assert_eq!(low.1, high.1);
        assert_eq!(low.2, high.2);
    }
}
