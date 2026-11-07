//! Source type-0 building boundary response. Physical boundaries are independent
//! of the extruded display panels and their visible end textures.
use crate::collision::Motion;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<Surface>,
    pub path: u32,
    pub building: bool,
    pub sample: u32,
    pub road: [i32; 2],
    pub outer: [i32; 2],
    pub selectors: [u8; 2],
    pub flags: u8,
}
/// Native sub-slice boundary and ground geometry, independent of display panels.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    pub kind: u8,
    pub center: i32,
    pub base: [i32; 2],
    pub top: [i32; 2],
    pub height: [i32; 2],
    pub side_height: [i32; 2],
    pub points: [[[i32; 2]; 3]; 2],
    pub event_type: u8,
    pub rail_offset: i8,
    pub rail_flags: u8,
    pub drawn_rail_height: Option<i32>,
}
impl Surface {
    pub fn validate(&self) -> bool {
        matches!(self.kind, 0 | 1 | 3 | 4 | 255)
            && self.center.unsigned_abs() <= 1_000_000
            && self
                .base
                .iter()
                .chain(&self.top)
                .chain(&self.height)
                .chain(&self.side_height)
                .chain(self.points.iter().flatten().flatten())
                .all(|value| value.unsigned_abs() <= 1_000_000)
            && self
                .drawn_rail_height
                .is_none_or(|height| (0..=1_000_000).contains(&height))
    }
}
impl Boundary {
    pub fn translate(&mut self, offset: i32) {
        for edge in self.road.iter_mut().chain(&mut self.outer) {
            *edge += offset;
        }
        if let Some(surface) = &mut self.surface {
            surface.center += offset;
            for edge in surface.base.iter_mut().chain(&mut surface.top) {
                *edge += offset;
            }
        }
    }
}

/// Exact integer terrain-height query used after the boundary response.
pub fn ground_height(
    current: &Boundary,
    next: &Boundary,
    lateral: i32,
    fraction: u8,
) -> crate::Result<i32> {
    let Some(surface) = &current.surface else {
        return Ok(0);
    };
    let ns = next.surface.as_ref().unwrap_or(surface);
    let offset = surface.center;
    let next_center = ns.center.wrapping_sub(offset);
    let x = lateral.wrapping_sub(offset.wrapping_mul(256)) >> 8;
    let left = current.road[0] - offset;
    let right = current.road[1] - offset;
    if left <= x && x <= right {
        return Ok(0);
    }
    let add = i32::wrapping_add;
    let sub = i32::wrapping_sub;
    let mul = i32::wrapping_mul;
    let blend = |a, b| add(a, mul(sub(b, a), i32::from(fraction)) / 256);
    if surface.kind == 1 {
        let base = [surface.base[0] - offset, surface.base[1] - offset];
        let top = [surface.top[0] - offset, surface.top[1] - offset];
        if base[1] < x {
            let height = surface.height[1];
            if height == 0 {
                return Ok(0);
            }
            let span = sub(top[1], base[1]);
            return Ok(
                if mul(height.wrapping_abs(), 2) < span && sub(x, base[1]) < add(top[1], offset) {
                    mul(height, sub(x, base[1])) / span
                } else {
                    height
                },
            );
        }
        if base[0] <= x {
            return Ok(0);
        }
        let height = surface.height[0];
        if height == 0 {
            return Ok(0);
        }
        let span = sub(base[0], top[0]);
        return Ok(
            if mul(height.wrapping_abs(), 2) < span && add(top[0], offset) < sub(base[0], x) {
                mul(sub(add(left, offset), sub(base[0], x)), height) / span
            } else {
                height
            },
        );
    }
    if surface.kind == 3 {
        return Ok(if surface.side_height[0] == 0 {
            16
        } else {
            surface.side_height[usize::from(x > left)]
        });
    }
    let side = usize::from(x > right);
    let shoulder = current.outer[side] - offset;
    let height = surface.side_height[side];
    if surface.kind == 0 {
        if current.flags & 32 != 0 {
            return Ok(0);
        }
        return Ok(
            if (if side == 1 {
                x >= shoulder
            } else {
                x <= shoulder
            }) && current.flags & 24 == 0
                && current.selectors[side] != 255
            {
                999
            } else {
                height
            },
        );
    }
    if surface.kind != 4 {
        return Ok(0);
    }
    let next_edge = next.road[side] - ns.center;
    let edge = blend(
        if side == 1 { right } else { left },
        add(next_center, next_edge),
    );
    if if side == 1 { x <= edge } else { x >= edge } {
        return Ok(0);
    }
    let mut inner = blend(shoulder, add(next_center, next.outer[side] - ns.center));
    if if side == 1 { x < inner } else { x > inner } {
        return Ok(height);
    }
    let mut band = 0;
    let mut outer = blend(
        surface.points[side][0][0],
        add(next_center, ns.points[side][0][0]),
    );
    while band < 2 && (if side == 1 { outer < x } else { x < outer }) {
        band += 1;
        inner = outer;
        outer = blend(
            surface.points[side][band][0],
            add(next_center, ns.points[side][band][0]),
        );
    }
    if if side == 1 { outer < x } else { x < outer } {
        return Ok(blend(surface.points[side][2][1], ns.points[side][2][1]));
    }
    let inner_height = if band == 0 {
        blend(height, ns.side_height[side])
    } else {
        blend(
            surface.points[side][band - 1][1],
            ns.points[side][band - 1][1],
        )
    };
    let outer_height = blend(surface.points[side][band][1], ns.points[side][band][1]);
    let span = sub(outer, inner);
    if span == 0 {
        return Err(crate::Error::Invalid(
            "degenerate source terrain ground segment",
        ));
    }
    Ok(add(
        mul(sub(outer_height, inner_height), sub(x, inner)) / span,
        inner_height,
    ))
}
pub struct Query {
    pub progress: i32,
    pub previous_progress: i32,
    pub lateral: i32,
    pub previous_lateral: i32,
    pub half_width: i32,
    pub half_height: i32,
    pub fallen_height: i32,
    pub suspension: i32,
    pub height: i32,
    pub contact: i32,
    pub category: u32,
    pub player: bool,
    pub owner_crashed: bool,
}
pub struct Neighbors<'a> {
    pub current: &'a Boundary,
    pub next: &'a Boundary,
    pub previous: &'a Boundary,
    pub previous_next: &'a Boundary,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contact {
    pub hit: bool,
    pub lateral: i32,
    pub restore_progress: bool,
    pub crash: bool,
    pub visual_heading: Option<i32>,
    pub rail_crash: bool,
    pub crash_lateral: Option<i32>,
}

pub fn resolve_surface(neighbors: Neighbors<'_>, query: &Query, motion: &mut Motion) -> Contact {
    let Neighbors {
        current,
        next,
        previous,
        previous_next,
    } = neighbors;
    let Some(surface) = &current.surface else {
        return resolve_actor(
            current,
            previous,
            query.progress,
            query.lateral,
            query.half_width,
            motion,
            query.category,
        );
    };
    let mut result = Contact {
        lateral: query.lateral,
        ..Default::default()
    };
    let hi = query.lateral.wrapping_add(query.half_width) >> 8;
    let lo = query.lateral.wrapping_sub(query.half_width) >> 8;
    if current.road[0] < lo && hi < current.road[1] {
        return result;
    }
    let mut hit = 0;
    let mut angled = false;
    let mut general = true;
    let rail_height = if surface.rail_flags & 252 != 0 {
        i32::from(surface.rail_flags & 252) << 8
    } else {
        64
    };
    let blend = |a: i32, b: i32, fraction: i32| {
        a.wrapping_add(b.wrapping_sub(a).wrapping_mul(fraction) / 256)
    };
    if surface.event_type == 5
        && (query.category == 4 || (query.category == 8 && query.owner_crashed))
        && rail_height > query.height
    {
        let fraction = query.progress & 255;
        let old_fraction = query.previous_progress & 255;
        if surface.rail_flags & 1 != 0 {
            let x =
                blend(current.outer[0], next.outer[0], fraction) - i32::from(surface.rail_offset);
            if lo < x && x < hi {
                hit = 1;
                let old_offset = previous
                    .surface
                    .as_ref()
                    .map_or(0, |s| i32::from(s.rail_offset));
                let px =
                    blend(previous.outer[0], previous_next.outer[0], old_fraction) - old_offset;
                result.lateral = if query.previous_lateral < px * 256 {
                    (x - 4) * 256 - query.half_width
                } else {
                    (x + 4) * 256 + query.half_width
                };
            }
        }
        if hit == 0 && surface.rail_flags & 2 != 0 {
            let x =
                blend(current.outer[1], next.outer[1], fraction) + i32::from(surface.rail_offset);
            if lo < x && x < hi {
                hit = 1;
                let old_offset = previous
                    .surface
                    .as_ref()
                    .map_or(0, |s| i32::from(s.rail_offset));
                let px = blend(previous.outer[1], previous_next.outer[1], old_fraction);
                result.lateral = if (px + old_offset) * 256 < query.previous_lateral {
                    (x + 4) * 256 + query.half_width
                } else {
                    (x - 4) * 256 - query.half_width
                };
            }
        }
        if hit != 0 {
            general = false;
        }
        if hit != 0 && query.contact == 0 && rail_height < query.half_height + query.height {
            if query.category == 4 && !motion.crashed {
                hit = 3;
            } else if query.category == 8 {
                hit = 0;
            }
        }
    }
    if general {
        match surface.kind {
            0 => {
                if current.flags & 0x38 != 0 {
                    return result;
                }
                let widen = (((query.progress as u32 & 255) * 250 / 3) >> 8) as i32;
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
                    result.lateral = right * 256 - query.half_width;
                    (1, hi - right)
                } else if lo < left {
                    result.lateral = left * 256 + query.half_width;
                    (0, left - lo)
                } else {
                    return result;
                };
                hit = 1;
                if ((previous.flags & 0x18 != 0)
                    || (current.selectors[side] != 255 && previous.selectors[side] == 255))
                    && ((query.half_width + query.half_width / 2) >> 8) < depth
                {
                    angled = true;
                    if depth < (query.half_width * 2) >> 8 {
                        hit = 2;
                    }
                }
            }
            1 => {
                if query.player {
                    if hi <= surface.base[1] {
                        let mut left = surface.base[0];
                        if lo < left {
                            if surface.height[0] < 1 {
                                left = surface.top[0];
                            }
                            if lo < left {
                                hit = 1;
                                result.lateral = query.half_width + left * 256;
                            }
                        }
                    } else if surface.height[1] < 1 {
                        if hi > surface.top[1] {
                            hit = 1;
                            result.lateral = surface.top[1] * 256 - query.half_width;
                        }
                    } else {
                        hit = 1;
                        result.lateral = surface.base[1] * 256 - query.half_width;
                    }
                } else if current.outer[1] < hi {
                    hit = 1;
                    result.lateral = current.outer[1] * 256 - query.half_width;
                } else if lo < current.outer[0] {
                    hit = 1;
                    result.lateral = query.half_width + current.outer[0] * 256;
                }
            }
            3 => {
                let depth = if current.outer[1] < hi {
                    result.lateral = current.outer[1] * 256 - query.half_width;
                    hi - current.outer[1]
                } else if lo < current.outer[0] {
                    result.lateral = query.half_width + current.outer[0] * 256;
                    current.outer[0] - lo
                } else {
                    return result;
                };
                hit = 1;
                if previous.flags & 0x40 == 0 && ((query.half_width / 2) >> 8) < depth {
                    angled = true;
                    if depth < (query.half_width / 2 + query.half_width) >> 8 {
                        hit = 2;
                    }
                }
            }
            _ => return result,
        }
    }
    if hit == 0 {
        result.lateral = query.lateral;
        return result;
    }
    result.hit = true;
    if angled {
        if motion.speed.abs() < 2500 {
            angled = false;
        } else if hit != 2 {
            motion.speed = 0;
            result.restore_progress = true;
        }
    } else {
        if hit < 3 {
            motion.lateral = motion.lateral.wrapping_mul(-100) / 256;
        } else if hit == 3 && query.category == 4 {
            result.rail_crash = true;
            result.crash_lateral = Some(motion.lateral);
            if query.fallen_height + query.height + query.suspension < rail_height {
                motion.lateral = motion.lateral.wrapping_mul(-100) / 256;
            } else {
                result.lateral = query.lateral;
            }
        }
        if query.category == 4
            && ((motion.lateral > 0 && motion.lean > 0) || (motion.lateral < 0 && motion.lean < 0))
        {
            motion.lean = 0;
        }
        if query.category == 8 {
            result.visual_heading = Some(if motion.speed >= 0 { 0 } else { 0x800000 });
        } else if query.category == 4 {
            result.visual_heading = Some(0);
        }
    }
    result.crash = query.category == 4
        && (result.rail_crash || angled || (motion.lateral.abs() > 2499 && hit != 3));
    result
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
        } else if category == 4 {
            result.visual_heading = Some(0);
        }
    }
    result.crash = category == 4 && (angled || motion.lateral.abs() > 2499);
    result
}
