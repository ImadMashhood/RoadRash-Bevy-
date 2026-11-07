//! Source-order detached recovery. Historical recordings retain recovery.rs's
//! original transaction; this profile advances think/animation before motion,
//! then performs the walk/pickup decision using the moved body.
use crate::{
    Error, Result,
    handling::Bike,
    recovery::*,
    riding::{Configuration, atan2},
    topology::Topology,
};
impl State {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step_source(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        walk: Option<Walk>,
        tick: u32,
        mut random: impl FnMut() -> u32,
    ) -> Result<Vec<RecoveryEvent>> {
        self.step_source_context(bike, c, topology, law, walk, tick, &mut random, None)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step_source_context(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        walk: Option<Walk>,
        tick: u32,
        mut random: impl FnMut() -> u32,
        body_context: Option<(&Configuration, &Topology)>,
    ) -> Result<Vec<RecoveryEvent>> {
        if matches!(self.phase, Phase::Mounted | Phase::Ended) {
            return Ok(vec![]);
        }
        if !self.bike_available
            || !topology.valid(self.bike_location)
            || self.body.as_ref().is_none_or(|b| {
                !topology.valid(b.location)
                    || !topology.connected(b.location.path, self.bike_location.path)
            })
        {
            self.terminal(Outcome::Wrecked, bike);
            return Ok(vec![RecoveryEvent::Wrecked]);
        }
        let mut crash = match self.phase {
            Phase::Sliding => 1,
            Phase::Mounting => 3,
            _ => 2,
        };
        let mut events = vec![];
        if (crash == 1 || crash == 2)
            && let Some(frame) = self
                .bike_animation
                .crash_bike(c.recovery_animation(), bike.speed, tick, &mut random)
                .map_err(|_| Error::Invalid("source bike recovery animation"))?
        {
            let (cell, _) = c
                .recovery_animation()
                .select(0, &frame, 0, 0)
                .map_err(|_| Error::Invalid("source bike recovery view"))?;
            self.bike_frame = cell.frame;
            self.bike_mirror = cell.flags & 1 != 0;
        }
        let body = self.body.as_mut().unwrap();
        let before = crash;
        let frame = body
            .animation
            .crash_rider(
                c.recovery_animation(),
                &mut crash,
                body.motion.speed,
                body.motion.height,
                tick,
                &mut random,
            )
            .map_err(|_| Error::Invalid("source rider recovery animation"))?;
        if crash != before {
            self.elapsed = 0;
            events.push(RecoveryEvent::GetUp);
        }
        if crash == 2 {
            self.elapsed = self.elapsed.saturating_add(1);
        }
        let bike_law = BodyLaw {
            source_angles: vec![],
            recovery_version: 1,
            motion_version: law.motion_version,
            drag: c.tables.object[9].max(0),
            contact_drag: 95,
            landing: c.spec().landing,
            suspension: c.spec().suspension,
            rebound: c.spec().rebound,
            gravity: c.spec().gravity,
            launch_cap: c.spec().launch_cap,
        };
        bike.drive = 0;
        bike.lateral_force = 0;
        bike.lateral_speed = 0;
        motion_step_bike(bike, &mut self.bike_location, c, topology, &bike_law)?;
        let body = self.body.as_mut().unwrap();
        body.previous_location = Some(body.location);
        let heading = body.location.heading;
        let (body_config, body_topology) = body_context.unwrap_or((c, topology));
        let leg = if body.location.path != self.bike_location.path
            && (body_context.is_none() || body.animation.code == 38)
        {
            topology.leg(body.location.path, self.bike_location.path)
        } else {
            None
        };
        motion_step(
            &mut body.motion,
            &mut body.location,
            heading,
            body_config,
            body_topology,
            law,
            leg,
        )?;
        let mut remount = false;
        let mut wrecked = false;
        if crash == 3 {
            remount = body.animation.code != 41;
        } else if crash == 2 {
            if self.elapsed > 1200 && !self.police {
                if bike.condition == 0 {
                    body.motion.speed = 0;
                    if body.animation.code != 37 {
                        body.animation.set(c.recovery_animation(), 0, 37);
                    }
                    wrecked = true;
                } else {
                    body.animation.set(c.recovery_animation(), 0, 41);
                    body.animation.queued = Some(2);
                    crash = 3;
                }
            }
            if body.animation.code == 38 {
                let forward = self.bike_location.progress - body.location.progress;
                let lateral = bike
                    .lateral
                    .wrapping_sub(body.motion.lateral)
                    .wrapping_sub(6000);
                if body.location.path == self.bike_location.path
                    && forward.abs() <= 9
                    && lateral.abs() < 6001
                {
                    body.motion.speed = 0;
                    if bike.condition != 0 {
                        body.motion.lateral = body.motion.lateral.wrapping_add(6000);
                        body.location.lateral = body.motion.lateral;
                        bike.lateral = body.motion.lateral;
                        self.bike_location.lateral = bike.lateral;
                        body.animation.set(c.recovery_animation(), 0, 41);
                        body.animation.queued = Some(2);
                        crash = 3;
                    } else {
                        body.animation.set(c.recovery_animation(), 0, 37);
                    }
                } else {
                    let leg = if body.location.path == self.bike_location.path {
                        None
                    } else {
                        topology.leg(body.location.path, self.bike_location.path)
                    };
                    let mut toward = leg.map_or(forward, |edge| {
                        if edge.end {
                            topology.paths[&body.location.path].length - body.location.progress + 16
                        } else {
                            -body.location.progress - 16
                        }
                    });
                    let mut side = lateral;
                    let hold = walk == Some(Walk::default());
                    if let Some(walk) = walk
                        && !hold
                    {
                        toward = if (walk.longitudinal < 0 && toward < 0)
                            || (walk.longitudinal > 0 && toward > 0)
                        {
                            i64::from(walk.longitudinal.signum())
                        } else {
                            0
                        };
                        side = i32::from(walk.lateral) * 256;
                        self.elapsed = 0;
                    }
                    let bits = ((atan2(
                        toward.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
                        side / 256,
                    ) & 255) as u32)
                        * 65536;
                    body.location.heading = if bits > 0x800000 {
                        (i64::from(bits) - 0x1000000) as i32
                    } else {
                        bits as i32
                    };
                    body.visual_heading = Some(body.location.heading);
                    body.motion.speed = if hold { 0 } else { 1200 };
                }
            } else if body.animation.code == 37 {
                let difference = (self.bike_location.progress as u32)
                    .wrapping_sub(body.location.progress as u32);
                let sign = difference >> 31;
                let distance = ((difference ^ sign).wrapping_sub(sign)) as i32;
                if bike.condition == 0 && distance < 80 {
                    if body.animation.loops >= 1 {
                        if self.police {
                            bike.condition = c.spec().condition / 4;
                            bike.failed = false;
                        } else {
                            wrecked = true;
                        }
                    }
                } else if body.animation.loops > 1 {
                    body.animation.queued = Some(38);
                }
            }
        }
        if wrecked {
            self.terminal(Outcome::Wrecked, bike);
            events.push(RecoveryEvent::Wrecked);
            return Ok(events);
        }
        if remount {
            bike.crashed = false;
            bike.force = 0;
            bike.drive = 0;
            bike.lateral_force = 0;
            bike.lateral_speed = 0;
            bike.lean = 0;
            bike.skid = 0;
            bike.skid_ticks = 0;
            bike.failed = bike.condition == 0;
            self.bike_animation.set(c.recovery_animation(), 0, 0);
            self.bike_animation.queued = Some(0);
            self.phase = Phase::Mounted;
            self.body = None;
            self.elapsed = 0;
            events.push(RecoveryEvent::Remount);
            return Ok(events);
        }
        self.phase = match crash {
            1 => Phase::Sliding,
            3 => Phase::Mounting,
            _ => match body.animation.code {
                37 => Phase::Standing,
                38 => Phase::Walking,
                _ => Phase::GettingUp,
            },
        };
        if let Some(frame) = frame {
            let (cell, roll) = c
                .recovery_animation()
                .select(
                    0,
                    &frame,
                    body.visual_heading.unwrap_or(body.location.heading),
                    0,
                )
                .map_err(|_| Error::Invalid("source recovery view"))?;
            body.frame = cell.frame;
            body.mirror = cell.flags & 1 != 0;
            body.view_flags = cell.flags;
            body.roll = roll;
        }
        body.location.height = body.motion.height;
        body.projected = body_topology.projected(body.location);
        body.branch = topology.paths[&body.location.path].branch;
        Ok(events)
    }
}
