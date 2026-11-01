//! Bevy transport setup and stable profile lookup, shared by game and preview.
use bevy::prelude::*;
use rr_input::{calibration::device_key, devices::PadState};
pub fn install_calibration(app: &mut App) {
    app.add_systems(
        PreUpdate,
        configure_gamepads
            .after(bevy::input::gamepad::gamepad_connection_system)
            .before(bevy::input::gamepad::gamepad_event_processing_system),
    );
}
pub fn configure_gamepads(mut pads: Query<&mut GamepadSettings, Added<Gamepad>>) {
    for mut settings in &mut pads {
        settings.default_axis_settings =
            bevy::input::gamepad::AxisSettings::new(-1., 0., 0., 1., 0.)
                .expect("valid transparent axis settings");
        settings.default_button_axis_settings = bevy::input::gamepad::ButtonAxisSettings {
            low: 0.,
            high: 1.,
            threshold: 0.,
        };
        settings.axis_settings.clear();
        settings.button_axis_settings.clear();
    }
}
pub fn pad_state(pad: &Gamepad) -> PadState {
    PadState {
        buttons: rr_input::devices::Buttons {
            punch: pad.pressed(GamepadButton::West),
            kick: pad.pressed(GamepadButton::North),
            backhand: pad.pressed(GamepadButton::LeftTrigger),
            nitro: pad.pressed(GamepadButton::RightTrigger),
            dismount: pad.pressed(GamepadButton::Select),
            lean: i8::from(pad.pressed(GamepadButton::DPadUp))
                - i8::from(pad.pressed(GamepadButton::DPadDown)),
        },
        stick_x: pad.get(GamepadAxis::LeftStickX).unwrap_or(0.),
        right_trigger: pad.get(GamepadButton::RightTrigger2).unwrap_or(0.),
        left_trigger: pad.get(GamepadButton::LeftTrigger2).unwrap_or(0.),
        south: pad.pressed(GamepadButton::South),
        east: pad.pressed(GamepadButton::East),
        left: pad.pressed(GamepadButton::DPadLeft),
        right: pad.pressed(GamepadButton::DPadRight),
        start: pad.pressed(GamepadButton::Start),
    }
}
pub fn pad_key(pad: &Gamepad, name: Option<&Name>) -> String {
    device_key(
        pad.vendor_id(),
        pad.product_id(),
        name.map_or("Unnamed controller", |name| name.as_str()),
    )
}
