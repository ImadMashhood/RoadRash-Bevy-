//! Native police setup and lifecycle. A cop is a shared encounter rider, not a
//! kinematic road prop. The optional config preserves historical recordings.
use super::*;
use rr_formats::{binary::u32le, pe::SupportedExe};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub bike_template: [i32; 26],
    pub rider_template: [i32; 26],
    pub atan: Vec<u8>,
    pub personality: Vec<i32>,
    pub personalities: Vec<Vec<i32>>,
    pub overtake_eagerness: [i32; 5],
    pub stats: [i32; 16],
    pub bikes: [u8; 5],
    pub throttle_percent: [i32; 5],
    pub messages: [i32; 8],
}
impl Config {
    pub fn read(exe: &[u8]) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let pe = SupportedExe::read(exe)?;
        let words =
            |va: u32, count: usize| -> std::result::Result<Vec<i32>, Box<dyn std::error::Error>> {
                let bytes = pe.read_va(va, count * 4)?;
                (0..count)
                    .map(|i| Ok(u32le(bytes, i * 4)? as i32))
                    .collect()
            };
        let value = Self {
            bike_template: words(0x464310, 26)?.try_into().unwrap(),
            rider_template: words(0x464378, 26)?.try_into().unwrap(),
            atan: pe.read_va(0x476768, 257)?.to_vec(),
            personality: words(0x464558 + 9 * 0x204, 129)?,
            personalities: (0..10)
                .map(|i| words(0x464558 + i * 0x204, 129))
                .collect::<std::result::Result<_, _>>()?,
            overtake_eagerness: words(0x465a10, 5)?.try_into().unwrap(),
            stats: words(0x463c78 + 15 * 64, 16)?.try_into().unwrap(),
            bikes: pe.read_va(0x4641c8 + 15 * 5, 5)?.try_into().unwrap(),
            throttle_percent: words(0x4642a0, 5)?.try_into().unwrap(),
            messages: words(0x464410, 8)?.try_into().unwrap(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        if self.bike_template[1] != 4
            || self.rider_template[1] != 8
            || self.atan.len() != 257
            || self.atan.iter().any(|v| *v > 128)
            || self.personality.len() != 129
            || self.personalities.len() != 10
            || self.personalities.iter().any(|p| p.len() != 129)
            || self.bikes.iter().any(|b| *b >= 64)
            || self.throttle_percent.iter().any(|v| !(1..=200).contains(v))
            || self.stats[3] < 2
            || self.stats[3] > 2048
        {
            return Err(Error::Invalid("native police configuration"));
        }
        self.stamina().validate()
    }
    pub fn stamina(&self) -> combat::Stats {
        combat::Stats {
            strength: self.stats[12],
            base: self.stats[13],
            maximum: self.stats[13],
            health: self.stats[13],
        }
    }
    pub fn weapon(&self, level: u8) -> animation::Weapon {
        match self.stats[6 + usize::from(level)] {
            1 => animation::Weapon::Club,
            2 => animation::Weapon::Chain,
            _ => animation::Weapon::Fists,
        }
    }
    /// Exact 0x00406b80 relationship reset, with native truncating division.
    pub fn pursuit(&self, level: u8, place: u8, personality: u8) -> Result<(Vec<i32>, i32)> {
        if level > 4 || place > 15 || personality > 9 {
            return Err(Error::Invalid("police pursuit input"));
        }
        let mut words = self.personality.clone();
        for i in 0..10 {
            words[i * 3 + 1] = words[0x7c / 4 + i * 3];
        }
        words[0x1a4 / 4] = words[0x1bc / 4 + usize::from(level)];
        let factor = 16 - if place < 4 { 4 - i32::from(place) } else { 0 };
        let relationship = factor * 4000 - i32::from(level) * 6000;
        words[0x80 / 4 + usize::from(personality) * 3] = relationship;
        words[0x8 / 4 + usize::from(personality) * 3] = relationship;
        let base = factor * 2000;
        Ok((
            words,
            base + (base - i32::from(level) * 3000) / 2 - i32::from(level) * 3000,
        ))
    }
    pub(crate) fn configuration(
        &self,
        base: &riding::Configuration,
        level: u8,
    ) -> Result<riding::Configuration> {
        let mut c = base.clone();
        c.selected = u32::from(self.bikes[usize::from(level)]) + 1;
        let spec = c
            .specs
            .iter_mut()
            .find(|s| s.id == c.selected)
            .ok_or(Error::Invalid("police bike specification"))?;
        spec.ai = true;
        spec.rider_value = Some(self.stats[3] / 2);
        spec.throttle_percent = Some(self.throttle_percent[usize::from(level)]);
        for (gear, reduction) in spec.gears.iter_mut().zip([25, 15, 8, 4]) {
            gear.force -= reduction;
        }
        c.tables.object = self.bike_template;
        c.recovery_animation = Some(base.animation.clone());
        c.animation.banks[0] = c.animation.banks[1].clone();
        c.animation.views[0] = c.animation.views[1].clone();
        c.validate()?;
        Ok(c)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Live {
    pub actor: ActorId,
    pub agent: u64,
    pub lane: u8,
    pub pursuit: Vec<i32>,
    pub player_relationship: i32,
    pub removal_pending: bool,
}

impl Simulation {
    pub(crate) fn police_spawn_sync(&mut self) -> Result<()> {
        let Some(p) = self.population.as_ref() else {
            return Ok(());
        };
        if p.setup.police.is_none() {
            return Ok(());
        }
        if let Some(live) = p.police.clone() {
            if live.removal_pending || !p.agents.contains_key(&live.agent) {
                self.population.as_mut().unwrap().remove(live.agent);
                self.despawn(live.actor)?;
                self.population.as_mut().unwrap().police = None;
            }
            return Ok(());
        }
        let Some(agent) = p.agents.values().find(|a| a.kind == Kind::Police).cloned() else {
            return Ok(());
        };
        let config = p.setup.police.as_ref().unwrap().clone();
        let e = self.encounter.as_ref().unwrap();
        let owner = &e.units[&ActorId(1)];
        let level = e.setup.rules.level;
        let mut c = config.configuration(self.handling.as_ref().unwrap(), level)?;
        let route = owner
            .recovery
            .body
            .as_ref()
            .and_then(|b| b.source_route)
            .or(owner.source_route);
        let mut at = agent.location;
        let distance = if let Some(roads) = &e.setup.rules.source_roads {
            let index = route.ok_or(Error::Invalid("police owner road context"))?;
            let profile = &roads.profiles[usize::from(index)];
            let Some(start) = profile.selected.get(&at.path) else {
                // A placement on the other fork arm is not a spawn on this
                // owner's physical route. Keep its event uninstantiated.
                self.population.as_mut().unwrap().remove(agent.id);
                return Ok(());
            };
            c.ground = roads.ground(index)?;
            i64::from(*start) * 256 + at.progress - roads.origin(&e.setup.topology).unwrap()
        } else {
            e.setup
                .topology
                .projected(at)
                .ok_or(Error::Invalid("police spawn projection"))?
        };
        let sample = (i64::from(c.start_sample) + (distance >> 8))
            .clamp(0, c.ground.len() as i64 - 1) as usize;
        at.lateral = if let (Some(roads), Some(route)) = (&e.setup.rules.source_roads, route) {
            roads.profiles[usize::from(route)]
                .identity(at.path, (at.progress >> 8) as u32)
                .ok_or(Error::Invalid("police physical spawn identity"))?[2]
                * 256
        } else {
            (c.ground[sample].left + c.ground[sample].right) * 128
        };
        at.height = 0;
        let inherited_half = e
            .scenery
            .as_ref()
            .and_then(|s| {
                s.object_roads
                    .get(&if owner.recovery.body.is_some() { 3 } else { 2 })
            })
            .copied();
        let place = e
            .units
            .iter()
            .filter(|(id, u)| {
                **id != ActorId(1) && !u.police && u.previous.progress > owner.previous.progress
            })
            .count();
        let place = if self.tick.0 == 0 {
            0
        } else {
            (place + 1).min(15) as u8
        };
        let personality = owner.police_relations.as_ref().map_or(0, |r| r.index);
        let (pursuit, player_relationship) = config.pursuit(level, place, personality)?;
        let lane = (self.gameplay_rng.next() % 3) as u8;
        at.lateral = super::police_ai::spawn_lateral(e, &c, route, at, lane, &config)?;
        let id = self.spawn()?;
        let mut rider = riding::Rider::new(&c)?;
        rider.animation.queued = Some(2);
        rider
            .animation
            .advance(&c.animation, 0, self.tick.0 as u32)
            .map_err(|_| Error::Invalid("police initial animation"))?;
        rider.bike.distance_q8 = distance;
        rider.bike.lateral = at.lateral;
        rider.bike.height = at.height;
        rider.bike.speed = 0;
        rider.bike.previous_ground = at.height;
        rider.bike.previous_rise = c.ground[(i64::from(c.start_sample) + (distance >> 8))
            .clamp(0, c.ground.len() as i64 - 1)
            as usize]
            .rise;
        rider.weapon = config.weapon(level);
        self.actors.get_mut(&id).unwrap().view.rider = Some(rider);
        let e = self.encounter.as_mut().unwrap();
        let unit = e.units.get_mut(&id).unwrap();
        unit.police = true;
        unit.police_ai = Some(super::police_ai::AiState::from_words(
            &pursuit,
            lane,
            level,
            config.weapon(level),
        )?);
        unit.source_route = route;
        unit.color = 0;
        unit.fighter.stats = config.stamina();
        unit.recovery = recovery::State::mounted(id, at);
        unit.recovery.police = true;
        unit.previous_location = at;
        unit.previous = combat::Position {
            route: 1,
            branch: e.setup.topology.paths[&at.path].branch,
            path: at.path,
            progress: distance,
            lateral: at.lateral,
            height: at.height,
        };
        let cop_index = unit.police_ai.as_ref().unwrap().index();
        if let Some(relations) = e
            .units
            .get_mut(&ActorId(1))
            .and_then(|u| u.police_relations.as_mut())
        {
            let at = usize::from(cop_index) * 3 + 2;
            relations.current[at] = player_relationship;
            relations.target[at] = player_relationship;
        }
        e.configs.insert(id, c);
        if let Some(half) = inherited_half
            && let Some(scenery) = &mut e.scenery
        {
            scenery.object_roads.insert(id.0 * 2, half);
        }
        e.inventory
            .items
            .retain(|key, item| key.0 < 0x10000 || item.owner.is_some());
        let weapon = config.weapon(level);
        if weapon != animation::Weapon::Fists {
            // Stable item identity is separate from the transient rider id.
            let weapon_id = combat::WeaponId(0x10000 + agent.id);
            e.inventory.items.insert(
                weapon_id,
                combat::Item {
                    kind: weapon,
                    owner: Some(id),
                },
            );
        }
        let p = self.population.as_mut().unwrap();
        let mirror = p.agents.get_mut(&agent.id).unwrap();
        mirror.rider = Some(id);
        mirror.location = at;
        mirror.previous = at;
        mirror.speed = 0;
        p.police = Some(Live {
            actor: id,
            agent: agent.id,
            lane,
            pursuit,
            player_relationship,
            removal_pending: false,
        });
        p.police_initialize(e, &mut self.actors, self.tick.0)?;
        Ok(())
    }
}

impl World {
    pub(crate) fn police_after_progress(
        &mut self,
        e: &Encounters,
        actors: &BTreeMap<ActorId, Actor>,
        tick: u64,
    ) {
        let Some(live) = &mut self.police else {
            return;
        };
        let Some(unit) = e.units.get(&live.actor) else {
            return;
        };
        let Some(actor) = actors.get(&live.actor) else {
            return;
        };
        let rider = actor.view.rider.as_ref().unwrap();
        let Some(agent) = self.agents.get_mut(&live.agent) else {
            live.removal_pending = true;
            return;
        };
        agent.previous = agent.location;
        agent.location = unit
            .recovery
            .body
            .as_ref()
            .map_or(unit.recovery.bike_location, |b| b.location);
        agent.speed = rider.bike.speed;
        agent.projected = e.setup.topology.projected(agent.location);
        agent.hit_count = unit.fighter.hit_count;
        agent.animation = unit
            .recovery
            .body
            .as_ref()
            .map_or_else(|| rider.animation.clone(), |b| b.animation.clone());
        agent.frame = unit.recovery.body.as_ref().map_or(rider.frame, |b| b.frame);
        if unit.recovery.phase != Phase::Mounted {
            agent.state = AgentState::Recovering;
        }
        let player = &e.units[&ActorId(1)];
        let target = player
            .recovery
            .body
            .as_ref()
            .map_or(player.recovery.bike_location, |b| b.location);
        // The host graph bridge has a bounded physical road window. This
        // replaces the native ring-residency flag, not the source AI law.
        let nearby = super::police_ai::within_road_window(
            &e.setup.topology,
            agent.location,
            target,
            self.setup.disengage,
        );
        if unit.recovery.outcome.is_some() || !nearby {
            live.removal_pending = true;
        }
        if (live.removal_pending
            || unit.recovery.phase != Phase::Mounted
            || player.recovery.outcome.is_some())
            && agent.siren
        {
            agent.siren = false;
            self.cues.push(Cue::SirenStop(agent.id));
        }
        if live.removal_pending {
            agent.state_since = tick;
        }
    }
}

impl World {
    pub(crate) fn police_progression(
        &mut self,
        e: &mut Encounters,
        actors: &BTreeMap<ActorId, Actor>,
    ) {
        let Some(live) = &self.police else {
            return;
        };
        if live.removal_pending {
            return;
        }
        let Some(cop) = e.units.get(&live.actor) else {
            return;
        };
        let Some(player) = e.units.get(&ActorId(1)) else {
            return;
        };
        if cop.recovery.phase != Phase::Mounted
            || cop.recovery.outcome.is_some()
            || player.recovery.outcome.is_some()
        {
            return;
        }
        let target = player
            .recovery
            .body
            .as_ref()
            .map_or(player.recovery.bike_location, |b| b.location);
        let Some(delta) = crate::source_roads::Roads::physical_delta(
            &e.setup.topology,
            target,
            cop.recovery.bike_location,
        ) else {
            return;
        };
        let speed = actors[&ActorId(1)].view.rider.as_ref().unwrap().bike.speed;
        let cop_speed = actors[&live.actor].view.rider.as_ref().unwrap().bike.speed;
        let crash = match player.recovery.phase {
            Phase::Mounted => 0,
            Phase::Sliding => 1,
            Phase::Mounting => 3,
            _ => 2,
        };
        if arrest(
            crash,
            speed,
            cop_speed,
            0,
            delta.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            true,
            false,
        ) {
            e.units.get_mut(&ActorId(1)).unwrap().terminal_request = Some(Outcome::Arrested);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> Simulation {
        let mut c = fixtures::riding();
        c.motion_version = 3;
        let mut e = encounter::Setup::authored(&c).unwrap();
        e.rules.combat_version = 4;
        e.rules.bike_contacts = false;
        e.seeds[0].distance = 12800;
        e.seeds[0].speed = 4000;
        let mut bike = c.tables.object;
        bike[1] = 4;
        let mut body = bike;
        body[1] = 8;
        let mut stats = [0; 16];
        stats[3] = 171;
        stats[6..11].fill(1);
        stats[12] = 6;
        stats[13] = 2200;
        let p = Setup {
            traffic: false,
            periodic_police: false,
            police: Some(Config {
                bike_template: bike,
                rider_template: body,
                atan: vec![0; 257],
                personality: vec![0; 129],
                personalities: vec![vec![0; 129]; 10],
                overtake_eagerness: [0; 5],
                stats,
                bikes: [0; 5],
                throttle_percent: [116; 5],
                messages: [0; 8],
            }),
            ..Default::default()
        };
        Simulation::with_population(1, c, e, p).unwrap()
    }
    fn spawn(s: &mut Simulation) -> (u64, ActorId) {
        let agent = s
            .spawn_road_user(&Spawn {
                id: 1,
                location: Location {
                    progress: 15000,
                    lateral: 16000,
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
        s.police_spawn_sync().unwrap();
        let actor = s.population().unwrap().police.as_ref().unwrap().actor;
        (agent, actor)
    }
    #[test]
    fn police_is_a_shared_rider_with_stamina_instead_of_a_one_hit_prop() {
        let mut s = setup();
        let (agent, id) = spawn(&mut s);
        let world = s.encounter.as_mut().unwrap();
        let config = s.handling.as_ref().unwrap();
        assert!(world.units[&id].police);
        assert!(world.units[&id].police_ai.is_some());
        assert!(world.units[&id].recovery.police);
        assert_eq!(
            s.population.as_ref().unwrap().agents[&agent].rider,
            Some(id)
        );
        assert_eq!(world.units[&id].fighter.stats.health, 2200);
        assert!(
            !world
                .hit(
                    &mut s.actors,
                    config,
                    id,
                    Impact {
                        reason: 4,
                        stamina_damage: 2,
                        ..Default::default()
                    },
                    0,
                    false,
                    &mut vec![]
                )
                .unwrap()
        );
        assert_eq!(world.units[&id].recovery.phase, Phase::Mounted);
        assert_eq!(world.units[&id].fighter.stats.health, 2072);
        assert!(
            world
                .hit(
                    &mut s.actors,
                    config,
                    id,
                    Impact {
                        reason: 4,
                        stamina_damage: 100,
                        ..Default::default()
                    },
                    1,
                    false,
                    &mut vec![]
                )
                .unwrap()
        );
        assert_eq!(world.units[&id].recovery.phase, Phase::Sliding);
        assert!(world.units[&id].recovery.body.is_some());
        assert_eq!(
            s.population.as_ref().unwrap().agents[&agent].state,
            AgentState::Patrol
        );
    }
    #[test]
    fn crashed_police_cannot_arrest_and_a_recovered_cop_uses_the_native_speed_gate() {
        let mut s = setup();
        let (_, id) = spawn(&mut s);
        let world = s.encounter.as_mut().unwrap();
        let player = world.units[&ActorId(1)].recovery.bike_location;
        world.units.get_mut(&id).unwrap().recovery.bike_location = player;
        s.actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .speed = 299;
        s.actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .speed = 0;
        world.units.get_mut(&id).unwrap().recovery.phase = Phase::Sliding;
        s.population
            .as_mut()
            .unwrap()
            .police_progression(world, &s.actors);
        assert_eq!(world.units[&ActorId(1)].terminal_request, None);
        world.units.get_mut(&id).unwrap().recovery.phase = Phase::Mounted;
        s.population
            .as_mut()
            .unwrap()
            .police_progression(world, &s.actors);
        assert_eq!(
            world.units[&ActorId(1)].terminal_request,
            Some(Outcome::Arrested)
        );
        world.units.get_mut(&ActorId(1)).unwrap().terminal_request = None;
        s.actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .speed = 300;
        s.population
            .as_mut()
            .unwrap()
            .police_progression(world, &s.actors);
        assert_eq!(world.units[&ActorId(1)].terminal_request, None);
    }
    #[test]
    fn police_spawn_inherits_the_owners_retained_road_half_and_cleanup_is_immediate() {
        let mut s = setup();
        s.encounter.as_mut().unwrap().scenery = Some(crate::scenery::State::default());
        s.encounter
            .as_mut()
            .unwrap()
            .scenery
            .as_mut()
            .unwrap()
            .object_roads
            .insert(2, 1);
        let (agent, id) = spawn(&mut s);
        assert_eq!(
            s.encounter
                .as_ref()
                .unwrap()
                .scenery
                .as_ref()
                .unwrap()
                .object_roads[&(id.0 * 2)],
            1
        );
        s.queue(QueuedCommand {
            tick: Tick(100),
            actor: id,
            sequence: 0,
            input: Command::default(),
        })
        .unwrap();
        s.despawn(id).unwrap();
        assert!(s.population.as_ref().unwrap().police.is_none());
        assert!(!s.population.as_ref().unwrap().agents.contains_key(&agent));
        assert!(!s.encounter.as_ref().unwrap().configs.contains_key(&id));
        assert!(
            !s.encounter
                .as_ref()
                .unwrap()
                .scenery
                .as_ref()
                .unwrap()
                .object_roads
                .contains_key(&(id.0 * 2))
        );
        assert!(s.queue.is_empty());
    }
    #[test]
    fn repeated_cop_lifecycles_keep_stolen_equipment_unique_and_drop_storage_bounded() {
        let mut s = setup();
        let (_, id) = spawn(&mut s);
        let item = s
            .encounter
            .as_ref()
            .unwrap()
            .inventory
            .carried(id)
            .unwrap()
            .0;
        assert!(
            s.encounter
                .as_mut()
                .unwrap()
                .inventory
                .transfer(item, id, ActorId(1))
        );
        s.despawn(id).unwrap();
        for _ in 0..80 {
            let (_, cop) = spawn(&mut s);
            assert_eq!(s.encounter.as_ref().unwrap().inventory.items.len(), 2);
            s.encounter.as_ref().unwrap().inventory.validate().unwrap();
            s.despawn(cop).unwrap();
            assert_eq!(s.encounter.as_ref().unwrap().inventory.items.len(), 1);
        }
        assert_eq!(
            s.encounter.as_ref().unwrap().inventory.items[&item].owner,
            Some(ActorId(1))
        );
        assert_eq!(s.snapshot().actors.len(), 1);
    }
    #[test]
    fn malformed_police_personality_is_rejected_without_panicking() {
        let sim = setup();
        let mut recording = sim.recording_setup(1).unwrap();
        let mut seed = encounter::Seed::player();
        seed.id = ActorId(2);
        recording.encounter.as_mut().unwrap().seeds.push(seed);
        recording.population.as_mut().unwrap().brains.insert(
            ActorId(2),
            Traits {
                personality: Some(10),
                ..Default::default()
            },
        );
        let bytes = serde_json::to_vec(&recording).unwrap();
        let decoded = replay::Replay::read(&bytes).unwrap();
        let result = std::panic::catch_unwind(|| replay::Playback::new(decoded));
        assert!(
            result.is_ok(),
            "Malformed replay caused an unchecked personality lookup"
        );
        assert!(result.unwrap().is_err());
    }
    #[test]
    fn absent_police_fields_preserve_historical_setup_bytes() {
        let mut s = setup();
        let p = s.population.as_mut().unwrap();
        p.setup.police = None;
        p.police = None;
        let bytes = serde_json::to_string(&p.setup).unwrap();
        assert!(
            serde_json::from_str::<serde_json::Value>(&bytes)
                .unwrap()
                .get("police")
                .is_none()
        );
        assert_eq!(serde_json::from_str::<Setup>(&bytes).unwrap(), p.setup);
        let old = fixtures::spec();
        let bytes = serde_json::to_string(&old).unwrap();
        assert!(!bytes.contains("rider_value"));
        assert!(!bytes.contains("throttle_percent"));
        assert!(!bytes.contains("\"ai\""));
    }
}
