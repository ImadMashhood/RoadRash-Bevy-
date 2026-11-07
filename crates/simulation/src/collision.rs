//! Integer collision transactions in source units. No ECS or rendering dependencies.
//! Axis order is progress, lateral, base height; heights use box centers in tests.
//! The native comparison harness, rather than visual overlap, is the acceptance oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    pub position: [i32; 3],
    pub previous: [i32; 3],
    pub half: [i32; 3],
    pub velocity: [i32; 3],
    pub speed: i32,
    pub mass: i32,
    pub category: u32,
    pub flags: u8,
    pub impulse: [i32; 3],
}
impl Body {
    pub fn overlaps(&self, other: &Self) -> bool {
        (0..3).all(|axis| {
            self.center_gap(other, axis, false).abs()
                <= i64::from(self.half[axis]) + i64::from(other.half[axis])
        })
    }
    fn center_gap(&self, other: &Self, axis: usize, previous: bool) -> i64 {
        let a = if previous {
            self.previous
        } else {
            self.position
        };
        let b = if previous {
            other.previous
        } else {
            other.position
        };
        i64::from(a[axis]) - i64::from(b[axis])
            + if axis == 2 {
                i64::from(self.half[2]) - i64::from(other.half[2])
            } else {
                0
            }
    }
    fn depth(&self, other: &Self, axis: usize) -> i32 {
        (i64::from(self.half[axis]) + i64::from(other.half[axis])
            - self.center_gap(other, axis, false).abs()) as i32
            + if axis == 0 { 8 } else { 2048 }
    }
}
// imul is 32-bit before the signed division, including static-object mass.
fn fraction(value: i32, mass: i32, total: i32) -> i32 {
    value.wrapping_mul(mass) / total
}
fn impulse(a: &Body, b: &Body, axis: usize) -> i32 {
    let delta = b.velocity[axis].wrapping_sub(a.velocity[axis]);
    if b.category == 1 {
        delta.wrapping_mul(b.mass.wrapping_mul(2) / (a.mass + b.mass))
    } else {
        delta.wrapping_mul(b.mass).wrapping_mul(2) / (a.mass + b.mass)
    }
}
fn stack(a: &mut Body, b: &mut Body) -> bool {
    let raised = if a.half[2] < b.half[2] / 2 && a.position[2] == 0 {
        Some(false)
    } else if b.half[2] < a.half[2] / 2 && b.position[2] == 0 {
        Some(true)
    } else {
        None
    };
    if let Some(first) = raised {
        let up = a.depth(b, 2) + 3;
        if first {
            a.position[2] += up;
        } else {
            b.position[2] += up;
        }
        true
    } else {
        false
    }
}
fn displace(a: &mut Body, b: &mut Body, axis: usize, direction: i32, first: bool, second: bool) {
    let depth = a.depth(b, axis);
    let total = a.mass + b.mass;
    if first {
        a.position[axis] += direction * (fraction(depth, b.mass, total) + 3);
    }
    if second {
        b.position[axis] -= direction * (fraction(depth, a.mass, total) + 3);
    }
    let bits = [[1, 2], [4, 8], [32, 16]][axis];
    a.flags |= bits[usize::from(direction < 0)];
    b.flags |= bits[usize::from(direction >= 0)];
    if axis == 2 {
        if direction > 0 {
            b.flags &= !64;
            if b.position[2] < 0 {
                a.position[2] -= b.position[2];
                b.position[2] = 0;
            }
        } else {
            a.flags &= !64;
            if a.position[2] < 0 {
                b.position[2] -= a.position[2];
                a.position[2] = 0;
            }
        }
    }
}
/// Separate a confirmed pair. Callers perform road/ownership/broadphase checks.
/// Positive masses and bounded validated source coordinates are required.
pub fn separate(a: &mut Body, b: &mut Body) {
    assert!(a.mass > 0 && b.mass > 0 && a.mass <= 0x1000000 && b.mass <= 0x1000000);
    a.flags = 0;
    b.flags = 0;
    a.impulse = [0; 3];
    b.impulse = [0; 3];
    let relative = a.velocity[0] - b.velocity[0];
    let fast = ((relative >> 9) + (relative >> 12) + (relative >> 7)).abs() >= 20;
    for axis in 0..3 {
        if a.center_gap(b, axis, true).abs() <= i64::from(a.half[axis] + b.half[axis]) {
            continue;
        }
        let direction = if b.previous[axis] < a.previous[axis] {
            1
        } else {
            -1
        };
        if axis == 0 {
            let contained = |inner: &Body, outer: &Body| {
                outer.position[1] - outer.half[1] < inner.position[1] - inner.half[1] / 2
                    && inner.position[1] + inner.half[1] / 2 < outer.position[1] + outer.half[1]
                    && outer.half[2] > 8192
            };
            if contained(a, b) {
                a.flags |= 64;
            }
            if contained(b, a) {
                b.flags |= 64;
            }
            let stacked = stack(a, b);
            // Source bike forward movement is suppressed only for the trailing
            // object at a large relative speed. Flags still classify the hit.
            displace(
                a,
                b,
                axis,
                direction,
                !stacked && (direction > 0 || a.category != 4 || !fast),
                !stacked && (direction < 0 || a.category != 4 || !fast),
            );
        } else {
            displace(a, b, axis, direction, true, true);
        }
        let value = impulse(a, b, axis);
        a.impulse[axis] = value;
        b.impulse[axis] = -value;
    }
    if a.flags == 0 && !stack(a, b) {
        let forward = impulse(a, b, 0);
        let lateral = impulse(a, b, 1);
        a.impulse[0] = forward;
        b.impulse[0] = -forward;
        a.impulse[1] = lateral;
        b.impulse[1] = -lateral;
        if (forward / 4).abs() < lateral.abs() || a.velocity[1] <= a.velocity[0] {
            // Contained bikes rely on the response's small lateral nudge.
            let depth = a.depth(b, 1);
            let total = a.mass + b.mass;
            if a.category != 4 {
                let sign = if lateral < 0 { -1 } else { 1 };
                a.position[1] += sign * (fraction(depth, b.mass, total) + 3);
                a.flags |= if sign < 0 { 8 } else { 4 };
            }
            if b.category != 4 {
                let sign = if -lateral < 1 { -1 } else { 1 };
                b.position[1] += sign * (fraction(depth, a.mass, total) + 3);
                b.flags |= if sign < 0 { 8 } else { 4 };
            }
        } else {
            let depth = a.depth(b, 0);
            let total = a.mass + b.mass;
            let sign_a = if forward < 0 { -1 } else { 1 };
            let sign_b = if -forward < 1 { -1 } else { 1 };
            a.flags |= if sign_a < 0 { 2 } else { 1 };
            b.flags |= if sign_b < 0 { 2 } else { 1 };
            if sign_a > 0 || a.category != 4 || !fast {
                a.position[0] += sign_a * (fraction(depth, b.mass, total) + 3);
            }
            if sign_b > 0 || b.category != 4 || !fast {
                b.position[0] += sign_b * (fraction(depth, a.mass, total) + 3);
            }
        }
    }
    a.previous = a.position;
    b.previous = b.position;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    pub speed: i32,
    pub lateral: i32,
    pub vertical: i32,
    pub lean: i32,
    pub steering_gain: i32,
    pub elasticity: i32,
    pub crashed: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Damage {
    pub reason: u8,
    pub stamina: u16,
    pub bike: u16,
    pub forced: bool,
}
/// Bike response to another bike. `damage` owns stamina, condition and linked
/// crash entry; it updates the transaction body/motion before returning down.
/// Sounds/presentation randomness do not enter this physical transaction.
fn respond_solid(
    body: &mut Body,
    other: &Body,
    motion: &mut Motion,
    mut damage: impl FnMut(&mut Body, &mut Motion, Damage) -> bool,
) {
    let static_object = other.category == 1;
    let raw = body.impulse;
    let full = raw.iter().map(|v| v.abs()).sum::<i32>();
    let mut target_rate = full >> 10;
    let mut class = 6;
    let mut down = false;
    if motion.crashed {
        motion.vertical += 336;
    } else {
        class = if other.half[2] < 8192 {
            0
        } else if body.flags & 64 == 0 {
            3
        } else if other.half[2] < 10240 && other.category & 16 == 0 {
            1
        } else {
            2
        };
        body.impulse = raw.map(|v| (i64::from(v) * i64::from(motion.elasticity) / 256) as i32);
        let old_rate = motion.lateral;
        if class != 0 {
            let amount = if static_object {
                full >> 10
            } else {
                full >> 14
            };
            down = damage(
                body,
                motion,
                Damage {
                    reason: class,
                    stamina: if static_object {
                        amount.min(65535) as u16
                    } else {
                        0
                    },
                    bike: amount.min(65535) as u16,
                    forced: false,
                },
            );
            if class == 1 || class == 2 {
                if down {
                    if class == 2 {
                        motion.speed += body.impulse[0];
                    }
                } else if motion.speed - other.velocity[0] < 6001 {
                    motion.speed += body.impulse[0];
                } else {
                    damage(
                        body,
                        motion,
                        Damage {
                            reason: class,
                            stamina: 0,
                            bike: 0,
                            forced: true,
                        },
                    );
                }
            } else if down {
                motion.vertical += body.impulse[0].abs() / 256;
            } else {
                let shove = body.impulse[0].abs();
                motion.lateral = if other.position[1] < body.position[1] {
                    motion.lateral.max(0) + shove / 2
                } else {
                    motion.lateral.min(0) - shove / 2
                };
                motion.speed = (motion.speed - shove / 4).max(0);
            }
        } else {
            if motion.speed < 300 && body.flags == 32 {
                body.position[0] += 2;
                class = 6;
            } else if motion.speed < 500 {
                class = 6;
            }
            if class == 0 {
                body.position[2] += other.half[2] / 2;
                let product = other.half[2]
                    .wrapping_mul(body.impulse[0].abs())
                    .wrapping_mul(2);
                motion.vertical += (product / 16384 / 16) * 4;
            }
        }
        target_rate = old_rate;
    }
    if class != 0 {
        let step = (body.half[1] + other.half[1]) / 16;
        body.position[1] += if other.position[1] < body.position[1] {
            step
        } else {
            -step
        };
    }
    if !down && target_rate != motion.lateral {
        let change = motion
            .steering_gain
            .wrapping_mul(motion.lateral - target_rate)
            .wrapping_mul(2)
            / 256;
        motion.lean = motion.lean.wrapping_add(change).clamp(-11776, 11776);
    }
    motion.speed = motion.speed.clamp(-1000, 30000);
    motion.vertical = if static_object {
        motion.vertical.min(1536)
    } else {
        motion.vertical.clamp(-10000, 1536)
    };
    body.speed = motion.speed;
    body.flags = 0;
    body.impulse = [0; 3];
}

pub fn respond_bike(
    body: &mut Body,
    other: &Body,
    motion: &mut Motion,
    damage: impl FnMut(&mut Body, &mut Motion, Damage) -> bool,
) {
    respond_solid(body, other, motion, damage)
}
pub fn respond_static(
    body: &mut Body,
    other: &Body,
    motion: &mut Motion,
    damage: impl FnMut(&mut Body, &mut Motion, Damage) -> bool,
) {
    let mut object = *other;
    if !motion.crashed {
        object.half[2] /= 2;
    }
    respond_solid(body, &object, motion, damage)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Width, length and half height, matching the source dimensions table.
    pub mounted: [i32; 3],
    pub fallen: [i32; 3],
    pub mass: i32,
    pub elasticity: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rider: Option<RiderProfile>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RiderProfile {
    pub dimensions: [i32; 3],
    pub mass: i32,
    pub elasticity: i32,
}
impl Profile {
    pub fn from_exe(bytes: &[u8]) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let pe = rr_formats::pe::SupportedExe::read(bytes)?;
        let read = |va| -> std::result::Result<i32, Box<dyn std::error::Error>> {
            Ok(rr_formats::binary::u32le(pe.read_va(va, 4)?, 0)? as i32)
        };
        let mut mounted = [0; 3];
        let mut fallen = [0; 3];
        for i in 0..3 {
            mounted[i] = read(0x4643e0 + i as u32 * 4)?;
            fallen[i] = read(0x4643f8 + i as u32 * 4)?;
        }
        let profile = Self {
            mounted,
            fallen,
            mass: read(0x463ba8 + 21 * 4)?,
            elasticity: read(0x463ba8 + 20 * 4)?,
            rider: Some(RiderProfile {
                dimensions: [
                    read(0x463c10 + 17 * 4)?,
                    read(0x463c10 + 18 * 4)?,
                    read(0x463c10 + 19 * 4)?,
                ],
                mass: read(0x463c10 + 21 * 4)?,
                elasticity: read(0x463c10 + 20 * 4)?,
            }),
        };
        profile.validate()?;
        Ok(profile)
    }
    pub fn validate(&self) -> crate::Result<()> {
        if let Some(rider) = &self.rider
            && (rider.dimensions.iter().any(|v| !(1..=65536).contains(v))
                || !(1..=0x1000000).contains(&rider.mass)
                || !(0..=1024).contains(&rider.elasticity))
        {
            return Err(crate::Error::Invalid("detached collider profile"));
        }
        if !(1..=0x1000000).contains(&self.mass)
            || !(0..=1024).contains(&self.elasticity)
            || self
                .mounted
                .iter()
                .chain(self.fallen.iter())
                .any(|v| !(1..=65536).contains(v))
        {
            return Err(crate::Error::Invalid("collision profile"));
        }
        Ok(())
    }
}

/// Bike-side response to a car. The car's own callback is a separate transaction.
pub fn respond_car(
    body: &mut Body,
    other: &Body,
    motion: &mut Motion,
    mut damage: impl FnMut(&mut Body, &mut Motion, Damage) -> bool,
) {
    let crashed = motion.crashed;
    if crashed {
        motion.vertical += 336;
    }
    let full = body.impulse.iter().map(|v| v.abs()).sum::<i32>();
    let magnitude = full >> 10;
    let contained = body.flags & 64 != 0;
    let target_rate = if crashed { magnitude } else { motion.lateral };
    body.impulse = body
        .impulse
        .map(|v| (i64::from(v) * i64::from(motion.elasticity) / 256) as i32);
    let shove = body.impulse[0].abs();
    let class = if contained { 2 } else { 3 };
    let down = !crashed
        && damage(
            body,
            motion,
            Damage {
                reason: class,
                stamina: magnitude.min(65535) as u16,
                bike: magnitude.min(65535) as u16,
                forced: false,
            },
        );
    if !contained {
        if down {
            motion.vertical += shove / 256;
        } else if !crashed {
            motion.lateral = if other.position[1] < body.position[1] {
                motion.lateral.max(0) + shove / 2
            } else {
                motion.lateral.min(0) - shove / 2
            };
            motion.speed = (motion.speed - shove / 4).max(0);
        }
    } else if down || motion.speed - other.speed < 6001 {
        motion.speed += body.impulse[0];
    } else {
        damage(
            body,
            motion,
            Damage {
                reason: 2,
                stamina: 0,
                bike: 0,
                forced: true,
            },
        );
    }
    let push = (body.half[1] + 16384) / 16;
    body.position[1] += if other.position[1] < body.position[1] {
        push
    } else {
        -push
    };
    motion.speed = motion.speed.clamp(-1000, 30000);
    motion.vertical = motion.vertical.clamp(-10000, 1536);
    if !down && target_rate != motion.lateral {
        let kick =
            i64::from(motion.steering_gain) * i64::from(motion.lateral - target_rate) * 2 / 256;
        motion.lean = (i64::from(motion.lean) + kick).clamp(-11776, 11776) as i32;
    }
    body.speed = motion.speed;
    body.flags = 0;
    body.impulse = [0; 3];
}

pub fn respond(
    body: &mut Body,
    other: &Body,
    motion: &mut Motion,
    damage: impl FnMut(&mut Body, &mut Motion, Damage) -> bool,
) {
    match other.category {
        16 => respond_car(body, other, motion, damage),
        1 => respond_static(body, other, motion, damage),
        _ => respond_bike(body, other, motion, damage),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HalfControl {
    pub state: u8,
    pub animation: i8,
    pub queued: Option<i8>,
    pub reset_animation: bool,
}
/// Detached rider response. This never applies mounted stamina/condition damage
/// or replaces the linked body; it may interrupt get-up/walking with another fall.
pub fn respond_half(
    body: &mut Body,
    other: &Body,
    motion: &mut Motion,
    control: &mut HalfControl,
    own_stationary_bike: bool,
    mut random: impl FnMut() -> u32,
) {
    if other.category != 2 && !own_stationary_bike {
        if control.state != 2 || other.speed >= 1700 {
            if other.category & 0x15 != 0 {
                body.position[2] = other.half[2] * 2;
            }
            body.position[2] = body.position[2].min(20480);
            let speed = motion.speed.abs();
            motion.vertical = if speed < 501 {
                0
            } else {
                (motion.vertical / 2).abs() + speed / 16
            };
            motion.speed -= motion.speed / 4;
            if !matches!(control.animation, 33 | 29) {
                control.animation = 28;
                control.queued = Some(28);
                control.reset_animation = true;
            }
            motion.vertical = motion.vertical.min(336);
            if control.state == 2 && other.speed > 1700 {
                motion.speed += other.speed / 4;
                control.state = 1;
                control.animation = 34;
                control.queued = Some(38);
                control.reset_animation = true;
            }
        }
        if motion.vertical < 3000 {
            let step = (body.half[1] + other.half[1]) / 16;
            body.position[1] += if other.position[1] < body.position[1] {
                step
            } else {
                -step
            };
            body.position[0] += (random() % 33) as i32 - 16;
        }
    }
    body.flags = 0;
    body.impulse = [0; 3];
    motion.speed = motion.speed.clamp(-2000, 30000);
    motion.vertical = motion.vertical.clamp(-10000, 10000);
    body.speed = motion.speed;
}
