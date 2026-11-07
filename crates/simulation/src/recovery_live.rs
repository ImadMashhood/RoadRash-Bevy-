//! Fixed-dt composition of the native LIVE half and quiet-bike recovery callbacks.
//! Version1 remains the historical combined NONLIVE transaction.
use crate::{
    Error, Result,
    animation::Player,
    handling::Bike,
    recovery::*,
    riding::{Configuration, atan2},
    topology::Topology,
};

fn crash_state(phase: Phase) -> u8 {
    match phase {
        Phase::Sliding => 1,
        Phase::Mounting => 3,
        _ => 2,
    }
}
fn phase_for(crash: u8, code: i8) -> Phase {
    match crash {
        1 => Phase::Sliding,
        3 => Phase::Mounting,
        _ => match code {
            37 => Phase::Standing,
            38 => Phase::Walking,
            _ => Phase::GettingUp,
        },
    }
}
impl State {
    /// Walk/pickup decisions run before the arrest test, crash phases and body motion.
    /// A returned animation continues on the mounted rider after the body is removed.
    pub(crate) fn prepare_live_recovery(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        walk: Option<Walk>,
        player: bool,
    ) -> Result<(Vec<RecoveryEvent>, Option<Player>)> {
        if matches!(self.phase, Phase::Mounted | Phase::Ended) {
            return Ok((vec![], None));
        }
        if !self.bike_available
            || !topology.valid(self.bike_location)
            || self.body.as_ref().is_none_or(|body| {
                !topology.valid(body.location)
                    || !topology.connected(body.location.path, self.bike_location.path)
            })
        {
            self.terminal(Outcome::Wrecked, bike);
            return Ok((vec![RecoveryEvent::Wrecked], None));
        }
        let mut crash = crash_state(self.phase);
        let mut events = vec![];
        let body = self.body.as_mut().unwrap();
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
                    wrecked = player;
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
                    if player
                        && let Some(walk) = walk
                        && walk != Walk::default()
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
                    body.motion.speed = 1200;
                }
            } else if body.animation.code == 37 {
                let difference = (self.bike_location.progress as u32)
                    .wrapping_sub(body.location.progress as u32);
                let sign = difference >> 31;
                let distance = ((difference ^ sign).wrapping_sub(sign)) as i32;
                if bike.condition == 0 && distance < 80 {
                    if body.animation.loops >= 1 {
                        if !player {
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
            return Ok((events, None));
        }
        if remount {
            bike.crashed = false;
            bike.drive = 0;
            bike.lateral_force = 0;
            bike.lateral_speed = 0;
            bike.lean = 0;
            bike.skid = 0;
            bike.skid_ticks = 0;
            bike.failed = bike.condition == 0;
            body.animation.set(c.recovery_animation(), 0, 0);
            body.animation.queued = Some(0);
            let continued = body.animation.clone();
            self.retained_body_animation = Some(continued.clone());
            self.bike_animation.set(c.recovery_animation(), 0, 0);
            self.bike_animation.queued = Some(0);
            self.bike_animation.selected = None;
            self.phase = Phase::Mounted;
            self.body = None;
            self.elapsed = 0;
            events.push(RecoveryEvent::Remount);
            return Ok((events, Some(continued)));
        }
        self.phase = phase_for(crash, body.animation.code);
        Ok((events, None))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn live_half_motion(
        &mut self,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        tick: u32,
        random: impl FnMut() -> u32,
        body_context: Option<(&Configuration, &Topology)>,
    ) -> Result<Vec<RecoveryEvent>> {
        let (events, ground) =
            self.live_half_motion_horizontal(c, topology, law, tick, random, body_context)?;
        if let Some(ground) = ground {
            let (c, topology) = body_context.unwrap_or((c, topology));
            self.finish_live_half_motion(c, topology, law, ground);
        }
        Ok(events)
    }
    pub(crate) fn finish_live_half_motion(
        &mut self,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        ground: crate::handling::Ground,
    ) {
        if let Some(body) = &mut self.body {
            body.motion.vertical_profile(
                &law.spec(c.spec()),
                &c.tables,
                ground,
                crate::handling::VerticalKind::Rider,
            );
            body.location.height = body.motion.height;
            body.projected = topology.projected(body.location);
        }
    }

    /// Crash animation phases then detached half motion; the quiet bike has not moved yet.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn live_half_motion_horizontal(
        &mut self,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        tick: u32,
        mut random: impl FnMut() -> u32,
        body_context: Option<(&Configuration, &Topology)>,
    ) -> Result<(Vec<RecoveryEvent>, Option<crate::handling::Ground>)> {
        if matches!(self.phase, Phase::Mounted | Phase::Ended) {
            return Ok((vec![], None));
        }
        let mut crash = crash_state(self.phase);
        let before = crash;
        let body = self
            .body
            .as_mut()
            .ok_or(Error::Invalid("missing live crash body"))?;
        if !law.source_angles.is_empty() && body.visual_heading.is_none() {
            body.visual_heading = Some(body.location.heading);
        }
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
            .map_err(|_| Error::Invalid("live rider recovery animation"))?;
        let mut events = vec![];
        if crash != before {
            self.elapsed = 0;
            events.push(RecoveryEvent::GetUp);
        }
        if crash == 2 {
            self.elapsed = self.elapsed.saturating_add(1);
        }
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
        let ground = motion_step_horizontal(
            &mut body.motion,
            &mut body.location,
            heading,
            body_config,
            body_topology,
            law,
            leg,
        )?;
        if let Some(frame) = frame {
            let (cell, roll) = c
                .recovery_animation()
                .select(
                    0,
                    &frame,
                    body.visual_heading.unwrap_or(body.location.heading),
                    0,
                )
                .map_err(|_| Error::Invalid("live rider recovery view"))?;
            body.frame = cell.frame;
            body.mirror = cell.flags & 1 != 0;
            body.view_flags = cell.flags;
            body.roll = roll;
        }
        body.projected = body_topology.projected(body.location);
        body.branch = topology.paths[&body.location.path].branch;
        self.phase = phase_for(crash, body.animation.code);
        Ok((events, Some(ground)))
    }

    /// Quiet bike callback after the half: animation, movement and the source speed-eighth coupling.
    pub(crate) fn live_bike_motion(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        tick: u32,
        random: impl FnMut() -> u32,
    ) -> Result<()> {
        if let Some(ground) =
            self.live_bike_motion_horizontal(bike, c, topology, law, tick, random)?
        {
            self.finish_live_bike_motion(bike, c, topology, law, ground);
        }
        Ok(())
    }

    pub(crate) fn live_bike_motion_horizontal(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        tick: u32,
        mut random: impl FnMut() -> u32,
    ) -> Result<Option<crate::handling::Ground>> {
        if matches!(self.phase, Phase::Mounted | Phase::Ended) {
            return Ok(None);
        }
        if matches!(
            self.phase,
            Phase::Sliding | Phase::GettingUp | Phase::Standing | Phase::Walking
        ) {
            if let Some(frame) = self
                .bike_animation
                .crash_bike(c.recovery_animation(), bike.speed, tick, &mut random)
                .map_err(|_| Error::Invalid("live quiet bike animation"))?
            {
                let (cell, _) = c
                    .recovery_animation()
                    .select(0, &frame, 0, 0)
                    .map_err(|_| Error::Invalid("live quiet bike view"))?;
                self.bike_frame = cell.frame;
                self.bike_mirror = cell.flags & 1 != 0;
            }
        } else {
            self.bike_animation.selected = None;
        }
        let bike_law = BodyLaw {
            source_angles: vec![],
            recovery_version: 2,
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
        bike.skid = 0;
        let ground =
            motion_step_bike_horizontal(bike, &mut self.bike_location, c, topology, &bike_law)?;
        Ok(Some(ground))
    }

    pub(crate) fn finish_live_bike_motion(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        _law: &BodyLaw,
        ground: crate::handling::Ground,
    ) {
        bike.vertical_profile(
            c.spec(),
            &c.tables,
            ground,
            crate::handling::VerticalKind::BikeCurrent,
        );
        self.bike_location.height = bike.height;
        if let Some(body) = &self.body {
            let gap = crate::source_roads::Roads::physical_delta(
                topology,
                self.bike_location,
                body.location,
            )
            .or_else(|| {
                Some(topology.projected(body.location)? - topology.projected(self.bike_location)?)
            });
            if gap.is_some_and(|gap| gap.abs() > 0x500) {
                let own = bike.speed;
                let other = body.motion.speed;
                bike.speed = if other.wrapping_abs() < own.wrapping_abs() {
                    own.wrapping_sub(own >> 3)
                } else {
                    own.wrapping_add(other.wrapping_abs().wrapping_sub(own.wrapping_abs()) >> 3)
                };
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step_live_context(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        walk: Option<Walk>,
        player: bool,
        tick: u32,
        mut random: impl FnMut() -> u32,
        body_context: Option<(&Configuration, &Topology)>,
    ) -> Result<Vec<RecoveryEvent>> {
        let (mut events, continued) =
            self.prepare_live_recovery(bike, c, topology, walk, player)?;
        if continued.is_some() {
            let old = bike.distance_q8;
            let index = (i64::from(c.start_sample) + (old >> 8)).clamp(0, c.ground.len() as i64 - 1)
                as usize;
            let ground = bike.source_physics_motion(
                c.spec(),
                &c.tables,
                c.ground[index],
                c.motion_version,
                |old, new| c.swept_ground(old, new),
            );
            bike.finish_motion(c.spec(), &c.tables, ground, c.motion_version);
            topology.advance(&mut self.bike_location, bike.distance_q8 - old, None)?;
            self.bike_location.lateral = bike.lateral;
            self.bike_location.height = bike.height;
            return Ok(events);
        }
        events.extend(self.live_half_motion(c, topology, law, tick, &mut random, body_context)?);
        self.live_bike_motion(bike, c, topology, law, tick, &mut random)?;
        Ok(events)
    }
}
