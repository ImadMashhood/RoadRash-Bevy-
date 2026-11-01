//! Composition root: input adapter -> fixed simulation -> presentation snapshot.
use bevy::prelude::*;
use rr_presentation::{ClassicPresentationPlugin, PresentedSnapshot};
use rr_simulation::{Command, Simulation, TICKS_PER_SECOND};

#[derive(Resource, Default)]
struct Race(Simulation);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "RoadRash Rust — classic foundation".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Time::<Fixed>::from_hz(f64::from(TICKS_PER_SECOND)))
        .init_resource::<Race>()
        .add_plugins(ClassicPresentationPlugin)
        .add_systems(FixedUpdate, tick)
        .run();
}

fn tick(
    keys: Res<ButtonInput<KeyCode>>,
    mut race: ResMut<Race>,
    mut presented: ResMut<PresentedSnapshot>,
) {
    let left = keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA);
    let right = keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD);
    race.0.step(Command {
        throttle: keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW),
        brake: keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS),
        steer: i8::from(right) - i8::from(left),
    });
    presented.0 = race.0.snapshot();
}
