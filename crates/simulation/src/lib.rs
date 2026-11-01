#![forbid(unsafe_code)]
//! Renderer-independent simulation contract. Current handling is explicitly authored,
//! while wrapping/rounding helpers have fixed documented integer semantics.
pub mod animation;
pub mod arithmetic;
pub mod handling;
pub mod replay;
pub mod riding;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub const TICKS_PER_SECOND: u32 = 60;
pub const SIMULATION_SCHEMA: u32 = 3;
macro_rules! id {
    ($name:ident,$t:ty) => {
        #[derive(
            Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub $t);
    };
}
id!(Tick, u64);
id!(ActorId, u64);
id!(RouteId, u32);
id!(BranchId, u32);
id!(SectionId, u32);
id!(Millimetres, i64);
id!(SpeedMmPerTick, u32);
impl Millimetres {
    pub fn from_metres(value: i64) -> Result<Self> {
        value.checked_mul(1000).map(Self).ok_or(Error::Overflow)
    }
    pub fn whole_metres(self) -> i64 {
        self.0 / 1000
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Invalid(&'static str),
    Overflow,
    MissingActor(ActorId),
    DuplicateCommand,
    PastCommand,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Command {
    pub throttle: bool,
    pub brake: bool,
    pub steer: i8,
    /// Proportional steering, -32767..=32767; absent in schema-1 replays.
    /// Cannot be combined with a nonzero legacy digital `steer`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steer_q15: Option<i16>,
    pub fork: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pose: Option<animation::Pose>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weapon: Option<animation::Weapon>,
    #[serde(skip_serializing_if = "is_zero")]
    pub damage: u32,
}
fn is_zero(v: &u32) -> bool {
    *v == 0
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QueuedCommand {
    pub tick: Tick,
    pub actor: ActorId,
    pub sequence: u64,
    pub input: Command,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    pub to: SectionId,
    pub branch: BranchId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Section {
    pub id: SectionId,
    pub length: Millimetres,
    pub next: Vec<Edge>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Route {
    pub id: RouteId,
    pub root: SectionId,
    pub sections: BTreeMap<SectionId, Section>,
}
impl Route {
    pub fn straight() -> Self {
        Self {
            id: RouteId(1),
            root: SectionId(0),
            sections: BTreeMap::from([(
                SectionId(0),
                Section {
                    id: SectionId(0),
                    length: Millimetres(1_000_000_000),
                    next: vec![],
                },
            )]),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.sections.is_empty()
            || self.sections.len() > 65536
            || !self.sections.contains_key(&self.root)
        {
            return Err(Error::Invalid("route root/count"));
        }
        for (key, section) in &self.sections {
            if *key != section.id
                || section.length.0 <= 0
                || section.next.len() > 2
                || section
                    .next
                    .iter()
                    .any(|e| !self.sections.contains_key(&e.to))
            {
                return Err(Error::Invalid("route section/edge"));
            }
        }
        fn visit(
            route: &Route,
            id: SectionId,
            depth: usize,
            active: &mut std::collections::BTreeSet<SectionId>,
            seen: &mut std::collections::BTreeSet<SectionId>,
        ) -> Result<()> {
            if depth > 256 || !active.insert(id) {
                return Err(Error::Invalid("route cycle/depth"));
            }
            if !seen.insert(id) {
                active.remove(&id);
                return Ok(());
            }
            for edge in &route.sections[&id].next {
                visit(route, edge.to, depth + 1, active, seen)?;
            }
            active.remove(&id);
            Ok(())
        }
        let mut seen = Default::default();
        visit(self, self.root, 0, &mut Default::default(), &mut seen)?;
        if seen.len() != self.sections.len() {
            return Err(Error::Invalid("unreachable route sections"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutePosition {
    pub route: RouteId,
    pub branch: BranchId,
    pub section: SectionId,
    pub progress: Millimetres,
    pub lateral: Millimetres,
    pub elevation: Millimetres,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorSnapshot {
    pub id: ActorId,
    pub position: RoutePosition,
    pub speed: SpeedMmPerTick,
    pub finished: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rider: Option<riding::Rider>,
}
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u64,
    pub speed_mm_per_tick: u32,
    pub distance_mm: u64,
    pub lateral_mm: i32,
    pub actors: Vec<ActorSnapshot>,
}
#[derive(Clone)]
struct Actor {
    view: ActorSnapshot,
    input: Command,
    total: u64,
    spawn_random: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Stage {
    Input,
    Spawn,
    Decisions,
    Movement,
    Contacts,
    Combat,
    Progression,
    Snapshot,
}
pub const STAGES: [Stage; 8] = [
    Stage::Input,
    Stage::Spawn,
    Stage::Decisions,
    Stage::Movement,
    Stage::Contacts,
    Stage::Combat,
    Stage::Progression,
    Stage::Snapshot,
];
pub fn ordered_stages(mut stages: Vec<Stage>) -> Vec<Stage> {
    stages.sort();
    stages.dedup();
    stages
}
/// MSVC-style LCG semantics from pinned career research; dedicated streams isolate consumers.
#[derive(Clone)]
struct Random(u32);
impl Random {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        (self.0 >> 16) & 0x7fff
    }
}
pub struct Simulation {
    schema_version: u32,
    tick: Tick,
    route: Route,
    actors: BTreeMap<ActorId, Actor>,
    next_actor: u64,
    queue: BTreeMap<(Tick, ActorId, u64), Command>,
    spawn_rng: Random,
    gameplay_rng: Random,
    last_stages: Vec<Stage>,
    handling: Option<riding::Configuration>,
}
impl Default for Simulation {
    fn default() -> Self {
        Self::new(1, Route::straight()).expect("valid authored default route")
    }
}
impl Simulation {
    pub fn new(seed: u32, route: Route) -> Result<Self> {
        Self::with_schema(seed, route, 2)
    }
    pub fn with_schema(seed: u32, route: Route, schema_version: u32) -> Result<Self> {
        if !(1..=2).contains(&schema_version) {
            return Err(Error::Invalid("unsupported simulation schema"));
        }
        route.validate()?;
        let mut sim = Self {
            schema_version,
            tick: Tick(0),
            route,
            actors: BTreeMap::new(),
            next_actor: 1,
            queue: BTreeMap::new(),
            spawn_rng: Random(seed ^ 0x53504157),
            gameplay_rng: Random(seed),
            last_stages: vec![],
            handling: None,
        };
        sim.spawn()?;
        Ok(sim)
    }
    pub fn with_handling(seed: u32, config: riding::Configuration) -> Result<Self> {
        config.validate()?;
        let mut sim = Self::with_schema(seed, Route::straight(), 2)?;
        sim.schema_version = 3;
        sim.actors.get_mut(&ActorId(1)).unwrap().view.rider = Some(riding::Rider::new(&config)?);
        sim.handling = Some(config);
        Ok(sim)
    }
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn spawn(&mut self) -> Result<ActorId> {
        let id = ActorId(self.next_actor);
        self.next_actor = self.next_actor.checked_add(1).ok_or(Error::Overflow)?;
        let actor = Actor {
            view: ActorSnapshot {
                id,
                position: RoutePosition {
                    route: self.route.id,
                    branch: BranchId(0),
                    section: self.route.root,
                    progress: Millimetres(0),
                    lateral: Millimetres(0),
                    elevation: Millimetres(0),
                },
                speed: SpeedMmPerTick(0),
                finished: false,
                rider: self.handling.as_ref().map(riding::Rider::new).transpose()?,
            },
            input: Command::default(),
            total: 0,
            spawn_random: self.spawn_rng.next(),
        };
        self.actors.insert(id, actor);
        Ok(id)
    }
    pub fn despawn(&mut self, id: ActorId) -> Result<()> {
        self.actors.remove(&id).ok_or(Error::MissingActor(id))?;
        self.queue.retain(|(_, actor, _), _| *actor != id);
        Ok(())
    }
    pub fn tick(&self) -> Tick {
        self.tick
    }
    pub fn stages(&self) -> &[Stage] {
        &self.last_stages
    }
    pub fn draw_gameplay_random(&mut self) -> u32 {
        self.gameplay_rng.next()
    }
    pub fn queue(&mut self, command: QueuedCommand) -> Result<()> {
        if command.tick < self.tick {
            return Err(Error::PastCommand);
        }
        if !self.actors.contains_key(&command.actor) {
            return Err(Error::MissingActor(command.actor));
        }
        if (self.schema_version < 3
            && (command.input.pose.is_some()
                || command.input.weapon.is_some()
                || command.input.damage != 0))
            || !(-1..=1).contains(&command.input.steer)
            || command.input.steer_q15.is_some_and(|axis| {
                axis == i16::MIN || command.input.steer != 0 || self.schema_version == 1
            })
            || command.input.fork.is_some_and(|b| b > 1)
            || command.tick.0 - self.tick.0 > 1_000_000
            || self.queue.len() >= 65536
        {
            return Err(Error::Invalid("command range/queue limit"));
        }
        let key = (command.tick, command.actor, command.sequence);
        if self.queue.contains_key(&key) {
            return Err(Error::DuplicateCommand);
        }
        self.queue.insert(key, command.input);
        Ok(())
    }
    pub fn step(&mut self, input: Command) -> Result<()> {
        let sequence = self
            .queue
            .range((self.tick, ActorId(1), 0)..=(self.tick, ActorId(1), u64::MAX))
            .next_back()
            .map(|((_, _, seq), _)| seq.checked_add(1).ok_or(Error::Overflow))
            .transpose()?
            .unwrap_or(0);
        self.queue(QueuedCommand {
            tick: self.tick,
            actor: ActorId(1),
            sequence,
            input,
        })?;
        self.advance()
    }
    pub fn advance(&mut self) -> Result<()> {
        let next_tick = self.tick.0.checked_add(1).ok_or(Error::Overflow)?;
        self.last_stages.clear();
        for stage in STAGES {
            self.last_stages.push(stage);
            match stage {
                Stage::Input => {
                    while self
                        .queue
                        .first_key_value()
                        .is_some_and(|((tick, _, _), _)| *tick == self.tick)
                    {
                        let ((_, id, _), input) = self.queue.pop_first().unwrap();
                        self.actors
                            .get_mut(&id)
                            .ok_or(Error::MissingActor(id))?
                            .input = input;
                    }
                }
                Stage::Movement => {
                    for actor in self.actors.values_mut() {
                        if actor.view.finished {
                            continue;
                        }
                        if let (Some(config), Some(rider)) = (&self.handling, &mut actor.view.rider)
                        {
                            rider.step(config, actor.input, self.tick.0 as u32)?;
                            actor.input.damage = 0;
                            actor.input.pose = None;
                            actor.input.weapon = None;
                            actor.total = rider.bike.distance_q8.max(0) as u64 * 8;
                            actor.view.speed =
                                SpeedMmPerTick((rider.bike.speed.max(0) / 256) as u32);
                            actor.view.position.lateral =
                                Millimetres(i64::from(rider.bike.lateral) * 10 / 256);
                            actor.view.position.elevation =
                                Millimetres(i64::from(rider.bike.height) * 10 / 256);
                            actor.view.position.progress = Millimetres(actor.total as i64);
                            if i64::from(config.start_sample) + (rider.bike.distance_q8 >> 8)
                                >= config.ground.len() as i64 - 1
                            {
                                actor.view.finished = true;
                                rider.bike.speed = 0;
                            }
                            continue;
                        }
                        let delta = if actor.input.brake {
                            -25
                        } else if actor.input.throttle {
                            8
                        } else {
                            -3
                        };
                        actor.view.speed.0 =
                            (i64::from(actor.view.speed.0) + delta).clamp(0, 1000) as u32;
                        let steering = actor
                            .input
                            .steer_q15
                            .map_or(i64::from(actor.input.steer) * 32767, i64::from);
                        actor.view.position.lateral.0 = (actor.view.position.lateral.0
                            + steering * i64::from(actor.view.speed.0 / 12) / 32767)
                            .clamp(-4000, 4000);
                        let mut remaining = i64::from(actor.view.speed.0);
                        for _ in 0..=self.route.sections.len() {
                            let section = &self.route.sections[&actor.view.position.section];
                            let available = section.length.0 - actor.view.position.progress.0;
                            let used = remaining.min(available);
                            actor.view.position.progress.0 += used;
                            actor.total = actor.total.saturating_add(used as u64);
                            remaining -= used;
                            if actor.view.position.progress != section.length {
                                break;
                            }
                            if section.next.is_empty() {
                                actor.view.finished = true;
                                actor.view.speed = SpeedMmPerTick(0);
                                break;
                            }
                            let choice = usize::from(actor.input.fork.unwrap_or(0))
                                .min(section.next.len() - 1);
                            let edge = &section.next[choice];
                            actor.view.position.section = edge.to;
                            actor.view.position.branch = edge.branch;
                            actor.view.position.progress = Millimetres(0);
                            if remaining == 0 {
                                break;
                            }
                        }
                    }
                }
                Stage::Contacts => {
                    for actor in self.actors.values_mut() {
                        if actor.view.rider.is_none() {
                            actor.view.position.lateral.0 =
                                actor.view.position.lateral.0.clamp(-4000, 4000);
                        }
                    }
                }
                // Explicit reserved stages: no opponent AI, combat or career rules are claimed here.
                Stage::Spawn
                | Stage::Decisions
                | Stage::Combat
                | Stage::Progression
                | Stage::Snapshot => {}
            }
        }
        self.tick = Tick(next_tick);
        Ok(())
    }
    pub fn snapshot(&self) -> Snapshot {
        let player = self.actors.get(&ActorId(1));
        Snapshot {
            tick: self.tick.0,
            speed_mm_per_tick: player.map_or(0, |p| p.view.speed.0),
            distance_mm: player.map_or(0, |p| p.total),
            lateral_mm: player.map_or(0, |p| p.view.position.lateral.0 as i32),
            actors: self.actors.values().map(|a| a.view.clone()).collect(),
        }
    }
    /// Fixed-width little-endian fields in stable ID order. Excludes rendering and allocation addresses.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = vec![];
        macro_rules! put {
            ($v:expr) => {
                b.extend_from_slice(&$v.to_le_bytes())
            };
        }
        put!(self.schema_version);
        if self.schema_version >= 3 {
            let config = serde_json::to_vec(&self.handling).expect("finite integer configuration");
            put!(config.len() as u32);
            b.extend(config);
        }
        put!(self.tick.0);
        put!(self.next_actor);
        put!(self.spawn_rng.0);
        put!(self.gameplay_rng.0);
        put!(self.route.id.0);
        put!(self.route.root.0);
        put!(self.route.sections.len() as u32);
        for section in self.route.sections.values() {
            put!(section.id.0);
            put!(section.length.0);
            put!(section.next.len() as u32);
            for edge in &section.next {
                put!(edge.to.0);
                put!(edge.branch.0);
            }
        }
        put!(self.actors.len() as u32);
        fn input(b: &mut Vec<u8>, c: Command, schema: u32) {
            b.extend([
                u8::from(c.throttle),
                u8::from(c.brake),
                c.steer as u8,
                c.fork.unwrap_or(255),
            ]);
            if schema >= 3 {
                let c = serde_json::to_vec(&c).expect("integer command");
                b.extend_from_slice(&(c.len() as u32).to_le_bytes());
                b.extend(c);
            }
            if schema >= 2 {
                b.push(u8::from(c.steer_q15.is_some()));
                b.extend_from_slice(&c.steer_q15.unwrap_or(0).to_le_bytes());
            }
        }
        for (id, actor) in &self.actors {
            put!(id.0);
            put!(actor.view.position.route.0);
            put!(actor.view.position.branch.0);
            put!(actor.view.position.section.0);
            put!(actor.view.position.progress.0);
            put!(actor.view.position.lateral.0);
            put!(actor.view.position.elevation.0);
            put!(actor.view.speed.0);
            put!(actor.total);
            put!(actor.spawn_random);
            b.push(u8::from(actor.view.finished));
            input(&mut b, actor.input, self.schema_version);
            if self.schema_version >= 3 {
                let state = serde_json::to_vec(&actor.view.rider).expect("integer state");
                put!(state.len() as u32);
                b.extend(state);
            }
        }
        put!(self.queue.len() as u32);
        for ((tick, actor, seq), command) in &self.queue {
            put!(tick.0);
            put!(actor.0);
            put!(*seq);
            input(&mut b, *command, self.schema_version);
        }
        b
    }
    pub fn state_hash(&self) -> String {
        format!("{:x}", Sha256::digest(self.canonical_bytes()))
    }
}
