//! Source-unit fixed-step handling. Only the startup decoder knows binary offsets.
//! Reference: pinned bike.cpp and Race::vertical, documented in docs/HANDLING.md.
use crate::{Command, Error, Result};
use rr_formats::{
    binary::{range, u32le},
    pe::SupportedExe,
    resource::Directory,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Gear {
    pub ratio: i32,
    pub force: i32,
    pub up: i32,
    pub down: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BikeSpec {
    #[serde(default, skip_serializing_if = "crate::is_false")]
    pub ai: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rider_value: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub throttle_percent: Option<i32>,
    pub id: u32,
    pub mass: i32,
    #[serde(default, skip_serializing_if = "zero_i32")]
    pub contact_steer_gain: i32,
    pub gears: Vec<Gear>,
    pub throttle: i32,
    pub throttle_on: i32,
    pub throttle_off: i32,
    pub brake: i32,
    pub brake_on: i32,
    pub brake_off: i32,
    pub brake_steer: i32,
    pub steer_on: i32,
    pub steer_off: i32,
    pub steer_scale: i32,
    pub yaw_gain: i32,
    pub skid_yaw_gain: i32,
    pub lean_on: i32,
    pub lean_off: i32,
    pub grip_scale: i32,
    pub skid_threshold: i32,
    pub landing: i32,
    pub suspension: i32,
    pub rebound: i32,
    pub gravity: i32,
    pub launch_cap: i32,
    pub condition: i32,
}
fn zero_i32(value: &i32) -> bool {
    *value == 0
}
impl BikeSpec {
    pub fn validate(&self) -> Result<()> {
        if self.rider_value.is_some_and(|v| !(1..=1024).contains(&v))
            || self
                .throttle_percent
                .is_some_and(|v| !(1..=200).contains(&v))
            || self.id == 0
            || self.id > 64
            || !(1..=1000).contains(&self.mass)
            || !(0..=65536).contains(&self.contact_steer_gain)
            || self.gears.len() != 7
            || self.gears.iter().any(|g| {
                !(0..=1024).contains(&g.force)
                    || !(0..=65536).contains(&g.ratio)
                    || g.up < g.down
                    || !(-10000..=0x1000000).contains(&g.up)
                    || !(-10000..=0x1000000).contains(&g.down)
            })
            || !(1..=256).contains(&self.throttle)
            || !(-512..=-1).contains(&self.brake)
            || !(0..=1024).contains(&self.rebound)
            || !(0..=1024).contains(&self.launch_cap)
            || !(0..=65536).contains(&self.suspension)
            || !(-65536..=0).contains(&self.landing)
            || !(0..=1024).contains(&self.grip_scale)
            || !(0..=1024).contains(&self.brake_steer)
            || !(0..=65536).contains(&self.yaw_gain)
            || !(0..=65536).contains(&self.skid_yaw_gain)
            || self.brake >= 0
            || [
                self.throttle_on,
                self.throttle_off,
                self.brake_on,
                self.brake_off,
                self.steer_on,
                self.steer_off,
                self.steer_scale,
                self.gravity,
                self.condition,
            ]
            .iter()
            .any(|v| !(1..=65536).contains(v))
        {
            return Err(Error::Invalid("bike specification field/range"));
        }
        Ok(())
    }
    pub fn read(bytes: &[u8]) -> std::result::Result<Vec<Self>, Box<dyn std::error::Error>> {
        let dir = Directory::read(bytes)?;
        let mut out = vec![];
        for r in dir.entries.iter().filter(|r| r.tag == "SPEC") {
            let b = range(bytes, r.offset as usize, r.length as usize)?;
            if b.len() != 356 {
                return Err("SPEC requires 89 words".into());
            }
            let mut d = [0i32; 89];
            for (i, v) in d.iter_mut().enumerate() {
                *v = u32le(b, i * 4)? as i32;
            }
            let w = |offset: usize| d[7 + (offset - 0x1c4) / 4];
            let spec = Self {
                ai: false,
                rider_value: None,
                throttle_percent: None,
                id: r.id,
                mass: d[6],
                contact_steer_gain: w(0x2d8),
                gears: (0..7)
                    .map(|i| Gear {
                        ratio: w(0x1cc + i * 16),
                        force: w(0x1d0 + i * 16),
                        up: w(0x1d4 + i * 16),
                        down: w(0x1d8 + i * 16),
                    })
                    .collect(),
                throttle: w(0x250),
                throttle_on: w(0x254),
                throttle_off: w(0x258),
                brake: w(0x26c),
                brake_on: w(0x270),
                brake_off: w(0x274),
                brake_steer: w(0x25c),
                steer_on: w(0x2a0),
                steer_off: w(0x2a4),
                steer_scale: w(0x2b0),
                yaw_gain: w(0x2b8),
                skid_yaw_gain: w(0x2bc),
                lean_on: w(0x2d0),
                lean_off: w(0x2d4),
                grip_scale: w(0x294),
                skid_threshold: w(0x290),
                landing: d[1],
                suspension: d[2],
                rebound: d[3],
                gravity: d[4],
                launch_cap: d[5],
                condition: w(0x2ec),
            };
            spec.validate()?;
            if out.iter().any(|s: &Self| s.id == spec.id) {
                return Err("duplicate SPEC ID".into());
            }
            out.push(spec);
        }
        out.sort_by_key(|s| s.id);
        if out.is_empty() || out.iter().enumerate().any(|(i, s)| s.id as usize != i + 1) {
            return Err("missing SPEC IDs".into());
        }
        Ok(out)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tables {
    pub contact: i32,
    pub drag: [i32; 4],
    pub grip: [i32; 4],
    pub gain: [i32; 4],
    pub damp: [i32; 4],
    pub skid: [i32; 4],
    pub curve: [i32; 32],
    pub sine: Vec<i32>,
    pub object: [i32; 26],
}
impl Tables {
    pub fn read(exe: &[u8]) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let pe = SupportedExe::read(exe)?;
        let word = |va| -> std::result::Result<i32, Box<dyn std::error::Error>> {
            Ok(u32le(pe.read_va(va, 4)?, 0)? as i32)
        };
        let arr = |va| -> std::result::Result<[i32; 4], Box<dyn std::error::Error>> {
            Ok([word(va)?, word(va + 4)?, word(va + 8)?, word(va + 12)?])
        };
        let mut curve = [0; 32];
        for (i, v) in curve.iter_mut().enumerate() {
            *v = word(0x46dd28 + i as u32 * 4)?;
        }
        let mut object = [0; 26];
        for (i, v) in object.iter_mut().enumerate() {
            *v = word(0x463ba8 + i as u32 * 4)?;
        }
        let d = pe.read_va(0x46dce4, 4)?;
        let t = Self {
            contact: word(0x46dcd8)?,
            drag: std::array::from_fn(|i| i32::from(d[i] as i8)),
            grip: arr(0x46dce8)?,
            gain: arr(0x46dcf8)?,
            damp: arr(0x46dd08)?,
            skid: arr(0x46dd18)?,
            curve,
            sine: (0..1024)
                .map(|i| word(0x475768 + i * 4))
                .collect::<std::result::Result<_, _>>()?,
            object,
        };
        t.validate()?;
        Ok(t)
    }
    pub fn validate(&self) -> Result<()> {
        if self.contact != 74
            || self.sine.len() != 1024
            || self.sine[0] != 0
            || self.sine[512] != 23170
            || self.grip[0] != 256
            || self.drag.iter().any(|v| !(0..=127).contains(v))
            || self.sine.iter().any(|v| !(0..=32768).contains(v))
            || self
                .grip
                .iter()
                .chain(self.gain.iter())
                .chain(self.damp.iter())
                .chain(self.skid.iter())
                .chain(self.curve.iter())
                .any(|v| !(0..=4096).contains(v))
            || self.object[7..=10].iter().any(|v| v.unsigned_abs() > 65536)
        {
            return Err(Error::Invalid("physics table identity"));
        }
        Ok(())
    }
    pub(crate) fn sin(&self, a: i32) -> i32 {
        let a = a as usize & 4095;
        let i = a & 1023;
        match a >> 10 {
            0 => self.sine[i] * 2,
            1 => self.sine[1023 - i] * 2,
            2 => -self.sine[i] * 2,
            _ => -self.sine[1023 - i] * 2,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ground {
    pub left: i32,
    pub right: i32,
    pub left_shoulder: i32,
    pub right_shoulder: i32,
    pub flags: u8,
    pub kind: u8,
    pub rise: i32,
    #[serde(default)]
    pub bend: i32,
    #[serde(default)]
    pub bend_movement: i32,
    pub height: i32,
    pub movement: i32,
}
impl Ground {
    pub fn flat() -> Self {
        Self {
            left: -500,
            right: 500,
            left_shoulder: -750,
            right_shoulder: 750,
            ..Self::default()
        }
    }
    pub fn surface(&self, x_q8: i32) -> usize {
        let x = x_q8 >> 8;
        if self.left < x && x < self.right {
            return 0;
        }
        if self.flags != 0 {
            if self.flags & 0x20 != 0 {
                return 0;
            }
            if self.flags & 0x18 != 0 {
                return if self.kind == 0 { 1 } else { 2 };
            }
        }
        if self.kind == 3 {
            return 1;
        }
        if self.right < x {
            if self.right_shoulder <= x {
                return 3;
            }
            if self.kind == 0 {
                return 1;
            }
            return if x <= self.right + 48 { 0 } else { 2 };
        }
        if x <= self.left_shoulder {
            return 3;
        }
        if self.kind == 0 {
            return 1;
        }
        if self.left - 48 <= x { 0 } else { 2 }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bike {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_yaw: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_motion_flags: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_roll: Option<i32>,
    #[serde(default, skip_serializing_if = "zero_i32")]
    pub scenery_timer: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenery_mark: Option<crate::scenery::Mark>,
    pub speed: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_velocity: Option<[i32; 3]>,
    #[serde(default, skip_serializing_if = "zero_i32")]
    pub air_lateral_speed: i32,
    pub lateral: i32,
    pub lateral_speed: i32,
    pub distance_q8: i64,
    pub lean: i32,
    pub previous_lean: i32,
    pub gear: usize,
    pub force: i32,
    pub drive: i32,
    pub drag: i32,
    pub acceleration: i32,
    pub lateral_force: i32,
    pub contact: i32,
    pub skid: u8,
    pub skid_ticks: i32,
    pub failed: bool,
    pub crashed: bool,
    pub condition: i32,
    pub height: i32,
    pub vertical_speed: i32,
    pub gravity: i32,
    pub previous_above: i32,
    pub previous_ground: i32,
    pub previous_rise: i32,
    pub pitch: i32,
    pub airborne: bool,
    pub landed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VerticalKind {
    BikeCurrent,
    Legacy,
    Bike,
    Rider,
}
impl Bike {
    pub fn refresh_scenery(&mut self, sample: i64, tick: u64) {
        if let Some(mark) = &mut self.scenery_mark {
            if mark.frame != 0 {
                let elapsed = (tick as u32).wrapping_sub(mark.tick as u32);
                mark.frame = if sample == mark.sample && elapsed < 12 {
                    (elapsed / 4 + 1) as u8
                } else {
                    0
                };
            }
            mark.display_frame = mark.frame;
        }
    }
    pub fn new(spec: &BikeSpec, t: &Tables) -> Result<Self> {
        spec.validate()?;
        t.validate()?;
        Ok(Self {
            ai_roll: spec.ai.then_some(0),
            condition: spec.condition,
            contact: t.contact,
            gravity: -37,
            ..Self::default()
        })
    }
    pub fn integrate_source_speed(
        &mut self,
        tables: &Tables,
        surface: usize,
        drag_coefficient: i32,
        contact_coefficient: i32,
    ) {
        let magnitude = self.speed.saturating_abs();
        let base = if self.contact == 0 {
            magnitude >> 2
        } else {
            magnitude
        };
        let contact = if self.contact == 0 {
            0
        } else {
            ((tables.drag[surface & 3] * magnitude) >> 14) + contact_coefficient
        };
        let drag = (drag_coefficient * (base >> 8)) >> 8;
        let total = drag + contact;
        let lateral = self.lateral_force.saturating_abs() >> 4;
        self.drag = total;
        if self.speed >= 0 {
            self.acceleration = self.drive - total - lateral;
            let next = self.speed.wrapping_add(self.acceleration);
            self.speed = if next < 1 {
                if self.drive < 1 { 0 } else { 100 }
            } else {
                next
            };
        } else {
            self.acceleration = self.drive + (lateral + total).saturating_abs();
            self.speed = self.speed.wrapping_add(self.acceleration).min(0);
        }
    }
    pub fn damage(&mut self, amount: u32) {
        self.condition = self
            .condition
            .saturating_sub(amount.min(i32::MAX as u32) as i32)
            .max(0);
        if self.condition == 0 {
            self.failed = true;
            self.drive = 0;
        }
    }
    pub fn step(&mut self, s: &BikeSpec, t: &Tables, input: Command, g: Ground) {
        self.step_on_route(s, t, input, g, |old, new| {
            let mut g = g;
            let displacement = (new - old) as i32;
            g.movement += g.rise * displacement;
            g.bend_movement -= g.bend * displacement;
            g
        });
    }
    pub fn step_on_route(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        input: Command,
        g: Ground,
        resolve: impl FnOnce(i64, i64) -> Ground,
    ) {
        self.step_with_boost(s, t, input, g, 256, resolve);
    }
    pub(crate) fn step_with_boost(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        input: Command,
        g: Ground,
        boost_q8: i32,
        resolve: impl FnOnce(i64, i64) -> Ground,
    ) {
        self.step_with_boost_profile(s, t, input, g, boost_q8, 0, resolve);
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step_with_boost_profile(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        input: Command,
        g: Ground,
        boost_q8: i32,
        motion_version: u8,
        resolve: impl FnOnce(i64, i64) -> Ground,
    ) {
        let ground =
            self.step_motion_with_boost_profile(s, t, input, g, boost_q8, motion_version, resolve);
        self.finish_motion(s, t, ground, motion_version);
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step_motion_with_boost_profile(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        input: Command,
        g: Ground,
        boost_q8: i32,
        motion_version: u8,
        resolve: impl FnOnce(i64, i64) -> Ground,
    ) -> Ground {
        self.source_controls(s, t, input, boost_q8);
        self.source_physics_motion(s, t, g, motion_version, resolve)
    }
    pub(crate) fn source_controls(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        input: Command,
        boost_q8: i32,
    ) {
        self.source_controls_profile(s, t, input, boost_q8, false);
    }
    pub(crate) fn source_controls_exact(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        input: Command,
        boost_q8: i32,
    ) {
        self.source_controls_profile(s, t, input, boost_q8, true);
    }
    fn source_controls_profile(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        input: Command,
        boost_q8: i32,
        native: bool,
    ) {
        if !s.ai {
            self.source_drive_mode_profile(
                s,
                t,
                if input.brake {
                    2
                } else if input.throttle {
                    1
                } else {
                    0
                },
                native,
            );
        }
        if self.drive > 0 {
            self.drive = ((i64::from(self.drive) * i64::from(boost_q8.clamp(0, 1024))) >> 8) as i32;
        }
        if !s.ai {
            let mut steering = input
                .steer_q15
                .map_or(i32::from(input.steer) * 32767, i32::from);
            let mut lean = i32::from(input.lean);
            if steering == 0 && lean != 0 {
                steering = lean * 32767;
                lean = -lean;
            }
            self.source_steer(s, t, (i64::from(steering) * 0x200000 / 32767) as i32, lean);
        }
    }
    /// Movement after an already completed source think; no control or boost is applied again.
    pub(crate) fn source_physics_motion(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        g: Ground,
        motion_version: u8,
        resolve: impl FnOnce(i64, i64) -> Ground,
    ) -> Ground {
        // 60 Hz host bridge: original physics receives dt=1; source scheduler ticks are 60 Hz.
        self.source_traction(s, t, g);
        self.source_motion_after_traction(s, t, g, motion_version, None, resolve)
    }
    pub(crate) fn source_traction(&mut self, s: &BikeSpec, t: &Tables, g: Ground) {
        self.traction(t, s, g.surface(self.lateral));
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn source_motion_after_traction(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        g: Ground,
        motion_version: u8,
        contact_override: Option<i32>,
        resolve: impl FnOnce(i64, i64) -> Ground,
    ) -> Ground {
        let surface = g.surface(self.lateral);
        if motion_version >= 1 {
            self.integrate_source_speed(
                t,
                surface,
                t.object[9],
                contact_override.unwrap_or(t.object[10]),
            );
        } else {
            let base = if self.contact == 0 {
                self.speed.abs() >> 2
            } else {
                self.speed.abs()
            };
            let contact_drag = if self.contact == 0 {
                0
            } else {
                ((t.drag[surface] * self.speed.abs()) >> 14) + t.object[10]
            };
            self.drag = ((t.object[9] * (base >> 8)) >> 8) + contact_drag;
            self.acceleration = self.drive - self.drag - (self.lateral_force.abs() >> 4);
            let next = self.speed.saturating_add(self.acceleration).min(1_000_000);
            self.speed = if next < 1 {
                if self.drive < 1 { 0 } else { 100 }
            } else {
                next
            };
        }
        if s.gears[self.gear].up < self.speed {
            self.gear = (self.gear + 1).min(6);
        } else if self.speed < s.gears[self.gear].down {
            self.gear = self.gear.saturating_sub(1);
        }
        // 0x004395e0: longitudinal speed is Q8 world-distance per second;
        // source sample Q8 position advances trunc((speed*2-accel)/2)/256.
        let displacement = ((self.speed * 2 - self.acceleration) >> 1) / 256;
        let old_distance = self.distance_q8;
        self.distance_q8 += i64::from(displacement);
        let old_rise = g.rise;
        let g = resolve(old_distance, self.distance_q8);
        if old_distance >> 8 != self.distance_q8 >> 8 {
            self.previous_rise = old_rise;
        }
        if motion_version >= 2 && self.contact > 0 {
            self.air_lateral_speed = self.lateral_speed;
        }
        let lateral = if motion_version >= 2 {
            self.air_lateral_speed
        } else {
            self.lateral_speed
        };
        if contact_override.is_some() {
            self.update_source_yaw(s, lateral);
        }
        let mut collision_lateral = lateral;
        if self.speed > 300 {
            let gain = if self.skid != 0 && (self.skid as i8) >= 51 {
                800
            } else {
                400
            };
            self.lateral = self
                .lateral
                .saturating_add(lateral)
                .saturating_add(g.bend_movement * gain / 256);
            collision_lateral += g.bend_movement * gain / 256;
        }
        if motion_version >= 2 {
            self.contact_velocity = Some([self.speed, collision_lateral, 0]);
        }
        g
    }
    pub(crate) fn update_source_yaw(&mut self, s: &BikeSpec, lateral: i32) {
        let Some(flags) = self.source_motion_flags else {
            return;
        };
        let (forward, side) = if self.speed < 0 {
            (self.speed.wrapping_mul(-4), -lateral)
        } else {
            (self.speed.wrapping_mul(4), lateral)
        };
        let gain = if flags & 4 != 0 {
            s.skid_yaw_gain
        } else {
            s.yaw_gain
        };
        let mut yaw =
            (gain / 256).wrapping_mul(crate::riding::atan2(forward, side).wrapping_mul(65536));
        if flags & 1 != 0 {
            yaw = if yaw < 1 { -0x3f0000 } else { 0x3f0000 };
        }
        if self.speed < 4000 {
            yaw = yaw.clamp(-0x180000, 0x180000);
        }
        self.source_yaw = Some(yaw);
    }
    pub(crate) fn source_drive_mode(&mut self, s: &BikeSpec, t: &Tables, mode: u8) {
        self.source_drive_mode_profile(s, t, mode, true);
    }
    fn source_drive_mode_profile(&mut self, s: &BikeSpec, _t: &Tables, mode: u8, native: bool) {
        let mass = s.mass + 75;
        let rider_value = s.rider_value.unwrap_or(75);
        let weight = mass + ((rider_value - 75) >> 3);
        let max = s.throttle * mass * 256 / weight;
        let min = s.brake * mass * 256 / weight;
        let throttle_limit = max * s.throttle_percent.unwrap_or(100) / 100;
        let target = if self.failed || mode == 2 {
            min
        } else if mode == 1 {
            throttle_limit
        } else {
            0
        };
        if self.failed || mode == 2 || (self.force < 0 && (!native || mode != 1)) {
            if self.speed == 0 {
                self.force = 0;
            } else {
                let target = target.max(min);
                if self.force < target {
                    let rate = min / s.brake_off.min(15);
                    self.force = if self.force - target > rate {
                        self.force - rate
                    } else {
                        target
                    };
                } else if self.force > target {
                    let rate = min / s.brake_on.min(15);
                    self.force = if self.force > rate {
                        self.force + rate
                    } else {
                        target
                    };
                }
            }
            self.drive = self.force >> 8;
        } else {
            let target = target.min(throttle_limit);
            if self.force < target {
                self.force += (max / s.throttle_on.min(15)).min(target - self.force);
            } else if self.force > target {
                self.force -= (max / s.throttle_off.min(15)).min(self.force - target);
            }
            self.drive = if self.force < 1 {
                self.force >> 8
            } else {
                ((i64::from(s.gears[self.gear].force) * i64::from(self.force)) >> 16) as i32
            };
        }
    }
    pub(crate) fn source_steer(&mut self, s: &BikeSpec, t: &Tables, angle: i32, lean: i32) {
        if self.speed <= 0 {
            self.lateral_force = 0;
            self.lateral_speed = 0;
        } else {
            let limit = 0x2e00 + if lean != 0 { 0x2e00 / 4 } else { 0 };
            let sn = t.sin(((i64::from(angle.clamp(-0x200000, 0x200000)) * 16) >> 16) as i32);
            let target = ((i64::from(sn / 256) * i64::from(self.speed) / i64::from(s.steer_scale))
                as i32)
                .clamp(-limit, limit);
            self.previous_lean = self.lean;
            let mass = s.mass + 75;
            let rider_value = s.rider_value.unwrap_or(75);
            let on = s.steer_on * mass / (s.mass + rider_value / 4);
            let off = s.steer_off * mass / (s.mass + rider_value / 4);
            let rate = 0x2e00
                / if target == 0 || target.abs() <= self.lean.abs() {
                    off
                } else {
                    on
                };
            let delta = target - self.lean;
            let next = if delta < -rate {
                let current = self.lean - if lean < 0 { s.lean_on * rate / 256 } else { 0 };
                if lean < 1 {
                    current - rate
                } else {
                    current - s.lean_off * rate / 256
                }
            } else if delta > rate {
                let current = self.lean + if lean > 0 { s.lean_on * rate / 256 } else { 0 };
                if lean < 0 {
                    current + s.lean_off * rate / 256
                } else {
                    current + rate
                }
            } else {
                target
            };
            self.lean = next;
            self.lateral_speed = (self.lean / 256) * s.steer_scale;
        }
    }
    pub(crate) fn finish_motion(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        g: Ground,
        motion_version: u8,
    ) {
        if motion_version >= 2 {
            self.vertical_profile(
                s,
                t,
                g,
                if motion_version >= 3 {
                    VerticalKind::BikeCurrent
                } else {
                    VerticalKind::Bike
                },
            );
        } else {
            self.vertical(s, t, g);
        }
    }
    fn traction(&mut self, t: &Tables, s: &BikeSpec, surface: usize) {
        self.lateral_force = 0;
        if self.contact == 0 || self.crashed {
            if s.ai {
                self.ai_roll = Some(source_roll(t.contact, self.lean / 256));
            }
            self.drive = 0;
            self.lateral_speed = 0;
            self.skid = 0;
            return;
        }
        if self.speed < 1 {
            self.lean = 0;
        }
        let base = if self.skid == 0 {
            t.object[7]
        } else {
            t.object[8]
        };
        self.scenery_timer = (self.scenery_timer - 1).max(0);
        let base = base - self.scenery_timer * 16;
        let mut grip =
            (i64::from(base) * i64::from(s.grip_scale) / 256).clamp(0, i64::from(i32::MAX)) as i32;
        grip = (i64::from(grip) * i64::from(t.grip[surface]) / 256).clamp(0, i64::from(i32::MAX))
            as i32;
        grip =
            (i64::from(grip) * i64::from(self.contact) / 256).clamp(0, i64::from(i32::MAX)) as i32;
        grip = grip.max(0);
        let mut source_flags = 0u8;
        if !s.ai && grip < s.skid_threshold && self.lean.abs() >= 0x2e00 {
            source_flags = 1;
            self.skid_ticks += 1;
            if self.skid_ticks > 25 {
                self.crashed = true;
            }
        } else {
            self.skid_ticks = 0;
        }
        let u = if self.drag < self.drive {
            self.drive - self.drag
        } else if self.drive < 1 && self.speed >= 1 {
            self.drive
        } else {
            0
        };
        let demand = u.abs() + ((self.speed / 2048) * (self.lean >> 8)).abs();
        let source_term;
        if grip < demand && self.speed >= 0 {
            source_flags |= if grip < u.abs() {
                if u < 1 { 0x14 } else { 0xc }
            } else {
                4
            };
            if surface > 1 {
                self.lean = (t.curve[surface + 4 * ((self.distance_q8 as usize >> 8) & 7)]
                    * self.lean
                    / 256)
                    .clamp(-0x2e00, 0x2e00);
            }
            let damp = (self.lean - self.previous_lean) * t.damp[surface] / 256;
            source_term = t.gain[surface] * self.lean / 256 + damp;
            self.lateral_force = source_term >> 8;
            self.skid = (demand as u8).wrapping_sub(grip as u8);
            if self.skid as i8 > 40 {
                self.speed = (self.speed - t.skid[surface]).max(0);
            }
        } else {
            self.skid = 0;
            source_term = self.lean;
            self.lateral_force = self.lean >> 8;
        }
        if u < 0 {
            self.lateral_force = if s.ai {
                (s.brake_steer * source_term / 256) >> 8
            } else {
                s.brake_steer * self.lateral_force / 256
            };
        }
        if self.source_motion_flags.is_some() {
            self.source_motion_flags = Some(source_flags);
        }
        if s.ai {
            let hv = if self.speed < 4000 {
                self.lean / 2
            } else if self.speed > 8000 {
                self.lean + self.lean / 8
            } else {
                self.lean
            };
            self.ai_roll = Some(source_roll(t.contact, hv / 256));
        }
    }
    pub(crate) fn vertical(&mut self, s: &BikeSpec, t: &Tables, g: Ground) {
        self.vertical_profile(s, t, g, VerticalKind::Legacy);
    }
    pub(crate) fn vertical_profile(
        &mut self,
        s: &BikeSpec,
        t: &Tables,
        g: Ground,
        kind: VerticalKind,
    ) {
        let ground_move = (g.height - self.previous_ground) * 256 + g.movement;
        let above = (i64::from(self.vertical_speed) - i64::from(ground_move)
            + i64::from(self.height))
        .clamp(-1_000_000_000, 1_000_000_000) as i32;
        let old_height = self.height;
        let mut height = (i64::from(above) * 100 / 99).clamp(-100_000_000, 100_000_000) as i32;
        let vy = self.gravity.saturating_add(self.vertical_speed);
        self.vertical_speed = vy;
        let threshold = s.suspension;
        self.landed = false;
        if threshold < above {
            if threshold != old_height && old_height <= threshold {
                self.vertical_speed = vy.min(s.gravity * s.launch_cap);
            }
            self.contact = 0;
            self.gravity = -s.gravity;
        } else if threshold < old_height && vy < s.landing {
            if kind != VerticalKind::Rider {
                self.lean = ((((i64::from(vy).abs() * i64::from(self.lean)) >> 12) + 1) as i32)
                    .clamp(-0x5c00, 0x5c00);
            }
            let rebound = (-(i64::from(s.rebound) * i64::from(vy) / 256))
                .clamp(-1_000_000_000, 1_000_000_000) as i32;
            let slope = if kind != VerticalKind::Legacy && self.speed < 0 {
                -5 - g.rise
            } else {
                g.rise + 5
            };
            self.vertical_speed = slope * (self.speed >> 7) + rebound;
            self.gravity = -s.gravity * 2;
            self.pitch = -4096;
            self.contact = t.contact;
            height = 0;
            self.landed = true;
        } else {
            if kind == VerticalKind::BikeCurrent && threshold < old_height {
                self.skid = 1;
            }
            self.vertical_speed = ground_move;
            self.contact =
                (((g.rise - self.previous_rise) * self.speed / 256) / 8 + t.contact).max(0);
            height = 0;
        }
        self.height = height;
        self.pitch = (self.pitch + (height - self.pitch) / 64).clamp(-8192, 8192);
        self.airborne = self.contact == 0;
        self.previous_above = old_height;
        self.previous_ground = g.height;
    }
}

fn source_roll(contact: i32, lateral: i32) -> i32 {
    let angle = crate::riding::atan2(contact, lateral) * 65536;
    if angle > 0x800000 {
        angle - 0x1000000
    } else {
        angle
    }
}
