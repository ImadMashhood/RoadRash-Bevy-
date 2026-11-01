//! Schema-5 population decisions. Source primitives and explicit modern policies
//! are documented separately in docs/OPPONENTS.md and docs/ROAD_USERS.md.
use crate::{
    combat::{AttackKind, Request, Side},
    encounter::World as Encounters,
    recovery::{Impact, Outcome, Phase},
    topology::{Location, Portal, Topology},
    *,
};
mod opponents;
mod primitives;
mod road_users;
mod source;
pub use primitives::{Timers, arrest, live_interval, speed_from_pace, vehicle_choice};
use serde::{Deserialize, Serialize};
pub use source::vehicle_masks;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Traits {
    pub personality: Option<u8>,
    pub pace: i32,
    pub corner_pace: i32,
    /// None means source semantics unverified; policy uses a documented default.
    pub aggression: Option<u8>,
    pub overtake_index: Option<u8>,
    pub kick_roll: Option<i8>,
    pub backhand_roll: Option<i8>,
    pub cooldown: u16,
    pub bike: u32,
    pub branch_choice: Option<u8>,
}
impl Default for Traits {
    fn default() -> Self {
        Self {
            personality: None,
            pace: 110,
            corner_pace: 85,
            aggression: None,
            overtake_index: None,
            kick_roll: None,
            backhand_roll: None,
            cooldown: 90,
            bike: 1,
            branch_choice: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Brain {
    pub traits: Traits,
    pub next_tick: u64,
    pub last_tick: Option<u64>,
    pub interval: u16,
    pub decisions: u64,
    pub desired_lane: i32,
    pub target_speed: i32,
    pub target: Option<ActorId>,
    pub attack_after: u64,
    pub yielding: bool,
    pub command: Command,
}
impl Brain {
    pub fn new(traits: Traits) -> Self {
        Self {
            traits,
            next_tick: 0,
            last_tick: None,
            interval: 4,
            decisions: 0,
            desired_lane: 0,
            target_speed: 0,
            target: None,
            attack_after: 0,
            yielding: false,
            command: Command::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Vehicle,
    Pedestrian,
    Police,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Cruise,
    Waiting,
    Crossing,
    Hit,
    Recovering,
    Patrol,
    Chase,
    Disengaged,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Spawn {
    pub id: u64,
    pub location: Location,
    pub kind: Kind,
    pub family: Option<u16>,
    pub selector: u8,
    pub subtype: u8,
    pub speed: i32,
    pub direction: i8,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    pub id: u64,
    pub path: u32,
    pub start: i64,
    pub end: i64,
    pub density: i8,
    pub variant: u8,
    pub lanes: u8,
    #[serde(default, skip_serializing_if = "byte_zero")]
    pub flags: u8,
}
fn false_value(value: &bool) -> bool {
    !value
}
fn byte_zero(value: &u8) -> bool {
    *value == 0
}
impl Zone {
    pub fn from_event(id: u64, path: u32, length: i64, event: &rr_course::Event) -> Option<Self> {
        let start = i64::from(event.start.max(0)) * 256;
        let end = (i64::from(event.end) * 256).min(length);
        (end > start).then_some(Self {
            id,
            path,
            start,
            end,
            density: event.parameters[1] as i8,
            variant: event.raw_type.saturating_add(1).min(15),
            lanes: event.parameters[0].min(16),
            flags: event.parameters[2],
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PedestrianAppearance {
    pub family: u16,
    pub selector: u8,
    pub animation: animation::Library,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    #[serde(default, skip_serializing_if = "false_value")]
    pub functional_traffic: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pedestrian_appearances: Vec<PedestrianAppearance>,
    pub brains: BTreeMap<ActorId, Traits>,
    pub zones: Vec<Zone>,
    pub spawns: Vec<Spawn>,
    pub traffic: bool,
    pub periodic_police: bool,
    pub detection: i64,
    pub disengage: i64,
    pub vehicle_types: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vehicle_masks: Vec<u32>,
}
impl Default for Setup {
    fn default() -> Self {
        Self {
            functional_traffic: false,
            pedestrian_appearances: vec![],
            brains: BTreeMap::new(),
            zones: vec![],
            spawns: vec![],
            traffic: true,
            periodic_police: true,
            detection: 8192,
            disengage: 16384,
            vehicle_types: 1,
            vehicle_masks: vec![],
        }
    }
}
impl Setup {
    pub fn validate(&self, c: &riding::Configuration, e: &encounter::Setup) -> Result<()> {
        if self.pedestrian_appearances.len() > 256 {
            return Err(Error::Invalid("Pedestrian appearance budget"));
        }
        let mut appearances = BTreeSet::new();
        for appearance in &self.pedestrian_appearances {
            if appearance.family == 0
                || appearance.selector > 63
                || !appearances.insert((appearance.family, appearance.selector))
            {
                return Err(Error::Invalid("Pedestrian appearance identity"));
            }
            appearance
                .animation
                .validate()
                .map_err(|_| Error::Invalid("Pedestrian animation bank"))?;
        }
        if self.brains.len() > 14
            || self.zones.len() > 20000
            || self.spawns.len() > 20000
            || !(512..=65536).contains(&self.detection)
            || !(self.detection..=131072).contains(&self.disengage)
            || self.vehicle_types == 0
            || self.vehicle_types > 64
            || (!self.vehicle_masks.is_empty()
                && self.vehicle_masks.len() != usize::from(self.vehicle_types))
        {
            return Err(Error::Invalid("population bounds"));
        }
        for (id, t) in &self.brains {
            if id.0 <= 1
                || !e.seeds.iter().any(|s| s.id == *id)
                || !(1..=1000).contains(&t.pace)
                || !(1..=1000).contains(&t.corner_pace)
                || t.cooldown < 10
                || t.cooldown > 3600
                || t.branch_choice.is_some_and(|v| v > 1)
                || !c.specs.iter().any(|s| s.id == t.bike)
            {
                return Err(Error::Invalid("opponent traits/identity"));
            }
        }
        let mut ids = BTreeSet::new();
        for s in &self.spawns {
            if s.id == 0
                || !ids.insert(s.id)
                || !e.topology.valid(s.location)
                || !(0..=50000).contains(&s.speed)
                || ![-1, 1].contains(&s.direction)
            {
                return Err(Error::Invalid("population spawn"));
            }
        }
        ids.clear();
        for z in &self.zones {
            if z.id == 0
                || !ids.insert(z.id)
                || z.start < 0
                || z.start >= z.end
                || e.topology
                    .paths
                    .get(&z.path)
                    .is_none_or(|p| z.end > p.length)
                || z.variant > 15
                || z.lanes > 16
            {
                return Err(Error::Invalid("population zone"));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cruise_speed: Option<i32>,
    pub id: u64,
    pub kind: Kind,
    pub state: AgentState,
    pub location: Location,
    pub previous: Location,
    pub speed: i32,
    pub direction: i8,
    pub family: Option<u16>,
    pub selector: u8,
    pub subtype: u8,
    pub born: u64,
    pub state_since: u64,
    pub hit_count: u32,
    pub projected: Option<i64>,
    pub siren: bool,
    pub animation: animation::Player,
    pub frame: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Controller {
    pub zone: u64,
    pub since: u64,
    pub next_tick: u64,
    pub phase: u8,
    pub next_spawn: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct World {
    pub setup: Setup,
    pub brains: BTreeMap<ActorId, Brain>,
    pub agents: BTreeMap<u64, Agent>,
    pub timers: Timers,
    pub consumed: BTreeSet<u64>,
    pub active_zones: BTreeSet<u64>,
    pub controllers: BTreeMap<u64, Controller>,
    pub next_id: u64,
    pub previous_progress: i64,
    pub contacts: BTreeSet<(u64, u64)>,
    pub pedestrian_hits: u32,
    pub navigation: BTreeMap<ActorId, Portal>,
    #[serde(skip)]
    pub trace_enabled: bool,
    #[serde(skip)]
    pub trace: Vec<Draw>,
    #[serde(skip)]
    pub cues: Vec<Cue>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draw {
    pub tick: u64,
    pub actor: u64,
    pub category: &'static str,
    pub value: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cue {
    SirenStart(u64),
    SirenStop(u64),
    Warning(u64),
    PedestrianHit(u64),
}
impl World {
    pub fn new(setup: Setup) -> Self {
        Self {
            brains: setup
                .brains
                .iter()
                .map(|(id, t)| (*id, Brain::new(t.clone())))
                .collect(),
            setup,
            agents: BTreeMap::new(),
            timers: Timers::default(),
            consumed: BTreeSet::new(),
            active_zones: BTreeSet::new(),
            controllers: BTreeMap::new(),
            next_id: 1,
            previous_progress: 0,
            contacts: BTreeSet::new(),
            pedestrian_hits: 0,
            navigation: BTreeMap::new(),
            trace_enabled: false,
            trace: vec![],
            cues: vec![],
        }
    }
    fn draw(
        &mut self,
        tick: u64,
        actor: u64,
        category: &'static str,
        random: &mut impl FnMut() -> u32,
    ) -> u32 {
        let value = random();
        if self.trace_enabled {
            self.trace.push(Draw {
                tick,
                actor,
                category,
                value,
            });
        }
        value
    }
    pub fn audible_sirens(&self, paused: bool) -> Vec<u64> {
        if paused {
            vec![]
        } else {
            self.agents
                .values()
                .filter(|a| a.siren)
                .map(|a| a.id)
                .collect()
        }
    }
    pub fn remove(&mut self, id: u64) {
        if self.agents.remove(&id).is_some_and(|a| a.siren) {
            self.cues.push(Cue::SirenStop(id));
        }
        self.contacts.retain(|(_, a)| *a != id);
    }
    pub fn reset(&mut self) {
        let sirens = self.audible_sirens(false);
        let trace = self.trace_enabled;
        *self = Self::new(self.setup.clone());
        self.trace_enabled = trace;
        self.cues = sirens.into_iter().map(Cue::SirenStop).collect();
    }
    pub fn spawn(&mut self, s: &Spawn, tick: u64, t: &Topology) -> Result<Option<u64>> {
        if !t.valid(s.location)
            || !(0..=50000).contains(&s.speed)
            || ![-1, 1].contains(&s.direction)
            || s.subtype > 15
        {
            return Err(Error::Invalid("spawn route"));
        }
        let cap = match s.kind {
            Kind::Vehicle => 16,
            Kind::Pedestrian => 15,
            Kind::Police => 1,
        };
        if self.agents.values().filter(|a| a.kind == s.kind).count() >= cap {
            return Ok(None);
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or(Error::Overflow)?;
        self.agents.insert(
            id,
            Agent {
                cruise_speed: None,
                id,
                kind: s.kind,
                state: match s.kind {
                    Kind::Vehicle => {
                        if s.subtype == 1 {
                            AgentState::Crossing
                        } else {
                            AgentState::Cruise
                        }
                    }
                    Kind::Pedestrian => AgentState::Waiting,
                    Kind::Police => AgentState::Patrol,
                },
                location: s.location,
                previous: s.location,
                speed: s.speed,
                direction: s.direction,
                family: s.family,
                selector: s.selector,
                subtype: s.subtype,
                born: tick,
                state_since: tick,
                hit_count: 0,
                projected: t.projected(s.location),
                siren: false,
                animation: animation::Player::default(),
                frame: 2,
            },
        );
        Ok(Some(id))
    }
}

/// Presentation-side loop ownership. Reconcile on pause/resume/restart/despawn;
/// Phase 16 can attach audio voices without owning pursuit decisions.
#[derive(Default)]
pub struct SirenBus {
    active: BTreeSet<u64>,
}
impl SirenBus {
    pub fn reconcile(&mut self, desired: impl IntoIterator<Item = u64>) -> Vec<Cue> {
        let desired: BTreeSet<_> = desired.into_iter().collect();
        let mut events: Vec<_> = self
            .active
            .difference(&desired)
            .copied()
            .map(Cue::SirenStop)
            .collect();
        events.extend(
            desired
                .difference(&self.active)
                .copied()
                .map(Cue::SirenStart),
        );
        self.active = desired;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ongoing_pursuit_takes_the_real_fork_portal() {
        let c = fixtures::riding();
        let mut e = encounter::Setup::authored(&c).unwrap();
        let mut t = Topology::straight(2048);
        t.paths.get_mut(&0).unwrap().portals = vec![
            Portal {
                end: true,
                target: 1,
                target_end: false,
            },
            Portal {
                end: true,
                target: 2,
                target_end: false,
            },
        ];
        for path in [1, 2] {
            t.paths.insert(
                path,
                crate::topology::Path {
                    length: 10000,
                    branch: path,
                    selected_start: None,
                    rises: vec![],
                    portals: vec![Portal {
                        end: false,
                        target: 0,
                        target_end: true,
                    }],
                },
            );
        }
        e.topology = t;
        e.rules.finish_progress = Some(100000);
        e.seeds[0].location = Some(Location {
            path: 2,
            progress: 1000,
            ..Default::default()
        });
        e.seeds[0].distance = 1000;
        let mut s = Simulation::with_population(
            1,
            c,
            e,
            Setup {
                traffic: false,
                periodic_police: false,
                ..Default::default()
            },
        )
        .unwrap();
        let id = s
            .spawn_road_user(&Spawn {
                id: 1,
                location: Location {
                    path: 0,
                    progress: 2040,
                    ..Default::default()
                },
                kind: Kind::Police,
                family: None,
                selector: 0,
                subtype: 0,
                speed: 6000,
                direction: 1,
            })
            .unwrap()
            .unwrap();
        let cop = s.population.as_mut().unwrap().agents.get_mut(&id).unwrap();
        cop.state = AgentState::Chase;
        cop.siren = true;
        s.advance().unwrap();
        let cop = &s.population().unwrap().agents[&id];
        assert_eq!(cop.location.path, 2);
        assert!(cop.location.progress < 100);
        assert!(cop.location.progress < 1000);
        assert!(cop.siren);
    }
}
