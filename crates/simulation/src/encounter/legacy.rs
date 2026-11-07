//! Frozen published combat profiles 0–2. New source-order transactions use profile 3.
//! Keep these transitions stable for historical replay hashes.
use super::*;
impl World {
    pub(crate) fn legacy_movement(
        &mut self,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        external_targets: &[Position],
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
            let mut continuation = None;
            if u.fighter
                .swing
                .as_ref()
                .is_some_and(|s| tick - s.started >= u64::from(s.definition.duration))
            {
                if self.setup.rules.combat_version == 2 {
                    continuation = u
                        .fighter
                        .swing
                        .as_ref()
                        .filter(|s| s.repeat)
                        .map(|s| s.request);
                }
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
                .or(continuation)
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
                    let target_lateral =
                        chosen.map(|other| positions[&other].lateral).or_else(|| {
                            (self.setup.rules.combat_version == 2)
                                .then(|| {
                                    external_targets
                                        .iter()
                                        .filter(|p| {
                                            positions[id].same_road(**p)
                                                && (positions[id].progress - p.progress).abs() < 480
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
                    request.side =
                        if target_lateral.is_some_and(|lateral| positions[id].lateral < lateral) {
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
    pub(crate) fn legacy_combat(
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
            if (swing.resolved && self.setup.rules.combat_version < 2)
                || !swing.definition.active(age)
            {
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
            let selected = if self.setup.rules.combat_version == 2 {
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
            let Some(target) = selected.filter(|t| {
                self.units.contains_key(t)
                    && self.units[t].recovery.phase == Phase::Mounted
                    && self.units[t].recovery.outcome.is_none()
                    && (self.setup.rules.combat_version < 2
                        || actors[t].view.rider.as_ref().unwrap().animation.code != 9)
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
                    && self.units[&target].fighter.swing.as_ref().is_some_and(|s| {
                        if self.setup.rules.combat_version == 2 {
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
                self.units.get_mut(&target).unwrap().fighter.swing = None;
                self.units.get_mut(&id).unwrap().fighter.swing = None;
                let victim_code = actors[&target].view.rider.as_ref().unwrap().animation.code;
                let left = victim_code == c.animation.attacks[0][0][0]
                    || victim_code == c.animation.attacks[0][1][0];
                let receiver = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
                if self.setup.rules.combat_version == 2
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
                if self.setup.rules.combat_version == 2 {
                    victim.animation.queued = Some(2);
                }
                Self::emit(events, tick, id, Some(target), EffectKind::Stolen);
                continue;
            }
            if tick <= self.units[&target].fighter.hit_until {
                if self.setup.rules.combat_version == 2 && slot == 3 {
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
            if self.setup.rules.combat_version == 2
                && random(id, "attack_chain").is_multiple_of(u32::from(6 - self.setup.rules.level))
            {
                self.units
                    .get_mut(&id)
                    .unwrap()
                    .fighter
                    .swing
                    .as_mut()
                    .unwrap()
                    .repeat = true;
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
            if swing.request.kind == AttackKind::Backhand {
                target_rider.bike.speed = if self.setup.rules.combat_version == 2 {
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
        }
        Ok(())
    }
}
