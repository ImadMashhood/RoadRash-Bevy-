use rr_input::{calibration::*, devices::PadState};
#[test]
fn default_drift_is_neutral_and_endpoints_reach_full_lock() {
    let c = Calibration::default();
    for raw in [-0.15, -0.02, 0., 0.03, 0.15] {
        assert_eq!(c.steering_q15(raw), 0);
    }
    assert_eq!(c.steering_q15(-0.99997), -32767);
    assert_eq!(c.steering_q15(0.99997), 32767);
    assert_eq!(c.steering_q15(-5.), -32767);
    assert_eq!(c.steering_q15(5.), 32767);
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(c.steering_q15(value), 0);
    }
}
#[test]
fn every_response_is_symmetric_monotonic_and_bounded() {
    for response in [Response::Linear, Response::Square, Response::Cubic] {
        let c = Calibration {
            response,
            ..Default::default()
        };
        let mut previous = 0;
        for n in 0..=1000 {
            let x = n as f32 / 1000.;
            let positive = c.steering_q15(x);
            assert_eq!(positive, -c.steering_q15(-x));
            assert!(positive >= previous);
            previous = positive;
        }
        assert_eq!(previous, 32767);
    }
    for (response, expected) in [
        (Response::Linear, 16384),
        (Response::Square, 8192),
        (Response::Cubic, 4096),
    ] {
        let profile = Calibration {
            dead_zone: 0.,
            saturation: 1.,
            response,
            ..Default::default()
        };
        assert_eq!(profile.steering_q15(0.5), expected);
        assert_eq!(profile.steering_q15(-0.5), -expected);
    }
    let value = |response| {
        Calibration {
            response,
            ..Default::default()
        }
        .steering_q15(0.55)
    };
    assert!(
        value(Response::Cubic) < value(Response::Square)
            && value(Response::Square) < value(Response::Linear)
    );
}
#[test]
fn calibrated_center_and_asymmetric_travel_produce_symmetric_steering() {
    let c = Calibration {
        minimum: -0.8,
        center: 0.1,
        maximum: 0.7,
        ..Default::default()
    };
    c.validate().unwrap();
    assert_eq!(c.steering_q15(0.1), 0);
    assert!((i32::from(c.steering_q15(-0.35)) + i32::from(c.steering_q15(0.4))).abs() <= 1);
    assert_eq!(c.steering_q15(-0.8), -32767);
    assert_eq!(c.steering_q15(0.7), 32767);
    let inverted = Calibration { invert: true, ..c };
    assert_eq!(inverted.steering_q15(0.4), -c.steering_q15(0.4));
}
#[test]
fn captures_learn_drift_and_travel_and_reject_moving_or_one_sided_input() {
    let mut center = Capture::default();
    for n in 0..120 {
        center.push(0.06 + if n % 2 == 0 { 0.004 } else { -0.004 });
    }
    let learned = center.center(Calibration::default()).unwrap();
    assert!((learned.center - 0.06).abs() < 0.00001);
    assert!(learned.dead_zone >= 0.02 && learned.dead_zone < 0.04);
    for x in [0.056, 0.06, 0.064] {
        assert_eq!(learned.steering_q15(x), 0);
    }
    let mut range = Capture::default();
    for n in 0..100 {
        range.push(-0.85 + n as f32 / 99. * 1.55);
    }
    let learned = range.range(learned).unwrap();
    assert_eq!(learned.steering_q15(-0.85), -32767);
    assert_eq!(learned.steering_q15(0.7), 32767);
    let mut moving = Capture::default();
    for _ in 0..20 {
        moving.push(-0.5);
        moving.push(0.5);
    }
    assert!(moving.center(learned).is_err());
    let mut one_side = Capture::default();
    for _ in 0..20 {
        one_side.push(0.8);
    }
    assert!(one_side.range(learned).is_err());
    let mut invalid = Capture::default();
    for _ in 0..100 {
        invalid.push(f32::NAN);
    }
    assert!(invalid.center(learned).is_err());
}
#[test]
fn model_profiles_are_stable_scoped_and_round_trip_without_transient_ids() {
    let a = device_key(Some(0x12), Some(0x34), "  Test PAD\n");
    let b = device_key(Some(0x12), Some(0x34), "test pad");
    assert_eq!(a, b);
    let mut file = CalibrationFile::default();
    let custom = Calibration {
        dead_zone: 0.25,
        response: Response::Cubic,
        ..Default::default()
    };
    file.set(a.clone(), custom).unwrap();
    assert_eq!(file.profile(&a), custom);
    assert_eq!(file.profile("different device"), file.default);
    let root = std::env::temp_dir().join(format!("rr-calibration-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("profile.json");
    file.save(&path).unwrap();
    assert_eq!(CalibrationFile::read(&path).unwrap(), file);
    let before = std::fs::read(&path).unwrap();
    file.default.center = f64::NAN;
    assert!(file.save(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn invalid_settings_and_schema_leave_the_previous_profile_intact() {
    let mut file = CalibrationFile::default();
    let before = file.clone();
    let key = device_key(None, None, "test");
    for invalid in [
        Calibration {
            minimum: 0.,
            ..Default::default()
        },
        Calibration {
            dead_zone: 0.9,
            ..Default::default()
        },
        Calibration {
            saturation: 0.5,
            dead_zone: 0.5,
            ..Default::default()
        },
        Calibration {
            trigger_threshold: 0.,
            ..Default::default()
        },
        Calibration {
            maximum: f64::INFINITY,
            ..Default::default()
        },
    ] {
        assert!(file.set(key.clone(), invalid).is_err());
        assert_eq!(file, before);
        assert_eq!(invalid.steering_q15(1.), 0);
    }
    assert!(
        file.set("not a model identity".into(), Calibration::default())
            .is_err()
    );
    assert!(
        file.set("0001:0002:UPPERCASE".into(), Calibration::default())
            .is_err()
    );
    let mut unknown = serde_json::to_value(file).unwrap();
    unknown["schema_version"] = serde_json::json!(2);
    let parsed: CalibrationFile = serde_json::from_value(unknown).unwrap();
    assert!(parsed.validate().is_err());
}
#[test]
fn preview_curve_and_final_pad_sample_use_the_same_transform() {
    let profile = Calibration {
        dead_zone: 0.08,
        response: Response::Cubic,
        trigger_threshold: 0.3,
        ..Default::default()
    };
    for [raw, output] in profile.curve() {
        let sample = PadState {
            stick_x: raw as f32,
            right_trigger: 0.31,
            ..Default::default()
        }
        .sample_with(profile);
        assert_eq!(f64::from(sample.drive.steer_q15) / 32767., output);
        assert!(sample.drive.throttle);
    }
    let sample = PadState {
        stick_x: 0.4,
        left: true,
        ..Default::default()
    }
    .sample_with(profile);
    assert_eq!(
        sample.drive.steer_q15, -32767,
        "D-pad direction is semantic, not inverted analog wiring"
    );
}

#[test]
fn long_sweeps_keep_late_extremes_in_constant_memory() {
    let mut capture = Capture::default();
    for _ in 0..10000 {
        capture.push(0.);
    }
    capture.push(-0.9);
    capture.push(0.85);
    let learned = capture.range(Calibration::default()).unwrap();
    assert_eq!(learned.steering_q15(-0.9), -32767);
    assert_eq!(learned.steering_q15(0.85), 32767);
}
