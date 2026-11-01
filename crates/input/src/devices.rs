//! Deterministic arbitration after physical values have been quantized.
use crate::calibration::Calibration;
use serde::Serialize;
use std::collections::BTreeMap;
pub const MAX_GAMEPADS: usize = 16;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DriveInput {
    pub throttle: bool,
    pub brake: bool,
    pub steer_q15: i16,
}
impl DriveInput {
    pub fn neutral(self) -> bool {
        !self.throttle && !self.brake && self.steer_q15 == 0
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeviceSample {
    pub drive: DriveInput,
    pub pause: bool,
    pub resume: bool,
    pub pause_edge: bool,
    pub resume_edge: bool,
    /// Raw digital activity, even when opposed steering cancels to zero.
    pub engaged: bool,
}
impl DeviceSample {
    pub fn neutral(self) -> bool {
        self.drive.neutral()
            && !self.pause
            && !self.resume
            && !self.pause_edge
            && !self.resume_edge
            && !self.engaged
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct PadState {
    pub stick_x: f32,
    pub right_trigger: f32,
    pub left_trigger: f32,
    pub south: bool,
    pub east: bool,
    pub left: bool,
    pub right: bool,
    pub start: bool,
}
pub fn stick_q15(value: f32) -> i16 {
    Calibration::default().steering_q15(value)
}
impl PadState {
    pub fn sample(self) -> DeviceSample {
        self.sample_with(Calibration::default())
    }
    pub fn sample_with(self, calibration: Calibration) -> DeviceSample {
        let trigger = |value| calibration.trigger(value);
        let drive = DriveInput {
            throttle: self.south || trigger(self.right_trigger),
            brake: self.east || trigger(self.left_trigger),
            steer_q15: if self.left || self.right {
                (i16::from(self.right) - i16::from(self.left)) * 32767
            } else {
                calibration.steering_q15(self.stick_x)
            },
        };
        DeviceSample {
            drive,
            pause: self.start,
            resume: self.start,
            pause_edge: false,
            resume_edge: false,
            engaged: !drive.neutral() || self.left || self.right || self.start,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Device {
    #[default]
    Keyboard,
    Gamepad(u64),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RoutedInput {
    pub device: Device,
    pub drive: DriveInput,
    pub pause_pressed: bool,
    pub resume_pressed: bool,
    pub active_disconnected: bool,
    pub connected_gamepads: usize,
}
#[derive(Default)]
struct PadHistory {
    sample: DeviceSample,
    armed: bool,
    activity_steer_q15: i16,
}
#[derive(Default)]
pub struct InputRouter {
    active: Device,
    keyboard: DeviceSample,
    pads: BTreeMap<u64, PadHistory>,
}
impl InputRouter {
    /// Keyboard activity wins simultaneous input. Otherwise retain ownership
    /// until another armed pad changes meaningfully. Newly connected pads
    /// must first report neutral, so held-on-connect input cannot take over.
    pub fn update(
        &mut self,
        keyboard: DeviceSample,
        pads: impl IntoIterator<Item = (u64, DeviceSample)>,
    ) -> RoutedInput {
        let mut current = BTreeMap::new();
        for (id, sample) in pads {
            current.insert(id, sample);
            if current.len() > MAX_GAMEPADS {
                current.pop_last();
            }
        }
        let lost = matches!(self.active,Device::Gamepad(id) if !current.contains_key(&id));
        let mut candidates = vec![];
        for (&id, &sample) in &current {
            let history = self.pads.entry(id).or_default();
            if sample.neutral() {
                history.armed = true;
            }
            // 1.5% change avoids tiny stick jitter stealing another device.
            let changed = sample.drive.throttle != history.sample.drive.throttle
                || sample.drive.brake != history.sample.drive.brake
                || (i32::from(sample.drive.steer_q15) - i32::from(history.activity_steer_q15))
                    .abs()
                    >= 512
                || sample.pause != history.sample.pause
                || sample.resume != history.sample.resume
                || (sample.engaged && !history.sample.engaged);
            if changed {
                history.activity_steer_q15 = sample.drive.steer_q15;
            }
            if history.armed && !sample.neutral() && changed {
                candidates.push(id);
            }
        }
        let keyboard_active = !keyboard.neutral();
        let previous_device = self.active;
        if lost || keyboard_active {
            self.active = Device::Keyboard;
        } else if let Some(id) = candidates
            .iter()
            .find(|id| self.active != Device::Gamepad(**id))
        {
            self.active = Device::Gamepad(*id);
        }
        let (sample, old) = match self.active {
            Device::Keyboard => (keyboard, self.keyboard),
            Device::Gamepad(id) => (current[&id], self.pads[&id].sample),
        };
        // Do not invent a pause/resume edge by transferring ownership to a
        // button which was already held on another device.
        let same_device = self.active == previous_device;
        let routed = RoutedInput {
            device: self.active,
            drive: sample.drive,
            pause_pressed: sample.pause_edge
                || (sample.pause && !old.pause && (same_device || !lost)),
            resume_pressed: sample.resume_edge
                || (sample.resume && !old.resume && (same_device || !lost)),
            active_disconnected: lost,
            connected_gamepads: current.len(),
        };
        self.keyboard = keyboard;
        self.pads.retain(|id, _| current.contains_key(id));
        for (id, sample) in current {
            self.pads.get_mut(&id).unwrap().sample = sample;
        }
        routed
    }
}
