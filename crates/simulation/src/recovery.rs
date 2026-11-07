//! Linked detached-body recovery. Rendering never owns or deletes these entities.
use crate::{
    ActorId, Error, Result,
    animation::Player,
    handling::{Bike, BikeSpec, Tables},
    riding::{Configuration, atan2},
    topology::{Location, Topology},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Mounted,
    Sliding,
    GettingUp,
    Standing,
    Walking,
    Mounting,
    Ended,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Wrecked,
    Finished,
    Arrested,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Assists {
    #[serde(default)]
    pub fast_recovery: bool,
    #[serde(default)]
    pub rescue_unreachable: bool,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Walk {
    pub longitudinal: i8,
    pub lateral: i8,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Impact {
    pub reason: u8,
    pub stamina_damage: u16,
    pub bike_damage: u16,
    pub rider_velocity: Option<[i32; 3]>,
    pub bike_velocity: Option<[i32; 3]>,
}
impl Default for Impact {
    fn default() -> Self {
        Self {
            reason: 1,
            stamina_damage: 0,
            bike_damage: 0,
            rider_velocity: None,
            bike_velocity: None,
        }
    }
}
impl Impact {
    pub fn validate(self) -> Result<()> {
        if self.reason > 6
            || self
                .rider_velocity
                .iter()
                .chain(self.bike_velocity.iter())
                .flatten()
                .any(|v| v.unsigned_abs() > 1_000_000)
        {
            return Err(Error::Invalid("impact reason/velocity"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BodyLaw {
    #[serde(default, skip_serializing_if = "legacy_motion")]
    pub motion_version: u8,
    #[serde(default, skip_serializing_if = "legacy_motion")]
    pub recovery_version: u8,
    pub drag: i32,
    pub contact_drag: i32,
    pub landing: i32,
    pub suspension: i32,
    pub rebound: i32,
    pub gravity: i32,
    pub launch_cap: i32,
}
fn legacy_motion(version: &u8) -> bool {
    *version == 0
}
impl Default for BodyLaw {
    fn default() -> Self {
        Self {
            motion_version: 0,
            recovery_version: 0,
            drag: 130,
            contact_drag: 70,
            landing: -2560,
            suspension: 256,
            rebound: 24,
            gravity: 74,
            launch_cap: 4,
        }
    }
}
impl BodyLaw {
    pub fn from_exe(bytes: &[u8]) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let pe = rr_formats::pe::SupportedExe::read(bytes)?;
        let raw = pe.read_va(0x463c10, 26 * 4)?;
        let w = |i: usize| -> std::result::Result<i32, Box<dyn std::error::Error>> {
            Ok(rr_formats::binary::u32le(raw, i * 4)? as i32)
        };
        let value = Self {
            motion_version: 0,
            recovery_version: 0,
            drag: w(9)?,
            contact_drag: 70,
            landing: w(12)?,
            suspension: w(13)?,
            rebound: w(16)?,
            gravity: w(14)?,
            launch_cap: w(15)?,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        if self.motion_version > 1
            || self.recovery_version > 1
            || !(0..=4096).contains(&self.drag)
            || !(0..=4096).contains(&self.contact_drag)
            || !(-65536..=0).contains(&self.landing)
            || !(0..=65536).contains(&self.suspension)
            || !(0..=1024).contains(&self.rebound)
            || !(1..=1024).contains(&self.gravity)
            || !(0..=16).contains(&self.launch_cap)
        {
            return Err(Error::Invalid("crash body law"));
        }
        Ok(())
    }
    fn spec(&self, base: &BikeSpec) -> BikeSpec {
        let mut result = base.clone();
        result.landing = self.landing;
        result.suspension = self.suspension;
        result.rebound = self.rebound;
        result.gravity = self.gravity;
        result.launch_cap = self.launch_cap;
        result
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Body {
    pub entity: u64,
    pub owner: ActorId,
    pub location: Location,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_location: Option<Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_heading: Option<i32>,
    pub motion: Bike,
    pub animation: Player,
    pub frame: usize,
    pub mirror: bool,
    pub view_flags: u32,
    pub roll: i32,
    pub projected: Option<i64>,
    pub branch: u32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    pub phase: Phase,
    pub body: Option<Body>,
    pub bike_entity: u64,
    pub bike_location: Location,
    pub bike_available: bool,
    pub bike_animation: Player,
    pub bike_frame: usize,
    pub bike_mirror: bool,
    pub elapsed: u32,
    pub incident: u32,
    pub outcome: Option<Outcome>,
    pub assisted: bool,
}
impl State {
    pub fn mounted(owner: ActorId, at: Location) -> Self {
        Self {
            bike_entity: owner.0 * 2,
            bike_location: at,
            bike_available: true,
            ..Self::default()
        }
    }
    pub fn terminal(&mut self, outcome: Outcome, bike: &mut Bike) {
        if self.outcome.is_none_or(|old| old < outcome) {
            self.outcome = Some(outcome);
        }
        self.phase = Phase::Ended;
        bike.speed = 0;
        bike.drive = 0;
        if let Some(body) = &mut self.body {
            body.motion.speed = 0;
            body.motion.lateral_speed = 0;
            body.motion.vertical_speed = 0;
        }
    }
    pub fn begin(
        &mut self,
        owner: ActorId,
        bike: &mut Bike,
        c: &Configuration,
        at: Location,
        impact: Impact,
        topology: &Topology,
    ) -> bool {
        self.begin_profile(owner, bike, c, at, impact, topology, false)
    }
    /// Source crash entry is idempotent while the linked crash body is active.
    pub fn begin_source(
        &mut self,
        owner: ActorId,
        bike: &mut Bike,
        c: &Configuration,
        at: Location,
        impact: Impact,
        topology: &Topology,
    ) -> bool {
        self.begin_profile(owner, bike, c, at, impact, topology, true)
    }
    #[allow(clippy::too_many_arguments)]
    fn begin_profile(
        &mut self,
        owner: ActorId,
        bike: &mut Bike,
        c: &Configuration,
        at: Location,
        impact: Impact,
        topology: &Topology,
        source: bool,
    ) -> bool {
        if self.outcome.is_some() || (source && self.phase != Phase::Mounted) {
            return false;
        }
        let first = self.phase == Phase::Mounted;
        if first {
            let mut motion = bike.clone();
            motion.height = motion.height.saturating_add(0x4b00);
            motion.drive = 0;
            motion.lateral_force = 0;
            motion.gravity = 0;
            motion.lean = 0;
            motion.skid = 0;
            self.body = Some(Body {
                entity: owner.0 * 2 + 1,
                owner,
                location: Location {
                    height: motion.height,
                    ..at
                },
                motion,
                previous_location: None,
                visual_heading: None,
                animation: Player::default(),
                frame: 2,
                mirror: false,
                view_flags: 0,
                roll: 0,
                projected: Some(bike.distance_q8),
                branch: topology.paths.get(&at.path).map_or(0, |p| p.branch),
            });
            bike.height = bike.height.saturating_add(c.spec().suspension);
            self.bike_location = at;
        }
        let body = self.body.as_mut().expect("detached body");
        let rv = impact.rider_velocity.unwrap_or(if impact.reason == 4 {
            [-80, 0, 0]
        } else {
            [-256, 0, 1792]
        });
        let bv = impact.bike_velocity.unwrap_or(if impact.reason == 4 {
            [0, 0, 0]
        } else {
            [0, 0, 1024]
        });
        for (motion, velocity) in [(&mut body.motion, rv), (&mut *bike, bv)] {
            if source {
                motion.speed = motion.speed.wrapping_add(velocity[0]);
                motion.lateral_speed = motion.lateral_speed.wrapping_add(velocity[1]);
                motion.vertical_speed = motion.vertical_speed.wrapping_add(velocity[2]);
            } else {
                motion.speed = motion.speed.saturating_add(velocity[0]).clamp(0, 1_000_000);
                motion.lateral_speed = motion.lateral_speed.saturating_add(velocity[1]);
                motion.vertical_speed = motion.vertical_speed.saturating_add(velocity[2]);
            }
        }
        body.animation.set(
            &c.animation,
            0,
            if impact.reason == 2 {
                29
            } else if impact.reason == 4 || impact.reason == 5 {
                35
            } else {
                33
            },
        );
        body.animation.queued = None;
        body.location.height = body.motion.height;
        bike.crashed = true;
        bike.drive = 0;
        bike.lean = 0;
        self.phase = Phase::Sliding;
        self.elapsed = 0;
        self.incident = self.incident.saturating_add(1);
        self.bike_animation = Player::default();
        self.bike_animation.set(&c.animation, 0, 46);
        self.bike_animation.queued = Some(44);
        first
    }
    pub fn can_mount(&self, topology: &Topology) -> bool {
        let Some(body) = &self.body else {
            return false;
        };
        if !self.bike_available
            || !topology.valid(body.location)
            || !topology.valid(self.bike_location)
            || body.location.path != self.bike_location.path
        {
            return false;
        }
        let forward = self.bike_location.progress - body.location.progress;
        let side = i64::from(self.bike_location.lateral) - i64::from(body.location.lateral) - 6000;
        let desired = atan2(
            forward.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            (side / 256) as i32,
        ) * 65536;
        let angle = (i64::from(desired) - i64::from(body.location.heading)).rem_euclid(0x1000000);
        forward.abs() <= 9 && side.abs() < 6001 && angle.min(0x1000000 - angle) <= 0x400000
    }
    // Inputs are explicit domain dependencies, shared by headless replay and Bevy.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        assists: Assists,
        walk: Option<Walk>,
        tick: u32,
    ) -> Result<Vec<RecoveryEvent>> {
        self.step_with_random(bike, c, topology, law, assists, walk, tick, || 0)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn step_with_random(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        assists: Assists,
        walk: Option<Walk>,
        tick: u32,
        random: impl FnMut() -> u32,
    ) -> Result<Vec<RecoveryEvent>> {
        if law.recovery_version == 1 && !assists.fast_recovery && !assists.rescue_unreachable {
            self.step_source(bike, c, topology, law, walk, tick, random)
        } else {
            self.step_legacy(bike, c, topology, law, assists, walk, tick)
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn step_legacy(
        &mut self,
        bike: &mut Bike,
        c: &Configuration,
        topology: &Topology,
        law: &BodyLaw,
        assists: Assists,
        walk: Option<Walk>,
        tick: u32,
    ) -> Result<Vec<RecoveryEvent>> {
        let mut events = vec![];
        if self.phase == Phase::Mounted || self.phase == Phase::Ended {
            return Ok(events);
        }
        self.elapsed = self.elapsed.saturating_add(1);
        let valid = self.bike_available
            && topology.valid(self.bike_location)
            && self.body.as_ref().is_some_and(|b| {
                topology.valid(b.location)
                    && topology.connected(b.location.path, self.bike_location.path)
            });
        if !valid {
            if assists.rescue_unreachable
                && self.bike_available
                && topology.valid(self.bike_location)
            {
                if let Some(body) = &mut self.body {
                    body.location = Location {
                        lateral: self.bike_location.lateral - 6000,
                        ..self.bike_location
                    };
                    body.motion.distance_q8 = topology
                        .projected(body.location)
                        .unwrap_or(bike.distance_q8);
                    body.motion.lateral = body.location.lateral;
                    body.motion.height = 0;
                    body.motion.speed = 0;
                    self.phase = Phase::Walking;
                    self.assisted = true;
                    events.push(RecoveryEvent::Rescue);
                }
            } else {
                self.terminal(Outcome::Wrecked, bike);
                return Ok(vec![RecoveryEvent::Wrecked]);
            }
        }
        // Riderless bike and rider body retain independent motion and animation clocks.
        if self.phase != Phase::Mounting {
            let mut bike_law = BodyLaw {
                motion_version: law.motion_version,
                recovery_version: 0,
                drag: c.tables.object[9],
                contact_drag: 95,
                landing: c.spec().landing,
                suspension: c.spec().suspension,
                rebound: c.spec().rebound,
                gravity: c.spec().gravity,
                launch_cap: c.spec().launch_cap,
            };
            bike_law.drag = bike_law.drag.max(0);
            motion_step(
                bike,
                &mut self.bike_location,
                0,
                c,
                topology,
                &bike_law,
                None,
            )?;
            if bike.speed == 0 {
                self.bike_animation.set(&c.animation, 0, 48);
            }
            let f = self
                .bike_animation
                .advance(&c.animation, 0, tick)
                .map_err(|_| Error::Invalid("crash bike animation"))?;
            if let Some(f) = f {
                let (cell, _) = c
                    .animation
                    .select(0, &f, 0, 0)
                    .map_err(|_| Error::Invalid("crash bike view"))?;
                self.bike_frame = cell.frame;
                self.bike_mirror = cell.flags & 1 != 0;
            }
        }
        let mount_ready = self.phase == Phase::Walking && self.can_mount(topology);
        if self.phase == Phase::Walking && !mount_ready {
            let body = self.body.as_mut().unwrap();
            let target = self.bike_location;
            let leg = if body.location.path == target.path {
                None
            } else {
                topology.leg(body.location.path, target.path)
            };
            let forward = if let Some(edge) = leg {
                if edge.end {
                    topology.paths[&body.location.path].length - body.location.progress + 16
                } else {
                    -body.location.progress - 16
                }
            } else {
                target.progress - body.location.progress
            };
            let side = target.lateral - body.location.lateral - 6000;
            let (toward, side) = if let Some(walk) = walk {
                (i64::from(walk.longitudinal), i32::from(walk.lateral) * 256)
            } else {
                (forward, side / 256)
            };
            body.location.heading = atan2(
                toward.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
                side,
            ) * 65536;
            body.motion.speed = if walk == Some(Walk::default()) {
                0
            } else {
                1200
            };
            let heading = body.location.heading;
            motion_step(
                &mut body.motion,
                &mut body.location,
                heading,
                c,
                topology,
                law,
                leg,
            )?;
        } else if !matches!(self.phase, Phase::Mounting | Phase::Walking) {
            let body = self.body.as_mut().unwrap();
            let heading = body.location.heading;
            motion_step(
                &mut body.motion,
                &mut body.location,
                heading,
                c,
                topology,
                law,
                None,
            )?;
        }
        let body = self.body.as_mut().unwrap();
        if self.phase == Phase::Sliding && body.motion.speed > 0 && body.animation.code < 0 {
            body.animation.set(&c.animation, 0, 29);
            body.animation.queued = Some(29);
        }
        match self.phase {
            Phase::Sliding => {
                if body.motion.speed == 0 && body.motion.height <= 0 {
                    self.phase = Phase::GettingUp;
                    self.elapsed = 0;
                    body.animation.set(&c.animation, 0, 36);
                    body.animation.queued = Some(37);
                    events.push(RecoveryEvent::GetUp);
                } else if body.motion.height <= 0xfff && matches!(body.animation.code, 33 | 35) {
                    body.animation.set(
                        &c.animation,
                        0,
                        if body.animation.code == 33 { 34 } else { 29 },
                    );
                    body.animation.queued = Some(29);
                }
            }
            Phase::GettingUp if assists.fast_recovery => {
                body.animation.set(&c.animation, 0, 37);
                self.phase = Phase::Standing;
                self.assisted = true;
            }
            Phase::Standing if (body.animation.loops > 1 || assists.fast_recovery) => {
                body.animation.queued = Some(38);
            }
            _ => {}
        }
        let f = body
            .animation
            .advance(&c.animation, 0, tick)
            .map_err(|_| Error::Invalid("recovery animation"))?;
        if let Some(f) = f {
            let (cell, roll) = c
                .animation
                .select(0, &f, body.location.heading, 0)
                .map_err(|_| Error::Invalid("recovery view"))?;
            body.frame = cell.frame;
            body.mirror = cell.flags & 1 != 0;
            body.view_flags = cell.flags;
            body.roll = roll;
        }
        if self.phase == Phase::GettingUp && body.animation.code == 37 {
            self.phase = Phase::Standing;
            self.elapsed = 0;
        }
        if self.phase == Phase::Standing && body.animation.code == 38 {
            self.phase = Phase::Walking;
            self.elapsed = 0;
        }
        body.projected = topology.projected(body.location);
        body.branch = topology.paths[&body.location.path].branch;
        body.motion.lateral = body.location.lateral;
        body.motion.height = body.location.height;
        let timed_out = self.elapsed > 1200;
        if self.phase == Phase::Walking && (self.can_mount(topology) || timed_out) {
            if bike.condition == 0 || !self.bike_available {
                self.terminal(Outcome::Wrecked, bike);
                events.push(RecoveryEvent::Wrecked);
            } else {
                let body = self.body.as_mut().unwrap();
                body.animation.set(&c.animation, 0, 41);
                body.animation.queued = Some(2);
                body.motion.speed = 0;
                self.phase = Phase::Mounting;
                self.elapsed = 0;
            }
        }
        if self.phase == Phase::Mounting && self.body.as_ref().unwrap().animation.code != 41 {
            bike.crashed = false;
            bike.force = 0;
            bike.drive = 0;
            bike.lateral_force = 0;
            bike.lateral_speed = 0;
            bike.lean = 0;
            bike.skid = 0;
            bike.skid_ticks = 0;
            bike.failed = bike.condition == 0;
            self.phase = Phase::Mounted;
            self.body = None;
            self.elapsed = 0;
            events.push(RecoveryEvent::Remount);
        }
        Ok(events)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryEvent {
    GetUp,
    Remount,
    Rescue,
    Wrecked,
}
/// Source generic actor integrator (advance_actor_motion), including 32-bit wrap products.
pub fn actor_delta(speed: i32, acceleration: i32, heading: i32, tables: &Tables) -> (i32, i32) {
    let [forward, lateral] = actor_velocity(speed, heading, tables);
    (
        forward.wrapping_mul(2).wrapping_sub(acceleration) >> 9,
        lateral,
    )
}
pub fn actor_velocity(speed: i32, heading: i32, tables: &Tables) -> [i32; 2] {
    let (forward, lateral) = if heading == 0 {
        (speed, 0)
    } else {
        let angle = ((heading as u32 & 0xfff000) >> 12) as i32;
        (
            (tables.sin(angle + 1024) / 256).wrapping_mul(speed) / 256,
            (tables.sin(angle) / 256).wrapping_mul(speed) / 256,
        )
    };
    [forward, lateral]
}
/// Fixed-dt counterpart of the source speed integrator for detached actors.
/// Profile 0 is retained byte-for-byte for historical recordings.
pub fn recovery_speed_step(motion: &mut Bike, tables: &Tables, surface: usize, law: &BodyLaw) {
    if law.motion_version == 0 {
        let speed = motion.speed.max(0);
        let base = if motion.contact == 0 {
            speed >> 2
        } else {
            speed
        };
        let contact_drag = if motion.contact == 0 {
            0
        } else {
            ((i64::from(tables.drag[surface]) * i64::from(speed)) >> 14) as i32 + law.contact_drag
        };
        let drag = (law.drag * (base >> 8)) >> 8;
        motion.acceleration = -(drag + contact_drag);
        motion.speed = (speed + motion.acceleration).max(0);
        return;
    }
    motion.integrate_source_speed(tables, surface, law.drag, law.contact_drag);
}
pub fn motion_step(
    motion: &mut Bike,
    location: &mut Location,
    heading: i32,
    c: &Configuration,
    topology: &Topology,
    law: &BodyLaw,
    planned: Option<crate::topology::Portal>,
) -> Result<()> {
    motion_step_kind(
        motion,
        location,
        heading,
        c,
        topology,
        law,
        planned,
        if law.recovery_version == 1 {
            crate::handling::VerticalKind::Rider
        } else {
            crate::handling::VerticalKind::Legacy
        },
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn motion_step_bike(
    motion: &mut Bike,
    location: &mut Location,
    c: &Configuration,
    topology: &Topology,
    law: &BodyLaw,
) -> Result<()> {
    motion_step_kind(
        motion,
        location,
        0,
        c,
        topology,
        law,
        None,
        crate::handling::VerticalKind::Bike,
    )
}
#[allow(clippy::too_many_arguments)]
fn motion_step_kind(
    motion: &mut Bike,
    location: &mut Location,
    heading: i32,
    c: &Configuration,
    topology: &Topology,
    law: &BodyLaw,
    planned: Option<crate::topology::Portal>,
    kind: crate::handling::VerticalKind,
) -> Result<()> {
    let projected = topology.projected(*location).unwrap_or(motion.distance_q8);
    let sample =
        (i64::from(c.start_sample) + (projected >> 8)).clamp(0, c.ground.len() as i64 - 1) as usize;
    let ground = c.ground[sample];
    let surface = ground.surface(location.lateral);
    recovery_speed_step(motion, &c.tables, surface, law);
    let (delta, lateral) = if kind == crate::handling::VerticalKind::Bike {
        if motion.contact > 0 {
            motion.air_lateral_speed = motion.lateral_speed;
        }
        (
            ((motion
                .speed
                .wrapping_mul(2)
                .wrapping_sub(motion.acceleration))
                >> 1)
                / 256,
            if motion.speed > 300 {
                motion.air_lateral_speed
            } else {
                0
            },
        )
    } else {
        actor_delta(motion.speed, motion.acceleration, heading, &c.tables)
    };
    if kind != crate::handling::VerticalKind::Legacy {
        let [forward, side] = if kind == crate::handling::VerticalKind::Bike {
            [motion.speed, motion.air_lateral_speed]
        } else {
            actor_velocity(motion.speed, heading, &c.tables)
        };
        let vertical = if kind == crate::handling::VerticalKind::Bike {
            0
        } else {
            motion.contact_velocity.map_or(0, |v| v[2])
        };
        motion.contact_velocity = Some([forward, side, vertical]);
    }
    let old_path = location.path;
    let old_position = location.progress;
    let old_grade = topology.grade(*location);
    let grade_movement = topology.advance_with_rise(location, i64::from(delta), planned)?;
    location.lateral = location
        .lateral
        .saturating_add(lateral)
        .clamp(-256_000_000, 256_000_000);
    let new_projected = topology
        .projected(*location)
        .unwrap_or(projected + i64::from(delta));
    motion.distance_q8 = new_projected;
    motion.lateral = location.lateral;
    let mut ground = if delta >= 0 {
        c.swept_ground(projected, new_projected.max(projected))
    } else {
        let mut g = c.swept_ground(new_projected.min(projected), projected);
        g.movement = -g.movement;
        g.bend_movement = -g.bend_movement;
        g
    };
    if old_path != location.path || old_position >> 8 != location.progress >> 8 {
        motion.previous_rise = old_grade;
    }
    ground.height = 0;
    ground.rise = topology.grade(*location);
    ground.movement = grade_movement;
    if kind == crate::handling::VerticalKind::Bike && motion.speed > 300 {
        let bend = ground.bend_movement * 400 / 256;
        location.lateral = location.lateral.saturating_add(bend);
        motion.lateral = location.lateral;
        if let Some(v) = &mut motion.contact_velocity {
            v[1] += bend;
        }
    }
    motion.vertical_profile(&law.spec(c.spec()), &c.tables, ground, kind);
    location.height = motion.height;
    Ok(())
}
