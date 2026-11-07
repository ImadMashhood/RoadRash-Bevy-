//! Source type-0 building boundary response. Physical boundaries are independent
//! of the extruded display panels and their visible end textures.
use crate::collision::Motion;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    pub path: u32,
    pub building: bool,
    pub sample: u32,
    pub road: [i32; 2],
    pub outer: [i32; 2],
    pub selectors: [u8; 2],
    pub flags: u8,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contact {
    pub hit: bool,
    pub lateral: i32,
    pub restore_progress: bool,
    pub crash: bool,
    pub visual_heading: Option<i32>,
}
pub fn resolve(
    current: &Boundary,
    previous: &Boundary,
    progress: i32,
    lateral: i32,
    half_width: i32,
    motion: &mut Motion,
) -> Contact {
    resolve_actor(current, previous, progress, lateral, half_width, motion, 4)
}
pub fn resolve_actor(
    current: &Boundary,
    previous: &Boundary,
    progress: i32,
    lateral: i32,
    half_width: i32,
    motion: &mut Motion,
    category: u32,
) -> Contact {
    let hi = (lateral + half_width) >> 8;
    let lo = (lateral - half_width) >> 8;
    let mut result = Contact {
        lateral,
        ..Default::default()
    };
    if !current.building {
        return result;
    }
    if (current.road[0] < lo && hi < current.road[1]) || current.flags & 0x38 != 0 {
        return result;
    }
    let widen = (((progress as u32 & 255) * 250 / 3) >> 8) as i32;
    let right = current.outer[1]
        + if current.selectors[1] == 255 {
            widen
        } else {
            0
        };
    let left = current.outer[0]
        - if current.selectors[0] == 255 {
            widen
        } else {
            0
        };
    let (side, depth) = if right < hi {
        result.lateral = right * 256 - half_width;
        (1, hi - right)
    } else if lo < left {
        result.lateral = left * 256 + half_width;
        (0, left - lo)
    } else {
        return result;
    };
    result.hit = true;
    let mut angled = ((previous.flags & 0x18) != 0
        || (current.selectors[side] != 255 && previous.selectors[side] == 255))
        && ((half_width + half_width / 2) >> 8) < depth;
    let shallow = depth < (half_width * 2) >> 8;
    if angled {
        if motion.speed.abs() < 2500 {
            angled = false;
        } else if !shallow {
            motion.speed = 0;
            result.restore_progress = true;
        }
    } else {
        motion.lateral = motion.lateral.wrapping_mul(-100) / 256;
        if category == 4 && motion.lateral.signum() == motion.lean.signum() {
            motion.lean = 0;
        }
        if category == 8 {
            result.visual_heading = Some(if motion.speed >= 0 { 0 } else { 0x800000 });
        }
    }
    result.crash = category == 4 && (angled || motion.lateral.abs() > 2499);
    result
}
