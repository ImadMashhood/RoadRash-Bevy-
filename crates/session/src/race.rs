use crate::Result;
use rr_simulation::{
    ActorId, Snapshot,
    recovery::{Outcome as Terminal, Phase as Recovery},
    topology::{Location, Topology},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Thrash,
    BigGame,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Won,
    Lost,
    Arrested,
    Wrecked,
    Quit,
    TimedOut,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub actor: ActorId,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RaceIdentity {
    pub id: u64,
    pub mode: Mode,
    pub course: u8,
    pub level: u8,
    pub bike: u8,
    pub seed: u32,
    pub roster: Vec<Entry>,
}
impl RaceIdentity {
    pub fn validate(&self) -> Result<()> {
        if self.id == 0
            || !(1..=5).contains(&self.course)
            || !(1..=5).contains(&self.level)
            || self.bike >= 15
            || self.roster.is_empty()
            || self.roster.len() > 15
            || self
                .roster
                .iter()
                .any(|e| e.name.is_empty() || e.name.len() > 80)
            || self
                .roster
                .iter()
                .map(|e| e.actor)
                .collect::<BTreeSet<_>>()
                .len()
                != self.roster.len()
            || !self.roster.iter().any(|e| e.actor == ActorId(1))
        {
            return Err("invalid race identity".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
    Countdown { remaining: u16 },
    Racing,
    Results,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Standing {
    pub actor: ActorId,
    pub name: String,
    pub place: u8,
    pub progress: Option<i64>,
    pub terminal: Option<Terminal>,
    pub completed_at: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    identity: RaceIdentity,
    outcome: Outcome,
    elapsed_ticks: u64,
    standings: Vec<Standing>,
    player_place: u8,
    bike_condition: i32,
}
impl Summary {
    pub fn validate(&self) -> Result<()> {
        self.identity.validate()?;
        if self.elapsed_ticks > 1_000_000
            || !(0..=100).contains(&self.bike_condition)
            || self.standings.len() != self.identity.roster.len()
            || self.standings.iter().enumerate().any(|(i, r)| {
                r.place != i as u8 + 1
                    || !self
                        .identity
                        .roster
                        .iter()
                        .any(|e| e.actor == r.actor && e.name == r.name)
            })
            || self
                .standings
                .iter()
                .map(|r| r.actor)
                .collect::<BTreeSet<_>>()
                .len()
                != self.standings.len()
            || self
                .standings
                .iter()
                .find(|r| r.actor == ActorId(1))
                .is_none_or(|r| r.place != self.player_place)
            || matches!(self.outcome, Outcome::Won) && self.player_place > 3
            || matches!(self.outcome, Outcome::Lost) && self.player_place <= 3
        {
            return Err("invalid immutable race summary".into());
        }
        Ok(())
    }

    pub fn identity(&self) -> &RaceIdentity {
        &self.identity
    }
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
    pub fn standings(&self) -> &[Standing] {
        &self.standings
    }
    pub fn place(&self) -> u8 {
        self.player_place
    }
    pub fn elapsed_ticks(&self) -> u64 {
        self.elapsed_ticks
    }
    pub fn bike_condition(&self) -> i32 {
        self.bike_condition
    }
}
/// Comparable progress interpolates alternate branch length between selected-route anchors.
/// A disconnected path is unranked; never compare its raw path-local distance.
#[derive(Clone)]
pub struct ProgressMap {
    topology: Topology,
}
impl ProgressMap {
    pub fn new(topology: Topology) -> Result<Self> {
        topology.validate().map_err(|e| e.to_string())?;
        Ok(Self { topology })
    }
    fn anchor(&self, start: u32, forward: bool) -> Option<(i64, i64)> {
        let mut pending = BTreeMap::from([(start, 0i64)]);
        let mut visited = BTreeSet::new();
        while let Some((&id, &distance)) = pending.iter().min_by_key(|(id, d)| (**d, **id)) {
            pending.remove(&id);
            if !visited.insert(id) {
                continue;
            }
            let p = &self.topology.paths[&id];
            if let Some(offset) = p.selected_start {
                return Some((distance, if forward { offset } else { offset + p.length }));
            }
            for e in p.portals.iter().filter(|e| e.end == forward) {
                let d = distance + p.length;
                let old = pending.entry(e.target).or_insert(i64::MAX);
                *old = (*old).min(d);
            }
        }
        None
    }
    pub fn at(&self, l: Location) -> Option<i64> {
        let p = self.topology.paths.get(&l.path)?;
        if !self.topology.valid(l) {
            return None;
        }
        if let Some(offset) = p.selected_start {
            return Some(offset + l.progress);
        }
        // Start from neighboring selected anchors; distance includes intermediate alternate paths.
        let before = p
            .portals
            .iter()
            .filter(|e| !e.end)
            .filter_map(|e| self.anchor(e.target, false))
            .min_by_key(|(d, a)| (*d, *a))?;
        let after = p
            .portals
            .iter()
            .filter(|e| e.end)
            .filter_map(|e| self.anchor(e.target, true))
            .min_by_key(|(d, a)| (*d, *a))?;
        let total = before.0 + p.length + after.0;
        if total <= 0 || after.1 < before.1 {
            return None;
        }
        Some(
            before.1
                + ((i128::from(before.0 + l.progress) * i128::from(after.1 - before.1))
                    / i128::from(total)) as i64,
        )
    }
}
#[derive(Clone)]
pub struct RaceSession {
    identity: RaceIdentity,
    phase: Phase,
    progress: Option<ProgressMap>,
    finishes: BTreeMap<ActorId, u64>,
    standings: Vec<Standing>,
    elapsed: u64,
    start_tick: u64,
    timeout: Option<u64>,
    condition_capacity: i32,
    summary: Option<Summary>,
}
impl RaceSession {
    pub fn new(
        identity: RaceIdentity,
        topology: Option<Topology>,
        snapshot: &Snapshot,
        countdown: u16,
        timeout: Option<u64>,
    ) -> Result<Self> {
        identity.validate()?;
        if countdown > 600 || timeout == Some(0) {
            return Err("race timing bounds".into());
        }
        let mut s = Self {
            identity,
            phase: if countdown == 0 {
                Phase::Racing
            } else {
                Phase::Countdown {
                    remaining: countdown,
                }
            },
            progress: topology.map(ProgressMap::new).transpose()?,
            finishes: BTreeMap::new(),
            standings: vec![],
            elapsed: 0,
            start_tick: snapshot.tick,
            timeout,
            condition_capacity: snapshot
                .actors
                .iter()
                .find(|a| a.id == ActorId(1))
                .and_then(|a| a.rider.as_ref())
                .map_or(100, |r| r.bike.condition)
                .max(1),
            summary: None,
        };
        s.update_standings(snapshot);
        Ok(s)
    }
    pub fn with_condition_capacity(mut self, capacity: i32) -> Result<Self> {
        if capacity < self.condition_capacity || capacity > 10000 {
            return Err("invalid bike capacity".into());
        }
        self.condition_capacity = capacity;
        Ok(self)
    }
    pub fn identity(&self) -> &RaceIdentity {
        &self.identity
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn standings(&self) -> &[Standing] {
        &self.standings
    }
    pub fn summary(&self) -> Option<&Summary> {
        self.summary.as_ref()
    }
    /// Countdown consumes session ticks, freezes all simulation/RNG, unlocks on the following tick.
    pub fn permit_tick(&mut self) -> bool {
        match self.phase {
            Phase::Countdown { remaining: 1 } => {
                self.phase = Phase::Racing;
                false
            }
            Phase::Countdown { remaining } => {
                self.phase = Phase::Countdown {
                    remaining: remaining - 1,
                };
                false
            }
            Phase::Racing => true,
            Phase::Results => false,
        }
    }
    fn progress_of(&self, a: &rr_simulation::ActorSnapshot) -> Option<i64> {
        if let Some(map) = &self.progress {
            let u = a.encounter.as_ref()?;
            let location = if u.recovery.phase == Recovery::Mounted {
                u.recovery.bike_location
            } else {
                u.recovery
                    .body
                    .as_ref()
                    .map_or(u.recovery.bike_location, |b| b.location)
            };
            map.at(location)
        } else {
            Some(a.position.progress.0)
        }
    }
    fn update_standings(&mut self, snapshot: &Snapshot) {
        let mut rows: Vec<_> = self
            .identity
            .roster
            .clone()
            .iter()
            .map(|entry| {
                let actor = snapshot.actors.iter().find(|a| a.id == entry.actor);
                let progress = actor.and_then(|a| self.progress_of(a));
                let unit = actor.and_then(|a| a.encounter.as_ref());
                let terminal = unit
                    .and_then(|u| u.recovery.outcome)
                    .or_else(|| actor.filter(|a| a.finished).map(|_| Terminal::Finished));
                let completed_at = unit
                    .and_then(|u| u.completed_at)
                    .or_else(|| terminal.map(|_| snapshot.tick));
                if terminal == Some(Terminal::Finished) && !self.finishes.contains_key(&entry.actor)
                {
                    // Native stepping records integer completion ticks. This engine resolves
                    // same-tick ties by ActorId; no unobserved sub-tick precision is invented.
                    self.finishes
                        .insert(entry.actor, completed_at.unwrap_or(snapshot.tick));
                }
                Standing {
                    actor: entry.actor,
                    name: entry.name.clone(),
                    place: 0,
                    progress,
                    terminal,
                    completed_at,
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            let af = self.finishes.get(&a.actor);
            let bf = self.finishes.get(&b.actor);
            match (af, bf) {
                (Some(x), Some(y)) => x.cmp(y).then(a.actor.cmp(&b.actor)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a
                    .terminal
                    .is_some()
                    .cmp(&b.terminal.is_some())
                    .then(b.progress.cmp(&a.progress))
                    .then(a.actor.cmp(&b.actor)),
            }
        });
        for (i, r) in rows.iter_mut().enumerate() {
            r.place = i as u8 + 1;
        }
        self.standings = rows;
    }
    pub fn observe(&mut self, snapshot: &Snapshot) {
        if self.phase != Phase::Racing || self.summary.is_some() {
            return;
        }
        self.elapsed = self
            .elapsed
            .max(snapshot.tick.saturating_sub(self.start_tick));
        self.update_standings(snapshot);
        let player = self
            .standings
            .iter()
            .find(|r| r.actor == ActorId(1))
            .unwrap();
        let outcome = match player.terminal {
            Some(Terminal::Finished) => Some(if player.place <= 3 {
                Outcome::Won
            } else {
                Outcome::Lost
            }),
            Some(Terminal::Arrested) => Some(Outcome::Arrested),
            Some(Terminal::Wrecked) => Some(Outcome::Wrecked),
            None if self.timeout.is_some_and(|t| self.elapsed >= t) => Some(Outcome::TimedOut),
            _ => None,
        };
        if let Some(outcome) = outcome {
            self.complete(outcome, snapshot);
        }
    }
    pub fn quit(&mut self, snapshot: &Snapshot) {
        if self.summary.is_none() {
            self.update_standings(snapshot);
            self.complete(Outcome::Quit, snapshot);
        }
    }
    fn complete(&mut self, outcome: Outcome, snapshot: &Snapshot) {
        self.summary = Some(Summary {
            identity: self.identity.clone(),
            outcome,
            elapsed_ticks: self.elapsed,
            standings: self.standings.clone(),
            player_place: self
                .standings
                .iter()
                .find(|s| s.actor == ActorId(1))
                .unwrap()
                .place,
            bike_condition: snapshot
                .actors
                .iter()
                .find(|a| a.id == ActorId(1))
                .and_then(|a| a.rider.as_ref())
                .map_or(100, |r| {
                    (r.bike.condition * 100 / self.condition_capacity).clamp(0, 100)
                }),
        });
        self.phase = Phase::Results;
    }
}
