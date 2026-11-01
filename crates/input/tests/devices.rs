use rr_input::devices::*;
fn neutral() -> DeviceSample {
    DeviceSample::default()
}
fn throttle() -> DeviceSample {
    PadState {
        south: true,
        ..Default::default()
    }
    .sample()
}
fn left() -> DeviceSample {
    PadState {
        stick_x: -1.,
        ..Default::default()
    }
    .sample()
}
#[test]
fn finite_axes_quantize_symmetrically_and_invalid_axes_are_neutral() {
    for x in [0., 0.01, 0.1, 0.15, 0.25, 0.5, 0.75, 1.] {
        assert_eq!(stick_q15(x), -stick_q15(-x));
    }
    assert_eq!(stick_q15(0.1), 0);
    assert_eq!(stick_q15(1.), 32767);
    assert_eq!(stick_q15(-2.), -32767);
    assert!(stick_q15(0.5) > 0 && stick_q15(0.5) < 32767);
    for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(stick_q15(x), 0);
    }
}
#[test]
fn digital_pad_priority_opposed_directions_and_trigger_thresholds() {
    let p = PadState {
        stick_x: 1.,
        left: true,
        ..Default::default()
    };
    assert_eq!(p.sample().drive.steer_q15, -32767);
    assert_eq!(PadState { right: true, ..p }.sample().drive.steer_q15, 0);
    assert!(
        PadState {
            right_trigger: 0.5,
            ..Default::default()
        }
        .sample()
        .drive
        .throttle
    );
    assert!(
        !PadState {
            right_trigger: 0.49,
            ..Default::default()
        }
        .sample()
        .drive
        .throttle
    );
    assert!(
        !PadState {
            right_trigger: f32::NAN,
            ..Default::default()
        }
        .sample()
        .drive
        .throttle
    );
    let both = PadState {
        south: true,
        east: true,
        ..Default::default()
    }
    .sample()
    .drive;
    assert!(both.throttle && both.brake);
}
#[test]
fn held_on_connection_is_quarantined_until_neutral() {
    let mut router = InputRouter::default();
    assert_eq!(
        router.update(neutral(), [(7, throttle())]).device,
        Device::Keyboard
    );
    assert!(router.update(neutral(), [(7, throttle())]).drive.neutral());
    router.update(neutral(), [(7, neutral())]);
    let active = router.update(neutral(), [(7, throttle())]);
    assert_eq!(active.device, Device::Gamepad(7));
    assert!(active.drive.throttle);
}
#[test]
fn keyboard_wins_simultaneous_input_and_switching_does_not_inherit_held_controls() {
    let mut router = InputRouter::default();
    router.update(neutral(), [(1, neutral()), (2, neutral())]);
    let active = router.update(throttle(), [(1, left()), (2, neutral())]);
    assert_eq!(active.device, Device::Keyboard);
    assert_eq!(active.drive, throttle().drive);
    assert_eq!(
        router
            .update(neutral(), [(1, neutral()), (2, neutral())])
            .device,
        Device::Keyboard
    );
    assert_eq!(
        router
            .update(neutral(), [(1, throttle()), (2, neutral())])
            .device,
        Device::Gamepad(1)
    );
    let switched = router.update(neutral(), [(1, throttle()), (2, left())]);
    assert_eq!(switched.device, Device::Gamepad(2));
    assert!(!switched.drive.throttle);
    let quiet = router.update(neutral(), [(1, throttle()), (2, neutral())]);
    assert_eq!(quiet.device, Device::Gamepad(2));
    assert!(quiet.drive.neutral());
}
#[test]
fn only_active_disconnect_signals_loss_and_reconnect_needs_neutral() {
    let mut router = InputRouter::default();
    router.update(neutral(), [(1, neutral()), (2, neutral())]);
    router.update(neutral(), [(1, throttle()), (2, neutral())]);
    assert!(
        !router
            .update(neutral(), [(1, throttle())])
            .active_disconnected
    );
    let lost = router.update(neutral(), []);
    assert!(lost.active_disconnected);
    assert_eq!(lost.device, Device::Keyboard);
    assert!(lost.drive.neutral());
    assert!(!router.update(neutral(), []).active_disconnected);
    assert!(router.update(neutral(), [(1, throttle())]).drive.neutral());
}
#[test]
fn stable_device_ties_and_bounded_selection_do_not_depend_on_enumeration_order() {
    let mut a = InputRouter::default();
    let mut b = InputRouter::default();
    a.update(neutral(), [(20, neutral()), (10, neutral())]);
    b.update(neutral(), [(10, neutral()), (20, neutral())]);
    assert_eq!(
        a.update(neutral(), [(20, left()), (10, throttle())]),
        b.update(neutral(), [(10, throttle()), (20, left())])
    );
    let mut r = InputRouter::default();
    let out = r.update(neutral(), (0..100).rev().map(|i| (i, neutral())));
    assert_eq!(out.connected_gamepads, MAX_GAMEPADS);
    assert_eq!(
        r.update(neutral(), [(99, throttle())]).device,
        Device::Keyboard,
        "previously excluded pad is a new unarmed connection"
    );
}
#[test]
fn start_edges_are_single_frame_and_keyboard_alternatives_can_supply_explicit_edges() {
    let mut r = InputRouter::default();
    r.update(neutral(), [(1, neutral())]);
    let start = PadState {
        start: true,
        ..Default::default()
    }
    .sample();
    let edge = r.update(neutral(), [(1, start)]);
    assert!(edge.pause_pressed && edge.resume_pressed);
    let held = r.update(neutral(), [(1, start)]);
    assert!(!held.pause_pressed && !held.resume_pressed);
    r.update(neutral(), [(1, neutral())]);
    assert!(r.update(neutral(), [(1, start)]).pause_pressed);
    let keyboard = DeviceSample {
        resume: true,
        resume_edge: true,
        ..Default::default()
    };
    assert!(r.update(keyboard, []).resume_pressed);
}

#[test]
fn gradual_axis_motion_can_reclaim_ownership_without_a_sudden_flick() {
    let mut router = InputRouter::default();
    router.update(neutral(), [(1, neutral()), (2, neutral())]);
    let partial = |axis| DeviceSample {
        drive: DriveInput {
            steer_q15: axis,
            ..Default::default()
        },
        engaged: true,
        ..Default::default()
    };
    router.update(neutral(), [(1, neutral()), (2, partial(10000))]);
    assert_eq!(
        router
            .update(neutral(), [(1, throttle()), (2, partial(10000))])
            .device,
        Device::Gamepad(1)
    );
    let mut last = RoutedInput::default();
    for step in 1..=20 {
        last = router.update(
            neutral(),
            [(1, throttle()), (2, partial(10000 + step * 64))],
        );
    }
    assert_eq!(last.device, Device::Gamepad(2));
    assert!(!last.drive.throttle);
}
