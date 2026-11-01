//! Authored classic-style rendering, consuming a read-only snapshot.
use bevy::prelude::*;
use rr_simulation::Snapshot;

#[derive(Resource, Default)]
pub struct PresentedSnapshot(pub Snapshot);

pub struct ClassicPresentationPlugin;
impl Plugin for ClassicPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PresentedSnapshot>()
            .insert_resource(ClearColor(Color::srgb(0.35, 0.65, 0.85)))
            .add_systems(Startup, setup)
            .add_systems(Update, present);
    }
}

#[derive(Component)]
struct RoadRow(u32);
#[derive(Component)]
struct Player;

fn setup(mut commands: Commands) {
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
