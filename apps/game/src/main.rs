//! Authored placeholder: no game data or original artwork required.
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "RoadRash Rust — foundation scene".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .run();
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 0.1, 40.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.15, 0.15, 0.18))),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 1.0, 1.8))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.15, 0.1))),
        Transform::from_xyz(0.0, 0.6, 0.0),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.4, 0.0)),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
