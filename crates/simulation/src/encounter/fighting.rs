//! Mounted fight transactions and source-order animation advancement.
use super::*;
impl World {
    pub(crate) fn combat(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        mut random: impl FnMut(ActorId, &'static str) -> u32,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        if self.setup.rules.combat_version < 3 {
            return self.legacy_combat(actors, c, tick, random, events);
        }
        self.source_swing_gates.clear();
        if self.setup.rules.combat_version >= 4 {
            self.refresh_source_combat_order(actors);
        }
        let ids: Vec<_> = self.units.keys().copied().collect();
        for id in ids {
            let source_think = self.pending_regeneration.remove(&id);
            if self.setup.rules.combat_version >= 4 {
                self.source_fight_actor_impl(
                    id,
                    actors,
                    c,
                    tick,
                    source_think,
                    &mut random,
                    events,
                )?;
                if source_think {
                    self.source_animate_actor(id, actors, c, tick)?;
                }
                continue;
            }
            if source_think {
                self.source_swing_gates.insert(id, false);
            }
            if source_think
                && self.units[&id].recovery.phase == Phase::Mounted
                && self.units[&id].recovery.outcome.is_none()
            {
                let stats = &mut self.units.get_mut(&id).unwrap().fighter.stats;
                stats.health = (stats.health + 1).min(stats.maximum);
            }
            self.combat_actor(id, actors, c, tick, source_think, &mut random, events)?;
            if source_think && self.units[&id].recovery.phase == Phase::Mounted {
                actors
                    .get_mut(&id)
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap()
                    .advance_animation(c, tick as u32)?;
            }
        }
        Ok(())
    }
    pub(crate) fn source_fight_begin(
        &mut self,
        actors: &BTreeMap<ActorId, crate::Actor>,
        external_targets: &[Position],
    ) {
        self.source_combat_external_targets.clear();
        self.source_combat_external_targets
            .extend_from_slice(external_targets);
        self.pending_regeneration.clear();
        self.source_swing_gates.clear();
        self.refresh_source_combat_order(actors);
    }
    pub(crate) fn source_fight_actor(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        random: impl FnMut(ActorId, &'static str) -> u32,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        self.pending_regeneration.remove(&id);
        self.source_fight_actor_impl(id, actors, c, tick, true, random, events)
    }
    #[allow(clippy::too_many_arguments)]
    fn source_fight_actor_impl(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        source_think: bool,
        mut random: impl FnMut(ActorId, &'static str) -> u32,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        // Taking the actor config avoids cloning its course/animation data.
        // Restore it even when a transaction fails; target configs stay present.
        let own_config = self.configs.remove(&id);
        let config = own_config.as_ref().unwrap_or(c);
        let result = (|| -> Result<()> {
            if source_think {
                self.source_swing_gates.insert(id, false);
            }
            if source_think
                && self.units[&id].recovery.phase == Phase::Mounted
                && self.units[&id].recovery.outcome.is_none()
            {
                if !self.units[&id].police {
                    let stats = &mut self.units.get_mut(&id).unwrap().fighter.stats;
                    stats.health = (stats.health + 1).min(stats.maximum);
                }
                self.prepare_source_attack(id, actors, config, tick, events)?;
            }
            self.combat_actor(id, actors, config, tick, source_think, &mut random, events)
        })();
        if let Some(config) = own_config {
            self.configs.insert(id, config);
        }
        result
    }
    pub(crate) fn source_animate_actor(
        &self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
    ) -> Result<()> {
        if self.units[&id].recovery.phase == Phase::Mounted
            && self.units[&id].recovery.outcome.is_none()
        {
            let config = self.configs.get(&id).unwrap_or(c);
            self.prepare_source_animation(id, actors, config);
            actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .advance_animation(config, tick as u32)?;
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn combat_actor(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        source_think: bool,
        mut random: impl FnMut(ActorId, &'static str) -> u32,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        let u = &self.units[&id];
        if u.recovery.phase != Phase::Mounted
            || u.recovery.outcome.is_some()
            || (self.setup.rules.combat_version < 4 && tick < u.fighter.stun_until)
        {
            return Ok(());
        }
        let Some(mut swing) = u.fighter.swing.clone() else {
            return Ok(());
        };
        let backhand = if self.setup.rules.combat_version >= 4 {
            let code = actors[&id].view.rider.as_ref().unwrap().animation.code;
            let slot = combat::weapon_slot(swing.request.kind, swing.weapon);
            swing.request.side = if code == c.animation.attacks[1][0][slot]
                || code == c.animation.attacks[1][1][slot]
            {
                Side::Right
            } else {
                Side::Left
            };
            // Kick normal/backhand table entries may alias: the live source
            // derives this flag from the retained animation, not the command.
            code == c.animation.attacks[0][1][slot] || code == c.animation.attacks[1][1][slot]
        } else {
            swing.request.kind == AttackKind::Backhand
        };
        let age = tick - swing.started;
        let active = if source_think {
            let player = &actors[&id].view.rider.as_ref().unwrap().animation;
            tick > swing.started
                && player.code == swing.definition.code
                && (player.frame == 2
                    || player.selected.is_some_and(|(code, frame)| {
                        c.animation.banks[0][code].as_ref().unwrap()[frame].flags & 0x600000 != 0
                    }))
        } else {
            swing.definition.active(age)
        };
        if source_think {
            self.source_swing_gates.insert(id, active);
        }
        if (swing.resolved && self.setup.rules.combat_version < 2) || !active {
            return Ok(());
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
        let selected = if self.setup.rules.combat_version >= 4 {
            self.source_combat_target(id, actors)
        } else if self.setup.rules.combat_version >= 2 {
            let at = self.position(id, actors).unwrap();
            combat::target(
                id,
                at,
                self.units
                    .iter()
                    .filter(|(other, u)| {
                        **other != id
                            && u.recovery.phase == Phase::Mounted
                            && u.recovery.outcome.is_none()
                    })
                    .map(|(other, _)| (*other, self.position(*other, actors).unwrap())),
            )
        } else {
            swing.target
        };
        let Some(target) =
            selected.filter(|t| {
                self.units.contains_key(t)
                    && (self.units[t].recovery.phase == Phase::Mounted
                        || (self.setup.rules.combat_version >= 4
                            && self.units[t].recovery.body.is_some()))
                    && self.units[t].recovery.outcome.is_none()
                    && (self.setup.rules.combat_version < 2
                        || if self.setup.rules.combat_version >= 4 {
                            self.units[t].recovery.body.as_ref().map_or(
                                actors[t].view.rider.as_ref().unwrap().animation.code,
                                |b| b.animation.code,
                            ) != 9
                        } else {
                            actors[t].view.rider.as_ref().unwrap().animation.code != 9
                        })
            })
        else {
            return Ok(());
        };
        let a = self.position(id, actors).unwrap();
        let b = if self.setup.rules.combat_version >= 4 {
            self.source_combat_relative_position(id, target, actors)
                .unwrap()
        } else {
            self.position(target, actors).unwrap()
        };
        let slot = combat::weapon_slot(swing.request.kind, swing.weapon);
        if !combat::in_reach(a, b, swing.request.side, slot, self.setup.rules.reach) {
            return Ok(());
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
                && self.units[&target].fighter.swing.as_ref().is_some_and(|s| {
                    if self.setup.rules.combat_version >= 2 {
                        actors[&target].view.rider.as_ref().unwrap().animation.frame < 2
                    } else {
                        s.definition.frame(tick - s.started) < 2
                    }
                })
        });
        if let Some((item, kind)) = theft.filter(|_| random(id, "weapon_theft") & 3 == 3)
            && self.inventory.transfer(item, target, id)
        {
            self.units.get_mut(&id).unwrap().fighter.steal_until = tick + 300;
            // 0x004096f0 transfers both the carried and latched attack weapon.
            if let Some(ai) = self
                .units
                .get_mut(&target)
                .and_then(|u| u.police_ai.as_mut())
            {
                ai.attack_weapon = 0;
            }
            if let Some(ai) = self.units.get_mut(&id).and_then(|u| u.police_ai.as_mut()) {
                ai.attack_weapon = kind as u8;
            }

            self.units.get_mut(&target).unwrap().fighter.swing = None;
            self.units.get_mut(&id).unwrap().fighter.swing = None;
            let victim_code = actors[&target].view.rider.as_ref().unwrap().animation.code;
            let left = victim_code == c.animation.attacks[0][0][0]
                || victim_code == c.animation.attacks[0][1][0];
            let receiver = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
            if self.setup.rules.combat_version >= 2
                && let Some(codes) = self.setup.rules.steal_codes
            {
                receiver.pose = animation::Pose::Steal;
                receiver.animation.set(
                    &c.animation,
                    0,
                    codes[kind as usize + usize::from(left) * 4],
                );
                receiver.animation.queued = Some(2);
            } else {
                receiver.pose = animation::Pose::Ride;
                receiver.animation.set(&c.animation, 0, 2);
            }
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
            if self.setup.rules.combat_version >= 2 {
                victim.animation.queued = Some(2);
            }
            if self.setup.rules.combat_version >= 4 {
                let code = actors[&id].view.rider.as_ref().unwrap().animation.code;
                let mut resumed = None;
                for side in [Side::Left, Side::Right] {
                    for attack in [AttackKind::Punch, AttackKind::Backhand] {
                        let side_index = usize::from(side == Side::Right);
                        let variant = usize::from(attack == AttackKind::Backhand);
                        if c.animation.attacks[side_index][variant][kind as usize] == code {
                            let request = Request { kind: attack, side };
                            resumed = Some(Swing {
                                repeat: false,
                                request,
                                target: Some(target),
                                weapon: kind,
                                started: tick,
                                definition: Definition::from_library_version(
                                    &c.animation,
                                    request,
                                    kind,
                                    4,
                                )?,
                                resolved: false,
                                swish: false,
                            });
                            break;
                        }
                    }
                    if resumed.is_some() {
                        break;
                    }
                }
                self.units.get_mut(&id).unwrap().fighter.swing = resumed;
            }
            Self::emit(events, tick, id, Some(target), EffectKind::Stolen);
            return Ok(());
        }
        if tick <= self.units[&target].fighter.hit_until {
            if self.setup.rules.combat_version >= 2 && slot == 3 {
                let direction = if swing.request.side == Side::Right {
                    1
                } else {
                    -1
                };
                let rider = actors
                    .get_mut(&target)
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap();
                rider.bike.lean = (rider.bike.lean + direction * 8000).clamp(-11776, 11776);
            }
            return Ok(());
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
        if self.setup.rules.combat_version >= 4 {
            self.police_relationship(target, id, 2)?;
            self.police_relationship(id, target, 4)?;
        }
        self.units.get_mut(&target).unwrap().fighter.hit_until =
            tick + u64::from(self.setup.rules.immunity);
        if self.setup.rules.combat_version >= 2 {
            let repeat =
                random(id, "attack_chain").is_multiple_of(u32::from(6 - self.setup.rules.level));
            let fallback = self.setup.rules.combat_version >= 4
                && !repeat
                && id != ActorId(1)
                && random(id, "attack_chain_fallback") & 3 == 2;
            let attacker = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
            if repeat {
                attacker.animation.queued = Some(if self.setup.rules.combat_version >= 4 {
                    attacker.animation.code
                } else {
                    swing.definition.code
                });
            } else if fallback && let Some(code) = c.animation.resolve(0, 6) {
                attacker.animation.queued = Some(code as i8);
            }
        }
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
        if backhand {
            target_rider.bike.speed = if self.setup.rules.combat_version >= 2 {
                target_rider.bike.speed - 300
            } else {
                (target_rider.bike.speed - 300).max(0)
            };
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
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn source_combat_probe_tick(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        config: &Configuration,
        tick: u64,
        random: impl FnMut(ActorId, &'static str) -> u32,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        self.pending_regeneration.extend(self.units.keys().copied());
        self.combat(actors, config, tick, random, events)
    }
    pub(crate) fn source_combat_half_position(
        &self,
        id: ActorId,
        actors: &BTreeMap<ActorId, crate::Actor>,
    ) -> Option<Position> {
        let unit = self.units.get(&id)?;
        if let Some(body) = unit.recovery.body.as_ref() {
            let at = body.location;
            Some(Position {
                route: 1,
                branch: self.setup.topology.paths.get(&at.path)?.branch,
                path: at.path,
                progress: if self.setup.rules.source_roads.is_some() {
                    body.motion.distance_q8
                } else {
                    body.projected.unwrap_or(body.motion.distance_q8)
                },
                lateral: body.motion.lateral,
                height: body.motion.height,
            })
        } else {
            self.position(id, actors)
        }
    }
    pub(crate) fn source_combat_relative_position(
        &self,
        origin: ActorId,
        other: ActorId,
        actors: &BTreeMap<ActorId, crate::Actor>,
    ) -> Option<Position> {
        let from = self.source_combat_half_position(origin, actors)?;
        let mut target = self.source_combat_half_position(other, actors)?;
        if self.setup.rules.source_roads.is_none() {
            return (from.same_road(target)
                && self.setup.topology.connected(from.path, target.path))
            .then_some(target);
        }
        let object = |owner| ContactObject {
            owner,
            detached: self.units[&owner].recovery.body.is_some(),
        };
        let a = object(origin);
        let b = object(other);
        let at = self.contact_location(a);
        let bt = self.contact_location(b);
        let am = self.road_identity_object(a, at.path, (at.progress >> 8) as u32)?[3];
        let bm = self.road_identity_object(b, bt.path, (bt.progress >> 8) as u32)?[3];
        if am != 3 && bm != 3 && am != bm {
            return None;
        }
        let delta = crate::source_roads::Roads::physical_delta(&self.setup.topology, at, bt)?;
        target.progress = from.progress + delta;
        target.route = from.route;
        target.branch = from.branch;
        Some(target)
    }
    fn source_combat_external_position(
        &self,
        origin: ActorId,
        mut target: Position,
        actors: &BTreeMap<ActorId, crate::Actor>,
    ) -> Option<Position> {
        let from = self.source_combat_half_position(origin, actors)?;
        let Some(roads) = &self.setup.rules.source_roads else {
            return (from.same_road(target)
                && self.setup.topology.connected(from.path, target.path))
            .then_some(target);
        };
        let object = ContactObject {
            owner: origin,
            detached: self.units[&origin].recovery.body.is_some(),
        };
        let at = self.contact_location(object);
        let bt = Location {
            path: target.path,
            progress: target.progress,
            lateral: target.lateral,
            height: target.height,
            heading: 0,
        };
        let am = self.road_identity_object(object, at.path, (at.progress >> 8) as u32)?[3];
        let bm = roads.physical_identity(bt.path, (bt.progress >> 8) as u32)?[3];
        if am != 3 && bm != 3 && am != bm {
            return None;
        }
        target.progress = from.progress
            + crate::source_roads::Roads::physical_delta(&self.setup.topology, at, bt)?;
        target.route = from.route;
        target.branch = from.branch;
        Some(target)
    }
    fn refresh_source_combat_order(&mut self, actors: &BTreeMap<ActorId, crate::Actor>) {
        // A physical graph has no single route-distance axis after unequal
        // branches. Each observer retains a stable rebased half-list view.
        let mut histories = std::mem::take(&mut self.source_combat_order);
        histories.retain(|id, _| self.units.contains_key(id));
        for origin in self.units.keys().copied() {
            let order = histories
                .entry(origin)
                .or_insert_with(|| self.units.keys().copied().collect());
            order.retain(|id| self.units.contains_key(id));
            for id in self.units.keys() {
                if !order.contains(id) {
                    order.push(*id);
                }
            }
            let slots: Vec<_> = order
                .iter()
                .enumerate()
                .filter(|(_, id)| {
                    self.source_combat_relative_position(origin, **id, actors)
                        .is_some()
                })
                .map(|(slot, _)| slot)
                .collect();
            let mut visible: Vec<_> = slots.iter().map(|slot| order[*slot]).collect();
            visible.sort_by_key(|id| {
                self.source_combat_relative_position(origin, *id, actors)
                    .unwrap()
                    .progress
            });
            for (slot, id) in slots.into_iter().zip(visible) {
                order[slot] = id;
            }
        }
        self.source_combat_order = histories;
    }
    pub(crate) fn source_combat_target(
        &self,
        id: ActorId,
        actors: &BTreeMap<ActorId, crate::Actor>,
    ) -> Option<ActorId> {
        let origin = self.source_combat_half_position(id, actors)?;
        let order: Vec<_> = self
            .source_combat_order
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.units.keys().copied().collect());
        let mut halves: Vec<_> = order
            .into_iter()
            .filter(|other| {
                self.units
                    .get(other)
                    .is_some_and(|u| u.recovery.outcome.is_none())
            })
            .filter_map(|other| {
                self.source_combat_relative_position(id, other, actors)
                    .map(|at| (other, at))
            })
            .filter(|(_, at)| {
                origin.same_road(*at) && self.setup.topology.connected(origin.path, at.path)
            })
            .collect();
        halves.sort_by_key(|(_, at)| at.progress);
        let index = halves.iter().position(|(other, _)| *other == id)?;
        let behind = index.checked_sub(1).map(|i| halves[i]);
        let ahead = halves.get(index + 1).copied();
        let chosen = match (behind, ahead) {
            (Some(back), Some(front))
                if origin.progress - back.1.progress <= front.1.progress - origin.progress =>
            {
                Some(back)
            }
            (_, Some(front)) => Some(front),
            (Some(back), _) => Some(back),
            _ => None,
        }?;
        ((chosen.1.progress - origin.progress).abs() < 480).then_some(chosen.0)
    }
    fn prepare_source_attack(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        config: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        let input = actors[&id].input;
        let at = self.source_combat_half_position(id, actors).unwrap();
        let chosen = combat::target(
            id,
            at,
            self.units
                .iter()
                .filter(|(_, u)| u.recovery.outcome.is_none())
                .filter_map(|(other, _)| {
                    self.source_combat_relative_position(id, *other, actors)
                        .map(|at| (*other, at))
                }),
        );
        let target_lateral = chosen
            .and_then(|target| self.source_combat_relative_position(id, target, actors))
            .map(|p| p.lateral)
            .or_else(|| {
                self.source_combat_external_targets
                    .iter()
                    .filter_map(|p| self.source_combat_external_position(id, *p, actors))
                    .filter(|p| (p.progress - at.progress).abs() < 480)
                    .min_by_key(|p| ((p.progress - at.progress).abs(), p.progress >= at.progress))
                    .map(|p| p.lateral)
            });
        let fighter = &mut self.units.get_mut(&id).unwrap().fighter;
        let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
        if fighter.swing.as_ref().is_some_and(|s| {
            let slot = combat::weapon_slot(s.request.kind, s.weapon);
            rider.animation.code != s.definition.code
                || !config
                    .animation
                    .attacks
                    .iter()
                    .flat_map(|side| side.iter())
                    .any(|variant| variant[slot] == rider.animation.code)
        }) {
            fighter.swing = None;
        }
        if input.weapon == Some(animation::Weapon::Fists) && self.inventory.drop(id).is_some() {
            fighter.swing = None;
            rider.animation.set(&config.animation, 0, 2);
            Self::emit(events, tick, id, None, EffectKind::Dropped);
        }
        rider.weapon = self
            .inventory
            .carried(id)
            .map_or(animation::Weapon::Fists, |(_, w)| w);
        let request = input
            .attack
            .or_else(|| input.pose.and_then(Request::from_pose));
        if let Some(mut request) = request.filter(|_| fighter.swing.is_none()) {
            if request.side == Side::Auto {
                request.side = if target_lateral
                    .map_or(rider.bike.lean >= 0, |target| rider.bike.lateral < target)
                {
                    Side::Right
                } else {
                    Side::Left
                };
            }
            let definition =
                Definition::from_library_version(&config.animation, request, rider.weapon, 4)?;
            rider.pose = combat::pose(request);
            rider.animation.set(&config.animation, 0, definition.code);
            fighter.swing = Some(Swing {
                repeat: false,
                request,
                target: chosen,
                weapon: rider.weapon,
                started: tick,
                definition,
                resolved: false,
                swish: false,
            });
        }
        Ok(())
    }
    fn prepare_source_animation(
        &self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        config: &Configuration,
    ) {
        let camera = actors
            .get(&ActorId(1))
            .and_then(|a| a.view.rider.as_ref())
            .map_or(0, |r| r.bike.distance_q8 - 256);
        let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
        if matches!(rider.animation.code, -1 | 2 | 3 | 4 | 8) {
            let code = if rider.bike.speed < 300 {
                8
            } else if rider.bike.speed < 1800 && rider.bike.force > 30 {
                4
            } else if rider.bike.lateral_speed < -4096 || rider.bike.speed >= 9501 {
                3
            } else {
                2
            };
            rider.animation.set(&config.animation, 0, code);
        }
        if id == ActorId(1) {
            if matches!(rider.animation.code, 0 | 1) {
                rider.animation.set(&config.animation, 0, 2);
            }
        } else {
            if rider.animation.code == 6 && rider.animation.loops > 2 {
                rider.animation.set(&config.animation, 0, 2);
            }
            if matches!(rider.animation.code, 0..=2) {
                let distance = (rider.bike.distance_q8 - camera).abs();
                let code = if distance < 1281 {
                    2
                } else if distance < 2561 {
                    0
                } else {
                    1
                };
                rider.animation.set(&config.animation, 0, code);
            }
        }
    }
    pub(crate) fn swing_active(&self, id: ActorId, tick: u64) -> bool {
        self.source_swing_gates
            .get(&id)
            .copied()
            .unwrap_or_else(|| {
                self.units[&id]
                    .fighter
                    .swing
                    .as_ref()
                    .is_some_and(|s| s.definition.active(tick.saturating_sub(s.started)))
            })
    }
}

#[cfg(test)]
#[path = "fighting_tests.rs"]
mod tests;
