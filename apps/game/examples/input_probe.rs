//! Bounded Linux backend diagnostic; records only gamepad/action state.
#![forbid(unsafe_code)]
#[path = "../src/gamepad_calibration.rs"]
mod gamepad_calibration;
#[path = "../src/input_adapter.rs"]
mod input_adapter;
use bevy::{
    app::ScheduleRunnerPlugin,
    gilrs::GilrsPlugin,
    input::{InputPlugin, InputSystems},
    prelude::*,
};
use input_adapter::{
    Bindings, CalibrationProfiles, LiveInput, Router, pad_key, pad_state, poll_input, tick_command,
};
use rr_input::calibration::CalibrationFile;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
#[derive(Resource)]
struct Probe {
    path: PathBuf,
    start: Instant,
    last: Option<serde_json::Value>,
    history: Vec<serde_json::Value>,
}
fn record(
    input: Res<LiveInput>,
    profiles: Res<CalibrationProfiles>,
    pads: Query<(Entity, &Gamepad, Option<&Name>)>,
    mut probe: ResMut<Probe>,
    mut exit: MessageWriter<AppExit>,
) {
    let mut samples=pads.iter().map(|(entity,pad,name)| {
        let key=pad_key(pad,name);let raw=pad_state(pad);let profile=profiles.0.profile(&key);
        serde_json::json!({"entity":entity.to_bits(),"key":key,"raw_stick":if raw.stick_x.is_finite(){Some(raw.stick_x)}else{None},"steer_q15":profile.steering_q15(raw.stick_x)})
    }).collect::<Vec<_>>();
    samples.sort_by_key(|row| row["entity"].as_u64());
    let state =
        serde_json::json!({"input":input.0,"command":tick_command(&input.0),"pads":samples});
    if probe.last.as_ref() != Some(&state) {
        probe.last = Some(state.clone());
        let mut row = state;
        row["elapsed_ms"] = serde_json::json!(probe.start.elapsed().as_millis());
        probe.history.push(row);
        if probe.history.len() > 1024 {
            exit.write(AppExit::error());
            return;
        }
        let report = serde_json::json!({"backend":"Bevy Gilrs / Linux evdev","simulation_schema":rr_simulation::SIMULATION_SCHEMA,"observations":probe.history});
        let temp = probe.path.with_extension("tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        std::fs::rename(temp, &probe.path).unwrap();
    }
    if probe.path.with_extension("stop").exists() {
        exit.write(AppExit::Success);
    } else if probe.start.elapsed() > Duration::from_secs(45) {
        exit.write(AppExit::error());
    }
}
fn main() {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .expect("input_probe OUTPUT.json");
    let profiles = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .map(|path| CalibrationFile::read(&path))
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2)
        })
        .unwrap_or_default();
    let mut app = App::new();
    app.insert_resource(CalibrationProfiles(profiles));
    input_adapter::install_calibration(&mut app);
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(4))),
        InputPlugin,
        GilrsPlugin,
    ))
    .init_resource::<Bindings>()
    .init_resource::<LiveInput>()
    .init_resource::<Router>()
    .insert_resource(Probe {
        path,
        start: Instant::now(),
        last: None,
        history: vec![],
    })
    .add_systems(PreUpdate, poll_input.after(InputSystems))
    .add_systems(Update, record)
    .run();
}
