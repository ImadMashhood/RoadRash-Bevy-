//! Deterministic combat primitives. Source units and policies: docs/COMBAT.md.
use crate::{
    ActorId, Error, Result,
    animation::{Library, Pose, Weapon},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttackKind {
    Punch,
    Kick,
    Backhand,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
    #[default]
    Auto,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub kind: AttackKind,
    #[serde(default)]
    pub side: Side,
}
impl Request {
    pub fn from_pose(p: Pose) -> Option<Self> {
        let (kind, side) = match p {
            Pose::PunchLeft => (AttackKind::Punch, Side::Left),
            Pose::PunchRight => (AttackKind::Punch, Side::Right),
            Pose::KickLeft => (AttackKind::Kick, Side::Left),
            Pose::KickRight => (AttackKind::Kick, Side::Right),
            Pose::BackhandLeft => (AttackKind::Backhand, Side::Left),
            Pose::BackhandRight => (AttackKind::Backhand, Side::Right),
            _ => return None,
        };
        Some(Self { kind, side })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub start: u16,
    pub end: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub code: i8,
    pub duration: u16,
    pub windows: Vec<Window>,
    pub frame_starts: Vec<u16>,
}
impl Definition {
    pub fn from_library(lib: &Library, request: Request, weapon: Weapon) -> Result<Self> {
        let pose = pose(request);
        let code = lib.pose_code(pose, weapon);
        let index = lib
            .resolve(0, code)
            .ok_or(Error::Invalid("missing attack clip"))?;
        let frames = lib.banks[0][index]
            .as_ref()
            .ok_or(Error::Invalid("attack sequence"))?;
        let mut elapsed = 0u16;
        let mut windows = vec![];
        let mut frame_starts = vec![];
        for (i, f) in frames.iter().enumerate() {
            let start = if i == 0 { 0 } else { elapsed + 1 };
            let end = elapsed
                .checked_add(f.duration as u16)
                .and_then(|v| v.checked_add(1))
                .ok_or(Error::Invalid("attack duration overflow"))?;
            frame_starts.push(start);
            if f.flags & 0x600000 != 0 || i == 2 {
                windows.push(Window { start, end });
            }
            elapsed = end - 1;
        }
        let result = Self {
            code,
            duration: elapsed + 1,
            windows,
            frame_starts,
        };
        result.validate()?;
        Ok(result)
    }
    /// The source checks retained descriptor flags OR the frame counter AFTER
    /// animation advancement. Legacy definitions used the selected frame for both.
    pub fn from_library_version(
        lib: &Library,
        request: Request,
        weapon: Weapon,
        version: u8,
    ) -> Result<Self> {
        let mut result = Self::from_library(lib, request, weapon)?;
        if version == 0 {
            return Ok(result);
        }
        if !(1..=2).contains(&version) {
            return Err(Error::Invalid("combat rules version"));
        }
        let mut player = crate::animation::Player::default();
        player.set(lib, 0, result.code);
        player.queued = Some(2);
        result.windows.clear();
        let mut start = None;
        let attack_code = player.code;
        let mut previous_flags = 0;
        for age in 0..result.duration {
            // The source fight transaction reads the animation retained by
            // the preceding think, before this tick's animation advancement.
            let before_active = age != 0
                && player.code == attack_code
                && (previous_flags & 0x600000 != 0 || player.frame == 2);
            let selected = player
                .advance(lib, 0, u32::from(age))
                .map_err(|_| Error::Invalid("combat animation timing"))?;
            previous_flags = selected.as_ref().map_or(0, |f| f.flags);
            let active = if version == 2 {
                before_active
            } else {
                selected.is_some_and(|f| f.flags & 0x600000 != 0) || player.frame == 2
            };
            if active && start.is_none() {
                start = Some(age);
            }
            if !active && let Some(start) = start.take() {
                result.windows.push(Window { start, end: age });
            }
        }
        if let Some(start) = start {
            result.windows.push(Window {
                start,
                end: result.duration,
            });
        }
        for start in result.frame_starts.iter_mut().skip(1) {
            *start = start.saturating_sub(1);
        }
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<()> {
        if self.code < 0
            || self.duration == 0
            || self.duration > 10000
            || self.frame_starts.is_empty()
            || self.frame_starts.len() > 127
            || self.frame_starts[0] != 0
            || self.frame_starts.windows(2).any(|w| w[0] >= w[1])
            || self.frame_starts.iter().any(|v| *v >= self.duration)
            || self.windows.is_empty()
            || self
                .windows
                .iter()
                .any(|w| w.start >= w.end || w.end > self.duration)
        {
            return Err(Error::Invalid("attack phase windows"));
        }
        Ok(())
    }
    pub fn active(&self, age: u64) -> bool {
        self.windows
            .iter()
            .any(|w| u64::from(w.start) <= age && age < u64::from(w.end))
    }
    pub fn frame(&self, age: u64) -> usize {
        self.frame_starts
            .partition_point(|v| u64::from(*v) <= age)
            .saturating_sub(1)
    }
}
pub fn pose(request: Request) -> Pose {
    match (request.kind, request.side == Side::Right) {
        (AttackKind::Punch, false) => Pose::PunchLeft,
        (AttackKind::Punch, true) => Pose::PunchRight,
        (AttackKind::Kick, false) => Pose::KickLeft,
        (AttackKind::Kick, true) => Pose::KickRight,
        (AttackKind::Backhand, false) => Pose::BackhandLeft,
        (AttackKind::Backhand, true) => Pose::BackhandRight,
    }
}
pub fn weapon_slot(kind: AttackKind, weapon: Weapon) -> usize {
    if kind == AttackKind::Kick {
        3
    } else {
        weapon as usize
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub base: i32,
    pub maximum: i32,
    pub health: i32,
    pub strength: i32,
}
impl Default for Stats {
    fn default() -> Self {
        Self {
            base: 2500,
            maximum: 2500,
            health: 2500,
            strength: 7,
        }
    }
}
impl Stats {
    pub fn validate(&self) -> Result<()> {
        if !(1..=65536).contains(&self.base)
            || self.maximum < self.base / 2
            || self.maximum > self.base
            || self.health < 0
            || self.health > self.maximum
            || !(1..=1024).contains(&self.strength)
        {
            return Err(Error::Invalid("combat stamina/strength"));
        }
        Ok(())
    }
    pub fn damage(&self, multiplier: i32) -> i32 {
        let scaled = (((i64::from(self.maximum) * 256 / i64::from(self.base))
            * i64::from(self.strength))
            >> 8) as i32;
        let scaled = scaled.max(self.strength / 2);
        ((i64::from(multiplier) * i64::from(scaled)) / 256) as i32
    }
    pub fn hit(&mut self, amount: i32) {
        self.maximum =
            ((i64::from(self.maximum) - i64::from(amount) * 4) as i32).max(self.base / 2);
        self.health = ((i64::from(self.health) - i64::from(amount) * 64) as i32).max(0);
    }
    pub fn regenerate(&mut self) {
        if self.health > 0 {
            self.health = (self.health + 1).min(self.maximum);
        }
    }
}
/// Original-width knock-off reason scaling; useful independently of any collision producer.
pub fn reason_damage(value: i32, reason: u8) -> i32 {
    match reason {
        1 | 4 | 5 => value,
        2 => {
            value
                .wrapping_mul(7)
                .wrapping_add((value.wrapping_mul(7) >> 31) & 3)
                >> 2
        }
        3 => value.wrapping_add((value >> 31) & 7) >> 3,
        _ => 0,
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub route: u32,
    pub branch: u32,
    pub path: u32,
    pub progress: i64,
    pub lateral: i32,
    pub height: i32,
}
impl Position {
    pub fn same_road(self, other: Self) -> bool {
        self.route == other.route && self.branch == other.branch
    }
}
pub fn in_reach(a: Position, b: Position, side: Side, slot: usize, reach: [i32; 4]) -> bool {
    a.same_road(b)
        && side != Side::Auto
        && ((a.lateral < b.lateral) == (side == Side::Right))
        && (a.progress - b.progress).abs() < 64
        && (i64::from(a.lateral) - i64::from(b.lateral)).abs() < i64::from(reach[slot])
        && (i64::from(a.height) - i64::from(b.height)).abs() < 0x3000
}
pub fn target(
    self_id: ActorId,
    at: Position,
    candidates: impl IntoIterator<Item = (ActorId, Position)>,
) -> Option<ActorId> {
    candidates
        .into_iter()
        .filter(|(id, p)| {
            *id != self_id && at.same_road(*p) && (p.progress - at.progress).abs() < 480
        })
        .min_by_key(|(id, p)| {
            (
                (p.progress - at.progress).abs(),
                p.progress >= at.progress,
                *id,
            )
        })
        .map(|v| v.0)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub kind: Weapon,
    pub owner: Option<ActorId>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub items: BTreeMap<WeaponId, Item>,
}
impl Inventory {
    pub fn validate(&self) -> Result<()> {
        let mut owners = std::collections::BTreeSet::new();
        if self.items.len() > 4096 {
            return Err(Error::Invalid("weapon count"));
        }
        for (id, item) in &self.items {
            if id.0 == 0
                || item.kind == Weapon::Fists
                || item.owner.is_some_and(|owner| !owners.insert(owner))
            {
                return Err(Error::Invalid("weapon identity/duplicate owner"));
            }
        }
        Ok(())
    }
    pub fn carried(&self, owner: ActorId) -> Option<(WeaponId, Weapon)> {
        self.items
            .iter()
            .find(|(_, v)| v.owner == Some(owner))
            .map(|(k, v)| (*k, v.kind))
    }
    pub fn transfer(&mut self, item: WeaponId, from: ActorId, to: ActorId) -> bool {
        if from == to || self.carried(to).is_some() {
            return false;
        }
        let Some(value) = self.items.get_mut(&item) else {
            return false;
        };
        if value.owner != Some(from) {
            return false;
        }
        value.owner = Some(to);
        true
    }
    pub fn drop(&mut self, owner: ActorId) -> Option<WeaponId> {
        let (id, _) = self.carried(owner)?;
        self.items.get_mut(&id)?.owner = None;
        Some(id)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Swing {
    #[serde(default, skip_serializing_if = "crate::is_false")]
    pub repeat: bool,
    pub request: Request,
    pub target: Option<ActorId>,
    pub weapon: Weapon,
    pub started: u64,
    pub definition: Definition,
    pub resolved: bool,
    pub swish: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Combatant {
    pub stats: Stats,
    pub swing: Option<Swing>,
    pub stun_until: u64,
    pub hit_until: u64,
    pub steal_until: u64,
    pub hit_count: u32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    Swing,
    Hit,
    Kick,
    Stolen,
    Dropped,
    Crash,
    Land,
    GetUp,
    Remount,
    Arrest,
    Finish,
    Wrecked,
    Rescue,
    Contact,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Effect {
    pub tick: u64,
    pub actor: ActorId,
    pub other: Option<ActorId>,
    pub kind: EffectKind,
}
