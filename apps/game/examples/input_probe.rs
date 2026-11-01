//! Bounded Linux backend diagnostic; records only gamepad/action state.
#![forbid(unsafe_code)]
#[path = "../src/input_adapter.rs"]
mod input_adapter;
use bevy::{
    app::ScheduleRunnerPlugin,
    gilrs::GilrsPlugin,
    input::{InputPlugin, InputSystems},
    prelude::*,
};
use input_adapter::{Bindings, LiveInput, Router, poll_input, tick_command};
use rr_input::devices::RoutedInput;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
#[derive(Resource)]
struct Probe {
    path: PathBuf,
    start: Instant,
    last: Option<RoutedInput>,
    history: Vec<serde_json::Value>,
}
fn record(input: Res<LiveInput>, mut probe: ResMut<Probe>, mut exit: MessageWriter<AppExit>) {
    if probe.last != Some(input.0) {
        probe.last = Some(input.0);
        let elapsed = probe.start.elapsed().as_millis();
        probe.history.push(serde_json::json!({"elapsed_ms":elapsed,"input":input.0,"command":tick_command(&input.0)}));
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
    App::new()
        .add_plugins((
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
