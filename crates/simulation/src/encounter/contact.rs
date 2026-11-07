//! Source contact adapters for linked bikes/bodies, scenery and road boundaries.
//! The integer classifier and responses remain in collision.rs.
use super::*;
impl World {
    pub(crate) fn source_building_contact(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        if self.units[&id].recovery.outcome.is_some() {
            return Ok(());
        }
        let at = self.units[&id].recovery.bike_location;
        let sections = &self.setup.rules.building_boundaries;
        let Ok(index) = sections.binary_search_by_key(&(at.path, (at.progress >> 8) as u32), |b| {
            (b.path, b.sample)
        }) else {
            return Ok(());
        };
        let current = &sections[index];
        let previous_at = self.units[&id].previous_location;
        let previous = sections
            .binary_search_by_key(
                &(previous_at.path, (previous_at.progress >> 8) as u32),
                |b| (b.path, b.sample),
            )
            .ok()
            .map_or(current, |i| &sections[i]);
        let mut motion = self.contact_motion(id, actors, c);
        let profile = self.setup.rules.collision.as_ref().unwrap();
        let width = if motion.crashed {
            profile.fallen[0]
        } else {
            profile.mounted[0]
        };
        let lateral = actors[&id].view.rider.as_ref().unwrap().bike.lateral;
        let hit = crate::building_wall::resolve(
            current,
            previous,
            at.progress as i32,
            lateral,
            width,
            &mut motion,
        );
        if !hit.hit {
            return Ok(());
        }
        let unit = self.units.get_mut(&id).unwrap();
        let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
        if hit.restore_progress {
            let from = self
                .setup
                .topology
                .projected(unit.recovery.bike_location)
                .unwrap();
            let to = self.setup.topology.projected(previous_at).unwrap_or(from);
            self.setup
                .topology
                .advance(&mut unit.recovery.bike_location, to - from, None)?;
            rider.bike.distance_q8 += to - from;
        }
        rider.bike.speed = motion.speed;
        rider.bike.lateral_speed = motion.lateral;
        rider.bike.lean = motion.lean;
        if hit.crash {
            let config = self.configs.get(&id).unwrap_or(c);
            let entered = unit.recovery.begin_source(
                id,
                &mut rider.bike,
                config,
                unit.recovery.bike_location,
                Impact {
                    reason: 6,
                    rider_velocity: Some([0, 0, 80]),
                    bike_velocity: Some([0, 0, 16]),
                    ..Default::default()
                },
                &self.setup.topology,
            );
            if entered {
                unit.fighter.swing = None;
                Self::emit(events, tick, id, None, EffectKind::Crash);
            }
        }
        rider.bike.lateral = hit.lateral;
        unit.recovery.bike_location.lateral = hit.lateral;
        unit.recovery.bike_location.height = rider.bike.height;
        Ok(())
    }
    pub(super) fn source_detached_boundary(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
    ) -> Result<()> {
        if self.units[&id].recovery.outcome.is_some() {
            return Ok(());
        }
        let object = ContactObject {
            owner: id,
            detached: true,
        };
        let at = self.contact_location(object);
        let sections = &self.setup.rules.building_boundaries;
        let Ok(index) = sections.binary_search_by_key(&(at.path, (at.progress >> 8) as u32), |b| {
            (b.path, b.sample)
        }) else {
            return Ok(());
        };
        let current = &sections[index];
        let rider = self.units[&id].recovery.body.as_ref().unwrap();
        let old = rider.previous_location.unwrap_or(at);
        let previous = sections
            .binary_search_by_key(&(old.path, (old.progress >> 8) as u32), |b| {
                (b.path, b.sample)
            })
            .ok()
            .map_or(current, |i| &sections[i]);
        let mut body = self.proxy_body(object, actors, c);
        let mut motion = self.proxy_motion(object, actors, c);
        let hit = crate::building_wall::resolve_actor(
            current,
            previous,
            at.progress as i32,
            body.position[1],
            body.half[1],
            &mut motion,
            8,
        );
        if !hit.hit {
            return Ok(());
        }
        if hit.restore_progress {
            body.position[0] = body.previous[0];
        }
        body.position[1] = hit.lateral;
        self.store_proxy(object, actors, &body, &motion)?;
        let rider = self
            .units
            .get_mut(&id)
            .unwrap()
            .recovery
            .body
            .as_mut()
            .unwrap();
        if let Some(yaw) = hit.visual_heading {
            rider.visual_heading = Some(yaw);
        }
        if let Some((sequence, index)) = rider.animation.selected
            && let Some(frame) = c.animation.banks[0]
                .get(sequence)
                .and_then(Option::as_ref)
                .and_then(|frames| frames.get(index))
        {
            let (cell, roll) = c
                .animation
                .select(
                    0,
                    frame,
                    rider.visual_heading.unwrap_or(rider.location.heading),
                    0,
                )
                .map_err(|_| Error::Invalid("detached boundary view"))?;
            rider.frame = cell.frame;
            rider.mirror = cell.flags & 1 != 0;
            rider.view_flags = cell.flags;
            rider.roll = roll;
        }
        Ok(())
    }
    fn collision_body(
        &self,
        id: ActorId,
        actors: &BTreeMap<ActorId, crate::Actor>,
    ) -> crate::collision::Body {
        let profile = self
            .setup
            .rules
            .collision
            .as_ref()
            .expect("validated collision profile");
        let at = self.position(id, actors).unwrap();
        let bike = &actors[&id].view.rider.as_ref().unwrap().bike;
        let previous = self.units[&id].previous_location;
        let old_progress = self
            .setup
            .topology
            .projected(previous)
            .unwrap_or(at.progress);
        let dims = if bike.crashed {
            profile.fallen
        } else {
            profile.mounted
        };
        crate::collision::Body {
            position: [at.progress as i32, at.lateral, at.height],
            previous: [old_progress as i32, previous.lateral, previous.height],
            half: [dims[1], dims[0], dims[2]],
            velocity: bike.contact_velocity.unwrap_or([
                bike.speed,
                bike.lateral_speed,
                bike.vertical_speed,
            ]),
            speed: bike.speed,
            mass: profile.mass,
            category: 4,
            flags: 0,
            impulse: [0; 3],
        }
    }
    fn contact_motion(
        &self,
        id: ActorId,
        actors: &BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
    ) -> crate::collision::Motion {
        let bike = &actors[&id].view.rider.as_ref().unwrap().bike;
        crate::collision::Motion {
            speed: bike.speed,
            lateral: bike.lateral_speed,
            vertical: bike.vertical_speed,
            lean: bike.lean,
            crashed: bike.crashed,
            elasticity: self.setup.rules.collision.as_ref().unwrap().elasticity,
            steering_gain: self.configs.get(&id).unwrap_or(c).spec().contact_steer_gain,
        }
    }
    fn store_contact(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        body: &crate::collision::Body,
        motion: &crate::collision::Motion,
    ) -> Result<()> {
        let delta = i64::from(body.position[0]) - self.position(id, actors).unwrap().progress;
        let unit = self.units.get_mut(&id).unwrap();
        self.setup.topology.advance(
            &mut unit.recovery.bike_location,
            delta,
            self.navigation.get(&id).copied(),
        )?;
        unit.recovery.bike_location.lateral = body.position[1];
        unit.recovery.bike_location.height = body.position[2];
        let mut previous = unit.recovery.bike_location;
        self.setup.topology.advance(
            &mut previous,
            i64::from(body.previous[0]) - i64::from(body.position[0]),
            None,
        )?;
        previous.lateral = body.previous[1];
        previous.height = body.previous[2];
        unit.previous_location = previous;
        let bike = &mut actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.distance_q8 += delta;
        bike.lateral = body.position[1];
        bike.height = body.position[2];
        bike.speed = motion.speed;
        bike.lateral_speed = motion.lateral;
        bike.vertical_speed = motion.vertical;
        bike.lean = motion.lean;
        Ok(())
    }
    fn respond_contact(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        (body, other): (&mut crate::collision::Body, &crate::collision::Body),
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        let mut motion = self.contact_motion(id, actors, c);
        let mut failure = None;
        crate::collision::respond(body, other, &mut motion, |body, motion, damage| {
            let outcome = (|| -> Result<bool> {
                self.store_contact(id, actors, body, motion)?;
                let down = if damage.forced {
                    let config = self.configs.get(&id).unwrap_or(c);
                    let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
                    let unit = self.units.get_mut(&id).unwrap();
                    unit.fighter.swing = None;
                    let entered = unit.recovery.begin_source(
                        id,
                        &mut rider.bike,
                        config,
                        unit.recovery.bike_location,
                        Impact {
                            reason: damage.reason,
                            rider_velocity: Some([0, 0, 512]),
                            bike_velocity: Some([0, 0, 256]),
                            ..Default::default()
                        },
                        &self.setup.topology,
                    );
                    if entered {
                        Self::emit(events, tick, id, None, EffectKind::Crash);
                    }
                    rider.bike.crashed
                } else {
                    self.hit(
                        actors,
                        c,
                        id,
                        Impact {
                            reason: damage.reason,
                            stamina_damage: damage.stamina,
                            bike_damage: damage.bike,
                            ..Default::default()
                        },
                        tick,
                        false,
                        events,
                    )?
                };
                *motion = self.contact_motion(id, actors, c);
                let updated = self.collision_body(id, actors);
                body.position = updated.position;
                body.half = updated.half;
                // knock_off's common tail applies the scaled lateral impulse
                // before the handler's own steer-rate kick.
                if !motion.crashed {
                    motion.lean = motion
                        .lean
                        .wrapping_add(motion.steering_gain.wrapping_mul(body.impulse[1]) / 256)
                        .clamp(-11776, 11776);
                }
                Ok(down)
            })();
            match outcome {
                Ok(down) => down,
                Err(error) => {
                    failure = Some(error);
                    true
                }
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        self.store_contact(id, actors, body, &motion)
    }
    pub(super) fn contact_location(&self, object: ContactObject) -> Location {
        let recovery = &self.units[&object.owner].recovery;
        if object.detached {
            recovery.body.as_ref().unwrap().location
        } else {
            recovery.bike_location
        }
    }
    pub(super) fn proxy_body(
        &self,
        object: ContactObject,
        actors: &BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
    ) -> crate::collision::Body {
        if !object.detached {
            return self.collision_body(object.owner, actors);
        }
        let recovery = &self.units[&object.owner].recovery;
        let rider = recovery.body.as_ref().unwrap();
        let profile = self
            .setup
            .rules
            .collision
            .as_ref()
            .unwrap()
            .rider
            .as_ref()
            .unwrap();
        let previous = rider
            .previous_location
            .unwrap_or(self.units[&object.owner].previous_location);
        let progress = self
            .setup
            .topology
            .projected(rider.location)
            .unwrap_or(rider.motion.distance_q8);
        let velocity =
            crate::recovery::actor_velocity(rider.motion.speed, rider.location.heading, &c.tables);
        crate::collision::Body {
            position: [progress as i32, rider.motion.lateral, rider.motion.height],
            previous: [
                self.setup.topology.projected(previous).unwrap_or(progress) as i32,
                previous.lateral,
                previous.height,
            ],
            half: [
                profile.dimensions[1],
                profile.dimensions[0],
                profile.dimensions[2],
            ],
            velocity: rider.motion.contact_velocity.unwrap_or([
                velocity[0],
                velocity[1],
                rider.motion.vertical_speed,
            ]),
            speed: rider.motion.speed,
            mass: profile.mass,
            category: 8,
            flags: 0,
            impulse: [0; 3],
        }
    }
    fn proxy_motion(
        &self,
        object: ContactObject,
        actors: &BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
    ) -> crate::collision::Motion {
        if !object.detached {
            return self.contact_motion(object.owner, actors, c);
        }
        let rider = self.units[&object.owner].recovery.body.as_ref().unwrap();
        crate::collision::Motion {
            speed: rider.motion.speed,
            lateral: rider.motion.lateral_speed,
            vertical: rider.motion.vertical_speed,
            lean: rider.motion.lean,
            steering_gain: 0,
            elasticity: self
                .setup
                .rules
                .collision
                .as_ref()
                .unwrap()
                .rider
                .as_ref()
                .unwrap()
                .elasticity,
            crashed: true,
        }
    }
    fn store_proxy(
        &mut self,
        object: ContactObject,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        body: &crate::collision::Body,
        motion: &crate::collision::Motion,
    ) -> Result<()> {
        if !object.detached {
            return self.store_contact(object.owner, actors, body, motion);
        }
        let rider = self
            .units
            .get_mut(&object.owner)
            .unwrap()
            .recovery
            .body
            .as_mut()
            .unwrap();
        let from = self
            .setup
            .topology
            .projected(rider.location)
            .unwrap_or(rider.motion.distance_q8);
        let delta = i64::from(body.position[0]) - from;
        self.setup
            .topology
            .advance(&mut rider.location, delta, None)?;
        rider.motion.distance_q8 += delta;
        let mut previous = rider.location;
        self.setup.topology.advance(
            &mut previous,
            i64::from(body.previous[0]) - i64::from(body.position[0]),
            None,
        )?;
        previous.lateral = body.previous[1];
        previous.height = body.previous[2];
        rider.previous_location = Some(previous);
        rider.motion.lateral = body.position[1];
        rider.motion.height = body.position[2];
        rider.motion.speed = motion.speed;
        rider.motion.lateral_speed = motion.lateral;
        rider.motion.vertical_speed = motion.vertical;
        rider.motion.lean = motion.lean;
        rider.location.lateral = rider.motion.lateral;
        rider.location.height = rider.motion.height;
        rider.projected = self.setup.topology.projected(rider.location);
        rider.branch = self.setup.topology.paths[&rider.location.path].branch;
        Ok(())
    }
    fn respond_proxy(
        &mut self,
        (object, other_object): (ContactObject, Option<ContactObject>),
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        (body, other): (&mut crate::collision::Body, &crate::collision::Body),
        c: &Configuration,
        context: (u64, &mut Vec<Effect>),
        random: impl FnMut() -> u32,
    ) -> Result<()> {
        let own = other_object.is_some_and(|o| o.owner == object.owner);
        let bike = &actors[&object.owner].view.rider.as_ref().unwrap().bike;
        let quiet = own && bike.speed < 10 && bike.height == 0;
        if !object.detached {
            if quiet {
                let mut motion = self.contact_motion(object.owner, actors, c);
                motion.speed = motion.speed.clamp(-1000, 30000);
                motion.vertical = motion.vertical.clamp(-10000, 1536);
                body.flags = 0;
                body.impulse = [0; 3];
                return self.store_contact(object.owner, actors, body, &motion);
            }
            return self.respond_contact(
                object.owner,
                actors,
                (body, other),
                c,
                context.0,
                context.1,
            );
        }
        let mut motion = self.proxy_motion(object, actors, c);
        let unit = &self.units[&object.owner];
        let rider = unit.recovery.body.as_ref().unwrap();
        let mut control = crate::collision::HalfControl {
            state: match unit.recovery.phase {
                Phase::Sliding => 1,
                Phase::Mounting => 3,
                _ => 2,
            },
            animation: rider.animation.code,
            queued: rider.animation.queued,
            reset_animation: false,
        };
        crate::collision::respond_half(body, other, &mut motion, &mut control, quiet, random);
        self.store_proxy(object, actors, body, &motion)?;
        let recovery = &mut self.units.get_mut(&object.owner).unwrap().recovery;
        let rider = recovery.body.as_mut().unwrap();
        if control.reset_animation {
            rider.animation.set(&c.animation, 0, control.animation);
            rider.animation.queued = control.queued;
        }
        recovery.phase = match control.state {
            1 => Phase::Sliding,
            3 => Phase::Mounting,
            _ => match rider.animation.code {
                37 => Phase::Standing,
                38 => Phase::Walking,
                _ => Phase::GettingUp,
            },
        };
        Ok(())
    }
    pub(crate) fn source_proxy_pair(
        &mut self,
        (a, b): (ContactObject, ContactObject),
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        context: (u64, &mut Vec<Effect>),
        mut random: impl FnMut() -> u32,
    ) -> Result<bool> {
        let al = self.contact_location(a);
        let bl = self.contact_location(b);
        if !self.setup.topology.connected(al.path, bl.path)
            || self.setup.topology.paths[&al.path].branch
                != self.setup.topology.paths[&bl.path].branch
        {
            return Ok(false);
        }
        let mut first = self.proxy_body(a, actors, c);
        let mut second = self.proxy_body(b, actors, c);
        if !first.overlaps(&second) {
            return Ok(false);
        }
        if a.owner != b.owner {
            crate::collision::separate(&mut first, &mut second);
        }
        let ma = self.proxy_motion(a, actors, c);
        let mb = self.proxy_motion(b, actors, c);
        self.store_proxy(a, actors, &first, &ma)?;
        self.store_proxy(b, actors, &second, &mb)?;
        self.respond_proxy(
            (a, Some(b)),
            actors,
            (&mut first, &second),
            c,
            (context.0, context.1),
            &mut random,
        )?;
        self.respond_proxy(
            (b, Some(a)),
            actors,
            (&mut second, &first),
            c,
            (context.0, context.1),
            &mut random,
        )?;
        Ok(true)
    }
    pub(crate) fn source_detached_object_contact(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        other: &mut crate::collision::Body,
        c: &Configuration,
        context: (u64, &mut Vec<Effect>),
        random: impl FnMut() -> u32,
    ) -> Result<bool> {
        if self.units[&id].recovery.body.is_none()
            || self.setup.rules.collision.as_ref().unwrap().rider.is_none()
        {
            return Ok(false);
        }
        let object = ContactObject {
            owner: id,
            detached: true,
        };
        let mut body = self.proxy_body(object, actors, c);
        if other.category != 1 && !body.overlaps(other) {
            return Ok(false);
        }
        if other.category == 1 || body.position[0] <= other.position[0] {
            crate::collision::separate(&mut body, other);
        } else {
            crate::collision::separate(other, &mut body);
        }
        let motion = self.proxy_motion(object, actors, c);
        self.store_proxy(object, actors, &body, &motion)?;
        self.respond_proxy(
            (object, None),
            actors,
            (&mut body, other),
            c,
            context,
            random,
        )?;
        Ok(true)
    }
    pub(crate) fn source_object_contact(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        other: &mut crate::collision::Body,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<bool> {
        let mut body = self.collision_body(id, actors);
        if other.category != 1 && !body.overlaps(other) {
            return Ok(false);
        }
        if other.category == 1 || body.position[0] <= other.position[0] {
            crate::collision::separate(&mut body, other);
        } else {
            crate::collision::separate(other, &mut body);
        }
        let motion = self.contact_motion(id, actors, c);
        self.store_contact(id, actors, &body, &motion)?;
        self.respond_contact(id, actors, (&mut body, other), c, tick, events)?;
        Ok(true)
    }
    #[cfg(test)]
    pub(crate) fn source_pair_contact(
        &mut self,
        a: ActorId,
        b: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<bool> {
        let mut first = self.collision_body(a, actors);
        let mut second = self.collision_body(b, actors);
        if !self
            .position(a, actors)
            .unwrap()
            .same_road(self.position(b, actors).unwrap())
            || !first.overlaps(&second)
        {
            return Ok(false);
        }
        crate::collision::separate(&mut first, &mut second);
        let ma = self.contact_motion(a, actors, c);
        let mb = self.contact_motion(b, actors, c);
        self.store_contact(a, actors, &first, &ma)?;
        self.store_contact(b, actors, &second, &mb)?;
        self.respond_contact(a, actors, (&mut first, &second), c, tick, events)?;
        self.respond_contact(b, actors, (&mut second, &first), c, tick, events)?;
        Ok(true)
    }
}
