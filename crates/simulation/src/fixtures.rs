//! Original synthetic scenario data. No game payloads or commercial pixels.
use crate::{animation::*, handling::*, riding::*, *};
pub fn spec() -> BikeSpec {
    BikeSpec {
        id: 1,
        mass: 100,
        contact_steer_gain: 0,
        gears: (0..7)
            .map(|i| Gear {
                ratio: 256,
                force: 256 - i * 20,
                up: 1000 + i * 1000,
                down: i * 1000 - 200,
            })
            .collect(),
        throttle: 20,
        throttle_on: 5,
        throttle_off: 12,
        brake: -20,
        brake_on: 5,
        brake_off: 5,
        brake_steer: 128,
        steer_on: 50,
        steer_off: 25,
        steer_scale: 100,
        yaw_gain: 128,
        skid_yaw_gain: 256,
        lean_on: 256,
        lean_off: 128,
        grip_scale: 320,
        skid_threshold: 30,
        landing: -500,
        suspension: 1000,
        rebound: 128,
        gravity: 100,
        launch_cap: 2,
        condition: 100,
    }
}
pub fn tables() -> Tables {
    let mut object = [0; 26];
    object[7] = 10000;
    object[8] = 8000;
    object[9] = 256;
    object[10] = 1;
    let sine = (0..1024)
        .map(|i| ((i as f64 * std::f64::consts::FRAC_PI_2 / 1024.).sin() * 32768.) as i32)
        .collect();
    Tables {
        contact: 74,
        drag: [0, 2, 20, 30],
        grip: [256, 240, 128, 100],
        gain: [230, 230, 128, 100],
        damp: [6, 5, 1, 1],
        skid: [12, 12, 0, 0],
        curve: [256; 32],
        sine,
        object,
    }
}
pub fn library() -> Library {
    let frame = Frame {
        render: [0; 12],
        duration: 3,
        control: 1,
        flags: 0,
    };
    let mut lib = Library {
        banks: [
            vec![Some(vec![frame.clone()]); 52],
            vec![Some(vec![frame.clone()]); 52],
        ],
        views: [
            vec![vec![Cell { frame: 0, flags: 0 }]],
            vec![vec![Cell { frame: 0, flags: 0 }]],
        ],
        fallback: vec![2; 52],
        attacks: [
            [[10, 12, 14, 16], [18, 20, 22, 16]],
            [[11, 13, 15, 17], [19, 21, 23, 17]],
        ],
    };
    for bank in &mut lib.banks {
        for sequence in bank.iter_mut().take(24).skip(10) {
            let mut first = frame.clone();
            first.control = 0;
            let mut hit = frame.clone();
            hit.flags = 0x600000;
            hit.control = 2;
            *sequence = Some(vec![first, hit]);
        }
    }
    lib
}
pub fn riding() -> Configuration {
    Configuration {
        motion_version: 0,
        actions: None,
        specs: vec![spec()],
        selected: 1,
        tables: tables(),
        animation: library(),
        ground: vec![Ground::flat(); 128],
        start_sample: 0,
    }
}
pub fn gas() -> Command {
    Command {
        throttle: true,
        ..Default::default()
    }
}
