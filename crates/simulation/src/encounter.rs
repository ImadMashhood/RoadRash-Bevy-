//! Ordered schema-4 contact/combat/recovery systems. No Bevy, filesystem or presentation RNG.
use crate::{
    combat::{
        self, AttackKind, Combatant, Definition, Effect, EffectKind, Inventory, Position, Request,
        Side, Stats, Swing,
    },
    recovery::{self, Assists, BodyLaw, Impact, Outcome, Phase, State},
    riding::Configuration,
    topology::{Location, Topology},
    *,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObstacleKind {
    Decorative,
    Solid,
    Rail,
    Cliff,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Obstacle {
    pub id: u64,
    pub path: u32,
    pub progress: i64,
    pub half_length: i32,
    pub left: i32,
    pub right: i32,
    pub height: i32,
    pub kind: ObstacleKind,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub level: u8,
    pub reach: [i32; 4],
    pub damage: [i32; 4],
    pub immunity: u16,
    pub stun: u16,
    pub dimensions: [i32; 3],
    pub body: BodyLaw,
    pub assists: Assists,
    pub bike_contacts: bool,
    pub finish_progress: Option<i64>,
    pub obstacles: Vec<Obstacle>,
}
impl Rules {
    pub fn from_library(c: &Configuration, level: u8) -> Result<Self> {
        if level > 4 {
            return Err(Error::Invalid("combat level"));
        }
        let stun = c.animation.banks[0]
            .get(9)
            .and_then(Option::as_ref)
            .map_or(12, |v| v.iter().map(|f| f.duration as u16).sum::<u16>() + 1);
        Ok(Self {
            level,
            reach: [5 * 4096, 7 * 4096, 8 * 4096, 5 * 4096],
            damage: [256, 320, 384, 64],
            immunity: u16::from(5 - level) * 6,
            stun,
            dimensions: [7680, 64, 8960],
            body: BodyLaw::default(),
            assists: Assists::default(),
            bike_contacts: true,
            finish_progress: None,
            obstacles: vec![],
        })
    }
    pub fn validate(&self, topology: &Topology) -> Result<()> {
        self.body.validate()?;
        if self.level > 4
            || self.stun == 0
            || self.stun > 10000
            || self
                .finish_progress
                .is_some_and(|p| !(0..=256_000_000).contains(&p))
            || self.immunity > 10000
            || self.reach.iter().any(|v| !(1..=256000).contains(v))
            || self.damage.iter().any(|v| !(1..=4096).contains(v))
            || self.dimensions.iter().any(|v| !(1..=65536).contains(v))
            || self.obstacles.len() > 100000
        {
            return Err(Error::Invalid("combat/contact rules"));
        }
        let mut ids = BTreeSet::new();
        for o in &self.obstacles {
            if o.id == 0
                || o.id >= (1u64 << 63)
                || !ids.insert(o.id)
                || !topology.paths.contains_key(&o.path)
                || o.progress < 0
                || o.progress > topology.paths[&o.path].length
                || o.left >= o.right
                || o.half_length <= 0
                || o.half_length > 1024
                || o.height < 0
                || o.height > 1_000_000
                || [o.left, o.right]
                    .iter()
                    .any(|v| v.unsigned_abs() > 256_000_000)
            {
                return Err(Error::Invalid("obstacle identity/geometry"));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Seed {
    pub id: ActorId,
    pub distance: i64,
    pub lateral: i32,
    pub speed: i32,
    pub location: Option<Location>,
    pub stats: Stats,
    pub color: u8,
    pub drive: bool,
}
impl Seed {
    pub fn player() -> Self {
        Self {
            id: ActorId(1),
            distance: 0,
            lateral: 0,
            speed: 0,
            location: None,
            stats: Stats::default(),
            color: 0,
            drive: false,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub topology: Topology,
    pub rules: Rules,
    pub seeds: Vec<Seed>,
    pub inventory: Inventory,
}
impl Setup {
    pub fn authored(c: &Configuration) -> Result<Self> {
        Ok(Self {
            topology: Topology::straight(
                c.ground.len() as i64 * 256 - i64::from(c.start_sample) * 256,
            ),
            rules: Rules::from_library(c, 0)?,
            seeds: vec![Seed::player()],
            inventory: Inventory::default(),
        })
    }
    pub fn validate(&self, c: &Configuration) -> Result<()> {
        self.topology.validate()?;
        self.rules.validate(&self.topology)?;
        self.inventory.validate()?;
        if self.seeds.is_empty() || self.seeds.len() > 256 {
            return Err(Error::Invalid("encounter actor count"));
        }
        for (i, s) in self.seeds.iter().enumerate() {
            s.stats.validate()?;
            if s.id.0 != i as u64 + 1
                || s.color >= 15
                || !(0..=1000000).contains(&s.speed)
                || s.lateral.unsigned_abs() > 256_000_000
                || !self.topology.valid(self.location(s)?)
            {
                return Err(Error::Invalid("encounter actor seed"));
            }
        }
        if self.inventory.items.values().any(|item| {
            item.owner
                .is_some_and(|owner| !self.seeds.iter().any(|s| s.id == owner))
        }) {
            return Err(Error::Invalid("weapon owner is not an actor"));
        }
        for kind in [AttackKind::Punch, AttackKind::Kick, AttackKind::Backhand] {
            for side in [Side::Left, Side::Right] {
                for weapon in [
                    animation::Weapon::Fists,
                    animation::Weapon::Club,
                    animation::Weapon::Chain,
                ] {
                    Definition::from_library(&c.animation, Request { kind, side }, weapon)?;
                }
            }
        }
        Ok(())
    }
    fn location(&self, s: &Seed) -> Result<Location> {
        s.location
            .or_else(|| self.topology.locate(s.distance, s.lateral, 0))
            .ok_or(Error::Invalid("spawn outside source topology"))
    }
    pub fn source(
        c: &Configuration,
        course: &rr_course::Course,
        route: &rr_course::route::Route,
        exe: &[u8],
        level: u8,
        practice: u8,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        if practice > 14 {
            return Err("practice riders must be 0..14".into());
        }
        let mut setup = Self {
            topology: Topology::from_course(course, route, c.start_sample)?,
            rules: Rules::from_library(c, level)?,
            seeds: vec![Seed::player()],
            inventory: Inventory::default(),
        };
        setup.rules.finish_progress =
            Some((route.finish(course, level + 1)? as i64 - i64::from(c.start_sample)) * 256);
        setup.rules.body = BodyLaw::from_exe(exe)?;
        let pe = rr_formats::pe::SupportedExe::read(exe)?;
        let d = pe.read_va(0x4643e0, 12)?;
        for (i, value) in setup.rules.dimensions.iter_mut().enumerate() {
            *value = rr_formats::binary::u32le(d, i * 4)? as i32;
        }
        let raw = pe.read_va(0x463c78 + 0x30, 16)?;
        let w = |i| -> std::result::Result<i32, Box<dyn std::error::Error>> {
            Ok(rr_formats::binary::u32le(raw, i * 4)? as i32)
        };
        let source_stats = Stats {
            strength: w(0usize)?,
            base: w(1usize)?,
            maximum: w(2usize)?,
            health: w(3usize)?,
        };
        source_stats.validate()?;
        setup.seeds[0].stats = source_stats;
        for i in 0..practice {
            let mut s = Seed::player();
            s.id = ActorId(u64::from(i) + 2);
            s.distance = i64::from(i / 2) * 96 + 16;
            s.lateral = if i % 2 == 0 { 18000 } else { -18000 };
            s.stats = source_stats;
            s.color = (i + 1) % 15;
            s.drive = true;
            setup.seeds.push(s);
            if i < 2 {
                setup.inventory.items.insert(
                    combat::WeaponId(u64::from(i) + 1),
                    combat::Item {
                        kind: if i == 0 {
                            animation::Weapon::Club
                        } else {
                            animation::Weapon::Chain
                        },
                        owner: Some(ActorId(u64::from(i) + 2)),
                    },
                );
            }
        }
        let placements = rr_course::scenery::place(course, level + 1, 1)?;
        for instance in placements
            .instances
            .iter()
            .filter(|i| i.kind == "billboard" && i.raw[0] & 0x80000000 == 0)
        {
            let kind = if instance.raw[0] & 15 < 3 {
                ObstacleKind::Solid
            } else {
                ObstacleKind::Decorative
            };
            for (id, node) in course.nodes.iter().filter(|(_, n)| {
                n.kind == rr_course::NodeKind::Road
                    && n.link == instance.section
                    && instance.sample < n.length
            }) {
                let center = instance.lateral.unwrap_or(0) * 256;
                setup.rules.obstacles.push(Obstacle {
                    id: setup.rules.obstacles.len() as u64 + 1,
                    path: *id,
                    progress: i64::from(instance.sample) * 256 + 128,
                    half_length: 16,
                    left: center - 0x800,
                    right: center + 0x800,
                    height: 0x2300,
                    kind,
                });
                let _ = node;
            }
        }
        setup.validate(c)?;
        Ok(setup)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Unit {
    pub fighter: Combatant,
    pub recovery: State,
    pub previous: Position,
    pub previous_location: Location,
    pub completed_at: Option<u64>,
    pub color: u8,
    pub drive: bool,
    pub terminal_request: Option<Outcome>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub setup: Setup,
    pub units: BTreeMap<ActorId, Unit>,
    pub inventory: Inventory,
    pub active_contacts: BTreeSet<(u64, u64)>,
    #[serde(skip)]
    pub(crate) configs: BTreeMap<ActorId, Configuration>,
    #[serde(skip)]
    pub(crate) navigation: BTreeMap<ActorId, crate::topology::Portal>,
}
impl World {
    pub(crate) fn new(
        setup: Setup,
        c: &Configuration,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
    ) -> Result<Self> {
        setup.validate(c)?;
        let mut units = BTreeMap::new();
        for seed in &setup.seeds {
            let a = actors
                .get_mut(&seed.id)
                .ok_or(Error::MissingActor(seed.id))?;
            let at = setup.location(seed)?;
            let rider = a
                .view
                .rider
                .as_mut()
                .ok_or(Error::Invalid("missing source rider"))?;
            rider.bike.distance_q8 = setup.topology.projected(at).unwrap_or(seed.distance);
            rider.bike.lateral = at.lateral;
            rider.bike.height = at.height;
            rider.bike.speed = seed.speed;
            if let Some((_, w)) = setup.inventory.carried(seed.id) {
                rider.weapon = w;
            }
            units.insert(
                seed.id,
                Unit {
                    fighter: Combatant {
                        stats: seed.stats,
                        ..Default::default()
                    },
                    recovery: State::mounted(seed.id, at),
                    previous_location: at,
                    completed_at: None,
                    previous: Position {
                        route: 1,
                        branch: setup.topology.paths[&at.path].branch,
                        path: at.path,
                        progress: seed.distance,
                        lateral: at.lateral,
                        height: at.height,
                    },
                    color: seed.color,
                    drive: seed.drive,
                    terminal_request: None,
                },
            );
        }
        Ok(Self {
            inventory: setup.inventory.clone(),
            setup,
            units,
            active_contacts: BTreeSet::new(),
            configs: BTreeMap::new(),
            navigation: BTreeMap::new(),
        })
    }
    pub(crate) fn add(&mut self, id: ActorId, rider: &riding::Rider) -> Result<()> {
        if self.units.len() >= 256 {
            return Err(Error::Invalid("encounter actor limit"));
        }
        let at = self
            .setup
            .topology
            .locate(
                rider.bike.distance_q8,
                rider.bike.lateral,
                rider.bike.height,
            )
            .ok_or(Error::Invalid("spawn topology"))?;
        self.units.insert(
            id,
            Unit {
                fighter: Combatant::default(),
                recovery: State::mounted(id, at),
                previous_location: at,
                completed_at: None,
                previous: Position {
                    route: 1,
                    branch: self.setup.topology.paths[&at.path].branch,
                    path: at.path,
                    progress: rider.bike.distance_q8,
                    lateral: rider.bike.lateral,
                    height: rider.bike.height,
                },
                color: (id.0 % 15) as u8,
                drive: false,
                terminal_request: None,
            },
        );
        Ok(())
    }
    pub(crate) fn remove(&mut self, id: ActorId) {
        self.inventory.drop(id);
        self.units.remove(&id);
        self.configs.remove(&id);
        self.navigation.remove(&id);
        self.active_contacts
            .retain(|(a, b)| *a != id.0 && *b != id.0);
    }
    fn position(&self, id: ActorId, actors: &BTreeMap<ActorId, crate::Actor>) -> Option<Position> {
        let u = self.units.get(&id)?;
        let bike = &actors.get(&id)?.view.rider.as_ref()?.bike;
        let at = u.recovery.bike_location;
        Some(Position {
            route: 1,
            branch: self.setup.topology.paths.get(&at.path)?.branch,
            path: at.path,
            progress: self
                .setup
                .topology
                .projected(at)
                .unwrap_or(bike.distance_q8),
            lateral: bike.lateral,
            height: bike.height,
        })
    }
    fn emit(
        events: &mut Vec<Effect>,
        tick: u64,
        actor: ActorId,
        other: Option<ActorId>,
        kind: EffectKind,
    ) {
        events.push(Effect {
            tick,
            actor,
            other,
            kind,
        });
    }
    pub(crate) fn movement(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        let positions: BTreeMap<_, _> = self
            .units
            .keys()
            .filter_map(|id| self.position(*id, actors).map(|p| (*id, p)))
            .collect();
        let eligible: BTreeSet<_> = self
            .units
            .iter()
            .filter(|(_, u)| {
                u.recovery.phase == Phase::Mounted
                    && u.recovery.outcome.is_none()
                    && u.fighter.stats.health > 0
            })
            .map(|(id, _)| *id)
            .collect();
        for (id, u) in &mut self.units {
            let c = self.configs.get(id).unwrap_or(c);
            let a = actors.get_mut(id).ok_or(Error::MissingActor(*id))?;
            let r = a
                .view
                .rider
                .as_mut()
                .ok_or(Error::Invalid("encounter rider"))?;
            u.previous = positions[id];
            u.previous_location = u.recovery.bike_location;
            u.terminal_request = if u.completed_at.is_some() {
                None
            } else {
                a.input.terminal
            };
            if u.recovery.outcome.is_some() {
                if let Some(state) = &mut r.actions {
                    state.active = false;
                }
                continue;
            }
            if let Some(state) = &mut r.actions {
                let pressed = a.input.dismount && !state.dismount_held;
                state.dismount_held = a.input.dismount;
                if u.recovery.phase != Phase::Mounted {
                    state.active = false;
                } else if pressed && r.bike.speed.abs() < 300 && r.bike.height < 101 {
                    state.active = false;
                    u.fighter.swing = None;
                    u.recovery.begin(
                        *id,
                        &mut r.bike,
                        c,
                        u.recovery.bike_location,
                        Impact {
                            reason: 4,
                            rider_velocity: Some([0, 0, 0]),
                            bike_velocity: Some([0, 0, 0]),
                            ..Default::default()
                        },
                        &self.setup.topology,
                    );
                }
            }
            if u.recovery.phase != Phase::Mounted {
                // Contacts follow movement. Reserve an interrupted recovery transition
                // so a same-tick impact cannot complete a mount and discard its body.
                if a.input.impact.is_some() {
                    continue;
                }
                let recovery_events = u.recovery.step(
                    &mut r.bike,
                    c,
                    &self.setup.topology,
                    &self.setup.rules.body,
                    self.setup.rules.assists,
                    a.input.walk,
                    tick as u32,
                )?;
                for event in recovery_events {
                    let kind = match event {
                        recovery::RecoveryEvent::GetUp => EffectKind::GetUp,
                        recovery::RecoveryEvent::Remount => {
                            u.fighter.stats.health = u.fighter.stats.maximum;
                            r.pose = animation::Pose::Ride;
                            r.animation.set(&c.animation, 0, 2);
                            EffectKind::Remount
                        }
                        recovery::RecoveryEvent::Rescue => EffectKind::Rescue,
                        recovery::RecoveryEvent::Wrecked => continue,
                    };
                    Self::emit(events, tick, *id, None, kind);
                }
                continue;
            }
            u.fighter.stats.regenerate();
            if u.fighter
                .swing
                .as_ref()
                .is_some_and(|s| tick - s.started >= u64::from(s.definition.duration))
            {
                u.fighter.swing = None;
                r.pose = animation::Pose::Ride;
            }
            let carried = self
                .inventory
                .carried(*id)
                .map_or(animation::Weapon::Fists, |(_, w)| w);
            if a.input.weapon == Some(animation::Weapon::Fists)
                && self.inventory.drop(*id).is_some()
            {
                u.fighter.swing = None;
                r.pose = animation::Pose::Ride;
                r.animation.set(&c.animation, 0, 2);
                Self::emit(events, tick, *id, None, EffectKind::Dropped);
            }
            r.weapon = self
                .inventory
                .carried(*id)
                .map_or(animation::Weapon::Fists, |(_, w)| w);
            let request = a
                .input
                .attack
                .or_else(|| a.input.pose.and_then(Request::from_pose));
            let mut command = a.input;
            command.pose = None;
            command.weapon = None;
            command.damage = 0;
            if let Some(mut request) =
                request.filter(|_| u.fighter.swing.is_none() && tick >= u.fighter.stun_until)
            {
                let chosen = combat::target(
                    *id,
                    positions[id],
                    positions
                        .iter()
                        .filter(|(other, p)| {
                            eligible.contains(other)
                                && (request.side == Side::Auto
                                    || ((positions[id].lateral < p.lateral)
                                        == (request.side == Side::Right)))
                        })
                        .map(|(id, p)| (*id, *p)),
                );
                if request.side == Side::Auto {
                    request.side = if chosen
                        .is_some_and(|other| positions[id].lateral < positions[&other].lateral)
                    {
                        Side::Right
                    } else {
                        Side::Left
                    };
                }
                let definition = Definition::from_library(&c.animation, request, r.weapon)?;
                command.pose = Some(combat::pose(request));
                u.fighter.swing = Some(Swing {
                    request,
                    target: chosen,
                    weapon: r.weapon,
                    started: tick,
                    definition,
                    resolved: false,
                    swish: false,
                });
            }
            if u.fighter.swing.is_none()
                && r.pose == animation::Pose::HitReact
                && r.animation.code == 2
            {
                r.pose = animation::Pose::Ride;
            }
            if u.drive {
                command.throttle = true;
            }
            r.step(c, command, tick as u32)?;

            let delta = r.bike.distance_q8 - positions[id].progress;
            self.setup.topology.advance(
                &mut u.recovery.bike_location,
                delta,
                self.navigation.get(id).copied(),
            )?;
            u.recovery.bike_location.lateral = r.bike.lateral;
            u.recovery.bike_location.height = r.bike.height;
            if r.bike.crashed {
                u.fighter.swing = None;
                u.recovery.begin(
                    *id,
                    &mut r.bike,
                    c,
                    u.recovery.bike_location,
                    Impact::default(),
                    &self.setup.topology,
                );
                Self::emit(events, tick, *id, None, EffectKind::Crash);
            }
            let _ = carried;
        }
        Ok(())
    }
    // One transaction carries its tick and presentation-only event sink explicitly.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn hit(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        target: ActorId,
        impact: Impact,
        tick: u64,
        force: bool,
        events: &mut Vec<Effect>,
    ) -> Result<bool> {
        let c = self.configs.get(&target).unwrap_or(c);
        let unit = self
            .units
            .get_mut(&target)
            .ok_or(Error::MissingActor(target))?;
        if unit.recovery.outcome.is_some() {
            return Ok(false);
        }
        let rider = actors
            .get_mut(&target)
            .and_then(|a| a.view.rider.as_mut())
            .ok_or(Error::MissingActor(target))?;
        let damage = combat::reason_damage(i32::from(impact.stamina_damage), impact.reason);
        let bike_damage = combat::reason_damage(i32::from(impact.bike_damage), impact.reason);
        unit.fighter.stats.hit(damage);
        rider.bike.damage(bike_damage.max(0) as u32);
        unit.fighter.hit_count = unit.fighter.hit_count.saturating_add(1);
        unit.fighter.swing = None;
        let down = force
            || unit.fighter.stats.health == 0
            || rider.bike.condition == 0
            || unit.recovery.phase != Phase::Mounted;
        if down {
            events.retain(|e| {
                e.actor != target || !matches!(e.kind, EffectKind::Remount | EffectKind::GetUp)
            });
            unit.recovery.begin(
                target,
                &mut rider.bike,
                c,
                unit.recovery.bike_location,
                impact,
                &self.setup.topology,
            );
            Self::emit(events, tick, target, None, EffectKind::Crash);
        } else {
            unit.fighter.stun_until = tick + u64::from(self.setup.rules.stun);
            rider.pose = animation::Pose::HitReact;
            rider.animation.set(&c.animation, 0, 9);
            rider.animation.queued = Some(2);
        }
        Ok(down)
    }
    pub(crate) fn contacts(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        let mut active = BTreeSet::new();
        let ids: Vec<_> = self.units.keys().copied().collect();
        for id in &ids {
            let input = actors[id].input;
            if let Some(impact) = input.impact {
                self.hit(actors, c, *id, impact, tick, true, events)?;
            } else if input.damage > 0 {
                self.hit(
                    actors,
                    c,
                    *id,
                    Impact {
                        reason: 1,
                        bike_damage: input.damage.min(u32::from(u16::MAX)) as u16,
                        ..Default::default()
                    },
                    tick,
                    false,
                    events,
                )?;
            }
        }
        let [width, length, height] = self.setup.rules.dimensions;
        if self.setup.rules.bike_contacts {
            for (i, a) in ids.iter().enumerate() {
                for b in ids.iter().skip(i + 1) {
                    let pa = self.position(*a, actors).unwrap();
                    let pb = self.position(*b, actors).unwrap();
                    if !pa.same_road(pb)
                        || (pa.progress - pb.progress).abs() >= i64::from(length) * 2
                        || (i64::from(pa.lateral) - i64::from(pb.lateral)).abs()
                            >= i64::from(width) * 2
                        || (i64::from(pa.height) - i64::from(pb.height)).abs()
                            >= i64::from(height) * 2
                    {
                        continue;
                    }
                    let key = (a.0, b.0);
                    active.insert(key);
                    if self.active_contacts.contains(&key) {
                        continue;
                    }
                    let av = actors[a].view.rider.as_ref().unwrap().bike.speed;
                    let bv = actors[b].view.rider.as_ref().unwrap().bike.speed;
                    let relative = (av - bv).abs();
                    let overlap = width * 2 - (pa.lateral - pb.lateral).abs();
                    let sign = if pa.lateral <= pb.lateral { -1 } else { 1 };
                    let push = overlap / 2 + 3;
                    for (id, s) in [(*a, sign), (*b, -sign)] {
                        let r = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
                        r.bike.lateral = r.bike.lateral.saturating_add(s * push);
                        self.units
                            .get_mut(&id)
                            .unwrap()
                            .recovery
                            .bike_location
                            .lateral = r.bike.lateral;
                    }
                    if relative > 6000 {
                        let faster = if av >= bv { *a } else { *b };
                        self.hit(
                            actors,
                            c,
                            faster,
                            Impact {
                                reason: 1,
                                rider_velocity: Some([0, 0, 512]),
                                bike_velocity: Some([0, 0, 256]),
                                ..Default::default()
                            },
                            tick,
                            true,
                            events,
                        )?;
                    } else {
                        let transfer = (av + bv) / 2;
                        for id in [*a, *b] {
                            actors
                                .get_mut(&id)
                                .unwrap()
                                .view
                                .rider
                                .as_mut()
                                .unwrap()
                                .bike
                                .speed = transfer;
                        }
                    }
                    Self::emit(events, tick, *a, Some(*b), EffectKind::Contact);
                }
            }
        }
        for id in ids {
            if self.units[&id].recovery.outcome.is_some() {
                continue;
            }
            let p = self.position(id, actors).unwrap();
            let previous = self.units[&id].previous_location;
            let current = self.units[&id].recovery.bike_location;
            let previous_progress = if previous.path == current.path {
                previous.progress
            } else {
                current.progress
            };
            let hazard_hits: Vec<_> = self
                .setup
                .rules
                .obstacles
                .iter()
                .filter(|o| {
                    o.kind != ObstacleKind::Decorative
                        && o.path == p.path
                        && p.height < o.height
                        && (current.progress.min(previous_progress) - i64::from(length)
                            < o.progress + i64::from(o.half_length))
                        && (o.progress - i64::from(o.half_length)
                            < current.progress.max(previous_progress) + i64::from(length))
                        && i64::from(p.lateral) + i64::from(width) > i64::from(o.left)
                        && i64::from(p.lateral) - i64::from(width) < i64::from(o.right)
                })
                .map(|o| (o.id, o.kind))
                .collect();
            for (hazard, kind) in hazard_hits {
                let key = (id.0, (1u64 << 63) | hazard);
                active.insert(key);
                if !self.active_contacts.contains(&key) {
                    let speed = actors[&id].view.rider.as_ref().unwrap().bike.speed;
                    let reason = if kind == ObstacleKind::Rail { 3 } else { 1 };
                    self.hit(
                        actors,
                        c,
                        id,
                        Impact {
                            reason,
                            bike_damage: (speed / 1024).clamp(0, i32::from(u16::MAX)) as u16,
                            rider_velocity: Some([0, 0, 512]),
                            bike_velocity: Some([0, 0, 256]),
                            ..Default::default()
                        },
                        tick,
                        speed > 6000 || kind == ObstacleKind::Cliff,
                        events,
                    )?;
                    Self::emit(events, tick, id, None, EffectKind::Contact);
                }
            }
        }
        self.active_contacts = active;
        Ok(())
    }
    pub(crate) fn combat(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        mut random: impl FnMut(ActorId, &'static str) -> u32,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        let ids: Vec<_> = self.units.keys().copied().collect();
        for id in ids {
            let u = &self.units[&id];
            if u.recovery.phase != Phase::Mounted
                || u.recovery.outcome.is_some()
                || tick < u.fighter.stun_until
            {
                continue;
            }
            let Some(swing) = u.fighter.swing.clone() else {
                continue;
            };
            let age = tick - swing.started;
            if swing.resolved || !swing.definition.active(age) {
                continue;
            }
            if !swing.swish {
                self.units
                    .get_mut(&id)
                    .unwrap()
                    .fighter
                    .swing
                    .as_mut()
                    .unwrap()
                    .swish = true;
                Self::emit(events, tick, id, swing.target, EffectKind::Swing);
            }
            let Some(target) = swing.target.filter(|t| {
                self.units.contains_key(t)
                    && self.units[t].recovery.phase == Phase::Mounted
                    && self.units[t].recovery.outcome.is_none()
            }) else {
                continue;
            };
            let a = self.position(id, actors).unwrap();
            let b = self.position(target, actors).unwrap();
            let slot = combat::weapon_slot(swing.request.kind, swing.weapon);
            if !combat::in_reach(a, b, swing.request.side, slot, self.setup.rules.reach) {
                continue;
            }
            self.units
                .get_mut(&id)
                .unwrap()
                .fighter
                .swing
                .as_mut()
                .unwrap()
                .resolved = true;
            let theft = self.inventory.carried(target).filter(|_| {
                swing.weapon == animation::Weapon::Fists
                    && swing.request.kind != AttackKind::Kick
                    && tick > self.units[&target].fighter.steal_until
                    && self.units[&target]
                        .fighter
                        .swing
                        .as_ref()
                        .is_some_and(|s| s.definition.frame(tick - s.started) < 2)
            });
            if let Some((item, kind)) = theft.filter(|_| random(id, "weapon_theft") & 3 == 3)
                && self.inventory.transfer(item, target, id)
            {
                self.units.get_mut(&id).unwrap().fighter.steal_until = tick + 300;
                self.units.get_mut(&target).unwrap().fighter.swing = None;
                self.units.get_mut(&id).unwrap().fighter.swing = None;
                let receiver = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
                receiver.pose = animation::Pose::Ride;
                receiver.animation.set(&c.animation, 0, 2);
                actors
                    .get_mut(&id)
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap()
                    .weapon = kind;
                let victim = actors
                    .get_mut(&target)
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap();
                victim.weapon = animation::Weapon::Fists;
                victim.pose = animation::Pose::Ride;
                victim.animation.set(&c.animation, 0, 2);
                Self::emit(events, tick, id, Some(target), EffectKind::Stolen);
                continue;
            }
            if tick <= self.units[&target].fighter.hit_until {
                continue;
            }
            let damage = self.units[&id]
                .fighter
                .stats
                .damage(self.setup.rules.damage[slot]);
            let condition = if slot == 3 { damage / 4 + 1 } else { 0 };
            self.hit(
                actors,
                c,
                target,
                Impact {
                    reason: if slot == 3 { 5 } else { 4 },
                    stamina_damage: damage as u16,
                    bike_damage: condition as u16,
                    ..Default::default()
                },
                tick,
                false,
                events,
            )?;
            self.units.get_mut(&target).unwrap().fighter.hit_until =
                tick + u64::from(self.setup.rules.immunity);
            let direction = if swing.request.side == Side::Right {
                1
            } else {
                -1
            };
            let target_rider = actors
                .get_mut(&target)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            target_rider.bike.lean = (target_rider.bike.lean
                + direction * if slot == 3 { 11000 } else { 3000 })
            .clamp(-11776, 11776);
            if swing.request.kind == AttackKind::Backhand {
                target_rider.bike.speed = (target_rider.bike.speed - 300).max(0);
                actors
                    .get_mut(&id)
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap()
                    .bike
                    .speed += 300;
            }
            let attacker = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
            attacker.bike.lean = (attacker.bike.lean - direction * 1000).clamp(-11776, 11776);
            Self::emit(
                events,
                tick,
                id,
                Some(target),
                if slot == 3 {
                    EffectKind::Kick
                } else {
                    EffectKind::Hit
                },
            );
        }
        Ok(())
    }
    pub(crate) fn progress(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) {
        for (id, u) in &mut self.units {
            let rider = actors.get_mut(id).unwrap().view.rider.as_mut().unwrap();
            let progress = if u.recovery.phase == Phase::Mounted {
                Some(rider.bike.distance_q8)
            } else {
                u.recovery.body.as_ref().and_then(|b| b.projected)
            };
            let finish = progress.is_some_and(|p| {
                p >= self
                    .setup
                    .rules
                    .finish_progress
                    .unwrap_or((c.ground.len() as i64 - 1 - i64::from(c.start_sample)) * 256)
            });
            if u.completed_at.is_some() {
                continue;
            }
            let requested = u
                .terminal_request
                .take()
                .into_iter()
                .chain(finish.then_some(Outcome::Finished))
                .chain(u.recovery.outcome)
                .max();
            if let Some(requested) = requested {
                let previous = None::<Outcome>;
                u.completed_at = Some(tick);
                u.recovery.terminal(requested, &mut rider.bike);
                u.fighter.swing = None;
                events.retain(|e| e.actor != *id);
                if previous != u.recovery.outcome {
                    Self::emit(
                        events,
                        tick,
                        *id,
                        None,
                        match u.recovery.outcome.unwrap() {
                            Outcome::Wrecked => EffectKind::Wrecked,
                            Outcome::Finished => EffectKind::Finish,
                            Outcome::Arrested => EffectKind::Arrest,
                        },
                    );
                }
            }
        }
    }
}
