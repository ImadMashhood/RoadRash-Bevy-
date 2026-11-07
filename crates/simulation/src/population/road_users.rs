//! Bounded road-user spawning, movement, contacts and pursuit lifecycle.
use super::*;
use crate::combat::{Effect, EffectKind};
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
    pub(crate) fn movement(
        &mut self,
        e: &Encounters,
        config: &riding::Configuration,
        tick: u64,
    ) -> Result<()> {
        let Some(player) = e.units.get(&ActorId(1)) else {
            return Ok(());
        };
        let target = player
            .recovery
            .body
            .as_ref()
            .map_or(player.recovery.bike_location, |b| b.location);
        let t = &e.setup.topology;
        let occupied: Vec<_> = self
            .agents
            .values()
            .map(|a| (a.id, a.kind, a.state, a.location, a.speed, a.direction))
            .collect();
        let riders: Vec<_> = e
            .units
            .values()
            .filter(|u| u.recovery.outcome.is_none())
            .flat_map(|u| {
                std::iter::once(u.recovery.bike_location)
                    .chain(u.recovery.body.iter().map(|b| b.location))
            })
            .collect();
        let mut remove = vec![];
        let mut reservations: Vec<(u32, i64, i32)> = vec![];
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
                    if self.setup.functional_traffic && a.subtype != 1 {
                        let nominal = *a.cruise_speed.get_or_insert(a.speed);
                        let projected = t.projected(a.location).unwrap_or(0);
                        let index = (i64::from(config.start_sample) + (projected >> 8))
                            .clamp(0, config.ground.len() as i64 - 1)
                            as usize;
                        let ground = config.ground[index];
                        let center = (ground.left + ground.right) * 128;
                        let blocked = |lane: i32| {
                            occupied.iter().any(|(id, _, _, location, _, _)| {
                                *id != a.id
                                    && location.path == a.location.path
                                    && (location.progress - a.location.progress).abs() < 512
                                    && (location.lateral - lane).abs() < 40000
                            }) || reservations.iter().any(|(path, progress, reserved)| {
                                *path == a.location.path
                                    && (*progress - a.location.progress).abs() < 512
                                    && (*reserved - lane).abs() < 40000
                            }) || riders.iter().any(|location| {
                                location.path == a.location.path
                                    && (location.progress - a.location.progress).abs() < 512
                                    && (location.lateral - lane).abs() < 32000
                            })
                        };
                        let target = [center - 64000, center, center + 64000]
                            .into_iter()
                            .filter(|lane| {
                                *lane > (ground.left + 100) * 256
                                    && *lane < (ground.right - 100) * 256
                                    && !blocked(*lane)
                            })
                            .min_by_key(|lane| (*lane - a.location.lateral).abs());
                        let signal = self.setup.zones.iter().any(|zone| {
                            if zone.path != a.location.path {
                                return false;
                            }
                            let stop = if a.direction > 0 {
                                zone.start + 256
                            } else {
                                zone.end - 256
                            };
                            let ahead = (stop - a.location.progress) * i64::from(a.direction);
                            (0..=256).contains(&ahead)
                                && self.controllers.get(&zone.id).is_some_and(|c| c.phase != 2)
                        });
                        let crossing = occupied.iter().any(|(_, kind, state, location, _, _)| {
                            *kind == Kind::Pedestrian
                                && *state == AgentState::Crossing
                                && location.path == a.location.path
                                && (location.progress - a.location.progress).abs() < 768
                                && (location.lateral - a.location.lateral).abs() < 48000
                        });
                        let stop = signal || crossing || blocked(a.location.lateral);
                        a.speed = if stop { 0 } else { nominal };
                        a.state = if stop {
                            AgentState::Waiting
                        } else {
                            AgentState::Cruise
                        };
                        if !signal
                            && !crossing
                            && let Some(lane) = target
                        {
                            a.location.lateral += (lane - a.location.lateral).clamp(-1024, 1024);
                        }
                        reservations.push((
                            a.location.path,
                            a.location.progress,
                            target.unwrap_or(a.location.lateral),
                        ));
                    }
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
                        && (!self.setup.functional_traffic
                            || !occupied
                                .iter()
                                .any(|(_, kind, _, location, speed, direction)| {
                                    *kind == Kind::Vehicle
                                        && *speed > 0
                                        && location.path == a.location.path
                                        && (location.progress - a.location.progress).abs() < 2048
                                        && ((a.location.progress - location.progress)
                                            * i64::from(*direction))
                                            >= 0
                                }))
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
    pub(crate) fn combat_positions(&self, topology: &Topology) -> Vec<combat::Position> {
        self.agents
            .values()
            .filter(|a| !matches!(a.state, AgentState::Hit | AgentState::Recovering))
            .filter_map(|a| {
                Some(combat::Position {
                    route: 1,
                    branch: topology.paths.get(&a.location.path)?.branch,
                    path: a.location.path,
                    progress: topology.projected(a.location)?,
                    lateral: a.location.lateral,
                    height: a.location.height,
                })
            })
            .collect()
    }
    // Police remain population agents. This adapter shares mounted combat's
    // active window and strict reach instead of treating a button press as a hit.
    // Their complete rider physics/stamina model remains a separate parity gate.
    pub(crate) fn combat(
        &mut self,
        e: &Encounters,
        actors: &BTreeMap<ActorId, Actor>,
        tick: u64,
        effects: &mut Vec<combat::Effect>,
    ) {
        if !self.setup.functional_traffic || e.setup.rules.combat_version < 2 {
            return;
        }
        for (id, unit) in &e.units {
            if unit.recovery.phase != Phase::Mounted
                || unit.recovery.outcome.is_some()
                || tick < unit.fighter.stun_until
            {
                continue;
            }
            let Some(swing) = &unit.fighter.swing else {
                continue;
            };
            if !e.swing_active(*id, tick) {
                continue;
            }
            let Some(at) = e.position(*id, actors) else {
                continue;
            };
            let slot = combat::weapon_slot(swing.request.kind, swing.weapon);
            let selected = self
                .agents
                .values()
                .filter(|a| {
                    a.kind == Kind::Police
                        && !matches!(a.state, AgentState::Hit | AgentState::Recovering)
                        && e.setup
                            .topology
                            .paths
                            .get(&a.location.path)
                            .is_some_and(|p| p.branch == at.branch)
                })
                .filter_map(|a| {
                    let progress = e.setup.topology.projected(a.location)?;
                    let gap = (progress - at.progress).abs();
                    (gap < 480).then_some((gap, progress >= at.progress, a.id))
                })
                .min();
            let Some((gap, _, selected)) = selected else {
                continue;
            };
            if e.units.iter().any(|(other, u)| {
                other != id
                    && u.recovery.phase == Phase::Mounted
                    && u.recovery.outcome.is_none()
                    && e.position(*other, actors)
                        .is_some_and(|p| at.same_road(p) && (p.progress - at.progress).abs() <= gap)
            }) {
                continue;
            }
            for cop in self.agents.values_mut().filter(|a| a.id == selected) {
                let Some(path) = e.setup.topology.paths.get(&cop.location.path) else {
                    continue;
                };
                let Some(progress) = e.setup.topology.projected(cop.location) else {
                    continue;
                };
                let target = combat::Position {
                    route: 1,
                    branch: path.branch,
                    path: cop.location.path,
                    progress,
                    lateral: cop.location.lateral,
                    height: cop.location.height,
                };
                if !combat::in_reach(at, target, swing.request.side, slot, e.setup.rules.reach) {
                    continue;
                }
                cop.state = AgentState::Hit;
                cop.state_since = tick;
                cop.hit_count += 1;
                cop.siren = false;
                self.cues.push(Cue::SirenStop(cop.id));
                effects.push(Effect {
                    tick,
                    actor: *id,
                    other: None,
                    kind: EffectKind::Hit,
                });
            }
        }
    }
    pub(crate) fn contacts(
        &mut self,
        e: &mut Encounters,
        actors: &mut BTreeMap<ActorId, Actor>,
        c: &riding::Configuration,
        tick: u64,
        effects: &mut Vec<combat::Effect>,
        mut random: impl FnMut() -> u32,
    ) -> Result<()> {
        let mut active = BTreeSet::new();
        let ids: Vec<_> = self.agents.keys().copied().collect();
        for id in ids {
            let mut a = self.agents[&id].clone();
            if matches!(a.state, AgentState::Hit | AgentState::Recovering)
                || (a.kind == Kind::Pedestrian && a.hit_count > 0)
            {
                continue;
            }
            for (rider_id, unit) in e.units.clone() {
                if unit.recovery.outcome.is_some() {
                    continue;
                }
                if a.kind == Kind::Vehicle && e.setup.rules.collision_version == 2 {
                    let bike_at = unit.recovery.bike_location;
                    let bike_here = bike_at.path == a.location.path;
                    let rider_here = unit
                        .recovery
                        .body
                        .as_ref()
                        .is_some_and(|b| b.location.path == a.location.path);
                    if !bike_here && !rider_here {
                        continue;
                    }
                    let topology = &e.setup.topology;
                    let mut car = crate::collision::Body {
                        position: [
                            topology.projected(a.location).unwrap() as i32,
                            a.location.lateral,
                            a.location.height,
                        ],
                        previous: [
                            topology.projected(a.previous).unwrap() as i32,
                            a.previous.lateral,
                            a.previous.height,
                        ],
                        half: [64, 16384, 8960],
                        velocity: if a.state == AgentState::Crossing {
                            [0, a.speed * i32::from(a.direction) / 4, 0]
                        } else {
                            [a.speed * i32::from(a.direction), 0, 0]
                        },
                        speed: a.speed,
                        mass: 1800,
                        category: 16,
                        flags: 0,
                        impulse: [0; 3],
                    };
                    let start = car.position[0];
                    let bike_hit = bike_here
                        && e.source_object_contact(rider_id, actors, &mut car, c, tick, effects)?;
                    let rider_hit = rider_here
                        && e.source_detached_object_contact(
                            rider_id,
                            actors,
                            &mut car,
                            c,
                            (tick, effects),
                            &mut random,
                        )?;
                    for (detached, hit) in [(false, bike_hit), (true, rider_hit)] {
                        if !hit {
                            continue;
                        }
                        let key = (rider_id.0 * 2 + u64::from(detached), id);
                        active.insert(key);
                        if !self.contacts.contains(&key) {
                            effects.push(Effect {
                                tick,
                                actor: rider_id,
                                other: None,
                                kind: EffectKind::Contact,
                            });
                        }
                    }
                    if bike_hit || rider_hit {
                        let agent = self.agents.get_mut(&id).unwrap();
                        e.setup.topology.advance(
                            &mut agent.location,
                            i64::from(car.position[0] - start),
                            None,
                        )?;
                        agent.location.lateral = car.position[1];
                        agent.location.height = car.position[2];
                        agent.previous = agent.location;
                        agent.projected = e.setup.topology.projected(agent.location);
                        a = agent.clone();
                    }
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
                    if self.setup.functional_traffic
                        && unit.recovery.phase == Phase::Mounted
                        && (actors[&rider_id].view.rider.as_ref().unwrap().bike.speed > 1800
                            || (e.setup.rules.combat_version < 2
                                && actors[&rider_id].input.attack.is_some()))
                    {
                        let cop = self.agents.get_mut(&id).unwrap();
                        cop.state = AgentState::Hit;
                        cop.state_since = tick;
                        cop.hit_count += 1;
                        cop.siren = false;
                        self.cues.push(Cue::SirenStop(id));
                        effects.push(Effect {
                            tick,
                            actor: rider_id,
                            other: None,
                            kind: EffectKind::Contact,
                        });
                    }
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
        for agent in self
            .agents
            .values_mut()
            .filter(|a| a.kind == Kind::Pedestrian)
        {
            let Some(appearance) = self
                .setup
                .pedestrian_appearances
                .iter()
                .find(|a| Some(a.family) == agent.family && a.selector == agent.selector)
            else {
                continue;
            };
            let code = match agent.state {
                AgentState::Crossing => 0,
                AgentState::Hit => 3,
                AgentState::Recovering => 4,
                _ => 1,
            };
            let code = appearance
                .animation
                .resolve(0, code)
                .ok_or(Error::Invalid("Pedestrian state sequence"))? as i8;
            if agent.animation.code != code {
                agent.animation.set(&appearance.animation, 0, code);
                agent.animation.queued = Some(code);
            }
            if let Some(frame) = agent
                .animation
                .advance(&appearance.animation, 0, tick)
                .map_err(|_| Error::Invalid("Pedestrian playback"))?
            {
                agent.frame = appearance
                    .animation
                    .select(0, &frame, 0, 0)
                    .map_err(|_| Error::Invalid("Pedestrian view"))?
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
        let source = e.setup.rules.collision_version == 2;
        let bike_speed = actors[&ActorId(1)].view.rider.as_ref().unwrap().bike.speed;
        // Native arrest reads bike velocity even while targeting the detached
        // rider position, and distinguishes the sliding and get-up states.
        let speed = if source {
            bike_speed
        } else {
            player
                .recovery
                .body
                .as_ref()
                .map_or(bike_speed, |b| b.motion.speed)
        };
        let crash = if source {
            match player.recovery.phase {
                Phase::Mounted => 0,
                Phase::Sliding => 1,
                Phase::Mounting => 3,
                _ => 2,
            }
        } else if player.recovery.phase == Phase::Mounted {
            0
        } else {
            2
        };
        if self.agents.values().any(|cop| {
            cop.kind == Kind::Police
                && cop.state == AgentState::Chase
                && cop.location.path == target.path
                && (source || (cop.location.lateral - target.lateral).abs() < 32000)
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

#[cfg(test)]
mod combat_tests {
    use super::*;
    use crate::combat::{AttackKind, Definition, Request, Side, Swing};

    #[test]
    fn police_strikes_require_active_window_side_and_strict_reach() {
        for (active, side, lateral, height, expected) in [
            (false, Side::Right, 18000, 0, false),
            (true, Side::Left, 18000, 0, false),
            (true, Side::Right, 20479, 0, true),
            (true, Side::Right, 20480, 0, false),
            (true, Side::Right, 18000, 0x3000, false),
        ] {
            let c = fixtures::riding();
            let mut setup = encounter::Setup::authored(&c).unwrap();
            setup.rules.combat_version = 2;
            setup.rules.bike_contacts = false;
            let mut s = Simulation::with_encounters(1, c.clone(), setup).unwrap();
            let at = s.encounter.as_ref().unwrap().units[&ActorId(1)]
                .recovery
                .bike_location;
            let mut p = World::new(Setup {
                functional_traffic: true,
                ..Default::default()
            });
            p.spawn(
                &Spawn {
                    id: 1,
                    kind: Kind::Police,
                    location: Location {
                        lateral,
                        height,
                        ..at
                    },
                    family: None,
                    selector: 0,
                    subtype: 0,
                    speed: 0,
                    direction: 1,
                },
                0,
                &s.encounter.as_ref().unwrap().setup.topology,
            )
            .unwrap();
            let request = Request {
                kind: AttackKind::Kick,
                side,
            };
            let definition = Definition::from_library_version(
                &c.animation,
                request,
                animation::Weapon::Fists,
                2,
            )
            .unwrap();
            let age = if active {
                u64::from(definition.windows[0].start)
            } else {
                0
            };
            s.encounter
                .as_mut()
                .unwrap()
                .units
                .get_mut(&ActorId(1))
                .unwrap()
                .fighter
                .swing = Some(Swing {
                request,
                target: None,
                weapon: animation::Weapon::Fists,
                started: 0,
                definition,
                resolved: false,
                swish: false,
                repeat: false,
            });
            let mut effects = vec![];
            p.combat(s.encounter.as_ref().unwrap(), &s.actors, age, &mut effects);
            let cop = p.agents.values().next().unwrap();
            assert_eq!(
                cop.state == AgentState::Hit,
                expected,
                "{active} {side:?} {lateral} {height}"
            );
            assert_eq!(effects.len(), usize::from(expected));
            p.combat(s.encounter.as_ref().unwrap(), &s.actors, age, &mut effects);
            assert!(p.agents.values().next().unwrap().hit_count <= 1);
            assert!(effects.len() <= 1);
        }
    }
}

#[cfg(test)]
mod arrest_input_tests {
    use super::*;
    #[test]
    fn source_arrest_uses_the_bike_speed_and_preserves_crash_state() {
        for (speed, body_speed, phase, lateral, expected) in [
            (0, 10000, Phase::Sliding, 0, true),
            (2000, 0, Phase::Sliding, 0, false),
            (1700, 10000, Phase::Walking, 0, true),
            (1800, 0, Phase::Standing, 0, false),
            (0, 0, Phase::Standing, 48000, true),
        ] {
            let c = fixtures::riding();
            let mut setup = encounter::Setup::authored(&c).unwrap();
            setup.rules.collision_version = 2;
            setup.rules.combat_version = 2;
            setup.rules.collision = Some(crate::collision::Profile {
                mounted: [7680, 64, 8960],
                fallen: [7680, 64, 3840],
                mass: 256,
                elasticity: 192,
                rider: None,
            });
            let mut sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
            let e = sim.encounter.as_mut().unwrap();
            let rider = sim
                .actors
                .get_mut(&ActorId(1))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            let unit = e.units.get_mut(&ActorId(1)).unwrap();
            unit.recovery.begin_source(
                ActorId(1),
                &mut rider.bike,
                &c,
                unit.recovery.bike_location,
                Impact::default(),
                &e.setup.topology,
            );
            rider.bike.speed = speed;
            unit.recovery.body.as_mut().unwrap().motion.speed = body_speed;
            unit.recovery.phase = phase;
            let mut p = World::new(Setup::default());
            p.spawn(
                &Spawn {
                    id: 1,
                    kind: Kind::Police,
                    location: Location {
                        lateral,
                        ..unit.recovery.bike_location
                    },
                    family: None,
                    selector: 0,
                    subtype: 0,
                    speed: 0,
                    direction: 1,
                },
                0,
                &e.setup.topology,
            )
            .unwrap();
            p.agents.values_mut().next().unwrap().state = AgentState::Chase;
            p.progression(e, &sim.actors);
            assert_eq!(
                e.units[&ActorId(1)].terminal_request == Some(Outcome::Arrested),
                expected,
                "{speed} {body_speed} {phase:?} {lateral}"
            );
        }
    }
}
