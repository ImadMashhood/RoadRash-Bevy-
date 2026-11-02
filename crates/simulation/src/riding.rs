//! Composition of source handling and animation. No Bevy, disk reads or frame delta here.
use crate::{
    animation::{Library, Player, Pose, Weapon},
    handling::{Bike, BikeSpec, Ground, Tables},
    *,
};
use std::path::Path;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    #[serde(default, skip_serializing_if = "legacy_motion")]
    pub motion_version: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<crate::actions::Config>,
    pub specs: Vec<BikeSpec>,
    pub selected: u32,
    pub tables: Tables,
    pub animation: Library,
    pub ground: Vec<Ground>,
    pub start_sample: u32,
}
fn legacy_motion(version: &u8) -> bool {
    *version == 0
}
impl Configuration {
    pub fn validate(&self) -> Result<()> {
        self.tables.validate()?;
        if let Some(actions) = self.actions {
            actions.validate()?;
        }
        if self.motion_version > 1
            || self.specs.is_empty()
            || self.specs.len() > 64
            || self.ground.is_empty()
            || self.ground.len() > 1_000_000
            || self.start_sample as usize >= self.ground.len()
        {
            return Err(Error::Invalid("riding configuration count/start"));
        }
        if self.ground.iter().any(|g| {
            g.left > g.right
                || g.left_shoulder > g.left
                || g.right_shoulder < g.right
                || g.kind > 4
                || !(-128..=127).contains(&g.rise)
                || !(-128..=127).contains(&g.bend)
                || [
                    g.left,
                    g.right,
                    g.left_shoulder,
                    g.right_shoulder,
                    g.height,
                    g.movement,
                ]
                .iter()
                .any(|v| v.unsigned_abs() > 1_000_000)
        }) {
            return Err(Error::Invalid("ground profile range/edges"));
        }
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.specs {
            s.validate()?;
            if !ids.insert(s.id) {
                return Err(Error::Invalid("duplicate bike"));
            }
        }
        self.specs
            .iter()
            .find(|s| s.id == self.selected)
            .ok_or(Error::Invalid("missing selected bike"))?;
        self.animation
            .validate()
            .map_err(|_| Error::Invalid("animation bank"))?;
        Ok(())
    }
    pub fn load(
        data: &Path,
        route: &rr_course::route::Route,
        selected: u32,
        start_sample: u32,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let exe = std::fs::read(data.join("ROADRASH.EXE"))?;
        let config = Self {
            motion_version: 0,
            actions: None,
            specs: BikeSpec::read(&std::fs::read(data.join("DATA/BIKESPEC.RSC"))?)?,
            selected,
            tables: Tables::read(&exe)?,
            animation: Library::read(&exe, &std::fs::read(data.join("DATA/GLOBAL.RSC"))?)?,
            ground: route
                .points
                .iter()
                .map(|p| {
                    let s = &p.path;
                    Ground {
                        left: p.offset - rr_course::lane_world(s.lanes_q8[0]),
                        right: p.offset + rr_course::lane_world(s.lanes_q8[1]),
                        left_shoulder: p.offset
                            - rr_course::lane_world(s.lanes_q8[0])
                            - s.shoulder[0],
                        right_shoulder: p.offset
                            + rr_course::lane_world(s.lanes_q8[1])
                            + s.shoulder[1],
                        flags: s.marking_flags,
                        kind: 0,
                        rise: i32::from(s.rise),
                        bend: i32::from(s.bend),
                        bend_movement: 0,
                        height: 0,
                        movement: 0,
                    }
                })
                .collect(),
            start_sample,
        };
        config.validate()?;
        Ok(config)
    }
    /// Integrate signed grade/bend across each crossed source sample. No renderer access.
    pub fn swept_ground(&self, old: i64, new: i64) -> Ground {
        let start = i64::from(self.start_sample) * 256;
        let mut at = (start + old).max(0);
        let end = (start + new).max(at);
        let mut rise = 0i64;
        let mut bend = 0i64;
        while at < end {
            let index = (at >> 8).min(self.ground.len() as i64 - 1) as usize;
            let next = (((at >> 8) + 1) * 256).min(end);
            let distance = next - at;
            rise += i64::from(self.ground[index].rise) * distance;
            bend -= i64::from(self.ground[index].bend) * distance;
            at = next;
        }
        let mut ground = self.ground[((end >> 8).min(self.ground.len() as i64 - 1)) as usize];
        ground.movement = rise as i32;
        ground.bend_movement = bend as i32;
        ground
    }
    pub fn spec(&self) -> &BikeSpec {
        self.specs
            .iter()
            .find(|s| s.id == self.selected)
            .expect("validated selected spec")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rider {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<crate::actions::State>,
    pub bike: Bike,
    pub animation: Player,
    pub pose: Pose,
    pub weapon: Weapon,
    pub frame: usize,
    pub mirror: bool,
    pub view_flags: u32,
    pub roll: i32,
    pub flags: u32,
    pub contact_frame: bool,
    pub pose_ticks: u32,
}
impl Rider {
    pub fn new(c: &Configuration) -> Result<Self> {
        let mut animation = Player::default();
        animation.set(&c.animation, 0, 2);
        let mut bike = Bike::new(c.spec(), &c.tables)?;
        bike.previous_rise = c.ground[c.start_sample as usize].rise;
        Ok(Self {
            actions: None,
            bike,
            animation,
            pose: Pose::Ride,
            weapon: Weapon::Fists,
            frame: 2,
            mirror: false,
            view_flags: 0,
            roll: 0,
            flags: 0,
            contact_frame: false,
            pose_ticks: 0,
        })
    }
    pub fn step(&mut self, c: &Configuration, input: Command, tick: u32) -> Result<()> {
        let old_weapon = self.weapon;
        if let Some(w) = input.weapon {
            self.weapon = w;
        }
        self.bike.damage(input.damage); // explicit events consumed once by Simulation
        let wanted = input.pose.unwrap_or(
            if !matches!(
                self.pose,
                Pose::Ride | Pose::Idle | Pose::Accelerate | Pose::Tuck
            ) && self.animation.code >= 0
                && !self.bike.crashed
            {
                self.pose
            } else if self.bike.crashed {
                if self.animation.code == 37 && self.bike.speed < 2500 {
                    Pose::Stand
                } else {
                    crash_pose(self.bike.speed, self.bike.height)
                }
            } else if self.bike.speed < 300 {
                Pose::Idle
            } else if self.bike.speed < 1800 && self.bike.force > 30 {
                Pose::Accelerate
            } else if self.bike.lateral_speed < -4096 || self.bike.speed >= 9501 {
                Pose::Tuck
            } else {
                Pose::Ride
            },
        );
        if wanted != self.pose || self.animation.code < 0 || self.weapon != old_weapon {
            self.pose = wanted;
            self.pose_ticks = 0;
            self.animation
                .set(&c.animation, 0, c.animation.pose_code(wanted, self.weapon));
            self.animation.queued = match wanted {
                Pose::GetUp => Some(37),
                Pose::Remount => Some(2),
                _ => None,
            };
        }
        self.pose_ticks = self.pose_ticks.saturating_add(1);
        let f = self
            .animation
            .advance(&c.animation, 0, tick)
            .map_err(|_| Error::Invalid("animation playback"))?;
        if self.pose == Pose::GetUp && self.animation.code == 37 {
            self.pose = Pose::Stand;
        }
        if matches!(self.pose, Pose::Remount | Pose::Steal) && self.animation.code == 2 {
            self.pose = Pose::Ride;
        }
        if let Some(f) = f {
            let gain = if self.bike.skid != 0 {
                c.spec().skid_yaw_gain
            } else {
                c.spec().yaw_gain
            };
            let mut yaw =
                (gain / 256) * atan2(self.bike.speed * 4, self.bike.lateral_speed) * 65536;
            if self.bike.speed < 4000 {
                yaw = yaw.clamp(-0x180000, 0x180000);
            }
            let roll = atan2(c.tables.contact, self.bike.lean / 256) * 65536;
            let (cell, selected_roll) = c
                .animation
                .select(0, &f, yaw, roll)
                .map_err(|_| Error::Invalid("animation view"))?;
            self.frame = cell.frame;
            self.mirror = cell.flags & 1 != 0;
            self.view_flags = cell.flags;
            self.roll = if yaw > 0xe0000 {
                0x50000
            } else if yaw < -0xe0000 {
                -0x50000
            } else {
                selected_roll
            };
            self.flags = f.flags;
            self.contact_frame = f.flags & 0x600000 != 0;
        } else {
            self.contact_frame = false;
        }
        let index = (i64::from(c.start_sample) + (self.bike.distance_q8 >> 8))
            .clamp(0, c.ground.len() as i64 - 1) as usize;
        let ground = c.ground[index];
        if self.bike.crashed {
            self.bike.speed = (self.bike.speed - self.bike.speed / 32 - 20).max(0);
            self.bike.lateral_speed = 0;
        }
        let boost = match (&mut self.actions, c.actions) {
            (Some(state), Some(config)) => {
                state.tick(config, input, !self.bike.crashed && !self.bike.failed)
            }
            _ => 256,
        };
        self.bike.step_with_boost_profile(
            c.spec(),
            &c.tables,
            input,
            ground,
            boost,
            c.motion_version,
            |old, new| c.swept_ground(old, new),
        );
        Ok(())
    }
}
pub fn atan2(a: i32, b: i32) -> i32 {
    if a == 0 && b == 0 {
        return 0;
    }
    let x = i64::from(a).abs();
    let y = i64::from(b).abs();
    match (i32::from(a < 0)) | (i32::from(b < 0) << 1) | (i32::from(x < y) << 2) {
        0 => (y * 32 / x) as i32,
        1 => 128 - (y * 32 / x) as i32,
        2 => -(y * 32 / x) as i32,
        3 => (y * 32 / x) as i32 - 128,
        4 => 64 - (x * 32 / y) as i32,
        5 => (x * 32 / y) as i32 + 64,
        6 => (x * 32 / y) as i32 - 64,
        _ => -64 - (x * 32 / y) as i32,
    }
}

/// Visual dispatch from the source crash speed/height gates. Detached body
/// impulses and recovery decisions are owned by Phase 12, not this selector.
pub fn crash_pose(speed: i32, height: i32) -> Pose {
    if height > 0xfff {
        Pose::Crash
    } else if speed < 0x9c4 {
        Pose::GetUp
    } else {
        Pose::Slide
    }
}
