//! Authored classic-style rendering, consuming a read-only snapshot.
use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use rr_simulation::Snapshot;

#[derive(Resource, Default)]
pub struct PresentedSnapshot(pub Snapshot);

pub struct ClassicPresentationPlugin;
impl Plugin for ClassicPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PresentedSnapshot>()
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Startup, setup)
            .add_systems(Update, (fit_viewport, present));
    }
}

#[derive(Component)]
struct RoadRow(u32);
#[derive(Component)]
struct Player;

fn setup(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.35, 0.65, 0.85), Vec2::new(640.0, 480.0)),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::Fixed {
                width: 640.0,
                height: 480.0,
            },
            ..OrthographicProjection::default_2d()
        }),
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
    commands.spawn((
        Sprite::from_color(Color::srgb(0.8, 0.1, 0.15), Vec2::new(20.0, 42.0)),
        Transform::from_xyz(0.0, -170.0, 3.0),
        Player,
    ));
}

fn present(
    snapshot: Res<PresentedSnapshot>,
    mut rows: Query<(&RoadRow, &mut Sprite)>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    let phase = snapshot.0.distance_mm / 250;
    for (row, mut sprite) in &mut rows {
        let shade = if (u64::from(row.0) + phase) % 16 < 8 {
            0.20
        } else {
            0.24
        };
        sprite.color = Color::srgb(shade, shade, shade);
    }
    for mut transform in &mut player {
        transform.translation.x = snapshot.0.lateral_mm as f32 / 20.0;
    }
}

fn fit_viewport(
    window: Single<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera, With<Camera2d>>,
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
    for mut camera in &mut cameras {
        camera.viewport = Some(Viewport {
            physical_position: (size - extent) / 2,
            physical_size: extent,
            ..default()
        });
    }
}
