//! Source-compatible scalar rules, separately tested against native APIs.
use serde::{Deserialize, Serialize};
pub fn live_interval(
    slice: bool,
    position: i32,
    camera: i32,
    speed: i32,
    tick: i32,
    player: bool,
) -> u16 {
    if !slice {
        return 30;
    }
    let d = position.wrapping_sub(camera) >> 8;
    let mut v = if d < -2 {
        ((-2i32).wrapping_sub(d) >> 2) + 1
    } else if d >= 4 {
        ((d + 1) >> 2) + 1
    } else {
        1
    };
    if tick < 300 && speed < 0x1800 && !player {
        v += if speed < 0x1000 { 2 } else { 1 };
        if speed < 0x800 {
            v += 2;
        }
    }
    v.min(8) as u16
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Timers {
    pub cruise: i32,
    pub rate: i32,
    pub reload: i32,
    pub oncoming: i32,
    pub behind: i32,
    pub ahead: i32,
    pub density: i8,
}
impl Default for Timers {
    fn default() -> Self {
        let mut v = Self {
            cruise: 25,
            rate: 2500,
            reload: 0,
            oncoming: 0,
            behind: 0,
            ahead: 0,
            density: 6,
        };
        v.density(6);
        v.ahead = -v.reload;
        v
    }
}
impl Timers {
    pub fn density(&mut self, d: i8) {
        if d < 0 {
            self.cruise = -i32::from(d);
            self.rate = self.cruise * 100;
        } else {
            self.density = if d == 0 { 6 } else { d };
            self.reload = self.rate * i32::from(self.density) * 60 / 256;
            self.oncoming = self.reload;
        }
    }
    pub fn advance(&mut self, delta: i32) -> [bool; 3] {
        let rate = self.rate / 256;
        self.oncoming -= rate + delta;
        let oncoming = self.oncoming < 0;
        if oncoming {
            self.oncoming += self.reload;
        }
        self.behind += rate - delta;
        let behind = self.behind < 0;
        if behind {
            self.behind += self.reload;
        } else {
            self.behind = self.behind.min(self.reload * 2);
        }
        self.ahead += rate - delta;
        let ahead = self.ahead > 0;
        if ahead {
            self.ahead -= self.reload;
        } else {
            self.ahead = self.ahead.max(-self.reload * 2);
        }
        [oncoming, behind, ahead]
    }
}
/// Local offline source predicate; caller establishes same-road proximity and eligibility.
pub fn arrest(
    crash: u8,
    speed: i32,
    cop_speed: i32,
    position: i32,
    cop_position: i32,
    found: bool,
    cop_crashed: bool,
) -> bool {
    let magnitude = |v: i32| if v < 0 { v.wrapping_neg() } else { v };
    found
        && !cop_crashed
        && (if crash == 2 {
            magnitude(speed) < 1800
        } else {
            speed < 300
        })
        && magnitude(cop_position.wrapping_sub(position)) < 512
        && magnitude(cop_speed.wrapping_sub(speed)) < 5000
}
pub fn speed_from_pace(pace: i32) -> i32 {
    let (mut low, mut high) = (0, 30001);
    while low + 1 < high {
        let middle = (low + high) / 2;
        if (middle >> 12) + (middle >> 7) + (middle >> 9) <= pace {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

pub fn vehicle_choice(masks: &[u32], kind: u8, random: u32) -> Option<u8> {
    let filter = if kind & 1 != 0 {
        0x40000
    } else if kind == 2 {
        0x80000
    } else {
        0x100000
    };
    let accepted: Vec<_> = masks
        .iter()
        .enumerate()
        .take(masks.len().saturating_sub(1))
        .filter(|(_, m)| **m & 0x1c0000 == 0 || **m & filter != 0)
        .map(|(id, _)| id as u8)
        .collect();
    if accepted.is_empty() {
        None
    } else {
        Some(accepted[random as usize % accepted.len()])
    }
}
