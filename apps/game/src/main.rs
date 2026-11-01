//! Composition root: input adapter -> fixed simulation -> presentation snapshot.
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use rr_presentation::{ClassicPresentationPlugin, PresentedSnapshot};
use rr_simulation::{Command, Simulation, TICKS_PER_SECOND};
use std::path::PathBuf;

#[derive(Resource, Default)]
struct Race(Simulation);

#[derive(Resource, Default)]
struct LaunchOptions {
    smoke_frames: Option<u32>,
    capture: Option<PathBuf>,
}

fn options() -> Result<LaunchOptions, String> {
    let mut options = LaunchOptions::default();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {}", arg.to_string_lossy()))?;
        match arg.to_str() {
            Some("--smoke-frames") => {
                let count: u32 = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid frame count")?;
                if count < 120 {
                    return Err("smoke test needs at least 120 frames".into());
                }
                options.smoke_frames = Some(count);
            }
            Some("--capture") => options.capture = Some(value.into()),
            Some("--install-root") => {
                let install =
                    rr_import::active_install(&PathBuf::from(value)).map_err(|e| e.to_string())?;
                println!(
                    "Validated install: {} files, generation {}",
                    install.files, install.generation
                );
            }
            _ => return Err(format!("unknown argument: {}", arg.to_string_lossy())),
        }
    }
    Ok(options)
}

fn main() {
    let options = options().unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2);
    });
    App::new()
        .insert_resource(options)
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
        .add_systems(Update, smoke_test)
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

fn smoke_test(
    options: Res<LaunchOptions>,
    mut frame: Local<u32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    *frame += 1;
    if *frame == 30
        && let Some(path) = &options.capture
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    }
    if options.smoke_frames.is_some_and(|limit| *frame >= limit) {
        info!("foundation smoke completed after {} frames", *frame);
        exit.write(AppExit::Success);
    }
}
