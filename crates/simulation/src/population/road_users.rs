//! Bounded road-user spawning, movement, contacts and pursuit lifecycle.
use super::*;
impl World {
    pub(crate) fn spawn_tick(
        &mut self,
        e: &Encounters,
        actors: &BTreeMap<ActorId, Actor>,
        tick: u64,
        mut random: impl FnMut() -> u32,
    ) -> Result<()> {
        self.trace.clear();
        self.cues.clear();
        let Some(player) = e.units.get(&ActorId(1)) else {
            return Ok(());
        };
        if player.recovery.outcome.is_some() {
            return Ok(());
        }
        let at = player
            .recovery
            .body
            .as_ref()
            .map_or(player.recovery.bike_location, |b| b.location);
        let topology = &e.setup.topology;
        let progress = topology.projected(at).unwrap_or(self.previous_progress);
        let delta = (progress - self.previous_progress).clamp(-10000, 10000) as i32;
        self.previous_progress = progress;
        let zones = self.setup.zones.clone();
        let mut active = BTreeSet::new();
        for zone in zones
            .iter()
            .filter(|z| z.path == at.path && (z.start..z.end).contains(&at.progress))
        {
            active.insert(zone.id);
            if !self.active_zones.contains(&zone.id) {
                self.timers.density(zone.density);
                if (1..=3).contains(&zone.variant) && self.controllers.len() < 10 {
                    self.controllers.insert(
                        zone.id,
                        Controller {
                            zone: zone.id,
                            since: tick,
                            next_tick: tick,
                            phase: 0,
                            next_spawn: tick,
                        },
                    );
                }
            }
        }
        self.controllers.retain(|id, _| active.contains(id));
        self.active_zones = active;
        let requests: Vec<_> = self
            .setup
            .spawns
            .iter()
            .filter(|s| {
                s.location.path == at.path
                    && (s.location.progress - at.progress).abs() < 8192
                    && !self.consumed.contains(&s.id)
            })
            .cloned()
            .collect();
        for s in requests {
            self.consumed.insert(s.id);
            self.spawn(&s, tick, topology)?;
        }
        if self.setup.periodic_police
            && tick > 0
            && tick & 0xfff == 0
            && !self.agents.values().any(|a| a.kind == Kind::Police)
        {
            let mut location = at;
            topology.advance(&mut location, 960, None)?;
            location.lateral = at.lateral;
            self.spawn(
                &Spawn {
                    id: 1,
                    location,
                    kind: Kind::Police,
                    family: None,
                    selector: 0,
                    subtype: 0,
                    speed: 6000,
                    direction: -1,
                },
                tick,
                topology,
            )?;
        }
        if self.setup.traffic {
            for (mode, due) in self.timers.advance(delta).into_iter().enumerate() {
                if !due {
                    continue;
                }
                let lane = self.draw(tick, 0, "traffic_lane", &mut random) % 2;
                let choice = self.draw(tick, 0, "vehicle_identity", &mut random);
                let vehicle = if self.setup.vehicle_masks.is_empty() {
                    choice % u32::from(self.setup.vehicle_types)
                } else {
                    let Some(index) = vehicle_choice(
                        &self.setup.vehicle_masks,
                        if mode == 0 { 2 } else { 0 },
                        choice,
                    ) else {
                        continue;
                    };
                    u32::from(index)
                };
                let mut location = at;
                topology.advance(&mut location, if mode == 1 { -2048 } else { 8192 }, None)?;
                location.lateral = if lane == 0 { -64000 } else { 64000 };
                self.spawn(
                    &Spawn {
                        id: 1,
                        location,
                        kind: Kind::Vehicle,
                        family: None,
                        selector: vehicle as u8,
                        subtype: 0,
                        speed: self.timers.cruise * 256,
                        direction: if mode == 0 { -1 } else { 1 },
                    },
                    tick,
                    topology,
                )?;
            }
        }
        let controllers: Vec<_> = self.controllers.keys().copied().collect();
        for id in controllers {
            let c = self.controllers.get_mut(&id).unwrap();
            if tick < c.next_tick {
                continue;
            }
            c.next_tick = tick + 30;
            let age = (tick - c.since) % 1560;
            c.phase = if age < 600 {
                0
            } else if age < 780 {
                1
            } else {
                2
            };
            if c.phase != 2 || tick < c.next_spawn {
                continue;
            }
            c.next_spawn = tick + 120;
            let zone = zones.iter().find(|z| z.id == id).unwrap();
            let side = self.draw(tick, 0, "crossing_side", &mut random) & 1;
            let choice = self.draw(tick, 0, "crossing_vehicle_identity", &mut random);
            let vehicle = if self.setup.vehicle_masks.is_empty() {
                (choice % u32::from(self.setup.vehicle_types)) as u8
            } else {
                let Some(id) = vehicle_choice(&self.setup.vehicle_masks, 1, choice) else {
                    continue;
                };
                id
            };
            let location = Location {
                path: zone.path,
                progress: (zone.start + zone.end) / 2,
                lateral: if side == 0 { -256000 } else { 256000 },
                ..Default::default()
            };
            self.spawn(
                &Spawn {
                    id: 1,
                    location,
                    kind: Kind::Vehicle,
                    family: None,
                    selector: vehicle,
                    subtype: 1,
                    speed: 8000,
                    direction: if side == 0 { 1 } else { -1 },
                },
                tick,
                topology,
            )?;
        }
        let _ = actors;
        Ok(())
    }
    pub(crate) fn movement(&mut self, e: &Encounters, tick: u64) -> Result<()> {
        let Some(player) = e.units.get(&ActorId(1)) else {
            return Ok(());
        };
        let target = player
            .recovery
            .body
            .as_ref()
            .map_or(player.recovery.bike_location, |b| b.location);
        let t = &e.setup.topology;
        let mut remove = vec![];
        for a in self.agents.values_mut() {
            a.previous = a.location;
            if a.state == AgentState::Hit {
                if tick - a.state_since >= 180 {
                    a.state = AgentState::Recovering;
                    a.state_since = tick;
                } else {
                    continue;
                }
            }
            if a.state == AgentState::Recovering {
                if tick - a.state_since >= 90 {
                    a.state = if a.kind == Kind::Police {
                        AgentState::Disengaged
                    } else {
                        AgentState::Waiting
                    };
                    a.state_since = tick;
                } else {
                    continue;
                }
            }
            match a.kind {
                Kind::Vehicle => {
                    if a.state == AgentState::Crossing {
                        a.location.lateral = a
                            .location
                            .lateral
                            .saturating_add(a.speed * i32::from(a.direction) / 4);
                        if a.location.lateral.abs() > 270000 {
                            remove.push(a.id);
                        }
                    } else {
                        t.advance(
                            &mut a.location,
                            i64::from(a.speed / 256) * i64::from(a.direction),
                            None,
                        )?;
                    }
                }
                Kind::Pedestrian => {
                    if a.state == AgentState::Waiting
                        && a.location.path == target.path
                        && (a.location.progress - target.progress).abs() < 2048
                        && (4..=7).contains(&a.subtype)
                    {
                        a.state = AgentState::Crossing;
                        a.state_since = tick;
                    }
                    if a.state == AgentState::Crossing {
                        a.location.lateral = a
                            .location
                            .lateral
                            .saturating_add(a.speed * i32::from(a.direction));
                        if a.location.lateral.abs() > 270000 {
                            remove.push(a.id);
                        }
                    }
                }
                Kind::Police => {
                    let same = a.location.path == target.path;
                    let leg = if !same && a.state == AgentState::Chase {
                        t.leg(a.location.path, target.path)
                    } else {
                        None
                    };
                    let toward = leg.map_or(target.progress - a.location.progress, |edge| {
                        if edge.end {
                            t.paths[&a.location.path].length - a.location.progress + 16
                        } else {
                            -a.location.progress - 16
                        }
                    });
                    let gap = toward.abs();
                    let chase = player.recovery.outcome.is_none()
                        && (same || leg.is_some())
                        && gap
                            < if a.state == AgentState::Chase {
                                self.setup.disengage
                            } else {
                                self.setup.detection
                            };
                    if chase && a.state != AgentState::Chase {
                        a.state = AgentState::Chase;
                        a.state_since = tick;
                        a.siren = true;
                        self.cues.push(Cue::SirenStart(a.id));
                        self.cues.push(Cue::Warning(a.id));
                    }
                    if !chase && a.state == AgentState::Chase {
                        a.state = AgentState::Disengaged;
                        a.siren = false;
                        self.cues.push(Cue::SirenStop(a.id));
                    }
                    if chase {
                        let difference = toward;
                        a.direction = if difference < 0 { -1 } else { 1 };
                        a.speed = if same && gap < 32 {
                            0
                        } else {
                            (gap as i32 * 128).clamp(1000, 18000)
                        };
                        t.advance(
                            &mut a.location,
                            i64::from(a.speed / 256) * i64::from(a.direction),
                            leg,
                        )?;
                        a.location.lateral += ((target.lateral
                            + if a.id.is_multiple_of(2) {
                                24000
                            } else {
                                -24000
                            })
                            - a.location.lateral)
                            .clamp(-2000, 2000);
                    } else {
                        t.advance(&mut a.location, i64::from(a.speed / 256), None)?;
                    }
                }
            }
            a.projected = t.projected(a.location);
            if a.location.path == target.path
                && (a.location.progress - target.progress).abs() > 32768
            {
                remove.push(a.id);
            }
            if a.location.progress == 0 || a.location.progress == t.paths[&a.location.path].length {
                remove.push(a.id);
            }
        }
        for id in remove {
            self.remove(id);
        }
        Ok(())
    }
    pub(crate) fn contacts(
        &mut self,
        e: &mut Encounters,
        actors: &mut BTreeMap<ActorId, Actor>,
        c: &riding::Configuration,
        tick: u64,
        effects: &mut Vec<combat::Effect>,
    ) -> Result<()> {
        let mut active = BTreeSet::new();
        let ids: Vec<_> = self.agents.keys().copied().collect();
        for id in ids {
            let a = self.agents[&id].clone();
            if matches!(a.state, AgentState::Hit | AgentState::Recovering)
                || (a.kind == Kind::Pedestrian && a.hit_count > 0)
            {
                continue;
            }
            for (rider_id, unit) in e.units.clone() {
                if unit.recovery.outcome.is_some() {
                    continue;
                }
                let at = unit
                    .recovery
                    .body
                    .as_ref()
                    .map_or(unit.recovery.bike_location, |b| b.location);
                let long_half = if a.kind == Kind::Vehicle { 128 } else { 48 };
                let width = if a.kind == Kind::Vehicle { 16000 } else { 8000 };
                let swept = at.progress + long_half >= a.previous.progress.min(a.location.progress)
                    && at.progress - long_half <= a.previous.progress.max(a.location.progress);
                let side = at.lateral + width >= a.previous.lateral.min(a.location.lateral)
                    && at.lateral - width <= a.previous.lateral.max(a.location.lateral);
                if at.path != a.location.path || !swept || !side || at.height.abs() > 8960 {
                    continue;
                }
                active.insert((rider_id.0, id));
                if self.contacts.contains(&(rider_id.0, id)) {
                    continue;
                }
                if a.kind == Kind::Police {
                    continue;
                }
                let speed = actors[&rider_id].view.rider.as_ref().unwrap().bike.speed;
                if a.kind == Kind::Pedestrian {
                    let pedestrian = self.agents.get_mut(&id).unwrap();
                    pedestrian.state = AgentState::Hit;
                    pedestrian.state_since = tick;
                    pedestrian.hit_count += 1;
                    pedestrian.speed = 0;
                    self.pedestrian_hits += 1;
                    self.cues.push(Cue::PedestrianHit(id));
                    e.hit(
                        actors,
                        c,
                        rider_id,
                        Impact {
                            reason: 3,
                            bike_damage: 1,
                            ..Default::default()
                        },
                        tick,
                        false,
                        effects,
                    )?;
                    break;
                } else {
                    let relative = if a.state == AgentState::Crossing {
                        speed + a.speed
                    } else {
                        (speed - a.speed * i32::from(a.direction)).abs()
                    };
                    e.hit(
                        actors,
                        c,
                        rider_id,
                        Impact {
                            reason: 1,
                            bike_damage: (relative / 1024).clamp(1, 100) as u16,
                            rider_velocity: Some([0, 0, 512]),
                            bike_velocity: Some([0, 0, 256]),
                            ..Default::default()
                        },
                        tick,
                        relative > 6000,
                        effects,
                    )?;
                    if a.state != AgentState::Crossing {
                        actors
                            .get_mut(&rider_id)
                            .unwrap()
                            .view
                            .rider
                            .as_mut()
                            .unwrap()
                            .bike
                            .speed = (a.speed / 2).min(speed);
                    }
                }
            }
        }
        // A vehicle interrupts a crossing pedestrian once, even without a racer nearby.
        let cars: Vec<_> = self
            .agents
            .values()
            .filter(|a| a.kind == Kind::Vehicle)
            .cloned()
            .collect();
        for p in self.agents.values_mut().filter(|a| {
            (a.kind == Kind::Pedestrian && a.state == AgentState::Crossing && a.hit_count == 0)
                || (a.kind == Kind::Police
                    && !matches!(a.state, AgentState::Hit | AgentState::Recovering))
        }) {
            if cars.iter().any(|car| {
                car.location.path == p.location.path
                    && (car.location.progress - p.location.progress).abs() < 128
                    && (car.location.lateral - p.location.lateral).abs() < 20000
            }) {
                p.state = AgentState::Hit;
                p.state_since = tick;
                p.hit_count += 1;
                if p.kind == Kind::Pedestrian {
                    self.pedestrian_hits += 1;
                }
                if p.siren {
                    p.siren = false;
                    self.cues.push(Cue::SirenStop(p.id));
                }
            }
        }
        self.contacts = active;
        Ok(())
    }
    pub(crate) fn animate(&mut self, c: &riding::Configuration, tick: u32) -> Result<()> {
        for a in self.agents.values_mut().filter(|a| a.kind == Kind::Police) {
            let wanted = if a.state == AgentState::Hit { 33 } else { 2 };
            let code = c.animation.resolve(1, wanted).map_or(2, |i| i as i8);
            if a.animation.code != code {
                a.animation.set(&c.animation, 1, code);
                a.animation.queued = Some(code);
            }
            if let Some(frame) = a
                .animation
                .advance(&c.animation, 1, tick)
                .map_err(|_| Error::Invalid("cop animation"))?
            {
                a.frame = c
                    .animation
                    .select(1, &frame, 0, 0)
                    .map_err(|_| Error::Invalid("cop view"))?
                    .0
                    .frame;
            }
        }
        Ok(())
    }
    pub(crate) fn after_progress(&mut self, e: &Encounters) {
        if e.units
            .get(&ActorId(1))
            .is_none_or(|u| u.recovery.outcome.is_some())
        {
            for a in self.agents.values_mut().filter(|a| a.siren) {
                a.siren = false;
                a.state = AgentState::Disengaged;
                self.cues.push(Cue::SirenStop(a.id));
            }
        }
    }
    pub(crate) fn progression(&mut self, e: &mut Encounters, actors: &BTreeMap<ActorId, Actor>) {
        let Some(player) = e.units.get_mut(&ActorId(1)) else {
            return;
        };
        let target = player
            .recovery
            .body
            .as_ref()
            .map_or(player.recovery.bike_location, |b| b.location);
        let speed = player.recovery.body.as_ref().map_or(
            actors[&ActorId(1)].view.rider.as_ref().unwrap().bike.speed,
            |b| b.motion.speed,
        );
        let crash = if player.recovery.phase == Phase::Mounted {
            0
        } else {
            2
        };
        if self.agents.values().any(|cop| {
            cop.kind == Kind::Police
                && cop.state == AgentState::Chase
                && cop.location.path == target.path
                && (cop.location.lateral - target.lateral).abs() < 32000
                && arrest(
                    crash,
                    speed,
                    cop.speed,
                    target.progress as i32,
                    cop.location.progress as i32,
                    true,
                    false,
                )
        }) {
            player.terminal_request = Some(Outcome::Arrested);
        }
    }
}
