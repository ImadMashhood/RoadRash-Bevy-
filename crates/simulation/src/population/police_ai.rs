//! Source cop brain and fixed-tick host bridge. Native offsets stay inside this module.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Relations {
    pub index: u8,
    pub current: [i32; 30],
    pub target: [i32; 30],
    pub reaction: [i32; 15],
}
impl Relations {
    pub fn from_words(words: &[i32]) -> Result<Self> {
        if words.len() != 129 {
            return Err(Error::Invalid("police relationship words"));
        }
        let value = Self {
            index: ((words[0x164 / 4] as u32 >> 8) & 255) as u8,
            current: words[..30].try_into().unwrap(),
            target: words[0x78 / 4..0x78 / 4 + 30].try_into().unwrap(),
            reaction: words[0xf0 / 4..0xf0 / 4 + 15].try_into().unwrap(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        if self.index >= 10
            || self
                .current
                .iter()
                .chain(self.target.iter())
                .chain(self.reaction.iter())
                .any(|v| !(-0x100000..=0x100000).contains(v))
        {
            return Err(Error::Invalid("police relationships"));
        }
        Ok(())
    }
    pub(crate) fn react(&mut self, index: u8, event: usize) -> Result<()> {
        if index >= 10 || event >= 5 {
            return Err(Error::Invalid("police relationship event"));
        }
        for axis in 0..3 {
            let at = usize::from(index) * 3 + axis;
            self.current[at] =
                (self.current[at] + self.reaction[event * 3 + axis]).clamp(-0x100000, 0x100000);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectKey {
    pub actor: Option<ActorId>,
    pub agent: Option<u64>,
    pub kind: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiState {
    pub words: Vec<i32>,
    pub target: Option<ActorId>,
    pub ahead: Option<ActorId>,
    pub behind: Option<ActorId>,
    pub overtake: Option<ObjectKey>,
    pub attack_weapon: u8,
    pub initialized: bool,
    pub level: u8,
}
impl AiState {
    pub fn from_words(
        words: &[i32],
        lane: u8,
        level: u8,
        weapon: animation::Weapon,
    ) -> Result<Self> {
        let mut value = Self {
            words: words.to_vec(),
            target: None,
            ahead: None,
            behind: None,
            overtake: None,
            attack_weapon: match weapon {
                animation::Weapon::Fists => 0,
                animation::Weapon::Club => 1,
                animation::Weapon::Chain => 2,
            },
            initialized: false,
            level,
        };
        if value.words.len() != 129 || lane > 2 || level > 4 {
            return Err(Error::Invalid("police brain constructor"));
        }
        for offset in [
            0x160, 0x168, 0x16c, 0x170, 0x174, 0x178, 0x17c, 0x180, 0x188, 0x190, 0x194, 0x198,
            0x19c,
        ] {
            value.words[offset / 4] = 0;
        }
        value.words[0x164 / 4] &= !255;
        for at in 0x138 / 4..0x160 / 4 {
            value.words[at] = 0;
        }
        value.words[0x18c / 4] = i32::from(lane);
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        if self.words.len() != 129
            || self.level > 4
            || self.attack_weapon > 3
            || self
                .words
                .iter()
                .any(|v| !(-0x1000000..=0x1000000).contains(v))
            || !(0..=9).contains(&self.words[0x160 / 4])
            || !(0..=2).contains(&self.words[0x18c / 4])
            || self.overtake.is_some_and(|k| {
                k.actor.is_some() == k.agent.is_some() || !matches!(k.kind, 4 | 8 | 16 | 32)
            })
        {
            return Err(Error::Invalid("police brain state"));
        }
        Relations::from_words(&self.words)?.validate()
    }
    pub(crate) fn index(&self) -> u8 {
        ((self.words[0x164 / 4] as u32 >> 8) & 255) as u8
    }
    pub(crate) fn react(&mut self, index: u8, event: usize) -> Result<()> {
        let mut r = Relations::from_words(&self.words)?;
        r.react(index, event)?;
        self.words[..30].copy_from_slice(&r.current);
        Ok(())
    }
}
impl Encounters {
    pub(crate) fn police_relationship(
        &mut self,
        own: ActorId,
        other: ActorId,
        event: usize,
    ) -> Result<()> {
        let Some(other) = self.units.get(&other) else {
            return Ok(());
        };
        let index = other
            .police_ai
            .as_ref()
            .map(AiState::index)
            .or_else(|| other.police_relations.as_ref().map(|r| r.index));
        let Some(index) = index else { return Ok(()) };
        let Some(unit) = self.units.get_mut(&own) else {
            return Ok(());
        };
        if let Some(ai) = unit.police_ai.as_mut() {
            ai.react(index, event)?;
        } else if let Some(r) = unit.police_relations.as_mut() {
            r.react(index, event)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Initialize,
    Begin,
    Finish,
}
impl World {
    pub(crate) fn police_initialize(
        &mut self,
        e: &mut Encounters,
        actors: &mut BTreeMap<ActorId, Actor>,
        tick: u64,
    ) -> Result<()> {
        self.source_police_step(e, actors, tick, Pass::Initialize, || {
            unreachable!("native cop birth replan consumes no random draws")
        })
    }
    pub(crate) fn police_begin_decisions(
        &mut self,
        e: &mut Encounters,
        actors: &mut BTreeMap<ActorId, Actor>,
        tick: u64,
        random: impl FnMut() -> u32,
    ) -> Result<()> {
        self.source_police_step(e, actors, tick, Pass::Begin, random)
    }
    pub(crate) fn police_finish_decisions(
        &mut self,
        e: &mut Encounters,
        actors: &mut BTreeMap<ActorId, Actor>,
        tick: u64,
    ) -> Result<()> {
        self.source_police_step(e, actors, tick, Pass::Finish, || {
            unreachable!("native AI post-hit continuation consumes no random draws")
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Road {
    pub center: i32,
    pub left: i32,
    pub right: i32,
    pub lanes: [i32; 2],
}
#[derive(Clone, Debug)]
pub(crate) struct RiderView {
    pub id: ActorId,
    pub position: i32,
    pub lateral: i32,
    pub speed: i32,
    pub captured_speed: i32,
    pub half_lateral: i32,
    pub length: i32,
    pub crash: i32,
    pub present: bool,
    pub hit_until: u32,
    pub index: u8,
    pub road: Road,
    pub relations: Option<Relations>,
}
#[derive(Clone, Debug)]
pub(crate) struct ScanObject {
    pub key: ObjectKey,
    pub position: i32,
    pub lateral: i32,
    pub speed: i32,
    pub present: bool,
    pub road: Road,
    pub order: u64,
}
pub(crate) struct Decision {
    pub request: Option<Request>,
    pub target: Option<ActorId>,
}
pub(crate) struct Context<'a> {
    #[cfg(test)]
    pub before_wrap: bool,
    pub own: ActorId,
    pub tick: u32,
    pub busted: bool,
    pub riders: &'a BTreeMap<ActorId, RiderView>,
    pub objects: &'a [ScanObject],
    pub current_road: Road,
    pub next_road: Road,
    pub curve_current: i32,
    pub curve_next: i32,
    pub curve_limit: i32,
    pub eagerness: i32,
    pub place: i32,
}
fn source_angle(a: i32, b: i32) -> i32 {
    let v = (riding::atan2(a, b) & 255) * 65536;
    if v > 0x7fffff { v - 0x1000000 } else { v }
}
fn speed_units(v: i32) -> i32 {
    (v >> 12) + (v >> 7) + (v >> 9)
}
fn div_pow(v: i32, bits: u32) -> i32 {
    v / (1 << bits)
}
fn object_size(kind: u8) -> i32 {
    match kind {
        1 | 2 => 0x2000,
        4 | 8 | 32 => 0x3000,
        16 => 0x8000,
        _ => 0x1000,
    }
}
impl AiState {
    pub(crate) fn get(&self, off: usize) -> i32 {
        self.words[off / 4]
    }
    pub(crate) fn set(&mut self, off: usize, v: i32) {
        self.words[off / 4] = v;
    }
    fn set_state(&mut self, state: i32, target: Option<ActorId>) {
        if self.get(0x160) != state {
            self.set(0x160, state);
            self.words[0x164 / 4] &= !255;
            self.set(0x138 + state as usize * 4, 0);
            self.target = target;
        }
    }
    fn scan_next(
        ctx: &Context<'_>,
        from: usize,
        mask: u8,
        forward: bool,
        window: i32,
    ) -> Option<usize> {
        let own = ctx.objects[from].position;
        if forward {
            for k in from + 1..ctx.objects.len() {
                let o = &ctx.objects[k];
                if window <= o.position - own {
                    return None;
                }
                if o.present && o.key.kind & mask != 0 {
                    return Some(k);
                }
            }
        } else {
            for k in (0..from).rev() {
                let o = &ctx.objects[k];
                if window <= own - o.position {
                    return None;
                }
                if o.present && o.key.kind & mask != 0 {
                    return Some(k);
                }
            }
        }
        None
    }
    fn scan_replan(ctx: &Context<'_>, forward: bool, cop: bool) -> Option<ActorId> {
        let mut at = ctx
            .objects
            .iter()
            .position(|o| o.key.actor == Some(ctx.own) && o.key.kind == 8)?;
        loop {
            at = Self::scan_next(ctx, at, 8, forward, if cop { 0x5000 } else { 0xa00 })?;
            let o = &ctx.objects[at];
            let target = &ctx.riders[&o.key.actor?];
            let x = o.lateral >> 8;
            if cop || (target.road.left < x && x < target.road.right) {
                return o.key.actor;
            }
        }
    }
    fn replan(&mut self, ctx: &Context<'_>, cop: bool) {
        while self.get(0x188) > 59 {
            if cop {
                let mut speed = self.get(0x1a4) - 1;
                if speed < 50 {
                    speed = self.get(0x1bc + usize::from(self.level) * 4);
                }
                self.set(0x1a4, speed);
            }
            for k in 0..30 {
                let cur = self.words[k];
                let target = self.words[0x78 / 4 + k];
                let step = self.words[0x12c / 4 + k % 3];
                self.words[k] = if cur > target {
                    (cur - step).max(target)
                } else if cur < target {
                    (cur + step).min(target)
                } else {
                    cur
                };
            }
            self.set(0x188, self.get(0x188) - 60);
        }
        self.ahead = Self::scan_replan(ctx, true, cop);
        self.behind = Self::scan_replan(ctx, false, cop);
        let near = if ctx.place < 4 { 4 - ctx.place } else { 0 };
        let rate = |id: Option<ActorId>| -> i32 {
            let Some(id) = id else {
                return 0;
            };
            let index = usize::from(ctx.riders[&id].index);
            let value = if index < 10 {
                self.words[index * 3 + 1] - self.words[index * 3] + self.words[index * 3 + 2]
            } else {
                0
            };
            if value < (16 - near) * 1000 - i32::from(self.level) * 1500 {
                1
            } else {
                i32::from((16 - near) * 2000 - i32::from(self.level) * 3000 <= value) + 2
            }
        };
        let a = rate(self.ahead);
        let b = rate(self.behind);
        self.set(0x178, a);
        self.set(0x17c, b);
        let (mut pick, mut step) = (self.ahead, a);
        if a <= b {
            pick = self.behind;
            step = b;
            if b <= a {
                if self.behind.is_none() {
                    pick = self.ahead;
                    step = a;
                    if self.ahead.is_none() {
                        pick = None;
                        step = 0;
                    }
                } else if let (Some(ahead), Some(behind)) = (self.ahead, self.behind) {
                    let own = ctx.riders[&ctx.own].position;
                    let db = (own - ctx.riders[&behind].position).abs();
                    let da = (own - ctx.riders[&ahead].position).abs();
                    if da <= db {
                        pick = Some(ahead);
                        step = a;
                    }
                }
            }
        }
        if cop && pick == Some(ActorId(1)) && ctx.busted {
            pick = None;
            step = 0;
        }
        self.target = pick;
        self.set(0x174, step);
        if !cop {
            let mut current = self.get(0x1bc + usize::from(self.level) * 4);
            let current2 = self.get(0x1a8 + usize::from(self.level) * 4);
            let own = ctx.riders[&ctx.own].position;
            let Some(player) = ctx.riders.get(&ActorId(1)) else {
                self.set(0x1a4, current);
                self.set(0x1a0, current2);
                return;
            };
            // Spawn-time ordinary replan. Its output speed is overwritten by the
            // police pursuit tail; only its target/rating selection persists.
            if self.ahead == Some(ActorId(1)) || self.behind == Some(ActorId(1)) {
                current = 0;
            } else if pick.is_none() || step != 3 {
                current = match (i32::from(self.ahead.is_some()) * 2)
                    | i32::from(self.behind.is_some())
                {
                    0 => {
                        if own <= player.position {
                            div_pow(current, 2) + current
                        } else {
                            ((i32::from(self.level) + 44) * current * 2) / 100
                        }
                    }
                    1 => div_pow(current, 4) + current,
                    2 => ((i32::from(self.level) + 94) * current) / 100,
                    _ => {
                        if own <= player.position {
                            div_pow(current, 5) + current
                        } else {
                            (((i32::from(self.level) - 6) / 2 + 100) * current) / 100
                        }
                    }
                };
            } else {
                current += div_pow(current, 3);
            }
            self.set(0x1a4, current);
            self.set(0x1a0, current2);
        }
    }
    fn can_overtake(
        &self,
        ctx: &Context<'_>,
        bike: &handling::Bike,
        random: &mut impl FnMut() -> u32,
    ) -> bool {
        let Some(t) = self.target.and_then(|id| ctx.riders.get(&id)) else {
            return false;
        };
        let own = &ctx.riders[&ctx.own];
        if !(own.hit_until < ctx.tick && t.hit_until < ctx.tick) {
            return false;
        }
        let dpos = ((bike.speed - t.speed) >> 4) - t.position + own.position;
        if dpos.abs() >= self.get(0x1d4) * 16
            || (bike.lateral - t.lateral).abs() >= self.get(0x1d4) * 0x2000
        {
            return false;
        }
        let draw = (random() & 255) as i32;
        if draw >= ctx.eagerness {
            return false;
        }
        self.attack_weapon == 1
            && t.index < 10
            && self.words[0x78 / 4 + usize::from(t.index) * 3 + 1]
                < self.words[usize::from(t.index) * 3 + 1]
    }
    fn transition(
        &mut self,
        ctx: &Context<'_>,
        bike: &handling::Bike,
        attack_active: bool,
        random: &mut impl FnMut() -> u32,
    ) {
        let own = &ctx.riders[&ctx.own];
        if own.crash != 0 {
            self.set_state(if matches!(own.crash, 2 | 3) { 8 } else { 9 }, None);
            return;
        }
        let Some(t) = self
            .target
            .and_then(|id| ctx.riders.get(&id))
            .filter(|t| t.present)
        else {
            self.set_state(1, None);
            return;
        };
        if attack_active {
            return;
        }
        match self.get(0x174) {
            1 => self.set_state(
                if own.position + own.length < t.position - t.length {
                    2
                } else {
                    1
                },
                self.target,
            ),
            2 => {
                let lane = |r: &RiderView| {
                    let v = r.lateral - r.road.center * 256;
                    let at = (v / 256) / 250 - i32::from(v < 0);
                    at >= 0 && at < r.road.lanes[1] >> 8
                };
                if lane(own) && self.can_overtake(ctx, bike, random) {
                    self.set_state(3, self.target);
                } else if lane(t)
                    && t.position + t.length < own.position - own.length
                    && (own.position - own.length - t.position - t.length) < 240
                    && ((own.lateral - t.lateral).unsigned_abs() >> 8) < 128
                {
                    self.set_state(5, self.target);
                } else {
                    self.set_state(1, self.target);
                }
            }
            3 => {
                let state = if self.can_overtake(ctx, bike, random) {
                    3
                } else {
                    4
                };
                self.set_state(state, self.target);
            }
            _ => {}
        }
    }
    fn curve_limit(&self, ctx: &Context<'_>) -> i32 {
        let c = ctx.curve_limit.abs();
        let base = self.get(0x1a4);
        let q = base - div_pow(base, 2);
        if c > 0x30000 {
            let half = q / 2;
            (q - ((((i64::from(half) << 8) / 0x600) * i64::from(c >> 8)) as i32 >> 8)) + half
        } else {
            base
        }
    }
    fn straight(&mut self, ctx: &Context<'_>) {
        let lanes = ctx.next_road.lanes[1] >> 8;
        let lane = match self.get(0x18c) {
            1 => (lanes - 1) / 2,
            2 => (lanes - 1).abs(),
            _ => 0,
        };
        self.set(0x198, ctx.riders[&ctx.own].position + 960);
        self.set(
            0x194,
            div_pow(ctx.curve_next, 3)
                + (ctx.next_road.center + lane * 250) * 256
                + self.get(0x190),
        );
        self.set(0x19c, self.curve_limit(ctx));
    }
    fn curve(&mut self, ctx: &Context<'_>, bike: &handling::Bike) {
        if (ctx.riders[&ctx.own].position >> 8) - (self.get(0x198) >> 8) == -960 {
            return;
        }
        let c = ctx.curve_current;
        let magnitude = c.unsigned_abs();
        if magnitude < 0x30000 || speed_units(bike.speed) < self.curve_limit(ctx) / 2 {
            self.straight(ctx);
            return;
        }
        let half = (ctx.current_road.lanes[1] / 2) * 256;
        let m = magnitude.min(0x60000);
        let mut off = (((i64::from((m << 8) / 0x60000)) * i64::from(half)) >> 8) as i32;
        if c < 0 {
            off = -off;
        }
        let mut x = off + (ctx.current_road.left + ctx.current_road.lanes[0]) * 256 + half;
        if c < 1 {
            x = x.min(bike.lateral);
        } else {
            x = x.max(bike.lateral);
        }
        self.set(0x198, ctx.riders[&ctx.own].position + 960);
        self.set(0x194, x);
        self.set(0x19c, self.curve_limit(ctx));
    }
    fn overtake(&mut self, ctx: &Context<'_>, bike: &handling::Bike) -> bool {
        self.overtake = None;
        let Some(start) = ctx
            .objects
            .iter()
            .position(|o| o.key.actor == Some(ctx.own) && o.key.kind == 4)
        else {
            return false;
        };
        let own = &ctx.objects[start];
        let lo = own.lateral - 0x6000;
        let (mut at, mut best, mut best_time) = (start, None, 151);
        let mut gap = 0;
        while let Some(next) = Self::scan_next(ctx, at, 0x3f, true, 0x960) {
            at = next;
            let o = &ctx.objects[at];
            if o.key.actor == Some(ctx.own) && o.key.kind == 8 {
                continue;
            }
            let size = object_size(o.key.kind);
            let left = o.lateral - size / 2;
            if size + left <= lo || lo + 0xc000 <= left {
                break;
            }
            let closing = if o.key.kind & 0x3c != 0 {
                own.speed.wrapping_sub(o.speed)
            } else {
                own.speed
            };
            if best.is_none()
                || (closing > 0 && own.position < o.position)
                || (closing < 0 && o.position < own.position)
            {
                if closing == 0 {
                    gap = i32::MAX;
                    best_time = i32::MAX;
                    best = Some(at);
                } else {
                    let time = (own.position - o.position).abs() * 256 / closing.abs();
                    if time <= 150 && time < best_time {
                        gap = time;
                        best = Some(at);
                        best_time = time;
                    }
                }
            }
        }
        let Some(at) = best else {
            return false;
        };
        let o = &ctx.objects[at];
        if o.key.kind == 4 && o.key.actor == self.target {
            return false;
        }
        self.overtake = Some(o.key);
        let mut side = (object_size(o.key.kind) + 0x3000) / 2 + 0x5000;
        if bike.lateral <= o.lateral {
            side = -side;
        }
        let v = o.lateral - o.road.center * 256 + side;
        let lane = ((div_pow(v, 8) / 250 - i32::from(v < 0)) as i8) as i32;
        let right = o.road.lanes[1] >> 8;
        let left = o.road.lanes[0] >> 8;
        if right <= lane || (-lane != left && lane <= -left) {
            side = -side;
        }
        self.set(0x198, o.position + 960);
        self.set(0x194, o.lateral + side);
        if gap > 36 && (ctx.riders[&ctx.own].position - o.position).abs() > 799 {
            let limit = self.curve_limit(ctx);
            self.set(
                0x19c,
                if gap < 75 {
                    limit - div_pow(limit, 2)
                } else {
                    limit
                },
            );
            return true;
        }
        let angle = riding::atan2(
            o.position - ctx.riders[&ctx.own].position,
            div_pow(o.lateral - bike.lateral, 8),
        ) * 65536;
        if angle.abs() < 15 {
            self.set(0x19c, self.curve_limit(ctx) / 2);
        }
        true
    }
    fn speed_steer(&self, ctx: &Context<'_>, bike: &mut handling::Bike, c: &riding::Configuration) {
        let skid = bike.skid as i8;
        let k = if skid < 41 { 1 } else { 4 };
        let lead = -k * bike.lean * 4;
        let error = lead - bike.lateral + self.get(0x194);
        let angle = source_angle(self.get(0x198) - ctx.riders[&ctx.own].position, error / 256);
        #[cfg(test)]
        let angle = if ctx.before_wrap {
            riding::atan2(self.get(0x198) - ctx.riders[&ctx.own].position, error / 256) * 65536
        } else {
            angle
        };
        if bike.contact != 0 || bike.ai_roll.unwrap_or(0).abs() > 13 {
            bike.source_steer(c.spec(), &c.tables, angle * 8, 0);
        }
        let shown = speed_units(bike.speed);
        let want = self.get(0x19c);
        bike.source_drive_mode(
            c.spec(),
            &c.tables,
            if shown < want && (skid < 41 || shown < want / 2) {
                1
            } else if shown <= want {
                0
            } else {
                2
            },
        );
    }
    fn racing(&mut self, ctx: &Context<'_>, bike: &mut handling::Bike, c: &riding::Configuration) {
        if !self.overtake(ctx, bike) {
            self.curve(ctx, bike);
        }
        self.speed_steer(ctx, bike, c);
    }
    fn follow(&mut self, ctx: &Context<'_>, bike: &mut handling::Bike, c: &riding::Configuration) {
        let Some(t) = self.target.and_then(|id| ctx.riders.get(&id)) else {
            self.racing(ctx, bike, c);
            return;
        };
        let own = &ctx.riders[&ctx.own];
        let dpos = t.position - own.position;
        let distance = dpos.abs() >> 4;
        if dpos > 0 && distance > 500 {
            self.racing(ctx, bike, c);
            return;
        }
        if self.overtake(ctx, bike) {
            self.speed_steer(ctx, bike, c);
            return;
        }
        let target_speed = if t.id == ActorId(1) {
            t.captured_speed
        } else {
            t.speed
        };
        let alongside = (distance << 4) < own.length * 4;
        if alongside {
            let mut off = (distance * 4096 + 14000).min(28000);
            let v = off - t.road.center * 256 + t.half_lateral;
            if t.half_lateral < bike.lateral {
                let lane = div_pow(v, 8) / 250 - i32::from(v < 0);
                if lane >= 0 && lane < (t.road.lanes[1] >> 8) {
                    off = -off;
                }
            }
            self.set(0x198, t.position + 960);
            self.set(0x194, t.half_lateral - off);
        } else {
            self.curve(ctx, bike);
        }
        let shown = speed_units(bike.speed);
        let mut d = ((t.position - own.position) >> 4) + 1;
        let mut want = self.get(0x1a4);
        if d <= 500 {
            want = self.get(0x1a0);
            if d == -500 || -d < 500 {
                if d < 0 {
                    d = div_pow(d, 3);
                }
                want = d + speed_units(target_speed);
                if self.get(0x1a4) < want {
                    want = self.get(0x1a4);
                } else if want < self.get(0x1a0) {
                    want = self.get(0x1a0);
                }
            }
        }
        if want - shown <= 5 {
            if shown - want > 5 {
                want = shown - 5;
            }
        } else {
            want = shown + 5;
        }
        self.set(0x19c, want);
        if (target_speed - bike.speed).abs() < 2000 && alongside && bike.speed > 4000 {
            bike.speed = self.get(0x19c) * 100;
        } else if t.crash == 1 {
            self.set(0x19c, 0);
        } else if t.crash != 0 {
            self.set(0x19c, self.get(0x19c).min(20));
        }
        self.speed_steer(ctx, bike, c);
    }
    fn initialize(&mut self, ctx: &Context<'_>) {
        if !self.initialized {
            let max = self.get(0x1a4);
            let min = self.get(0x1a0);
            self.set(0x1a4, 0);
            self.set(0x1a0, 0);
            self.straight(ctx);
            self.replan(ctx, false);
            self.set(0x1a4, max);
            self.set(0x1a0, min);
            self.initialized = true;
        }
    }
    fn begin(
        &mut self,
        ctx: &Context<'_>,
        bike: &mut handling::Bike,
        c: &riding::Configuration,
        animation_code: i8,
        weapon: animation::Weapon,
        random: &mut impl FnMut() -> u32,
    ) -> Result<Decision> {
        self.initialize(ctx);
        let attack_active = c
            .animation
            .attacks
            .iter()
            .flatten()
            .any(|variant| variant[usize::from(self.attack_weapon)] == animation_code);
        self.transition(ctx, bike, attack_active, random);
        let mut attack = None;
        let mut attack_target = None;
        match self.get(0x160) {
            1 | 2 => self.racing(ctx, bike, c),
            3 => {
                self.follow(ctx, bike, c);
                if !attack_active {
                    let kick = (self.get(0x1d0) as u32 & 255) as i8;
                    let backhand = ((self.get(0x1d0) as u32 >> 8) & 255) as i8;
                    if kick == 0 || backhand == 0 {
                        return Err(Error::Invalid("police attack random divisor"));
                    }
                    let _kick_roll = (random() as i32) % i32::from(kick);
                    let back = (random() as i32) % i32::from(backhand) == 0;
                    self.attack_weapon = match weapon {
                        animation::Weapon::Fists => 0,
                        animation::Weapon::Club => 1,
                        animation::Weapon::Chain => 2,
                    };
                    let right = self
                        .target
                        .and_then(|id| ctx.riders.get(&id))
                        .map_or(bike.ai_roll.unwrap_or(0) >= 0, |t| {
                            bike.lateral < t.half_lateral
                        });
                    attack_target = self.target;
                    attack = Some(Request {
                        kind: if back {
                            AttackKind::Backhand
                        } else {
                            AttackKind::Punch
                        },
                        side: if right { Side::Right } else { Side::Left },
                    });
                    let code = c.animation.attacks[usize::from(right)][usize::from(back)]
                        [usize::from(self.attack_weapon)];
                    if code != animation_code
                        && let Some(target) = self.target.and_then(|id| ctx.riders.get(&id))
                    {
                        self.react(target.index, 3)?;
                    }
                }
            }
            4 => self.follow(ctx, bike, c),
            5 => {
                if !self.overtake(ctx, bike) {
                    self.straight(ctx);
                    if let Some(t) = self.target.and_then(|id| ctx.riders.get(&id)) {
                        self.set(0x194, t.lateral);
                    }
                }
                self.speed_steer(ctx, bike, c);
            }
            6 => {
                if !self.overtake(ctx, bike)
                    && let Some(t) = self.target.and_then(|id| ctx.riders.get(&id))
                {
                    self.set(0x19c, self.get(0x1a4));
                    self.set(0x198, t.position);
                    self.set(0x194, t.lateral);
                }
                self.speed_steer(ctx, bike, c);
            }
            _ => {}
        }
        Ok(Decision {
            request: attack,
            target: attack_target,
        })
    }
    fn finish(&mut self, ctx: &Context<'_>, bike: &mut handling::Bike, c: &riding::Configuration) {
        if self.get(0x160) == 3 {
            self.speed_steer(ctx, bike, c);
        }
        let state = self.get(0x160) as usize;
        self.set(0x138 + state * 4, self.get(0x138 + state * 4) + 1);
        self.set(0x188, self.get(0x188) + 1);
        if self.get(0x188) > 59 {
            self.replan(ctx, true);
        }
    }
    #[cfg(test)]
    pub(crate) fn tick(
        &mut self,
        ctx: &Context<'_>,
        bike: &mut handling::Bike,
        c: &riding::Configuration,
        code: i8,
        weapon: animation::Weapon,
        random: &mut impl FnMut() -> u32,
    ) -> Result<Decision> {
        let decision = self.begin(ctx, bike, c, code, weapon, random)?;
        self.finish(ctx, bike, c);
        Ok(decision)
    }
}

fn geometry(
    e: &Encounters,
    c: &riding::Configuration,
    mask: Option<u8>,
    at: Location,
) -> Result<Road> {
    if let Some(roads) = &e.setup.rules.source_roads {
        let mask = mask.ok_or(Error::Invalid("cop geometry route"))?;
        let profile = roads
            .profiles
            .get(usize::from(mask))
            .ok_or(Error::Invalid("cop geometry profile"))?;
        let ground = roads
            .ground
            .get(&at.path)
            .ok_or(Error::Invalid("cop geometry path"))?;
        let sample = (at.progress.max(0) as usize / 256).min(ground.len() - 1);
        let local = sample as u32;
        let id = profile
            .identity(at.path, local)
            .ok_or(Error::Invalid("cop geometry identity"))?;
        let g = ground[sample];
        let lanes = roads
            .lanes_q8
            .as_ref()
            .and_then(|r| r.get(&at.path))
            .and_then(|r| r.get(sample))
            .copied()
            .ok_or(Error::Invalid("missing source cop lane geometry"))?;
        Ok(Road {
            center: id[2],
            left: id[2] + g.left,
            right: id[2] + g.right,
            lanes,
        })
    } else {
        let sample = (i64::from(c.start_sample) + (at.progress >> 8))
            .clamp(0, c.ground.len() as i64 - 1) as usize;
        let g = c.ground[sample];
        Ok(Road {
            center: 0,
            left: g.left,
            right: g.right,
            lanes: [-g.left * 256 / 250, g.right * 256 / 250],
        })
    }
}
fn physical_scan_delta(
    e: &Encounters,
    mask: Option<u8>,
    own_at: Location,
    own_position: i32,
    at: Location,
) -> Option<i64> {
    if let Some(roads) = &e.setup.rules.source_roads {
        let profile = roads.profiles.get(usize::from(mask?))?;
        if let Some(start) = profile.selected.get(&at.path) {
            return Some(i64::from(*start) * 256 + at.progress - i64::from(own_position));
        }
        let local = (at.progress.max(0) / 256) as u32;
        profile.identity(at.path, local)?;
    }
    crate::source_roads::Roads::physical_delta(&e.setup.topology, own_at, at)
}
fn average_curve(ground: &[handling::Ground], at: usize, count: usize, atan: &[u8]) -> i32 {
    let mut sum = 0i64;
    for g in ground.iter().skip(at).take(count) {
        let x = g.bend;
        let v = i32::from(atan[x.unsigned_abs() as usize]) * if x < 0 { -1 } else { 1 };
        let a = v * 16384;
        sum += i64::from(if a > 0x7fffff { a - 0x1000000 } else { a });
    }
    (sum / count as i64) as i32
}
impl World {
    fn source_police_step(
        &mut self,
        e: &mut Encounters,
        actors: &mut BTreeMap<ActorId, Actor>,
        tick: u64,
        pass: Pass,
        mut random: impl FnMut() -> u32,
    ) -> Result<()> {
        let Some(live) = self.police.clone() else {
            return Ok(());
        };
        let id = live.actor;
        let config = self
            .setup
            .police
            .as_ref()
            .ok_or(Error::Invalid("cop AI configuration"))?;
        let unit = e
            .units
            .get(&id)
            .ok_or(Error::Invalid("missing cop AI rider"))?;
        let own_at = unit.recovery.bike_location;
        let own_bike = &actors[&id].view.rider.as_ref().unwrap().bike;
        let c = e
            .configs
            .get(&id)
            .ok_or(Error::Invalid("cop AI riding configuration"))?
            .clone();
        let origin = i64::from(c.start_sample) * 256;
        let own_position =
            i32::try_from(own_bike.distance_q8 + origin).map_err(|_| Error::Overflow)?;
        let mut views = BTreeMap::new();
        let mut objects = vec![];
        for (order, (&actor_id, u)) in e.units.iter().enumerate() {
            let Some(rider) = actors.get(&actor_id).and_then(|a| a.view.rider.as_ref()) else {
                continue;
            };
            let bike_at = u.recovery.bike_location;
            let half = u.recovery.body.as_ref();
            let half_at = half.map_or(bike_at, |b| b.location);
            let project = |at: Location| -> Result<Option<i32>> {
                physical_scan_delta(e, unit.source_route, own_at, own_position, at)
                    .map(|d| {
                        i32::try_from(d)
                            .map_err(|_| Error::Overflow)
                            .and_then(|d| own_position.checked_add(d).ok_or(Error::Overflow))
                    })
                    .transpose()
            };
            let bike_pos = project(bike_at)?;
            let half_pos = project(half_at)?;
            // A graph chaser follows the authoritative half. Its quiet bike can
            // occupy another arm and must not erase the reachable body object.
            let body_proxy =
                e.setup.rules.source_roads.is_some() && actor_id != id && half.is_some();
            let Some(pos) = (if body_proxy { half_pos } else { bike_pos }) else {
                continue;
            };
            let half_speed = half.map_or_else(
                || {
                    rider
                        .bike
                        .contact_velocity
                        .map_or(rider.bike.speed, |v| v[0])
                },
                |b| b.motion.contact_velocity.map_or(b.motion.speed, |v| v[0]),
            );
            let bike_road = bike_pos
                .map(|_| geometry(e, &c, u.source_route, bike_at))
                .transpose()?;
            let half_road = half_pos
                .map(|_| {
                    geometry(
                        e,
                        &c,
                        half.and_then(|b| b.source_route).or(u.source_route),
                        half_at,
                    )
                })
                .transpose()?;
            let road = if body_proxy { half_road } else { bike_road }
                .ok_or(Error::Invalid("cop reachable target geometry"))?;
            let index = u
                .police_ai
                .as_ref()
                .map(AiState::index)
                .or_else(|| u.police_relations.as_ref().map(|r| r.index))
                .ok_or(Error::Invalid("cop AI target personality"))?;
            let present = u.recovery.outcome.is_none();
            let v = RiderView {
                id: actor_id,
                position: pos,
                lateral: if body_proxy {
                    half_at.lateral
                } else {
                    rider.bike.lateral
                },
                speed: rider.bike.speed,
                captured_speed: rider
                    .bike
                    .contact_velocity
                    .map_or(rider.bike.speed, |v| v[0]),
                half_lateral: half.map_or(rider.bike.lateral, |b| b.location.lateral),
                length: e.setup.rules.dimensions[1],
                crash: match u.recovery.phase {
                    Phase::Mounted => 0,
                    Phase::Sliding => 1,
                    Phase::GettingUp | Phase::Standing => 2,
                    _ => 3,
                },
                present,
                hit_until: u.fighter.hit_until as u32,
                index,
                road,
                relations: u.police_relations.clone(),
            };
            if let (Some(bike_pos), Some(bike_road)) = (bike_pos, bike_road) {
                objects.push(ScanObject {
                    key: ObjectKey {
                        actor: Some(actor_id),
                        agent: None,
                        kind: 4,
                    },
                    position: bike_pos,
                    lateral: rider.bike.lateral,
                    speed: v.captured_speed,
                    present,
                    road: bike_road,
                    order: 0x10000000 + order as u64 * 2,
                });
            }
            if let (Some(half_pos), Some(half_road)) = (half_pos, half_road) {
                objects.push(ScanObject {
                    key: ObjectKey {
                        actor: Some(actor_id),
                        agent: None,
                        kind: 8,
                    },
                    position: half_pos,
                    lateral: v.half_lateral,
                    speed: half_speed,
                    present,
                    road: half_road,
                    order: 0x10000001 + order as u64 * 2,
                });
            }
            views.insert(actor_id, v);
        }
        for agent in self
            .agents
            .values()
            .filter(|a| a.rider.is_none() && matches!(a.kind, Kind::Vehicle | Kind::Pedestrian))
        {
            let Some(delta) =
                physical_scan_delta(e, unit.source_route, own_at, own_position, agent.location)
            else {
                continue;
            };
            let pos = own_position
                .checked_add(i32::try_from(delta).map_err(|_| Error::Overflow)?)
                .ok_or(Error::Overflow)?;
            let road = geometry(e, &c, unit.source_route, agent.location)?;
            let vehicle = agent.kind == Kind::Vehicle;
            objects.push(ScanObject {
                key: ObjectKey {
                    actor: None,
                    agent: Some(agent.id),
                    kind: if vehicle { 16 } else { 32 },
                },
                position: pos,
                lateral: agent.location.lateral,
                speed: agent.speed * i32::from(agent.direction),
                present: true,
                road,
                order: if vehicle {
                    0x80000000 + agent.id
                } else {
                    agent.id
                },
            });
        }
        objects.sort_by_key(|o| (o.position, o.order));
        let source_ground = e
            .setup
            .rules
            .source_roads
            .as_ref()
            .map(|r| r.ground(unit.source_route.unwrap()))
            .transpose()?;
        let ground = source_ground.as_deref().unwrap_or(&c.ground);
        let sample = (i64::from(c.start_sample) + (own_bike.distance_q8 >> 8))
            .clamp(0, ground.len() as i64 - 1) as usize;
        let current = views
            .get(&id)
            .ok_or(Error::Invalid("cop absent from own physical scan"))?
            .road;
        let next_index = (sample + 1).min(ground.len() - 1);
        let next = if let Some(roads) = &e.setup.rules.source_roads {
            let mask = unit.source_route.unwrap();
            let at = roads
                .location(mask, i64::from(next_index as u32) * 256, 0, 0, 0)
                .ok_or(Error::Invalid("cop next road geometry"))?;
            geometry(e, &c, Some(mask), at)?
        } else {
            let g = ground[next_index];
            Road {
                center: 0,
                left: g.left,
                right: g.right,
                lanes: [-g.left * 256 / 250, g.right * 256 / 250],
            }
        };
        let context = Context {
            #[cfg(test)]
            before_wrap: false,
            own: id,
            tick: tick as u32,
            busted: e.units[&ActorId(1)].recovery.outcome == Some(Outcome::Arrested)
                || e.units[&ActorId(1)].terminal_request == Some(Outcome::Arrested),
            riders: &views,
            objects: &objects,
            current_road: current,
            next_road: next,
            curve_current: average_curve(ground, sample, 8, &config.atan),
            curve_next: average_curve(ground, next_index, 8, &config.atan),
            curve_limit: average_curve(ground, (sample + 8).min(ground.len() - 1), 4, &config.atan),
            eagerness: config.overtake_eagerness[usize::from(e.setup.rules.level)],
            place: if tick == 0 {
                0
            } else {
                let player = &e.units[&ActorId(1)];
                let position = player.race_progress.unwrap_or(
                    actors[&ActorId(1)]
                        .view
                        .rider
                        .as_ref()
                        .unwrap()
                        .bike
                        .distance_q8,
                );
                1 + e
                    .units
                    .iter()
                    .filter(|(other, u)| {
                        **other != ActorId(1)
                            && !u.police
                            && u.race_progress.unwrap_or(
                                actors[other].view.rider.as_ref().unwrap().bike.distance_q8,
                            ) > position
                    })
                    .count() as i32
            },
        };
        let mut brain = e
            .units
            .get_mut(&id)
            .unwrap()
            .police_ai
            .take()
            .ok_or(Error::Invalid("missing authoritative cop brain"))?;
        if pass == Pass::Initialize {
            brain.initialize(&context);
            e.units.get_mut(&id).unwrap().police_ai = Some(brain);
            return Ok(());
        }
        let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
        let old_code = rider.animation.code;
        let decision = match pass {
            Pass::Begin => brain.begin(
                &context,
                &mut rider.bike,
                &c,
                old_code,
                rider.weapon,
                &mut random,
            )?,
            Pass::Finish => {
                brain.finish(&context, &mut rider.bike, &c);
                Decision {
                    request: None,
                    target: None,
                }
            }
            Pass::Initialize => unreachable!(),
        };
        let attack = decision.request;
        let attack_target = decision.target;
        let attack_weapon = brain.attack_weapon;
        let brain_index = brain.index();
        e.units.get_mut(&id).unwrap().police_ai = Some(brain);
        if let Some(request) = attack
            && let Some(target) = attack_target
        {
            let back = request.kind == AttackKind::Backhand;
            let right = request.side == Side::Right;
            let code = c.animation.attacks[usize::from(right)][usize::from(back)]
                [usize::from(attack_weapon)];
            if old_code != code {
                if let Some(relation) = views.get(&target).and_then(|t| t.relations.as_ref()) {
                    let at = usize::from(brain_index) * 3;
                    if relation.target[at + 1] - relation.target[at] + relation.target[at + 2]
                        == relation.current[at + 1] - relation.current[at]
                            + relation.current[at + 2]
                    {
                        e.police_relationship(target, id, 1)?;
                    }
                }
                e.police_relationship(target, id, 0)?;
            }
        }
        let phase = e.units[&id].recovery.phase;
        let unit = e.units.get_mut(&id).unwrap();
        if phase == Phase::Mounted && pass == Pass::Finish {
            unit.fighter.stats.regenerate();
        }
        if pass != Pass::Finish {
            let previous = actors[&id].input;
            actors.get_mut(&id).unwrap().input = Command {
                attack,
                impact: previous.impact,
                terminal: previous.terminal,
                damage: previous.damage,
                walk: previous.walk,
                ..Default::default()
            };
        }
        let state = unit.police_ai.as_ref().unwrap();
        let siren = phase == Phase::Mounted
            && !(matches!(state.get(0x160), 3 | 4) && state.target.is_none());
        let agent = self
            .agents
            .get_mut(&live.agent)
            .ok_or(Error::Invalid("missing cop mirror"))?;
        agent.state = if phase != Phase::Mounted {
            AgentState::Recovering
        } else if state.target.is_some() {
            AgentState::Chase
        } else {
            AgentState::Cruise
        };
        if agent.siren != siren {
            agent.siren = siren;
            self.cues.push(if siren {
                Cue::SirenStart(agent.id)
            } else {
                Cue::SirenStop(agent.id)
            });
        }
        if let Some(target) = state.target.and_then(|id| e.units.get(&id))
            && let Some(portal) = e
                .setup
                .topology
                .leg(own_at.path, target.recovery.bike_location.path)
        {
            self.navigation.insert(id, portal);
        }
        Ok(())
    }
}

pub(crate) fn spawn_lateral(
    e: &Encounters,
    c: &riding::Configuration,
    mask: Option<u8>,
    at: Location,
    lane: u8,
    config: &police::Config,
) -> Result<i32> {
    if lane > 2 {
        return Err(Error::Invalid("police spawn lane"));
    }
    let (ground, global, next_at) = if let Some(roads) = &e.setup.rules.source_roads {
        let mask = mask.ok_or(Error::Invalid("police spawn route"))?;
        let profile = roads
            .profiles
            .get(usize::from(mask))
            .ok_or(Error::Invalid("police spawn route profile"))?;
        let first = profile
            .selected
            .get(&at.path)
            .ok_or(Error::Invalid("police spawn physical node"))?;
        let global = i64::from(*first) * 256 + at.progress;
        let ground = roads.ground(mask)?;
        let next_sample = ((global >> 8) + 1).clamp(0, ground.len() as i64 - 1);
        let next = roads
            .location(mask, next_sample * 256, 0, 0, 0)
            .ok_or(Error::Invalid("police spawn next sample"))?;
        (ground, global, next)
    } else {
        let global = i64::from(c.start_sample) * 256 + at.progress;
        let mut next = at;
        next.progress = ((at.progress >> 8) + 1) * 256;
        (c.ground.clone(), global, next)
    };
    let sample = (((global >> 8) + 1).clamp(0, ground.len() as i64 - 1)) as usize;
    let road = geometry(e, c, mask, next_at)?;
    let count = road.lanes[1] >> 8;
    let lane = match lane {
        1 => (count - 1) / 2,
        2 => (count - 1).abs(),
        _ => 0,
    };
    Ok(div_pow(average_curve(&ground, sample, 8, &config.atan), 3)
        + (road.center + lane * 250) * 256)
}

/// Bounded functional graph visibility. Source Ring split/merge walks are not
/// available; a local sample of zero is an interior road point, not null data.
pub(crate) fn within_road_window(
    topology: &Topology,
    from: Location,
    to: Location,
    range: i64,
) -> bool {
    let Some(a) = topology.paths.get(&from.path) else {
        return false;
    };
    let Some(b) = topology.paths.get(&to.path) else {
        return false;
    };
    if range < 0
        || !(0..=a.length).contains(&from.progress)
        || !(0..=b.length).contains(&to.progress)
    {
        return false;
    }
    let mut pending = std::collections::VecDeque::from([(from.path, from.progress, range)]);
    let mut seen = BTreeMap::new();
    while let Some((path, start, remaining)) = pending.pop_front() {
        if seen
            .get(&(path, start))
            .is_some_and(|old| *old >= remaining)
        {
            continue;
        }
        seen.insert((path, start), remaining);
        if path == to.path && (to.progress - start).abs() <= remaining {
            return true;
        }
        let own = &topology.paths[&path];
        for portal in &own.portals {
            let distance = if portal.end {
                own.length - start
            } else {
                start
            };
            if distance > remaining {
                continue;
            }
            let target = &topology.paths[&portal.target];
            pending.push_back((
                portal.target,
                if portal.target_end { target.length } else { 0 },
                remaining - distance,
            ));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (riding::Configuration, Encounters) {
        let mut c = fixtures::riding();
        c.motion_version = 3;
        c.start_sample = 0;
        c.ground = vec![handling::Ground::flat(); 40];
        let mut setup = encounter::Setup::authored(&c).unwrap();
        setup.rules.contact_walk_version = 1;
        setup.rules.scenery_version = 1;
        setup.rules.collision_version = 2;
        setup.rules.combat_version = 4;
        setup.rules.collision = Some(collision::Profile {
            mounted: [10, 4, 10],
            fallen: [10, 4, 4],
            mass: 256,
            elasticity: 192,
            rider: None,
        });
        setup.seeds.truncate(1);
        setup.seeds[0].distance = 128;
        setup.seeds[0].speed = 0;
        let mut other = setup.seeds[0].clone();
        other.id = ActorId(2);
        other.distance = 256;
        setup.seeds.push(other);
        let mut paths = BTreeMap::new();
        for path in 0..4 {
            let mut portals = vec![];
            if path > 0 {
                portals.push(Portal {
                    end: false,
                    target: path - 1,
                    target_end: true,
                });
            }
            if path < 3 {
                portals.push(Portal {
                    end: true,
                    target: path + 1,
                    target_end: false,
                });
            }
            paths.insert(
                path,
                topology::Path {
                    length: 2560,
                    branch: 0,
                    selected_start: Some(i64::from(path) * 2560),
                    portals,
                    rises: vec![0; 10],
                },
            );
        }
        paths.insert(
            99,
            topology::Path {
                length: 2560,
                branch: 0,
                selected_start: None,
                portals: vec![],
                rises: vec![0; 10],
            },
        );
        setup.topology = Topology { paths };
        setup.rules.source_roads = Some(crate::source_roads::Roads {
            version: 1,
            initial: 0,
            lanes_q8: None,
            ground: (0..4)
                .chain([99])
                .map(|path| (path, vec![handling::Ground::flat(); 10]))
                .collect(),
            forks: BTreeMap::new(),
            profiles: vec![crate::source_roads::Profile {
                finish_progress: None,
                choices: vec![],
                selected: (0..4).map(|p| (p, p * 10)).collect(),
                identities: (0..4)
                    .map(|p| {
                        (
                            p,
                            (0..10)
                                .map(|local| [local as i32, (p * 10 + local) as i32, 0, 3])
                                .collect(),
                        )
                    })
                    .collect(),
            }],
        });
        let sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
        (c, sim.encounter.unwrap())
    }
    #[test]
    fn cop_scan_crosses_multiple_owned_sections_and_excludes_disconnected_nodes() {
        let (_, e) = fixture();
        let own = Location {
            path: 0,
            progress: 128,
            ..Default::default()
        };
        let target = Location {
            path: 3,
            progress: 64,
            ..Default::default()
        };
        assert_eq!(
            crate::source_roads::Roads::physical_delta(&e.setup.topology, own, target),
            None
        );
        assert_eq!(
            physical_scan_delta(&e, Some(0), own, 128, target),
            Some(3 * 2560 + 64 - 128)
        );
        assert_eq!(
            physical_scan_delta(&e, Some(0), own, 128, Location { path: 99, ..target }),
            None
        );
    }
    #[test]
    fn raw_lanes_are_optional_for_old_worlds_and_validate_every_owned_sample() {
        let (_, e) = fixture();
        let mut roads = e.setup.rules.source_roads.unwrap();
        assert!(!serde_json::to_string(&roads).unwrap().contains("lanes_q8"));
        roads.lanes_q8 = Some(
            roads
                .ground
                .keys()
                .map(|path| (*path, vec![[512; 2]; 10]))
                .collect(),
        );
        roads.validate(&e.setup.topology).unwrap();
        roads.lanes_q8.as_mut().unwrap().get_mut(&3).unwrap().pop();
        assert!(roads.validate(&e.setup.topology).is_err());
        roads
            .lanes_q8
            .as_mut()
            .unwrap()
            .insert(3, vec![[65537, 512]; 10]);
        assert!(roads.validate(&e.setup.topology).is_err());
    }
    #[test]
    fn cop_relationship_callbacks_are_immediate_and_authoritative_across_serialization() {
        let (_, mut e) = fixture();
        let mut words = vec![0; 129];
        words[0x164 / 4] = 9 << 8;
        words[0x1d0 / 4] = 0x0101;
        words[0xf0 / 4 + 9] = 11;
        words[0xf0 / 4 + 10] = -22;
        words[0xf0 / 4 + 11] = 33;
        assert!(!serde_json::to_string(&e).unwrap().contains("police_ai"));
        e.units.get_mut(&ActorId(2)).unwrap().police_ai =
            Some(AiState::from_words(&words, 0, 0, animation::Weapon::Club).unwrap());
        e.units.get_mut(&ActorId(2)).unwrap().police = true;
        words[0x164 / 4] = 3 << 8;
        e.units.get_mut(&ActorId(1)).unwrap().police_relations =
            Some(Relations::from_words(&words).unwrap());
        e.police_relationship(ActorId(2), ActorId(1), 3).unwrap();
        assert_eq!(
            &e.units[&ActorId(2)].police_ai.as_ref().unwrap().words[9..12],
            &[11, -22, 33]
        );
        let bytes = serde_json::to_vec(&e).unwrap();
        let saved: Encounters = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&saved).unwrap(), bytes);
        saved.units[&ActorId(2)]
            .police_ai
            .as_ref()
            .unwrap()
            .validate()
            .unwrap();
    }
}
