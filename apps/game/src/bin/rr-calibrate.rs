#![forbid(unsafe_code)]
#[path = "../calibration_view.rs"]
mod calibration_view;
#[path = "../gamepad_calibration.rs"]
mod gamepad_calibration;
use bevy::prelude::*;
use rr_input::calibration::CalibrationFile;
use std::path::PathBuf;
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut path = PathBuf::from("local/controller-calibration.json");
    let mut selected = None;
    let mut frames = None;
    let mut capture = None;
    let mut report = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or("Every option requires a value")?;
        match arg.as_str() {
            "--profile" => path = PathBuf::from(value),
            "--device-key" => selected = Some(value),
            "--smoke-frames" => {
                let count: u32 = value.parse()?;
                if count < 120 {
                    return Err("smoke capture needs at least 120 frames".into());
                }
                frames = Some(count);
            }
            "--capture" => capture = Some(PathBuf::from(value)),
            "--report" => report = Some(PathBuf::from(value)),
            _ => return Err(format!("Unknown option {arg}").into()),
        }
    }
    let profiles = if path.exists() {
        CalibrationFile::read(&path)?
    } else {
        CalibrationFile::default()
    };
    let mut editor = calibration_view::Editor::new(profiles, path, selected);
    editor.smoke_frames = frames;
    editor.capture = capture;
    editor.report = report;
    let mut app = App::new();
    gamepad_calibration::install_calibration(&mut app);
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "RoadRash — Controller calibration".into(),
            resolution: (1120, 760).into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(editor);
    calibration_view::install(&mut app);
    app.run();
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2)
    }
}
