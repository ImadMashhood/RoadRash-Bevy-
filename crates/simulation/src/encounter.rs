//! Ordered schema-4 contact/combat/recovery systems. No Bevy, filesystem or presentation RNG.
mod contact;
mod fighting;
mod legacy;
mod roads;
mod scenery;
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
    Building,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_box: Option<StaticBox>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StaticBox {
    /// Response geometry; some source billboard types lower its bottom only
    /// after the original frame box has passed the detection query.
    pub bounds: [i32; 4],
    pub lateral: i32,
    /// Absent fields preserve historical replay detection and serialization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_bounds: Option<[i32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenery: Option<crate::scenery::BoxRules>,
}
impl StaticBox {
    fn query_bounds(&self) -> [i32; 4] {
        self.query_bounds.unwrap_or(self.bounds)
    }
}
// Slab intersection with rational comparisons: a complete tick sweep cannot
// tunnel through a narrow wall, and a diagonal near miss stays a near miss.
// Axis 0 is longitudinal; axis 1 is lateral. No floating point enters the sim.
fn wall_entry(
    from: Location,
    to: Location,
    o: &Obstacle,
    length: i32,
    width: i32,
) -> Option<usize> {
    let starts = [from.progress, i64::from(from.lateral)];
    let ends = [to.progress, i64::from(to.lateral)];
    let lows = [
        o.progress - i64::from(o.half_length) - i64::from(length),
        i64::from(o.left) - i64::from(width),
    ];
    let highs = [
        o.progress + i64::from(o.half_length) + i64::from(length),
        i64::from(o.right) + i64::from(width),
    ];
    let embedded = (0..2).all(|i| starts[i] > lows[i] && starts[i] < highs[i]);
    let nearest_axis = if embedded {
        let along = (starts[0] - lows[0]).min(highs[0] - starts[0]) * 256;
        let across = (starts[1] - lows[1]).min(highs[1] - starts[1]);
        usize::from(across < along)
    } else {
        0
    };
    let mut entry = (0i128, 1i128);
    let mut exit = (1i128, 1i128);
    let mut axis = nearest_axis;
    for i in 0..2 {
        let delta = i128::from(ends[i]) - i128::from(starts[i]);
        if delta == 0 {
            if starts[i] <= lows[i] || starts[i] >= highs[i] {
                return None;
            }
            continue;
        }
        let (near, far) = if delta > 0 {
            (lows[i], highs[i])
        } else {
            (highs[i], lows[i])
        };
        let denominator = delta.abs();
        let numerator = |n: i64| (i128::from(n) - i128::from(starts[i])) * delta.signum();
        let n = (numerator(near), denominator);
        let f = (numerator(far), denominator);
        if n.0 * entry.1 > entry.0 * n.1 {
            entry = n;
            axis = i;
        }
        if f.0 * exit.1 < exit.0 * f.1 {
            exit = f;
        }
        if entry.0 * exit.1 >= exit.0 * entry.1 {
            return None;
        }
    }
    Some(axis)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_roads: Option<crate::source_roads::Roads>,
    #[serde(default, skip_serializing_if = "legacy_combat")]
    pub contact_walk_version: u8,
    #[serde(default, skip_serializing_if = "legacy_combat")]
    pub scenery_version: u8,
    #[serde(default, skip_serializing_if = "crate::is_false")]
    pub scenery_start_ready: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenery_roads: Vec<crate::scenery::RoadSpan>,
    #[serde(default, skip_serializing_if = "legacy_combat")]
    pub collision_version: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collision: Option<crate::collision::Profile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub building_boundaries: Vec<crate::building_wall::Boundary>,
    /// 0 preserves historical replays; 1 uses measured source attack/crash entry rules.
    #[serde(default, skip_serializing_if = "legacy_combat")]
    pub combat_version: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steal_codes: Option<[i8; 8]>,
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
fn legacy_combat(version: &u8) -> bool {
    *version == 0
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
            source_roads: None,
            contact_walk_version: 0,
            scenery_version: 0,
            scenery_start_ready: false,
            scenery_roads: vec![],
            collision_version: 0,
            collision: None,
            building_boundaries: vec![],
            combat_version: 0,
            steal_codes: None,
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
        if let Some(roads) = &self.source_roads {
            roads.validate(topology)?;
            if self.contact_walk_version == 0
                || self.contact_walk_version > 2
                || self.scenery_version != 1
            {
                return Err(Error::Invalid("physical source road rules"));
            }
        }
        if self.contact_walk_version > 2
            || (self.contact_walk_version >= 1
                && (self.collision_version != 2 || !self.bike_contacts))
        {
            return Err(Error::Invalid("contact walk version"));
        }
        if self.scenery_start_ready && self.scenery_version == 0 {
            return Err(Error::Invalid("source scenery initial visibility"));
        }
        if self.scenery_version > 1 || (self.scenery_version == 1 && self.collision_version != 2) {
            return Err(Error::Invalid("source scenery version"));
        }
        if self.scenery_roads.len() > 1_000_000
            || (!self.scenery_roads.is_empty() && self.scenery_version != 1)
        {
            return Err(Error::Invalid("source scenery road table"));
        }
        let mut last = None;
        for road in &self.scenery_roads {
            if !(1..=3).contains(&road.road_mask)
                || road.first > road.last
                || !topology
                    .paths
                    .get(&road.path)
                    .is_some_and(|p| i64::from(road.last) * 256 < p.length)
                || last.is_some_and(|(path, end)| {
                    path > road.path || (path == road.path && end >= road.first)
                })
            {
                return Err(Error::Invalid("source scenery road identity"));
            }
            last = Some((road.path, road.last));
        }
        self.body.validate()?;
        if let Some(profile) = &self.collision {
            profile.validate()?;
        }
        if self.collision_version == 2 && (self.collision.is_none() || self.combat_version < 2) {
            return Err(Error::Invalid(
                "collision profile 2 requires combat profile 2 and physical constants",
            ));
        }
        if self
            .steal_codes
            .is_some_and(|codes| codes.iter().any(|c| !(-1..=51).contains(c)))
            || self.collision_version > 2
            || self.combat_version > 4
            || self.level > 4
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
        if self.building_boundaries.len() > 1_000_000 {
            return Err(Error::Invalid("building boundary budget"));
        }
        let mut previous = None;
        for boundary in &self.building_boundaries {
            if boundary
                .surface
                .as_ref()
                .is_some_and(|surface| !surface.validate())
            {
                return Err(Error::Invalid("source terrain boundary profile"));
            }
            let key = (boundary.path, boundary.sample);
            if previous.is_some_and(|p| p >= key)
                || !topology.paths.contains_key(&boundary.path)
                || i64::from(boundary.sample) * 256 >= topology.paths[&boundary.path].length
                || boundary.road[0] > boundary.road[1]
                || boundary.outer[0] > boundary.road[0]
                || boundary.outer[1] < boundary.road[1]
                || boundary
                    .road
                    .iter()
                    .chain(boundary.outer.iter())
                    .any(|v| v.unsigned_abs() > 1_000_000)
            {
                return Err(Error::Invalid("source building boundary"));
            }
            previous = Some(key);
        }
        let mut ids = BTreeSet::new();
        for o in &self.obstacles {
            if o.source_box
                .as_ref()
                .is_some_and(|b| b.scenery.is_some() != (self.scenery_version == 1))
            {
                return Err(Error::Invalid("source scenery metadata/version mismatch"));
            }
            if let Some(rules) = o.source_box.as_ref().and_then(|b| b.scenery.as_ref()) {
                rules.validate()?;
            }
            if let Some(b) = &o.source_box
                && (b.lateral.unsigned_abs() > 256_000_000
                    || std::iter::once(&b.bounds)
                        .chain(b.query_bounds.iter())
                        .any(|bounds| {
                            bounds[0] >= bounds[2]
                                || bounds[1] >= bounds[3]
                                || bounds.iter().any(|v| v.unsigned_abs() > 256_000_000)
                                || bounds[2] - bounds[0] > 1_048_576
                                || bounds[3] - bounds[1] > 1_048_576
                        }))
            {
                return Err(Error::Invalid("source physical collision box"));
            }
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
                || o.height > 1_048_576
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_route: Option<u8>,
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
            source_route: None,
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
        if let Some(roads) = &self.rules.source_roads
            && roads.origin(&self.topology) != Some(i64::from(c.start_sample) * 256)
        {
            return Err(Error::Invalid("physical road configuration origin"));
        }
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
        if let Some(roads) = &self.rules.source_roads {
            let index = s.source_route.unwrap_or(roads.initial);
            let mut at = roads
                .location(
                    index,
                    s.distance,
                    roads
                        .origin(&self.topology)
                        .ok_or(Error::Invalid("source spawn origin"))?,
                    s.lateral,
                    0,
                )
                .ok_or(Error::Invalid("spawn outside physical source route"))?;
            if let Some(explicit) = s.location {
                if explicit.path != at.path || explicit.progress != at.progress {
                    return Err(Error::Invalid("physical spawn route mismatch"));
                }
                at = explicit;
            }
            Ok(at)
        } else {
            if s.source_route.is_some() {
                return Err(Error::Invalid("source spawn without road profiles"));
            }
            s.location
                .or_else(|| self.topology.locate(s.distance, s.lateral, 0))
                .ok_or(Error::Invalid("spawn outside source topology"))
        }
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
                    source_box: None,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub police_ai: Option<population::police_ai::AiState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub police_relations: Option<population::police_ai::Relations>,
    #[serde(default, skip_serializing_if = "crate::is_false")]
    pub police: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub race_progress: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_route: Option<u8>,
    pub fighter: Combatant,
    pub recovery: State,
    pub previous: Position,
    pub previous_location: Location,
    pub completed_at: Option<u64>,
    pub color: u8,
    pub drive: bool,
    pub terminal_request: Option<Outcome>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ContactObject {
    pub owner: ActorId,
    pub detached: bool,
}
impl ContactObject {
    fn key(self) -> u64 {
        self.owner.0 * 2 + u64::from(self.detached)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenery: Option<crate::scenery::State>,
    pub setup: Setup,
    pub units: BTreeMap<ActorId, Unit>,
    pub inventory: Inventory,
    pub active_contacts: BTreeSet<(u64, u64)>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    source_combat_order: BTreeMap<ActorId, Vec<ActorId>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    source_contact_order: Vec<u64>,
    #[serde(skip)]
    source_combat_external_targets: Vec<Position>,
    #[serde(skip)]
    pub(crate) configs: BTreeMap<ActorId, Configuration>,
    #[serde(skip)]
    road_configs: BTreeMap<ActorId, u8>,
    #[serde(skip)]
    road_topologies: BTreeMap<u8, Topology>,
    #[serde(skip)]
    body_configs: BTreeMap<ActorId, (u8, Configuration)>,
    #[serde(skip)]
    scenery_index: BTreeMap<(u32, u32), Vec<usize>>,
    #[serde(skip)]
    scenery_index_ready: bool,
    #[serde(skip)]
    pub(crate) navigation: BTreeMap<ActorId, crate::topology::Portal>,
    #[serde(skip)]
    obstacle_bins: BTreeMap<(u32, i64), Vec<usize>>,
    #[serde(skip)]
    building_keys: BTreeMap<u32, u64>,
    #[serde(skip)]
    building_groups: BTreeMap<u64, u64>,
    #[serde(skip)]
    building_groups_ready: bool,
    // Half-think stamina belongs to each combat transaction, after earlier
    // riders' hits. Direct contact/strike probes do not schedule think work.
    #[serde(skip)]
    pending_regeneration: BTreeSet<ActorId>,
    #[serde(skip)]
    source_swing_gates: BTreeMap<ActorId, bool>,
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
            if setup.rules.combat_version >= 3 {
                rider.animation.queued = Some(2);
            }
            rider.bike.distance_q8 = if setup.rules.source_roads.is_some() {
                seed.distance
            } else {
                setup.topology.projected(at).unwrap_or(seed.distance)
            };
            rider.bike.lateral = at.lateral;
            rider.bike.height = at.height;
            rider.bike.speed = seed.speed;
            if setup.rules.source_roads.is_some() {
                rider.bike.previous_rise = setup.topology.grade(at);
            }
            if let Some((_, w)) = setup.inventory.carried(seed.id) {
                rider.weapon = w;
            }
            let race_progress = setup.rules.source_roads.as_ref().and_then(|roads| {
                let finish = roads.profiles
                    [usize::from(seed.source_route.unwrap_or(roads.initial))]
                .finish_progress?;
                let base = setup
                    .rules
                    .finish_progress
                    .or(roads.profiles[usize::from(roads.initial)].finish_progress)
                    .unwrap_or(finish);
                Some(base - (finish - seed.distance))
            });
            units.insert(
                seed.id,
                Unit {
                    police_ai: None,
                    police_relations: None,
                    police: false,
                    race_progress,
                    source_route: setup
                        .rules
                        .source_roads
                        .as_ref()
                        .map(|r| seed.source_route.unwrap_or(r.initial)),
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
        let mut obstacle_bins: BTreeMap<(u32, i64), Vec<usize>> = BTreeMap::new();
        for (index, o) in setup.rules.obstacles.iter().enumerate() {
            for bin in (o.progress - i64::from(o.half_length)).div_euclid(1024)
                ..=(o.progress + i64::from(o.half_length)).div_euclid(1024)
            {
                obstacle_bins.entry((o.path, bin)).or_default().push(index);
            }
        }
        let mut building_keys = BTreeMap::new();
        for o in &setup.rules.obstacles {
            if o.kind == ObstacleKind::Building {
                building_keys.entry(o.path).or_insert(o.id);
            }
        }
        let start_ready = setup.rules.scenery_start_ready;
        let mut world = Self {
            scenery: (setup.rules.scenery_version == 1).then(crate::scenery::State::default),
            scenery_index: BTreeMap::new(),
            scenery_index_ready: false,
            building_keys,
            building_groups: BTreeMap::new(),
            building_groups_ready: false,
            pending_regeneration: BTreeSet::new(),
            source_swing_gates: BTreeMap::new(),
            inventory: setup.inventory.clone(),
            setup,
            units,
            active_contacts: BTreeSet::new(),
            source_combat_order: BTreeMap::new(),
            source_contact_order: Vec::new(),
            source_combat_external_targets: Vec::new(),
            obstacle_bins,
            configs: BTreeMap::new(),
            road_configs: BTreeMap::new(),
            road_topologies: BTreeMap::new(),
            body_configs: BTreeMap::new(),
            navigation: BTreeMap::new(),
        };
        if start_ready {
            world.scenery.as_mut().unwrap().pending_camera =
                Some(world.units[&ActorId(1)].recovery.bike_location);
            world.scenery_begin(actors, c, 0)?;
            world.scenery_prepare(true)?;
        }
        Ok(world)
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
                police_ai: None,
                police_relations: None,
                police: false,
                race_progress: None,
                source_route: self.setup.rules.source_roads.as_ref().map(|r| r.initial),
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
        let police = self.units.get(&id).is_some_and(|u| u.police);
        self.inventory.drop(id);
        if police {
            self.inventory
                .items
                .retain(|key, item| key.0 < 0x10000 || item.owner.is_some());
        }
        self.source_contact_order
            .retain(|key| *key != id.0 * 2 && *key != id.0 * 2 + 1);
        self.source_combat_order.remove(&id);
        for order in self.source_combat_order.values_mut() {
            order.retain(|actor| *actor != id);
        }
        self.pending_regeneration.remove(&id);
        self.source_swing_gates.remove(&id);
        if let Some(scenery) = &mut self.scenery {
            scenery.object_roads.remove(&(id.0 * 2));
            scenery.object_roads.remove(&(id.0 * 2 + 1));
        }
        if self.setup.rules.combat_version >= 4 {
            for unit in self.units.values_mut() {
                if let Some(swing) = &mut unit.fighter.swing
                    && swing.target == Some(id)
                {
                    swing.target = None;
                }
                if let Some(ai) = &mut unit.police_ai {
                    if ai.target == Some(id) {
                        ai.target = None;
                    }
                    if ai.ahead == Some(id) {
                        ai.ahead = None;
                    }
                    if ai.behind == Some(id) {
                        ai.behind = None;
                    }
                    if ai.overtake.is_some_and(|object| object.actor == Some(id)) {
                        ai.overtake = None;
                    }
                }
            }
        }
        self.units.remove(&id);
        self.configs.remove(&id);
        self.road_configs.remove(&id);
        self.body_configs.remove(&id);
        self.navigation.remove(&id);
        if self.setup.rules.contact_walk_version >= 1 {
            self.active_contacts.retain(|(a, b)| {
                ![id.0 * 2, id.0 * 2 + 1].contains(a) && ![id.0 * 2, id.0 * 2 + 1].contains(b)
            });
        } else {
            self.active_contacts
                .retain(|(a, b)| *a != id.0 && *b != id.0);
        }
    }
    pub(crate) fn obstacle_candidates(&self, path: u32, start: i64, end: i64) -> Vec<usize> {
        if self.obstacle_bins.is_empty() {
            return self
                .setup
                .rules
                .obstacles
                .iter()
                .enumerate()
                .filter(|(_, o)| o.path == path)
                .map(|(index, _)| index)
                .collect();
        }
        let mut indices = BTreeSet::new();
        for bin in start.div_euclid(1024)..=end.div_euclid(1024) {
            if let Some(found) = self.obstacle_bins.get(&(path, bin)) {
                indices.extend(found.iter().copied());
            }
        }
        indices.into_iter().collect()
    }
    pub(crate) fn position(
        &self,
        id: ActorId,
        actors: &BTreeMap<ActorId, crate::Actor>,
    ) -> Option<Position> {
        let u = self.units.get(&id)?;
        let bike = &actors.get(&id)?.view.rider.as_ref()?.bike;
        let at = u.recovery.bike_location;
        Some(Position {
            route: 1,
            branch: self.setup.topology.paths.get(&at.path)?.branch,
            path: at.path,
            progress: if self.setup.rules.source_roads.is_some() {
                bike.distance_q8
            } else {
                self.setup
                    .topology
                    .projected(at)
                    .unwrap_or(bike.distance_q8)
            },
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
        external_targets: &[Position],
        events: &mut Vec<Effect>,
        random: impl FnMut() -> u32,
    ) -> Result<()> {
        self.movement_selected(actors, c, tick, external_targets, events, random, None)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn movement_selected(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        external_targets: &[Position],
        events: &mut Vec<Effect>,
        mut random: impl FnMut() -> u32,
        only: Option<ActorId>,
    ) -> Result<()> {
        if self.setup.rules.combat_version < 3 {
            return self.legacy_movement(actors, c, tick, external_targets, events);
        }
        if self.setup.rules.combat_version >= 4 {
            self.source_combat_external_targets = external_targets.to_vec();
        }
        if only.is_none() {
            self.prepare_source_roads(actors, c, tick)?;
        } else {
            self.prepare_source_roads_selected(actors, c, tick, only)?;
        }
        let recovering_before_movement: BTreeSet<_> = self
            .units
            .iter()
            .filter(|(_, unit)| unit.recovery.body.is_some())
            .map(|(id, _)| *id)
            .collect();
        let mut vertical = BTreeMap::new();
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
        let base_configuration = c;
        let moving_ids: Vec<_> = self.units.keys().copied().collect();
        for id in &moving_ids {
            let u = self.units.get_mut(id).unwrap();
            if only.is_some_and(|selected| selected != *id) {
                continue;
            }
            let c = self.configs.get(id).unwrap_or(c);
            let a = actors.get_mut(id).ok_or(Error::MissingActor(*id))?;
            let r = a
                .view
                .rider
                .as_mut()
                .ok_or(Error::Invalid("encounter rider"))?;
            if matches!(u.recovery.phase, Phase::Mounted | Phase::Mounting) {
                r.bike
                    .refresh_scenery(i64::from(c.start_sample) + (r.bike.distance_q8 >> 8), tick);
            }
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
                    let impact = Impact {
                        reason: 4,
                        rider_velocity: Some([0; 3]),
                        bike_velocity: Some([0; 3]),
                        ..Default::default()
                    };
                    if self.setup.rules.body.recovery_version == 2 {
                        u.recovery.begin_live_source(
                            *id,
                            &mut r.bike,
                            c,
                            u.recovery.bike_location,
                            impact,
                            &self.setup.topology,
                        );
                        if let Some(body) = &mut u.recovery.body {
                            body.source_route = u.source_route;
                            if let Some(route) = body.source_route {
                                // Unlike a post-movement collision, this input creates
                                // a body after road preparation and moves it immediately.
                                // Its first half-motion needs the same independent route
                                // context that preparation supplies on following ticks.
                                self.body_configs.insert(*id, (route, c.clone()));
                            }
                        }
                    } else {
                        u.recovery.begin(
                            *id,
                            &mut r.bike,
                            c,
                            u.recovery.bike_location,
                            impact,
                            &self.setup.topology,
                        );
                    }
                }
            }
            if u.recovery.phase != Phase::Mounted {
                // Contacts follow movement. Reserve an interrupted recovery transition
                // so a same-tick impact cannot complete a mount and discard its body.
                if a.input.impact.is_some() {
                    continue;
                }
                // Source half-think regenerates twice in get-up/walk/mount;
                // the player wrapper contributes one additional dt. Sliding
                // gets no regeneration, including the tick it leaves phase 1.
                if self.setup.rules.body.recovery_version == 1 && u.recovery.phase != Phase::Sliding
                {
                    u.fighter.stats.health =
                        (u.fighter.stats.health + 2 + i32::from(*id == ActorId(1)))
                            .min(u.fighter.stats.maximum);
                }
                if self.setup.rules.body.recovery_version == 2 {
                    self.live_recovery_movement(
                        *id,
                        actors,
                        base_configuration,
                        tick,
                        events,
                        &mut random,
                    )?;
                    continue;
                }
                let recovery_events = if let (Some(route), Some(body_route)) = (
                    u.source_route,
                    u.recovery.body.as_ref().and_then(|b| b.source_route),
                ) {
                    u.recovery.step_source_context(
                        &mut r.bike,
                        c,
                        &self.road_topologies[&route],
                        &self.setup.rules.body,
                        a.input.walk,
                        tick as u32,
                        &mut random,
                        Some((&self.body_configs[id].1, &self.road_topologies[&body_route])),
                    )?
                } else {
                    u.recovery.step_with_random(
                        &mut r.bike,
                        c,
                        &self.setup.topology,
                        &self.setup.rules.body,
                        self.setup.rules.assists,
                        a.input.walk,
                        tick as u32,
                        &mut random,
                    )?
                };
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
            if self.setup.rules.combat_version >= 3 {
                if only.is_none() {
                    self.pending_regeneration.insert(*id);
                }
            } else {
                u.fighter.stats.regenerate();
            }
            let mut command = a.input;
            command.pose = None;
            command.weapon = None;
            command.damage = 0;
            if self.setup.rules.combat_version < 4 {
                if u.fighter.swing.as_ref().is_some_and(|s| {
                    if self.setup.rules.combat_version >= 3 {
                        r.animation.code != s.definition.code
                    } else {
                        tick - s.started >= u64::from(s.definition.duration)
                    }
                }) {
                    u.fighter.swing = None;
                    r.pose = animation::Pose::Ride;
                }
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
                        let target_lateral =
                            chosen.map(|other| positions[&other].lateral).or_else(|| {
                                (self.setup.rules.combat_version >= 3)
                                    .then(|| {
                                        external_targets
                                            .iter()
                                            .filter(|p| {
                                                positions[id].same_road(**p)
                                                    && (positions[id].progress - p.progress).abs()
                                                        < 480
                                            })
                                            .min_by_key(|p| {
                                                (
                                                    (positions[id].progress - p.progress).abs(),
                                                    p.progress >= positions[id].progress,
                                                )
                                            })
                                            .map(|p| p.lateral)
                                    })
                                    .flatten()
                            });
                        request.side = if target_lateral
                            .is_some_and(|lateral| positions[id].lateral < lateral)
                        {
                            Side::Right
                        } else {
                            Side::Left
                        };
                    }
                    let definition = Definition::from_library_version(
                        &c.animation,
                        request,
                        r.weapon,
                        self.setup.rules.combat_version,
                    )?;
                    command.pose = Some(combat::pose(request));
                    u.fighter.swing = Some(Swing {
                        repeat: false,
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
            }
            if u.drive {
                command.throttle = true;
            }
            let previous_distance = r.bike.distance_q8;
            if only.is_some() {
                r.pose_ticks = r.pose_ticks.saturating_add(1);
                let ground = if self.setup.rules.body.recovery_version == 2 {
                    let ground = r.source_traction(c);
                    if r.bike.crashed {
                        u.fighter.swing = None;
                        u.recovery.begin_live_source(
                            *id,
                            &mut r.bike,
                            c,
                            u.recovery.bike_location,
                            Impact {
                                reason: 0,
                                rider_velocity: Some([0; 3]),
                                bike_velocity: Some([0; 3]),
                                ..Default::default()
                            },
                            &self.setup.topology,
                        );
                        if let Some(body) = &mut u.recovery.body {
                            body.source_route = u.source_route;
                        }
                        Self::emit(events, tick, *id, None, EffectKind::Crash);
                    }
                    r.source_motion_after_traction(c, ground, r.bike.crashed)
                } else {
                    r.source_physics_motion(c)
                };
                vertical.insert(*id, ground);
            } else if self.setup.rules.source_roads.is_some() {
                vertical.insert(
                    *id,
                    r.step_with_combat_profile_motion(
                        c,
                        command,
                        tick as u32,
                        self.setup.rules.combat_version,
                    )?,
                );
            } else {
                r.step_with_combat_profile(
                    c,
                    command,
                    tick as u32,
                    self.setup.rules.combat_version,
                )?;
            }

            let delta = r.bike.distance_q8 - previous_distance;
            self.setup.topology.advance(
                &mut u.recovery.bike_location,
                delta,
                self.navigation.get(id).copied(),
            )?;
            u.recovery.bike_location.lateral = r.bike.lateral;
            u.recovery.bike_location.height = r.bike.height;
            if r.bike.crashed && u.recovery.body.is_none() {
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
        }
        if self.setup.rules.collision_version == 2 {
            let ids: Vec<_> = self.units.keys().copied().collect();
            for id in ids {
                if only.is_some_and(|selected| selected != id) {
                    continue;
                }
                let had_body = self.units[&id].recovery.body.is_some();
                if !(recovering_before_movement.contains(&id)
                    && self.setup.rules.body.recovery_version == 2)
                {
                    self.source_building_contact(id, actors, c, tick, events)?;
                }
                if let Some(ground) = vertical.remove(&id) {
                    let ground = self.source_vertical_ground(id, actors, ground)?;
                    let config = self.configs.get(&id).unwrap_or(c);
                    let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
                    rider.finish_motion(config, ground);
                    let unit = self.units.get_mut(&id).unwrap();
                    unit.recovery.bike_location.height = rider.bike.height;
                }
                if self.setup.rules.body.recovery_version != 2
                    && had_body
                    && self.units[&id].recovery.body.is_some()
                    && self.setup.rules.collision.as_ref().unwrap().rider.is_some()
                {
                    self.source_detached_boundary(id, actors, c)?;
                }
            }
        }
        if self.setup.rules.contact_walk_version >= 2 {
            for id in &moving_ids {
                if only.is_some_and(|selected| selected != *id) {
                    continue;
                }
                let unit = self.units.get_mut(id).unwrap();
                if unit.recovery.outcome.is_some() {
                    continue;
                }
                let bike = &mut actors
                    .get_mut(id)
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap()
                    .bike;
                bike.lateral = bike.lateral.clamp(-0xc8000, 0xc8000);
                unit.recovery.bike_location.lateral = bike.lateral;
            }
        }
        if only.is_none() {
            self.source_movement_end();
        }
        Ok(())
    }
    /// Each LIVE recovery object meets its boundary and ground before vertical motion.
    fn live_recovery_movement(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        base: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
        mut random: impl FnMut() -> u32,
    ) -> Result<()> {
        let (recovery_events, ground) = {
            let config = self.configs.get(&id).unwrap_or(base);
            let unit = self.units.get_mut(&id).unwrap();
            let topology = unit
                .source_route
                .map_or(&self.setup.topology, |route| &self.road_topologies[&route]);
            let body_context = unit
                .recovery
                .body
                .as_ref()
                .and_then(|body| body.source_route)
                .map(|route| (&self.body_configs[&id].1, &self.road_topologies[&route]));
            unit.recovery.live_half_motion_horizontal(
                config,
                topology,
                &self.setup.rules.body,
                tick as u32,
                &mut random,
                body_context,
            )?
        };
        if let Some(ground) = ground {
            let ground = if self.setup.rules.collision_version == 2 {
                self.source_recovery_surface(id, true, actors, base, tick, events, ground)?
            } else {
                ground
            };
            let unit = self.units.get_mut(&id).unwrap();
            let route = unit
                .recovery
                .body
                .as_ref()
                .and_then(|body| body.source_route)
                .or(unit.source_route);
            let topology =
                route.map_or(&self.setup.topology, |route| &self.road_topologies[&route]);
            let config = self
                .body_configs
                .get(&id)
                .map(|(_, config)| config)
                .unwrap_or_else(|| self.configs.get(&id).unwrap_or(base));
            unit.recovery
                .finish_live_half_motion(config, topology, &self.setup.rules.body, ground);
            if self.setup.rules.contact_walk_version >= 2
                && let Some(body) = &mut unit.recovery.body
            {
                body.motion.lateral = body.motion.lateral.clamp(-0xc8000, 0xc8000);
                body.location.lateral = body.motion.lateral;
            }
        }
        let ground = {
            let unit = self.units.get_mut(&id).unwrap();
            let topology = unit
                .source_route
                .map_or(&self.setup.topology, |route| &self.road_topologies[&route]);
            let config = self.configs.get(&id).unwrap_or(base);
            let bike = &mut actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            unit.recovery.live_bike_motion_horizontal(
                bike,
                config,
                topology,
                &self.setup.rules.body,
                tick as u32,
                &mut random,
            )?
        };
        if let Some(ground) = ground {
            let ground = if self.setup.rules.collision_version == 2 {
                self.source_recovery_surface(id, false, actors, base, tick, events, ground)?
            } else {
                ground
            };
            let unit = self.units.get_mut(&id).unwrap();
            let topology = unit
                .source_route
                .map_or(&self.setup.topology, |route| &self.road_topologies[&route]);
            let config = self.configs.get(&id).unwrap_or(base);
            let bike = &mut actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            unit.recovery.finish_live_bike_motion(
                bike,
                config,
                topology,
                &self.setup.rules.body,
                ground,
            );
        }
        for event in recovery_events {
            let kind = match event {
                recovery::RecoveryEvent::GetUp => EffectKind::GetUp,
                recovery::RecoveryEvent::Remount => EffectKind::Remount,
                recovery::RecoveryEvent::Rescue => EffectKind::Rescue,
                recovery::RecoveryEvent::Wrecked => continue,
            };
            Self::emit(events, tick, id, None, kind);
        }
        Ok(())
    }

    /// LIVE half-think recovery decisions precede the arrest scan and body move.
    pub(crate) fn source_prepare_recovery(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        id: ActorId,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        if self.setup.rules.body.recovery_version != 2 {
            return Ok(());
        }
        let u = self.units.get_mut(&id).ok_or(Error::MissingActor(id))?;
        if u.recovery.phase == Phase::Mounted || u.recovery.outcome.is_some() {
            return Ok(());
        }
        let a = actors.get_mut(&id).ok_or(Error::MissingActor(id))?;
        if a.input.impact.is_some() {
            return Ok(());
        }
        let r = a
            .view
            .rider
            .as_mut()
            .ok_or(Error::Invalid("encounter rider"))?;
        if u.recovery.phase != Phase::Sliding {
            u.fighter.stats.health = (u.fighter.stats.health + 2 + i32::from(id == ActorId(1)))
                .min(u.fighter.stats.maximum);
        }
        let c = self.configs.get(&id).unwrap_or(c);
        let topology = u
            .source_route
            .map_or(&self.setup.topology, |route| &self.road_topologies[&route]);
        let (recovery_events, mounted_clock) = u.recovery.prepare_live_recovery(
            &mut r.bike,
            c,
            topology,
            a.input.walk,
            id == ActorId(1),
        )?;
        if let Some(clock) = mounted_clock {
            r.animation = clock;
        }
        for event in recovery_events {
            let kind = match event {
                recovery::RecoveryEvent::GetUp => EffectKind::GetUp,
                recovery::RecoveryEvent::Remount => {
                    u.fighter.stats.health = u.fighter.stats.maximum;
                    r.pose = animation::Pose::Ride;
                    EffectKind::Remount
                }
                recovery::RecoveryEvent::Rescue => EffectKind::Rescue,
                recovery::RecoveryEvent::Wrecked => continue,
            };
            Self::emit(events, tick, id, None, kind);
        }
        Ok(())
    }
    pub(crate) fn source_movement_end(&mut self) {
        if let Some(state) = &mut self.scenery {
            state.frame_follow = self.units.get(&ActorId(1)).map(|u| {
                u.recovery
                    .body
                    .as_ref()
                    .map_or(u.recovery.bike_location, |b| b.location)
            });
        }
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
        let entry_drive = rider.bike.drive;
        rider.bike.damage(bike_damage.max(0) as u32);
        if self.setup.rules.body.recovery_version == 2 {
            rider.bike.drive = entry_drive;
        }

        unit.fighter.hit_count = unit.fighter.hit_count.saturating_add(1);
        let down = force
            || unit.fighter.stats.health == 0
            || rider.bike.condition == 0
            || unit.recovery.phase != Phase::Mounted;
        if down || self.setup.rules.combat_version < 2 || impact.reason == 4 {
            unit.fighter.swing = None;
        }
        if down {
            events.retain(|e| {
                e.actor != target || !matches!(e.kind, EffectKind::Remount | EffectKind::GetUp)
            });
            let entered = if self.setup.rules.body.recovery_version == 2 {
                unit.recovery.begin_live_source(
                    target,
                    &mut rider.bike,
                    c,
                    unit.recovery.bike_location,
                    impact,
                    &self.setup.topology,
                )
            } else if self.setup.rules.combat_version >= 1 {
                unit.recovery.begin_source(
                    target,
                    &mut rider.bike,
                    c,
                    unit.recovery.bike_location,
                    impact,
                    &self.setup.topology,
                )
            } else {
                unit.recovery.begin(
                    target,
                    &mut rider.bike,
                    c,
                    unit.recovery.bike_location,
                    impact,
                    &self.setup.topology,
                )
            };
            if entered || self.setup.rules.combat_version == 0 {
                Self::emit(events, tick, target, None, EffectKind::Crash);
            }
        } else if self.setup.rules.combat_version < 2 || impact.reason == 4 {
            unit.fighter.stun_until = tick + u64::from(self.setup.rules.stun);
            rider.pose = animation::Pose::HitReact;
            rider.animation.set(&c.animation, 0, 9);
            if self.setup.rules.combat_version < 4 {
                rider.animation.queued = Some(2);
            }
        }
        Ok(down)
    }
    #[allow(clippy::too_many_arguments)]
    fn mounted_statics(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
        active: &mut BTreeSet<(u64, u64)>,
        random: &mut impl FnMut() -> u32,
    ) -> Result<()> {
        if !self.building_groups_ready {
            let mut facades: BTreeMap<_, Vec<_>> = BTreeMap::new();
            for o in &self.setup.rules.obstacles {
                if o.kind == ObstacleKind::Building {
                    self.building_keys.entry(o.path).or_insert(o.id);
                    facades
                        .entry((o.path, o.left, o.right, o.height))
                        .or_default()
                        .push((
                            o.progress - i64::from(o.half_length),
                            o.progress + i64::from(o.half_length),
                            o.id,
                        ));
                }
            }
            for mut panels in facades.into_values() {
                panels.sort();
                let mut incident = None;
                let mut high = i64::MIN;
                for (start, end, id) in panels {
                    if start > high.saturating_add(1) {
                        incident = Some(id);
                        high = end;
                    } else {
                        high = high.max(end);
                    }
                    self.building_groups.insert(id, incident.unwrap());
                }
            }
            self.building_groups_ready = true;
        }
        let static_actor_key = if self.setup.rules.contact_walk_version >= 1 {
            id.0 * 2
        } else {
            id.0
        };
        let [width, length, _height] = self.setup.rules.dimensions;
        if self.units[&id].recovery.outcome.is_some() {
            return Ok(());
        }
        let p = self.position(id, actors).unwrap();
        let previous = self.units[&id].previous_location;
        let current = self.units[&id].recovery.bike_location;
        let previous_progress = if previous.path == current.path {
            previous.progress
        } else {
            current.progress
        };
        if self.obstacle_bins.is_empty() {
            for (index, o) in self.setup.rules.obstacles.iter().enumerate() {
                for bin in (o.progress - i64::from(o.half_length)).div_euclid(1024)
                    ..=(o.progress + i64::from(o.half_length)).div_euclid(1024)
                {
                    self.obstacle_bins
                        .entry((o.path, bin))
                        .or_default()
                        .push(index);
                }
            }
        }
        let mut candidates = BTreeSet::new();
        for bin in (current.progress.min(previous_progress) - i64::from(length)).div_euclid(1024)
            ..=(current.progress.max(previous_progress) + i64::from(length)).div_euclid(1024)
        {
            if let Some(indices) = self.obstacle_bins.get(&(p.path, bin)) {
                candidates.extend(indices.iter().copied());
            }
        }
        let hazard_hits: Vec<_> = candidates
            .iter()
            .filter_map(|index| {
                self.scenery_obstacle(
                    &self.setup.rules.obstacles[*index],
                    c,
                    ContactObject {
                        owner: id,
                        detached: false,
                    },
                )
            })
            .filter(|o| {
                if self.setup.rules.collision_version == 2
                    && let Some(b) = &o.source_box
                {
                    let bike = &actors[&id].view.rider.as_ref().unwrap().bike;
                    let profile = self.setup.rules.collision.as_ref().unwrap();
                    let dims = if bike.crashed {
                        profile.fallen
                    } else {
                        profile.mounted
                    };
                    let bounds = b.query_bounds();
                    return o.path == p.path
                        && (current.progress - o.progress).abs() < i64::from(dims[1])
                        && bounds[0] <= bike.lateral + dims[0]
                        && bike.lateral - dims[0] <= bounds[2]
                        && bounds[1] <= bike.height + dims[2]
                        && bike.height - dims[2] < bounds[3];
                }
                !(self.setup.rules.collision_version == 2
                    && !self.setup.rules.building_boundaries.is_empty()
                    && o.kind == ObstacleKind::Building)
                    && o.kind != ObstacleKind::Decorative
                    && o.path == p.path
                    && p.height < o.height
                    && (current.progress.min(previous_progress) - i64::from(length)
                        < o.progress + i64::from(o.half_length))
                    && (o.progress - i64::from(o.half_length)
                        < current.progress.max(previous_progress) + i64::from(length))
                    && if o.kind == ObstacleKind::Building {
                        wall_entry(
                            Location {
                                progress: previous_progress,
                                ..previous
                            },
                            current,
                            o,
                            length,
                            width,
                        )
                        .is_some()
                    } else {
                        i64::from(p.lateral) + i64::from(width) > i64::from(o.left)
                            && i64::from(p.lateral) - i64::from(width) < i64::from(o.right)
                    }
            })
            .collect();
        let mut hazard_hits = hazard_hits;
        if self.setup.rules.scenery_version == 1 {
            hazard_hits.sort_by_key(|o| {
                o.source_box
                    .as_ref()
                    .and_then(|b| b.scenery.as_ref())
                    .map_or((2, u32::MAX), |m| (u8::from(m.class != 16), m.ordinal))
            });
        }
        for obstacle in hazard_hits {
            if self.setup.rules.collision_version == 2
                && let Some(b) = &obstacle.source_box
            {
                if b.scenery
                    .as_ref()
                    .is_some_and(|m| m.class >= 3 && m.class < 16)
                {
                    self.scenery_mark(id, actors, &obstacle, c, tick);
                    active.insert((static_actor_key, (1u64 << 63) | obstacle.id));
                    break;
                }
                let at = self.units[&id].recovery.bike_location;
                let progress = if self.setup.rules.source_roads.is_some() {
                    actors[&id].view.rider.as_ref().unwrap().bike.distance_q8 + obstacle.progress
                        - at.progress
                } else {
                    self.setup
                        .topology
                        .projected(Location {
                            progress: obstacle.progress,
                            ..at
                        })
                        .unwrap()
                } as i32;
                let mut object = crate::collision::Body {
                    position: [progress, b.lateral, 0],
                    previous: [progress, b.lateral, 0],
                    half: [0, b.bounds[2] - b.bounds[0], b.bounds[3] - b.bounds[1]],
                    velocity: [0; 3],
                    speed: 0,
                    mass: 0x1000000,
                    category: 1,
                    flags: 0,
                    impulse: [0; 3],
                };
                if self.source_object_contact_with_random(
                    id,
                    actors,
                    &mut object,
                    c,
                    tick,
                    events,
                    random,
                )? {
                    let key = (static_actor_key, (1u64 << 63) | obstacle.id);
                    active.insert(key);
                    if !self.active_contacts.contains(&key) {
                        Self::emit(events, tick, id, None, EffectKind::Contact);
                    }
                }
                break;
            }
            let kind = obstacle.kind;
            // Adjacent source panels share one continuous wall-contact incident.
            let contact_id = if kind == ObstacleKind::Building {
                if self.setup.rules.contact_walk_version >= 1 {
                    *self
                        .building_groups
                        .get(&obstacle.id)
                        .unwrap_or(&obstacle.id)
                } else {
                    *self
                        .building_keys
                        .get(&obstacle.path)
                        .unwrap_or(&obstacle.id)
                }
            } else {
                obstacle.id
            };
            let key = (static_actor_key, (1u64 << 63) | contact_id);
            let mut frontal = true;
            let mut side_speed = 0;
            if kind == ObstacleKind::Building {
                let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
                let unit = self.units.get_mut(&id).unwrap();
                // Keep the bike outside the actual wall, even below the knock-off
                // threshold. Resolve every overlapping tick, damage only on entry.
                let mut axis = wall_entry(
                    Location {
                        progress: previous_progress,
                        ..previous
                    },
                    current,
                    &obstacle,
                    length,
                    width,
                )
                .unwrap_or(0);
                if self.setup.rules.collision_version >= 1 && axis == 0 {
                    let clearance = (i64::from(rider.bike.lateral) - i64::from(obstacle.left)
                        + i64::from(width))
                    .min(
                        i64::from(obstacle.right) + i64::from(width)
                            - i64::from(rider.bike.lateral),
                    );
                    // Shallow corner clipping must slide out along the facade.
                    // Stopping forward speed here strands a bike that cannot steer
                    // until it starts rolling again.
                    if clearance >= 0 && clearance <= i64::from(width) * 3 / 2 {
                        axis = 1;
                    }
                }
                frontal = axis == 0;
                side_speed = rider.bike.lateral_speed.abs();
                if axis == 1
                    && i64::from(previous.lateral) * 2
                        <= i64::from(obstacle.left) + i64::from(obstacle.right)
                {
                    rider.bike.lateral = obstacle.left - width - 1;
                    rider.bike.lateral_speed = 0;
                } else if axis == 1 {
                    rider.bike.lateral = obstacle.right + width + 1;
                    rider.bike.lateral_speed = 0;
                } else {
                    let stop = if previous_progress <= obstacle.progress {
                        obstacle.progress - i64::from(obstacle.half_length) - i64::from(length) - 1
                    } else {
                        obstacle.progress + i64::from(obstacle.half_length) + i64::from(length) + 1
                    };
                    let corrected = stop.clamp(0, self.setup.topology.paths[&p.path].length);
                    rider.bike.distance_q8 += corrected - unit.recovery.bike_location.progress;
                    unit.recovery.bike_location.progress = corrected;
                    rider.bike.drive = 0;
                }
                unit.recovery.bike_location.lateral = rider.bike.lateral;
            }
            let first_this_tick = active.insert(key);
            if first_this_tick
                && !self.active_contacts.contains(&key)
                && (kind != ObstacleKind::Building
                    || self.units[&id].recovery.phase == Phase::Mounted)
            {
                let speed = actors[&id].view.rider.as_ref().unwrap().bike.speed;
                let reason = if kind == ObstacleKind::Rail { 3 } else { 1 };
                let side_brush = self.setup.rules.collision_version == 2
                    && kind == ObstacleKind::Building
                    && !frontal;
                let strength = if side_brush { side_speed } else { speed };
                self.hit(
                    actors,
                    c,
                    id,
                    Impact {
                        reason,
                        bike_damage: (strength / 1024).clamp(0, i32::from(u16::MAX)) as u16,
                        rider_velocity: Some([0, 0, 512]),
                        bike_velocity: Some([0, 0, 256]),
                        ..Default::default()
                    },
                    tick,
                    (!side_brush && speed > 6000) || kind == ObstacleKind::Cliff,
                    events,
                )?;
                Self::emit(events, tick, id, None, EffectKind::Contact);
                if kind == ObstacleKind::Building && !side_brush {
                    actors
                        .get_mut(&id)
                        .unwrap()
                        .view
                        .rider
                        .as_mut()
                        .unwrap()
                        .bike
                        .speed = 0;
                }
            }
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn detached_statics(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
        active: &mut BTreeSet<(u64, u64)>,
        mut random: impl FnMut() -> u32,
    ) -> Result<()> {
        let recovery = &self.units[&id].recovery;
        let Some(rider) = recovery
            .body
            .as_ref()
            .filter(|_| recovery.outcome.is_none())
        else {
            return Ok(());
        };
        let at = rider.location;
        let dimensions = self
            .setup
            .rules
            .collision
            .as_ref()
            .unwrap()
            .rider
            .as_ref()
            .unwrap()
            .dimensions;
        let mut candidates: Vec<_> = if self.setup.rules.scenery_version == 1 {
            self.scenery_index
                .get(&(at.path, (at.progress >> 8) as u32))
                .into_iter()
                .flatten()
                .filter_map(|index| {
                    self.scenery_obstacle(
                        &self.setup.rules.obstacles[*index],
                        c,
                        ContactObject {
                            owner: id,
                            detached: true,
                        },
                    )
                })
                .collect()
        } else {
            self.setup
                .rules
                .obstacles
                .iter()
                .filter_map(|o| {
                    self.scenery_obstacle(
                        o,
                        c,
                        ContactObject {
                            owner: id,
                            detached: true,
                        },
                    )
                })
                .collect()
        };
        if self.setup.rules.scenery_version == 1 {
            candidates.sort_by_key(|o| {
                o.source_box
                    .as_ref()
                    .and_then(|b| b.scenery.as_ref())
                    .map_or((2, u32::MAX), |m| (u8::from(m.class != 16), m.ordinal))
            });
        }
        let obstacle = candidates.into_iter().find(|o| {
            let Some(b) = &o.source_box else {
                return false;
            };
            let bounds = b.query_bounds();
            o.path == at.path
                && (o.progress - at.progress).abs() < i64::from(dimensions[1])
                && bounds[0] <= rider.motion.lateral + dimensions[0]
                && rider.motion.lateral - dimensions[0] <= bounds[2]
                && bounds[1] <= rider.motion.height + dimensions[2]
                && rider.motion.height - dimensions[2] < bounds[3]
        });
        if let Some(obstacle) = obstacle {
            let b = obstacle.source_box.as_ref().unwrap();
            if b.scenery
                .as_ref()
                .is_some_and(|m| m.class >= 3 && m.class < 16)
            {
                return Ok(());
            }
            let progress = rider.motion.distance_q8 + obstacle.progress - at.progress;
            let mut solid = crate::collision::Body {
                position: [progress as i32, b.lateral, 0],
                previous: [progress as i32, b.lateral, 0],
                half: [0, b.bounds[2] - b.bounds[0], b.bounds[3] - b.bounds[1]],
                velocity: [0; 3],
                speed: 0,
                mass: 0x1000000,
                category: 1,
                flags: 0,
                impulse: [0; 3],
            };
            if self.source_detached_object_contact(
                id,
                actors,
                &mut solid,
                c,
                (tick, events),
                &mut random,
            )? {
                let key = (id.0 * 2 + 1, (1u64 << 63) | obstacle.id);
                active.insert(key);
                if !self.active_contacts.contains(&key) {
                    Self::emit(events, tick, id, None, EffectKind::Contact);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn contacts(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
        mut random: impl FnMut() -> u32,
    ) -> Result<()> {
        self.sync_body_routes();
        self.scenery_begin(actors, c, tick)?;
        let mut active = BTreeSet::new();
        let mut ids: Vec<_> = self.units.keys().copied().collect();
        if self.setup.rules.collision_version == 2 {
            ids.sort_by_key(|id| (self.position(*id, actors).unwrap().progress, *id));
        }
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
        if self.setup.rules.bike_contacts && self.setup.rules.collision_version == 2 {
            let mut objects = vec![];
            for id in &ids {
                if self.units[id].recovery.outcome.is_some() {
                    continue;
                }
                objects.push(ContactObject {
                    owner: *id,
                    detached: false,
                });
                if (self.units[id].recovery.body.is_some()
                    || self.setup.rules.contact_walk_version >= 1)
                    && self.setup.rules.collision.as_ref().unwrap().rider.is_some()
                {
                    objects.push(ContactObject {
                        owner: *id,
                        detached: true,
                    });
                }
            }
            let contact_position = |o: &ContactObject| {
                let at = self.contact_location(*o);
                if let Some(roads) = &self.setup.rules.source_roads {
                    roads
                        .profiles
                        .iter()
                        .find_map(|profile| profile.selected.get(&at.path))
                        .map_or(self.proxy_body(*o, actors, c).position[0], |start| {
                            (i64::from(*start) * 256 + at.progress) as i32
                        })
                } else {
                    self.proxy_body(*o, actors, c).position[0]
                }
            };
            let initial_positions: BTreeMap<_, _> = objects
                .iter()
                .map(|o| (o.key(), contact_position(o)))
                .collect();
            if self.setup.rules.contact_walk_version == 2 {
                let ranks: BTreeMap<_, _> = self
                    .source_contact_order
                    .iter()
                    .enumerate()
                    .map(|(index, key)| (*key, index))
                    .collect();
                objects.sort_by_key(|o| {
                    (
                        contact_position(o),
                        ranks.get(&o.key()).copied().unwrap_or(usize::MAX),
                        o.key(),
                    )
                });
            } else {
                objects.sort_by_key(|o| (contact_position(o), o.key()));
            }
            let single_path = objects.first().is_none_or(|first| {
                objects
                    .iter()
                    .all(|o| self.contact_location(*o).path == self.contact_location(*first).path)
            });
            for (index, a) in objects.iter().enumerate() {
                if a.detached && self.units[&a.owner].recovery.body.is_none() {
                    continue;
                }
                if self.setup.rules.contact_walk_version >= 1 {
                    if a.detached {
                        self.detached_statics(
                            a.owner,
                            actors,
                            c,
                            tick,
                            events,
                            &mut active,
                            &mut random,
                        )?;
                    } else {
                        self.mounted_statics(
                            a.owner,
                            actors,
                            c,
                            tick,
                            events,
                            &mut active,
                            &mut random,
                        )?;
                    }
                }
                for b in objects.iter().skip(index + 1) {
                    if b.detached && self.units[&b.owner].recovery.body.is_none() {
                        continue;
                    }
                    if self.setup.rules.contact_walk_version >= 1
                        && (self.setup.rules.source_roads.is_none() || single_path)
                    {
                        let first = self.proxy_body(*a, actors, c);
                        let second = self.proxy_body(*b, actors, c);
                        let gap = if self.setup.rules.source_roads.is_some() {
                            (self.contact_location(*a).progress
                                - self.contact_location(*b).progress)
                                .abs()
                        } else {
                            i64::from(first.position[0]) - i64::from(second.position[0])
                        }
                        .abs();
                        if gap > i64::from(first.half[0] + second.half[0]) {
                            break;
                        }
                    }
                    if self.source_proxy_pair((*a, *b), actors, c, (tick, events), &mut random)? {
                        let key = (a.key().min(b.key()), a.key().max(b.key()));
                        active.insert(key);
                        if a.owner != b.owner && !self.active_contacts.contains(&key) {
                            Self::emit(events, tick, a.owner, Some(b.owner), EffectKind::Contact);
                        }
                    }
                }
            }
            if self.setup.rules.contact_walk_version == 2 {
                // The source sorts again after the callback walk. Dormant halves
                // retain their pre-contact bike mirror until the next move.
                objects.sort_by_key(|o| {
                    if o.detached && self.units[&o.owner].recovery.body.is_none() {
                        return initial_positions[&o.key()];
                    }
                    let at = self.contact_location(*o);
                    self.setup
                        .rules
                        .source_roads
                        .as_ref()
                        .and_then(|roads| {
                            roads
                                .profiles
                                .iter()
                                .find_map(|profile| profile.selected.get(&at.path))
                        })
                        .map_or(self.proxy_body(*o, actors, c).position[0], |start| {
                            (i64::from(*start) * 256 + at.progress) as i32
                        })
                });
                self.source_contact_order = objects.iter().map(|object| object.key()).collect();
            }
        }
        if self.setup.rules.bike_contacts && self.setup.rules.collision_version != 2 {
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
        if self.setup.rules.contact_walk_version == 0 {
            for id in ids {
                self.mounted_statics(id, actors, c, tick, events, &mut active, &mut random)?;
            }
            if self.setup.rules.collision_version == 2
                && self.setup.rules.collision.as_ref().unwrap().rider.is_some()
            {
                for id in self.units.keys().copied().collect::<Vec<_>>() {
                    self.detached_statics(id, actors, c, tick, events, &mut active, &mut random)?;
                }
            }
        }
        self.sync_body_routes();
        self.active_contacts = active;
        self.scenery_prepare(false)?;
        Ok(())
    }
    pub(crate) fn progress(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        for (id, u) in &mut self.units {
            let rider = actors.get_mut(id).unwrap().view.rider.as_mut().unwrap();
            let progress = if u.recovery.phase == Phase::Mounted {
                Some(rider.bike.distance_q8)
            } else {
                u.recovery.body.as_ref().and_then(|b| b.projected)
            };
            let route = if u.recovery.phase == Phase::Mounted {
                u.source_route
            } else {
                u.recovery
                    .body
                    .as_ref()
                    .and_then(|b| b.source_route)
                    .or(u.source_route)
            };
            let route_finish = self.setup.rules.source_roads.as_ref().and_then(|roads| {
                route
                    .and_then(|index| roads.profiles.get(usize::from(index)))?
                    .finish_progress
            });
            let finish_progress = route_finish
                .or(self.setup.rules.finish_progress)
                .unwrap_or((c.ground.len() as i64 - 1 - i64::from(c.start_sample)) * 256);
            if let (Some(position), Some(own_finish)) = (progress, route_finish) {
                let base = self
                    .setup
                    .rules
                    .finish_progress
                    .or_else(|| {
                        self.setup.rules.source_roads.as_ref().and_then(|roads| {
                            roads.profiles[usize::from(roads.initial)].finish_progress
                        })
                    })
                    .unwrap_or(own_finish);
                u.race_progress = Some(base - (own_finish - position));
            }
            let finish = !u.police && progress.is_some_and(|position| position >= finish_progress);
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
        self.scenery_prepare(true)?;
        Ok(())
    }
}

#[cfg(test)]
mod wall_tests {
    use super::*;
    #[test]
    fn sweeps_stop_frontal_and_lateral_tunneling_but_allow_diagonal_misses() {
        let o = Obstacle {
            source_box: None,
            id: 1,
            path: 0,
            progress: 1000,
            half_length: 100,
            left: 1000,
            right: 2000,
            height: 10000,
            kind: ObstacleKind::Building,
        };
        let at = |progress, lateral| Location {
            progress,
            lateral,
            ..Default::default()
        };
        assert_eq!(wall_entry(at(500, 1500), at(1500, 1500), &o, 0, 0), Some(0));
        assert_eq!(wall_entry(at(1000, 0), at(1000, 3000), &o, 0, 0), Some(1));
        assert_eq!(wall_entry(at(500, 0), at(1000, 500), &o, 0, 0), None);
        assert_eq!(wall_entry(at(1000, 3000), at(1000, 0), &o, 0, 0), Some(1));
        assert_eq!(wall_entry(at(800, 2100), at(1200, 2500), &o, 0, 0), None);
    }
}
