//! Stable opponent decisions, gap reservations and command emission.
use super::*;
impl World {
    pub(crate) fn decisions(
        &mut self,
        e: &mut Encounters,
        actors: &mut BTreeMap<ActorId, Actor>,
        c: &riding::Configuration,
        tick: u64,
        mut random: impl FnMut() -> u32,
    ) -> Result<()> {
        let mut ids: Vec<_> = self
            .brains
            .keys()
            .copied()
            .filter(|id| actors.contains_key(id) && e.units.contains_key(id))
            .collect();
        ids.sort_by_key(|id| {
            (
                std::cmp::Reverse(actors[id].view.rider.as_ref().unwrap().bike.distance_q8),
                std::cmp::Reverse(*id),
            )
        });
        let mut reservations: Vec<(u32, i64, i32)> = vec![];
        self.navigation.clear();
        for id in ids {
            let mut brain = self.brains[&id].clone();
            let unit = &e.units[&id];
            let rider = actors[&id].view.rider.as_ref().unwrap();
            let at = unit.recovery.bike_location;
            let invalid = unit.recovery.phase != Phase::Mounted
                || unit.recovery.outcome.is_some()
                || unit.fighter.stats.health == 0
                || tick < unit.fighter.stun_until;
            if invalid {
                brain.target = None;
                let previous = actors[&id].input;
                actors.get_mut(&id).unwrap().input = Command {
                    impact: previous.impact,
                    terminal: previous.terminal,
                    damage: previous.damage,
                    walk: previous.walk,
                    ..Default::default()
                };
                brain.command = actors[&id].input;
                self.brains.insert(id, brain);
                continue;
            }
            if let Some(choice) = brain.traits.branch_choice {
                let mut portals: Vec<_> = e.setup.topology.paths[&at.path]
                    .portals
                    .iter()
                    .filter(|p| p.end)
                    .copied()
                    .collect();
                portals.sort_by_key(|p| p.target);
                if let Some(p) =
                    portals.get(usize::from(choice).min(portals.len().saturating_sub(1)))
                {
                    self.navigation.insert(id, *p);
                }
            }
            if tick < brain.next_tick {
                reservations.push((at.path, at.progress, brain.desired_lane));
                continue;
            }
            brain.interval = live_interval(
                (rider.bike.distance_q8
                    - actors[&ActorId(1)]
                        .view
                        .rider
                        .as_ref()
                        .unwrap()
                        .bike
                        .distance_q8)
                    .abs()
                    < 25600,
                rider.bike.distance_q8 as i32,
                actors[&ActorId(1)]
                    .view
                    .rider
                    .as_ref()
                    .unwrap()
                    .bike
                    .distance_q8 as i32,
                rider.bike.speed,
                tick.min(i64::from(i32::MAX) as u64) as i32,
                false,
            );
            brain.next_tick = tick + u64::from(brain.interval);
            brain.last_tick = Some(tick);
            brain.decisions += 1;
            let sample = (i64::from(c.start_sample) + (rider.bike.distance_q8 >> 8))
                .clamp(0, c.ground.len() as i64 - 1) as usize;
            let bend = c
                .ground
                .iter()
                .skip(sample)
                .take(12)
                .map(|g| g.bend.abs())
                .max()
                .unwrap_or(0);
            let spec = c.specs.iter().find(|s| s.id == brain.traits.bike).unwrap();
            // Native specifications retain unused trailing gear slots (up=0).
            // Those slots are not a 1000-unit top-speed limit.
            let speed_cap = if self.setup.functional_traffic {
                spec.gears
                    .iter()
                    .map(|g| g.up)
                    .filter(|up| *up > 0)
                    .max()
                    .unwrap_or(1000)
            } else {
                spec.gears.last().unwrap().up.max(1000)
            };
            brain.target_speed = speed_from_pace(if bend > 2 {
                brain.traits.corner_pace
            } else {
                brain.traits.pace
            })
            .min(speed_cap);
            let center = (c.ground[sample].left + c.ground[sample].right) * 128;
            let lanes: Vec<_> = if self.setup.functional_traffic {
                [-96000, -48000, 0, 48000, 96000]
                    .into_iter()
                    .map(|l| center + l)
                    .filter(|l| {
                        *l > (c.ground[sample].left + 50) * 256
                            && *l < (c.ground[sample].right - 50) * 256
                    })
                    .collect()
            } else {
                vec![center - 64000, center, center + 64000]
            };
            let mut obstacles: Vec<(i64, i32, i32)> = e
                .units
                .iter()
                .filter(|(other, u)| {
                    **other != id
                        && u.recovery.outcome.is_none()
                        && u.recovery.bike_location.path == at.path
                })
                .map(|(other, u)| {
                    (
                        u.recovery.bike_location.progress,
                        u.recovery.bike_location.lateral,
                        actors[other].view.rider.as_ref().unwrap().bike.speed,
                    )
                })
                .collect();
            obstacles.extend(
                self.agents
                    .values()
                    .filter(|a| a.location.path == at.path)
                    .map(|a| {
                        (
                            a.location.progress,
                            a.location.lateral,
                            if a.kind == Kind::Vehicle {
                                a.speed * i32::from(a.direction)
                            } else {
                                0
                            },
                        )
                    }),
            );
            let blocked = |lane: i32| {
                obstacles
                    .iter()
                    .any(|(p, l, _)| (*p - at.progress).abs() < 384 && (*l - lane).abs() < 32000)
                    || reservations.iter().any(|(path, p, l)| {
                        *path == at.path
                            && (*p - at.progress).abs() < 512
                            && (*l - lane).abs() < 32000
                    })
            };
            let slow = obstacles
                .iter()
                .filter(|(p, l, s)| {
                    *p >= at.progress
                        && *p - at.progress < 512
                        && (*l - rider.bike.lateral).abs() < 32000
                        && *s < rider.bike.speed + 1000
                })
                .min_by_key(|(p, _, _)| *p);
            if self.setup.tactics_version == 1 {
                // Score all available corridors over a speed-dependent horizon,
                // including actual building/scenery volumes and moving road users.
                // Aggressive riders also close laterally on rivals for combat.
                let horizon = (i64::from(rider.bike.speed.max(0)) / 8).clamp(384, 4096);
                let combat_target = e
                    .units
                    .iter()
                    .filter(|(other, u)| {
                        **other != id
                            && u.recovery.phase == Phase::Mounted
                            && u.recovery.outcome.is_none()
                            && u.recovery.bike_location.path == at.path
                            && (u.recovery.bike_location.progress - at.progress).abs() < 384
                    })
                    .min_by_key(|(other, u)| {
                        (
                            (u.recovery.bike_location.progress - at.progress).abs(),
                            **other,
                        )
                    });
                let aggression = i64::from(brain.traits.aggression.unwrap_or(64));
                let hazard_indices = e.obstacle_candidates(
                    at.path,
                    at.progress - 1088,
                    at.progress + horizon + 1024,
                );
                let combat_lane = combat_target
                    .filter(|_| aggression >= 96)
                    .map(|(target, u)| {
                        let side = if (id.0 + u64::from(brain.traits.personality.unwrap_or(0)))
                            .is_multiple_of(2)
                        {
                            1
                        } else {
                            -1
                        };
                        let clearance = (e.setup.rules.dimensions[0] * 2 + 512).clamp(17000, 19968);
                        (*target, u.recovery.bike_location.lateral + side * clearance)
                    });
                let mut candidates = lanes.clone();
                if let Some((_, lane)) = combat_lane
                    && lane > (c.ground[sample].left + 50) * 256
                    && lane < (c.ground[sample].right - 50) * 256
                {
                    candidates.push(lane);
                }
                let chosen = candidates
                    .iter()
                    .copied()
                    .map(|lane| {
                        let static_block = hazard_indices
                            .iter()
                            .map(|i| &e.setup.rules.obstacles[*i])
                            .any(|o| {
                                o.kind != crate::encounter::ObstacleKind::Decorative
                                    && o.path == at.path
                                    && o.progress + i64::from(o.half_length) > at.progress - 64
                                    && o.progress - i64::from(o.half_length) < at.progress + horizon
                                    && lane > o.left - 10000
                                    && lane < o.right + 10000
                            });
                        let mut score = if static_block { 1_000_000_000i64 } else { 0 };
                        for (p, l, speed) in &obstacles {
                            if *p < at.progress - 128 || *p > at.progress + horizon {
                                continue;
                            }
                            let combat_clearance = combat_lane.is_some_and(|(target, _)| {
                                let location = e.units[&target].recovery.bike_location;
                                location.progress == *p
                                    && location.lateral == *l
                                    && (lane - *l).abs() >= e.setup.rules.dimensions[0] * 2 + 256
                            });
                            if (lane - *l).abs() < 24000 && !combat_clearance {
                                let closing = (rider.bike.speed - *speed).max(0) as i64;
                                score += 2_000_000
                                    + (horizon - (*p - at.progress).max(0)) * 100
                                    + closing * 50;
                            }
                        }
                        if reservations.iter().any(|(path, p, l)| {
                            *path == at.path
                                && (*p - at.progress).abs() < 512
                                && (*l - lane).abs() < 24000
                        }) {
                            score += 1_000_000;
                        }
                        score += i64::from((lane - rider.bike.lateral).abs()) * 3;
                        if let Some((_, attack_lane)) = combat_lane {
                            score += i64::from((lane - attack_lane).abs()) * aggression / 16;
                        }
                        (score, lane, static_block)
                    })
                    .min_by_key(|(score, lane, _)| (*score, *lane));
                if let Some((_, lane, static_block)) = chosen {
                    brain.desired_lane = lane;
                    if static_block {
                        brain.target_speed = brain.target_speed.min(2000);
                    }
                }
                if let Some((progress, _, speed)) = obstacles
                    .iter()
                    .filter(|(p, l, _)| {
                        *p > at.progress
                            && *p - at.progress < horizon
                            && (*l - brain.desired_lane).abs() < 24000
                    })
                    .min_by_key(|(p, _, _)| *p)
                {
                    let gap = *progress - at.progress;
                    let safe_speed = (*speed).max(0) + (gap * 8).min(20000) as i32;
                    brain.target_speed = brain.target_speed.min(safe_speed);
                }
            } else {
                brain.desired_lane = lanes
                    .into_iter()
                    .filter(|l| !blocked(*l))
                    .min_by_key(|l| (*l - rider.bike.lateral).abs())
                    .unwrap_or(rider.bike.lateral);
            }
            brain.yielding = slow.is_some() && blocked(brain.desired_lane);
            if let Some((_, _, speed)) = slow.filter(|_| brain.yielding) {
                brain.target_speed = brain.target_speed.min((*speed - 500).max(0));
            }
            let error = brain.desired_lane - rider.bike.lateral;
            let steer = if error.abs() < 2048 {
                0
            } else {
                (error / 8).clamp(-32767, 32767) as i16
            };
            let throttle = rider.bike.speed < brain.target_speed - 100;
            let brake = rider.bike.speed > brain.target_speed + 300;
            let steer = if self.setup.functional_traffic {
                let config = e.configs.get(&id).unwrap_or(c);
                predictive_steering(rider, config, brain.desired_lane, throttle, brake, tick)
            } else {
                steer
            };
            let mut command = Command {
                throttle,
                brake,
                steer_q15: Some(steer),
                ..Default::default()
            };
            brain.target = e
                .units
                .iter()
                .filter(|(other, u)| {
                    **other != id
                        && u.recovery.phase == Phase::Mounted
                        && u.recovery.outcome.is_none()
                        && u.fighter.stats.health > 0
                        && u.recovery.bike_location.path == at.path
                        && (u.recovery.bike_location.progress - at.progress).abs() < 64
                        && (u.recovery.bike_location.lateral - at.lateral).abs() < 20480
                })
                .min_by_key(|(other, u)| {
                    (
                        (u.recovery.bike_location.progress - at.progress).abs(),
                        **other,
                    )
                })
                .map(|(id, _)| *id);
            if let Some(target) = brain
                .target
                .filter(|_| tick >= brain.attack_after && unit.fighter.swing.is_none())
            {
                let gate = self.draw(tick, id.0, "attack_gate", &mut random) & 255;
                if gate < u32::from(brain.traits.aggression.unwrap_or(64)) {
                    let roll = self.draw(tick, id.0, "attack_kind", &mut random) % 3;
                    let kind = match roll {
                        0 => AttackKind::Punch,
                        1 => AttackKind::Kick,
                        _ => AttackKind::Backhand,
                    };
                    command.attack = Some(Request {
                        kind,
                        side: if e.units[&target].recovery.bike_location.lateral >= at.lateral {
                            Side::Right
                        } else {
                            Side::Left
                        },
                    });
                    brain.attack_after = tick + u64::from(brain.traits.cooldown);
                }
            }
            reservations.push((at.path, at.progress, brain.desired_lane));
            let previous = actors[&id].input;
            command.impact = previous.impact;
            command.terminal = previous.terminal;
            command.damage = previous.damage;
            actors.get_mut(&id).unwrap().input = command;
            brain.command = command;
            self.brains.insert(id, brain);
        }
        Ok(())
    }
}

/// Modern lane controller: bounded integer prediction on a copy of the assigned
/// source bike. It consumes no RNG and never advances another actor's authority.
fn predictive_steering(
    rider: &riding::Rider,
    c: &riding::Configuration,
    target: i32,
    throttle: bool,
    brake: bool,
    tick: u64,
) -> i16 {
    let current = (i64::from(c.start_sample) + (rider.bike.distance_q8 >> 8))
        .clamp(0, c.ground.len() as i64 - 1) as usize;
    let center = (c.ground[current].left + c.ground[current].right) * 128;
    let mut best = (i128::MAX, 0i16);
    for steer in [-32767i16, -16384, -8192, -4096, 0, 4096, 8192, 16384, 32767] {
        let mut predicted = rider.clone();
        let mut score = 0i128;
        let input = Command {
            throttle,
            brake,
            steer_q15: Some(steer),
            ..Default::default()
        };
        for ahead in 1..=18 {
            if predicted.step(c, input, (tick + ahead) as u32).is_err() {
                score += 1_000_000_000;
                break;
            }
            let b = &predicted.bike;
            let sample = (i64::from(c.start_sample) + (b.distance_q8 >> 8))
                .clamp(0, c.ground.len() as i64 - 1) as usize;
            let ground = c.ground[sample];
            let next_center = (ground.left + ground.right) * 128;
            score += i128::from(
                (i64::from(b.lateral) - i64::from(target) - i64::from(next_center - center)).abs(),
            );
            if b.crashed || b.failed {
                score += 1_000_000_000;
            }
            if b.lateral < (ground.left + 50) * 256 || b.lateral > (ground.right - 50) * 256 {
                score += 20_000_000;
            }
        }
        score += i128::from(predicted.bike.lateral_speed.unsigned_abs()) * 4;
        if score < best.0 {
            best = (score, steer);
        }
    }
    best.1
}
