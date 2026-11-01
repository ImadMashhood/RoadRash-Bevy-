//! Bevy physical keys are translated only at the composition boundary.
use bevy::prelude::*;
use rr_input::{ActionMap, Key};
#[derive(Resource, Default)]
pub struct Bindings(pub ActionMap);
pub fn key_code(key: Key) -> KeyCode {
    match key {
        Key::ArrowUp => KeyCode::ArrowUp,
        Key::ArrowDown => KeyCode::ArrowDown,
        Key::ArrowLeft => KeyCode::ArrowLeft,
        Key::ArrowRight => KeyCode::ArrowRight,
        Key::KeyA => KeyCode::KeyA,
        Key::KeyB => KeyCode::KeyB,
        Key::KeyC => KeyCode::KeyC,
        Key::KeyD => KeyCode::KeyD,
        Key::KeyE => KeyCode::KeyE,
        Key::KeyF => KeyCode::KeyF,
        Key::KeyG => KeyCode::KeyG,
        Key::KeyH => KeyCode::KeyH,
        Key::KeyI => KeyCode::KeyI,
        Key::KeyJ => KeyCode::KeyJ,
        Key::KeyK => KeyCode::KeyK,
        Key::KeyL => KeyCode::KeyL,
        Key::KeyM => KeyCode::KeyM,
        Key::KeyN => KeyCode::KeyN,
        Key::KeyO => KeyCode::KeyO,
        Key::KeyP => KeyCode::KeyP,
        Key::KeyQ => KeyCode::KeyQ,
        Key::KeyR => KeyCode::KeyR,
        Key::KeyS => KeyCode::KeyS,
        Key::KeyT => KeyCode::KeyT,
        Key::KeyU => KeyCode::KeyU,
        Key::KeyV => KeyCode::KeyV,
        Key::KeyW => KeyCode::KeyW,
        Key::KeyX => KeyCode::KeyX,
        Key::KeyY => KeyCode::KeyY,
        Key::KeyZ => KeyCode::KeyZ,
        Key::Digit0 => KeyCode::Digit0,
        Key::Digit1 => KeyCode::Digit1,
        Key::Digit2 => KeyCode::Digit2,
        Key::Digit3 => KeyCode::Digit3,
        Key::Digit4 => KeyCode::Digit4,
        Key::Digit5 => KeyCode::Digit5,
        Key::Digit6 => KeyCode::Digit6,
        Key::Digit7 => KeyCode::Digit7,
        Key::Digit8 => KeyCode::Digit8,
        Key::Digit9 => KeyCode::Digit9,
        Key::Space => KeyCode::Space,
        Key::Escape => KeyCode::Escape,
        Key::Enter => KeyCode::Enter,
        Key::Tab => KeyCode::Tab,
        Key::Backspace => KeyCode::Backspace,
        Key::ShiftLeft => KeyCode::ShiftLeft,
        Key::ShiftRight => KeyCode::ShiftRight,
        Key::ControlLeft => KeyCode::ControlLeft,
        Key::ControlRight => KeyCode::ControlRight,
        Key::AltLeft => KeyCode::AltLeft,
        Key::AltRight => KeyCode::AltRight,
        Key::F1 => KeyCode::F1,
        Key::F2 => KeyCode::F2,
        Key::F3 => KeyCode::F3,
        Key::F4 => KeyCode::F4,
        Key::F5 => KeyCode::F5,
        Key::F6 => KeyCode::F6,
        Key::F7 => KeyCode::F7,
        Key::F8 => KeyCode::F8,
        Key::F9 => KeyCode::F9,
        Key::F10 => KeyCode::F10,
        Key::F11 => KeyCode::F11,
        Key::F12 => KeyCode::F12,
    }
}

pub use crate::gamepad_calibration::{install_calibration, pad_key, pad_state};
use rr_input::{Action, Context, devices::*};
#[derive(Resource, Default)]
pub struct CalibrationProfiles(pub rr_input::calibration::CalibrationFile);
#[derive(Resource, Default)]
pub struct Router(pub InputRouter);
#[derive(Resource, Default)]
pub struct LiveInput(pub RoutedInput);
pub fn keyboard_sample(keys: &ButtonInput<KeyCode>, bindings: &Bindings) -> DeviceSample {
    let actions = |context| {
        bindings.0.active(
            context,
            bindings.0.contexts[&context]
                .values()
                .flatten()
                .copied()
                .filter(|key| keys.pressed(key_code(*key))),
        )
    };
    let race = actions(Context::Race);
    let paused = actions(Context::Paused);
    let left = race.contains(&Action::SteerLeft);
    let right = race.contains(&Action::SteerRight);
    let drive = DriveInput {
        throttle: race.contains(&Action::Accelerate),
        brake: race.contains(&Action::Brake),
        steer_q15: (i16::from(right) - i16::from(left)) * 32767,
    };
    let pause = race.contains(&Action::Pause);
    let resume = paused.contains(&Action::Resume);
    DeviceSample {
        drive,
        pause,
        resume,
        pause_edge: bindings.0.contexts[&Context::Race][&Action::Pause]
            .iter()
            .any(|key| keys.just_pressed(key_code(*key))),
        resume_edge: bindings.0.contexts[&Context::Paused][&Action::Resume]
            .iter()
            .any(|key| keys.just_pressed(key_code(*key))),
        engaged: !drive.neutral()
            || left
            || right
            || pause
            || resume
            || race
                .iter()
                .any(|a| matches!(a, Action::Punch | Action::Kick | Action::Backhand)),
    }
}
pub fn poll_input(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    pads: Query<(Entity, &Gamepad, Option<&Name>)>,
    profiles: Option<Res<CalibrationProfiles>>,
    mut router: ResMut<Router>,
    mut input: ResMut<LiveInput>,
) {
    let samples = pads.iter().map(|(entity, pad, name)| {
        let profile = profiles
            .as_ref()
            .map(|profiles| profiles.0.profile(&pad_key(pad, name)))
            .unwrap_or_default();
        (entity.to_bits(), pad_state(pad).sample_with(profile))
    });
    input.0 = router.0.update(keyboard_sample(&keys, &bindings), samples);
}
pub fn tick_command(input: &RoutedInput) -> rr_simulation::Command {
    rr_simulation::Command {
        throttle: input.drive.throttle,
        brake: input.drive.brake,
        steer_q15: Some(input.drive.steer_q15),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::{
        InputPlugin, InputSystems,
        gamepad::{
            GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
            RawGamepadButtonChangedEvent, RawGamepadEvent,
        },
    };
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin))
            .init_resource::<Bindings>()
            .init_resource::<Router>()
            .init_resource::<LiveInput>()
            .add_systems(PreUpdate, poll_input.after(InputSystems));
        install_calibration(&mut app);
        app
    }
    pub(super) fn connect(app: &mut App) -> Entity {
        let id = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(GamepadConnectionEvent::new(
            id,
            GamepadConnection::Connected {
                name: "Authored test controller".into(),
                vendor_id: None,
                product_id: None,
            },
        ));
        id
    }
    fn axis(app: &mut App, id: Entity, x: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxis::LeftStickX,
                x,
            )));
    }
    fn button(app: &mut App, id: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                id, button, value,
            )));
    }
    #[test]
    fn actual_bevy_hotplug_axis_and_device_switch_pipeline() {
        let mut app = app();
        let one = connect(&mut app);
        app.update();
        assert!(app.world().get::<Gamepad>(one).is_some());
        axis(&mut app, one, 0.575);
        app.update();
        let partial = app.world().resource::<LiveInput>().0;
        assert_eq!(partial.device, Device::Gamepad(one.to_bits()));
        assert!(partial.drive.steer_q15 > 0 && partial.drive.steer_q15 < 32767);
        button(&mut app, one, GamepadButton::South, 1.);
        app.update();
        assert!(tick_command(&app.world().resource::<LiveInput>().0).throttle);
        let two = connect(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<LiveInput>().0.device,
            Device::Gamepad(one.to_bits())
        );
        axis(&mut app, two, -1.);
        app.update();
        let switched = app.world().resource::<LiveInput>().0;
        assert_eq!(switched.device, Device::Gamepad(two.to_bits()));
        assert_eq!(switched.drive.steer_q15, -32767);
        assert!(!switched.drive.throttle);
        app.world_mut().write_message(GamepadConnectionEvent::new(
            two,
            GamepadConnection::Disconnected,
        ));
        app.update();
        let lost = app.world().resource::<LiveInput>().0;
        assert!(lost.active_disconnected);
        assert!(lost.drive.neutral());
        assert!(app.world().get::<Gamepad>(two).is_none());
    }
    #[test]
    fn keyboard_and_full_stick_produce_the_same_command_and_simulation_hash() {
        let mut keyboard = app();
        keyboard
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        keyboard.update();
        let keyboard_command = tick_command(&keyboard.world().resource::<LiveInput>().0);
        let mut pad = app();
        let id = connect(&mut pad);
        pad.update();
        axis(&mut pad, id, 1.);
        pad.update();
        let pad_command = tick_command(&pad.world().resource::<LiveInput>().0);
        assert_eq!(keyboard_command, pad_command);
        let mut a = rr_simulation::Simulation::default();
        let mut b = rr_simulation::Simulation::default();
        for _ in 0..30 {
            a.step(rr_simulation::Command {
                throttle: true,
                ..keyboard_command
            })
            .unwrap();
            b.step(rr_simulation::Command {
                throttle: true,
                ..pad_command
            })
            .unwrap();
        }
        assert_eq!(a.state_hash(), b.state_hash());
        pad.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowLeft);
        pad.update();
        let mixed = pad.world().resource::<LiveInput>().0;
        assert_eq!(mixed.device, Device::Keyboard);
        assert_eq!(mixed.drive.steer_q15, -32767);
        pad.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        pad.update();
        assert_eq!(pad.world().resource::<LiveInput>().0.drive.steer_q15, 0);
    }
    #[test]
    fn held_on_hotplug_and_analog_trigger_brake_are_processed_safely() {
        let mut app = app();
        let id = connect(&mut app);
        button(&mut app, id, GamepadButton::South, 1.);
        app.update();
        assert!(app.world().resource::<LiveInput>().0.drive.neutral());
        button(&mut app, id, GamepadButton::South, 0.);
        app.update();
        button(&mut app, id, GamepadButton::RightTrigger2, 1.);
        button(&mut app, id, GamepadButton::LeftTrigger2, 1.);
        app.update();
        let command = tick_command(&app.world().resource::<LiveInput>().0);
        assert!(command.throttle && command.brake);
        let mut sim = rr_simulation::Simulation::default();
        sim.step(command).unwrap();
        assert_eq!(
            sim.snapshot().speed_mm_per_tick,
            0,
            "brake precedence stays in the simulation"
        );
    }
}

#[cfg(test)]
mod calibration_tests {
    use super::*;
    use bevy::input::{
        InputPlugin, InputSystems,
        gamepad::{
            GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent, RawGamepadEvent,
        },
    };
    use rr_input::calibration::{Calibration, CalibrationFile, Response, device_key};
    #[test]
    fn saved_model_profile_and_unfiltered_axes_reach_the_real_tick_command() {
        let root = std::env::temp_dir().join(format!("rr-profile-adapter-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("profile.json");
        let profile = Calibration {
            center: 0.07,
            dead_zone: 0.02,
            response: Response::Cubic,
            trigger_threshold: 0.3,
            ..Default::default()
        };
        let key = device_key(Some(0x1234), Some(0x5678), "Profile test");
        let mut file = CalibrationFile::default();
        file.set(key.clone(), profile).unwrap();
        file.save(&path).unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin))
            .init_resource::<Bindings>()
            .init_resource::<Router>()
            .init_resource::<LiveInput>();
        install_calibration(&mut app);
        app.insert_resource(CalibrationProfiles(CalibrationFile::read(&path).unwrap()))
            .add_systems(PreUpdate, poll_input.after(InputSystems));
        let id = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(GamepadConnectionEvent::new(
            id,
            GamepadConnection::Connected {
                name: "Profile test".into(),
                vendor_id: Some(0x1234),
                product_id: Some(0x5678),
            },
        ));
        app.world_mut()
            .write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxis::LeftStickX,
                0.07,
            )));
        app.update();
        assert!(app.world().resource::<LiveInput>().0.drive.neutral());
        // This sub-1% change used to be removed by Bevy's threshold. It must
        // reach the calibration curve before dead-zone suppression instead.
        app.world_mut()
            .write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxis::LeftStickX,
                0.074,
            )));
        app.update();
        assert!(
            (app.world()
                .get::<Gamepad>(id)
                .unwrap()
                .get(GamepadAxis::LeftStickX)
                .unwrap()
                - 0.074)
                .abs()
                < 0.00001
        );
        assert!(app.world().resource::<LiveInput>().0.drive.neutral());
        app.world_mut()
            .write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxis::LeftStickX,
                0.6,
            )));
        app.update();
        let command = tick_command(&app.world().resource::<LiveInput>().0);
        assert_eq!(command.steer_q15, Some(profile.steering_q15(0.6)));
        assert_ne!(
            command.steer_q15,
            Some(Calibration::default().steering_q15(0.6))
        );
        app.world_mut().write_message(RawGamepadEvent::Button(
            bevy::input::gamepad::RawGamepadButtonChangedEvent::new(
                id,
                GamepadButton::RightTrigger2,
                0.31,
            ),
        ));
        app.update();
        assert!(
            tick_command(&app.world().resource::<LiveInput>().0).throttle,
            "saved trigger threshold must apply below the engine's digital press threshold"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
