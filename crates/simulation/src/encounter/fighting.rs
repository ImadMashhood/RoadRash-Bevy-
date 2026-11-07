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
        let ids: Vec<_> = self.units.keys().copied().collect();
        for id in ids {
            let source_think = self.pending_regeneration.remove(&id);
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
            || tick < u.fighter.stun_until
        {
            return Ok(());
        }
        let Some(swing) = u.fighter.swing.clone() else {
            return Ok(());
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
        let selected = if self.setup.rules.combat_version >= 2 {
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
            return Ok(());
        };
        let a = self.position(id, actors).unwrap();
        let b = self.position(target, actors).unwrap();
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
        self.units.get_mut(&target).unwrap().fighter.hit_until =
            tick + u64::from(self.setup.rules.immunity);
        if self.setup.rules.combat_version >= 2
            && random(id, "attack_chain").is_multiple_of(u32::from(6 - self.setup.rules.level))
        {
            actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .animation
                .queued = Some(swing.definition.code);
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
