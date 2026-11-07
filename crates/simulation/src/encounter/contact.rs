//! Source contact adapters for linked bikes/bodies, scenery and road boundaries.
//! The integer classifier and responses remain in collision.rs.
use super::*;
impl World {
    fn source_boundary_at(
        &self,
        object: ContactObject,
        at: Location,
    ) -> Result<Option<crate::building_wall::Boundary>> {
        let sections = &self.setup.rules.building_boundaries;
        let Ok(index) = sections
            .binary_search_by_key(&(at.path, (at.progress >> 8) as u32), |boundary| {
                (boundary.path, boundary.sample)
            })
        else {
            return Ok(None);
        };
        let mut boundary = sections[index].clone();
        if self.setup.rules.source_roads.is_some() {
            let identity = self
                .road_identity_object(object, at.path, boundary.sample)
                .ok_or(Error::Invalid("missing physical terrain boundary"))?;
            boundary.translate(identity[2]);
            if let Some(surface) = &mut boundary.surface
                && surface.event_type == 5
                && let Some(height) = surface.drawn_rail_height
                && height > 0
            {
                let state = self.scenery.as_ref();
                let player = &self.units[&ActorId(1)];
                let follow = state
                    .and_then(|state| state.frame_follow.or(state.pending_camera))
                    .unwrap_or(player.previous_location);
                let route = state
                    .and_then(|state| state.source_route)
                    .or_else(|| {
                        player
                            .recovery
                            .body
                            .as_ref()
                            .and_then(|body| body.source_route)
                    })
                    .or(player.source_route);
                let camera_sample = route
                    .and_then(|route| {
                        self.setup
                            .rules
                            .source_roads
                            .as_ref()?
                            .profiles
                            .get(usize::from(route))?
                            .identity(follow.path, (follow.progress >> 8) as u32)
                    })
                    .map(|value| {
                        (i64::from(value[1]) * 256 + (follow.progress & 255)
                            - 256
                            - i64::from(state.map_or(0, |state| {
                                if self.setup.rules.contact_walk_version >= 2
                                    && state.frame_follow.is_none()
                                    && state.pending_camera.is_some()
                                {
                                    0
                                } else {
                                    state.camera_back
                                }
                            })))
                        .max(0)
                            / 256
                    });
                if camera_sample.is_some_and(|camera| i64::from(identity[1]) <= camera.max(0) + 31)
                {
                    surface.rail_flags = (((height / 4) << 2) as u8) | (surface.rail_flags & 3);
                }
            }
        }
        Ok(Some(boundary))
    }
    fn source_boundary_neighbors(
        &self,
        object: ContactObject,
        at: Location,
        previous: Location,
    ) -> Result<Option<[crate::building_wall::Boundary; 4]>> {
        let Some(current) = self.source_boundary_at(object, at)? else {
            return Ok(None);
        };
        let next = |at: Location| {
            let route = if object.detached {
                self.units[&object.owner]
                    .recovery
                    .body
                    .as_ref()
                    .and_then(|body| body.source_route)
            } else {
                self.units[&object.owner].source_route
            };
            let topology = route
                .and_then(|route| self.road_topologies.get(&route))
                .unwrap_or(&self.setup.topology);
            let mut next = at;
            next.progress = (at.progress >> 8) * 256;
            topology.advance(&mut next, 256, None)?;
            self.source_boundary_at(object, next)
        };
        let previous_boundary = self
            .source_boundary_at(object, previous)?
            .unwrap_or_else(|| current.clone());
        let next_boundary = next(at)?.unwrap_or_else(|| current.clone());
        let previous_next = next(previous)?.unwrap_or_else(|| previous_boundary.clone());
        Ok(Some([
            current,
            next_boundary,
            previous_boundary,
            previous_next,
        ]))
    }

    pub(super) fn source_vertical_ground(
        &self,
        id: ActorId,
        actors: &BTreeMap<ActorId, crate::Actor>,
        mut ground: crate::handling::Ground,
    ) -> Result<crate::handling::Ground> {
        let object = ContactObject {
            owner: id,
            detached: false,
        };
        let at = self.contact_location(object);
        if let Some([current, next, _, _]) = self.source_boundary_neighbors(object, at, at)?
            && current.surface.is_some()
        {
            let bike = &actors[&id].view.rider.as_ref().unwrap().bike;
            ground.height = crate::building_wall::ground_height(
                &current,
                &next,
                bike.lateral,
                at.progress as u8,
            )?;
            let route = self.units[&id]
                .source_route
                .and_then(|route| self.road_topologies.get(&route))
                .unwrap_or(&self.setup.topology);
            ground.rise = route.grade(at);
        }
        Ok(ground)
    }

    /// LIVE recovery moves each physical object through its wall response before
    /// evaluating the surface under that object's corrected endpoint.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn source_recovery_surface(
        &mut self,
        id: ActorId,
        detached: bool,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
        mut ground: crate::handling::Ground,
    ) -> Result<crate::handling::Ground> {
        if detached {
            self.source_detached_boundary(id, actors, c)?;
        } else {
            self.source_building_contact(id, actors, c, tick, events)?;
        }
        let object = ContactObject {
            owner: id,
            detached,
        };
        let at = self.contact_location(object);
        if let Some([current, next, _, _]) = self.source_boundary_neighbors(object, at, at)?
            && current.surface.is_some()
        {
            let lateral = if detached {
                self.units[&id]
                    .recovery
                    .body
                    .as_ref()
                    .unwrap()
                    .motion
                    .lateral
            } else {
                actors[&id].view.rider.as_ref().unwrap().bike.lateral
            };
            ground.height =
                crate::building_wall::ground_height(&current, &next, lateral, at.progress as u8)?;
            let route = if detached {
                self.units[&id]
                    .recovery
                    .body
                    .as_ref()
                    .and_then(|body| body.source_route)
            } else {
                self.units[&id].source_route
            };
            ground.rise = route
                .and_then(|route| self.road_topologies.get(&route))
                .unwrap_or(&self.setup.topology)
                .grade(at);
        }
        Ok(ground)
    }

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
        let object = ContactObject {
            owner: id,
            detached: false,
        };
        let previous_at = self.units[&id].previous_location;
        let Some([current, next, previous, previous_next]) =
            self.source_boundary_neighbors(object, at, previous_at)?
        else {
            return Ok(());
        };
        let mut motion = self.contact_motion(id, actors, c);
        let profile = self.setup.rules.collision.as_ref().unwrap();
        let width = if motion.crashed {
            profile.fallen[0]
        } else {
            profile.mounted[0]
        };
        let bike = &actors[&id].view.rider.as_ref().unwrap().bike;
        let hit = crate::building_wall::resolve_surface(
            crate::building_wall::Neighbors {
                current: &current,
                next: &next,
                previous: &previous,
                previous_next: &previous_next,
            },
            &crate::building_wall::Query {
                progress: at.progress as i32,
                previous_progress: previous_at.progress as i32,
                lateral: bike.lateral,
                previous_lateral: previous_at.lateral,
                half_width: width,
                half_height: if motion.crashed {
                    profile.fallen[2]
                } else {
                    profile.mounted[2]
                },
                fallen_height: profile.fallen[2],
                suspension: self.configs.get(&id).unwrap_or(c).spec().suspension,
                height: bike.height,
                contact: bike.contact,
                category: 4,
                player: id == ActorId(1),
                owner_crashed: motion.crashed,
            },
            &mut motion,
        );
        if !hit.hit {
            return Ok(());
        }
        let unit = self.units.get_mut(&id).unwrap();
        let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
        if hit.restore_progress {
            let from = if self.setup.rules.source_roads.is_some() {
                rider.bike.distance_q8
            } else {
                self.setup
                    .topology
                    .projected(unit.recovery.bike_location)
                    .unwrap()
            };
            let to = if self.setup.rules.source_roads.is_some() {
                unit.previous.progress
            } else {
                self.setup.topology.projected(previous_at).unwrap_or(from)
            };
            self.setup
                .topology
                .advance(&mut unit.recovery.bike_location, to - from, None)?;
            rider.bike.distance_q8 += to - from;
        }
        rider.bike.speed = motion.speed;
        rider.bike.lateral_speed = hit.crash_lateral.unwrap_or(motion.lateral);
        rider.bike.lean = motion.lean;
        if self.setup.rules.body.recovery_version == 2
            && !hit.rail_crash
            && let Some(yaw) = hit.visual_heading
        {
            rider.bike.source_yaw = Some(yaw);
        }
        if hit.crash {
            let config = self.configs.get(&id).unwrap_or(c);
            let impact = Impact {
                reason: if hit.rail_crash { 3 } else { 6 },
                rider_velocity: Some(if hit.rail_crash { [0; 3] } else { [0, 0, 80] }),
                bike_velocity: Some(if hit.rail_crash { [0; 3] } else { [0, 0, 16] }),
                ..Default::default()
            };
            let entered = if self.setup.rules.body.recovery_version == 2 {
                unit.recovery.begin_live_source(
                    id,
                    &mut rider.bike,
                    config,
                    unit.recovery.bike_location,
                    impact,
                    &self.setup.topology,
                )
            } else {
                unit.recovery.begin_source(
                    id,
                    &mut rider.bike,
                    config,
                    unit.recovery.bike_location,
                    impact,
                    &self.setup.topology,
                )
            };
            if entered {
                unit.fighter.swing = None;
                Self::emit(events, tick, id, None, EffectKind::Crash);
            }
        }
        if self.setup.rules.body.recovery_version == 2
            && let Some(yaw) = hit.visual_heading
        {
            rider.bike.source_yaw = Some(yaw);
        }
        rider.bike.lateral_speed = motion.lateral;
        if let Some(body) = &mut unit.recovery.body
            && body.source_route.is_none()
        {
            body.source_route = unit.source_route;
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
        let rider = self.units[&id].recovery.body.as_ref().unwrap();
        let old = rider.previous_location.unwrap_or(at);
        let Some([current, next, previous, previous_next]) =
            self.source_boundary_neighbors(object, at, old)?
        else {
            return Ok(());
        };
        let mut body = self.proxy_body(object, actors, c);
        let mut motion = self.proxy_motion(object, actors, c);
        let hit = crate::building_wall::resolve_surface(
            crate::building_wall::Neighbors {
                current: &current,
                next: &next,
                previous: &previous,
                previous_next: &previous_next,
            },
            &crate::building_wall::Query {
                progress: at.progress as i32,
                previous_progress: old.progress as i32,
                lateral: body.position[1],
                previous_lateral: body.previous[1],
                half_width: body.half[1],
                half_height: body.half[2],
                fallen_height: 0,
                suspension: 0,
                height: body.position[2],
                contact: self.units[&id]
                    .recovery
                    .body
                    .as_ref()
                    .unwrap()
                    .motion
                    .contact,
                category: 8,
                player: id == ActorId(1),
                owner_crashed: true,
            },
            &mut motion,
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
            rider.visual_heading = Some(if self.setup.rules.body.recovery_version == 2 {
                if rider
                    .motion
                    .contact_velocity
                    .map_or(rider.motion.speed, |v| v[0])
                    >= 0
                {
                    0
                } else {
                    0x800000
                }
            } else {
                yaw
            });
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
        let old_progress = if self.setup.rules.source_roads.is_some() {
            self.units[&id].previous.progress
        } else {
            self.setup
                .topology
                .projected(previous)
                .unwrap_or(at.progress)
        };
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
        if let Some(roads) = &self.setup.rules.source_roads {
            let (index, at) = roads.advance_contact(
                unit.source_route
                    .ok_or(Error::Invalid("contact route context"))?,
                unit.recovery.bike_location,
                delta,
                body.position[1],
                &self.setup.topology,
            )?;
            unit.source_route = Some(index);
            unit.recovery.bike_location = at;
            if let Some(identity) =
                roads.profiles[usize::from(index)].identity(at.path, (at.progress >> 8) as u32)
                && identity[3] != 3
                && let Some(state) = &mut self.scenery
            {
                state
                    .object_roads
                    .insert(id.0 * 2, (identity[3] as u8).trailing_zeros() as u8);
            }
        } else {
            self.setup.topology.advance(
                &mut unit.recovery.bike_location,
                delta,
                self.navigation.get(&id).copied(),
            )?;
        }
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
        if let Some(roads) = &self.setup.rules.source_roads {
            previous = roads
                .location(
                    unit.source_route
                        .ok_or(Error::Invalid("actor route context"))?,
                    i64::from(body.previous[0]),
                    roads
                        .origin(&self.setup.topology)
                        .ok_or(Error::Invalid("source route origin"))?,
                    body.previous[1],
                    body.previous[2],
                )
                .ok_or(Error::Invalid("previous physical contact pose"))?;
        }
        unit.previous_location = previous;
        if self.setup.rules.source_roads.is_some() {
            unit.previous.progress = i64::from(body.previous[0]);
        }
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
        if !motion.crashed
            && (other.category == 4 || other.category == 8)
            && let Some(mark) = &mut actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike
                .scenery_mark
        {
            mark.frame = 0;
        }
        let mut failure = None;
        crate::collision::respond(body, other, &mut motion, |body, motion, damage| {
            let outcome = (|| -> Result<bool> {
                self.store_contact(id, actors, body, motion)?;
                let down = if damage.forced {
                    let config = self.configs.get(&id).unwrap_or(c);
                    let rider = actors.get_mut(&id).unwrap().view.rider.as_mut().unwrap();
                    let unit = self.units.get_mut(&id).unwrap();
                    unit.fighter.swing = None;
                    let impact = Impact {
                        reason: damage.reason,
                        rider_velocity: Some([0, 0, 512]),
                        bike_velocity: Some([0, 0, 256]),
                        ..Default::default()
                    };
                    let entered = if self.setup.rules.body.recovery_version == 2 {
                        unit.recovery.begin_live_source(
                            id,
                            &mut rider.bike,
                            config,
                            unit.recovery.bike_location,
                            impact,
                            &self.setup.topology,
                        )
                    } else {
                        unit.recovery.begin_source(
                            id,
                            &mut rider.bike,
                            config,
                            unit.recovery.bike_location,
                            impact,
                            &self.setup.topology,
                        )
                    };
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
    pub(crate) fn contact_location(&self, object: ContactObject) -> Location {
        let recovery = &self.units[&object.owner].recovery;
        if object.detached {
            recovery
                .body
                .as_ref()
                .map_or(recovery.bike_location, |body| body.location)
        } else {
            recovery.bike_location
        }
    }
    pub(crate) fn proxy_body(
        &self,
        object: ContactObject,
        actors: &BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
    ) -> crate::collision::Body {
        if !object.detached || self.units[&object.owner].recovery.body.is_none() {
            // The native list contains dormant half nodes before crash entry.
            // Only their creation-order/position key is read while inactive.
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
        let progress = if self.setup.rules.source_roads.is_some() {
            rider.motion.distance_q8
        } else {
            self.setup
                .topology
                .projected(rider.location)
                .unwrap_or(rider.motion.distance_q8)
        };
        let velocity =
            crate::recovery::actor_velocity(rider.motion.speed, rider.location.heading, &c.tables);
        crate::collision::Body {
            position: [progress as i32, rider.motion.lateral, rider.motion.height],
            previous: [
                if let Some(roads) = &self.setup.rules.source_roads {
                    let profile = &roads.profiles[usize::from(
                        rider
                            .source_route
                            .or(self.units[&object.owner].source_route)
                            .unwrap(),
                    )];
                    profile
                        .selected
                        .get(&previous.path)
                        .map_or(progress, |start| {
                            i64::from(*start) * 256 - i64::from(c.start_sample) * 256
                                + previous.progress
                        }) as i32
                } else {
                    self.setup.topology.projected(previous).unwrap_or(progress) as i32
                },
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
        let route = self.units[&object.owner]
            .recovery
            .body
            .as_ref()
            .and_then(|b| b.source_route)
            .or(self.units[&object.owner].source_route);
        let rider = self
            .units
            .get_mut(&object.owner)
            .unwrap()
            .recovery
            .body
            .as_mut()
            .unwrap();
        let from = if self.setup.rules.source_roads.is_some() {
            rider.motion.distance_q8
        } else {
            self.setup
                .topology
                .projected(rider.location)
                .unwrap_or(rider.motion.distance_q8)
        };
        let delta = i64::from(body.position[0]) - from;
        if let Some(roads) = &self.setup.rules.source_roads {
            let (index, at) = roads.advance_contact(
                route.ok_or(Error::Invalid("body contact route context"))?,
                rider.location,
                delta,
                body.position[1],
                &self.setup.topology,
            )?;
            rider.source_route = Some(index);
            rider.location = at;
            if let Some(identity) =
                roads.profiles[usize::from(index)].identity(at.path, (at.progress >> 8) as u32)
                && identity[3] != 3
                && let Some(state) = &mut self.scenery
            {
                state
                    .object_roads
                    .insert(object.key(), (identity[3] as u8).trailing_zeros() as u8);
            }
        } else {
            self.setup
                .topology
                .advance(&mut rider.location, delta, None)?;
        }
        rider.motion.distance_q8 += delta;
        let mut previous = rider.location;
        self.setup.topology.advance(
            &mut previous,
            i64::from(body.previous[0]) - i64::from(body.position[0]),
            None,
        )?;
        previous.lateral = body.previous[1];
        previous.height = body.previous[2];
        if let Some(roads) = &self.setup.rules.source_roads {
            previous = roads
                .location(
                    rider
                        .source_route
                        .or(route)
                        .ok_or(Error::Invalid("body route context"))?,
                    i64::from(body.previous[0]),
                    roads
                        .origin(&self.setup.topology)
                        .ok_or(Error::Invalid("source route origin"))?,
                    body.previous[1],
                    body.previous[2],
                )
                .ok_or(Error::Invalid("previous detached physical contact pose"))?;
        }
        rider.previous_location = Some(previous);
        rider.motion.lateral = body.position[1];
        rider.motion.height = body.position[2];
        rider.motion.speed = motion.speed;
        rider.motion.lateral_speed = motion.lateral;
        rider.motion.vertical_speed = motion.vertical;
        rider.motion.lean = motion.lean;
        rider.location.lateral = rider.motion.lateral;
        rider.location.height = rider.motion.height;
        rider.projected = if self.setup.rules.source_roads.is_some() {
            Some(rider.motion.distance_q8)
        } else {
            self.setup.topology.projected(rider.location)
        };
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
        mut random: impl FnMut() -> u32,
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
            self.source_contact_sound_draws(object.owner, actors, body, other, &mut random);
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
        // The native half handler draws a human grunt variant before its
        // randomized nudge. The inner response always caps vertical speed
        // below the nudge gate when this sound branch runs.
        let mut sound = self.setup.rules.combat_version >= 4
            && object.owner == ActorId(1)
            && (control.state != 2 || other.speed >= 1700)
            && {
                let mut speed = motion.speed - motion.speed / 4;
                if control.state == 2 && other.speed > 1700 {
                    speed += other.speed / 4;
                }
                speed >= 2001
            };
        let before_nudge = body.position[0];
        crate::collision::respond_half(body, other, &mut motion, &mut control, quiet, || {
            if sound {
                sound = false;
                random();
            }
            random()
        });
        self.store_proxy(object, actors, body, &motion)?;
        if self.setup.rules.body.recovery_version == 2
            && !self.setup.rules.body.source_angles.is_empty()
        {
            let config = self
                .body_configs
                .get(&object.owner)
                .map(|(_, config)| config)
                .unwrap_or_else(|| self.configs.get(&object.owner).unwrap_or(c));
            let turn = crate::recovery::curve_sweep(
                config,
                i64::from(before_nudge),
                i64::from(body.position[0]),
                &self.setup.rules.body.source_angles,
            );
            let rider = self
                .units
                .get_mut(&object.owner)
                .unwrap()
                .recovery
                .body
                .as_mut()
                .unwrap();
            rider.location.heading = rider.location.heading.wrapping_sub(turn);
        }
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
        if (a.detached && self.units[&a.owner].recovery.body.is_none())
            || (b.detached && self.units[&b.owner].recovery.body.is_none())
        {
            return Ok(false);
        }
        let al = self.contact_location(a);
        let bl = self.contact_location(b);
        let shared = if self.setup.rules.source_roads.is_some() {
            let am = self
                .road_identity_object(a, al.path, (al.progress >> 8) as u32)
                .map(|i| i[3]);
            let bm = self
                .road_identity_object(b, bl.path, (bl.progress >> 8) as u32)
                .map(|i| i[3]);
            (al.path == bl.path
                || self.setup.topology.paths[&al.path]
                    .portals
                    .iter()
                    .any(|p| p.target == bl.path))
                && (am == Some(3) || bm == Some(3) || am == bm)
        } else {
            self.setup.topology.connected(al.path, bl.path)
                && self.setup.topology.paths[&al.path].branch
                    == self.setup.topology.paths[&bl.path].branch
        };
        if !shared {
            return Ok(false);
        }
        let mut first = self.proxy_body(a, actors, c);
        let mut second = self.proxy_body(b, actors, c);
        // Unequal branch lengths cannot share one global longitudinal axis.
        // Bridge each physical pair into the outer object's road coordinate.
        let bridge = if self.setup.rules.source_roads.is_some() {
            let gap = if al.path == bl.path {
                bl.progress - al.progress
            } else {
                let portal = self.setup.topology.paths[&al.path]
                    .portals
                    .iter()
                    .find(|p| p.target == bl.path)
                    .unwrap();
                if portal.end {
                    self.setup.topology.paths[&al.path].length - al.progress + bl.progress
                } else {
                    -al.progress - (self.setup.topology.paths[&bl.path].length - bl.progress)
                }
            };
            (i64::from(first.position[0]) + gap - i64::from(second.position[0])) as i32
        } else {
            0
        };
        second.position[0] += bridge;
        second.previous[0] += bridge;
        if !first.overlaps(&second) {
            return Ok(false);
        }
        if a.owner != b.owner {
            crate::collision::separate(&mut first, &mut second);
        }
        let ma = self.proxy_motion(a, actors, c);
        let mb = self.proxy_motion(b, actors, c);
        self.store_proxy(a, actors, &first, &ma)?;
        let mut stored_second = second;
        stored_second.position[0] -= bridge;
        stored_second.previous[0] -= bridge;
        self.store_proxy(b, actors, &stored_second, &mb)?;
        self.respond_proxy(
            (a, Some(b)),
            actors,
            (&mut first, &second),
            c,
            (context.0, context.1),
            &mut random,
        )?;
        second.position[0] -= bridge;
        second.previous[0] -= bridge;
        let mut other = first;
        other.position[0] -= bridge;
        other.previous[0] -= bridge;
        self.respond_proxy(
            (b, Some(a)),
            actors,
            (&mut second, &other),
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
    /// Sound variants share the native gameplay stream even when effects are muted.
    fn source_contact_sound_draws(
        &self,
        id: ActorId,
        actors: &BTreeMap<ActorId, crate::Actor>,
        body: &crate::collision::Body,
        other: &crate::collision::Body,
        random: &mut impl FnMut() -> u32,
    ) {
        if self.setup.rules.combat_version < 4
            || id != ActorId(1)
            || (other.category != 16 && actors[&id].view.rider.as_ref().unwrap().bike.crashed)
        {
            return;
        }
        if body.impulse[0].unsigned_abs() >= 2001 || body.impulse[1].unsigned_abs() >= 1001 {
            random();
            random();
        }
    }
    #[cfg(test)]
    pub(crate) fn source_object_contact(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        other: &mut crate::collision::Body,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<bool> {
        self.source_object_contact_with_random(id, actors, other, c, tick, events, || 0)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn source_object_contact_with_random(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        other: &mut crate::collision::Body,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
        mut random: impl FnMut() -> u32,
    ) -> Result<bool> {
        let mut body = self.collision_body(id, actors);
        if other.category != 1 && !body.overlaps(other) {
            return Ok(false);
        }
        let rider_outer = body.position[0] <= other.position[0];
        if other.category == 1 || rider_outer {
            crate::collision::separate(&mut body, other);
        } else {
            crate::collision::separate(other, &mut body);
        }
        let motion = self.contact_motion(id, actors, c);
        self.store_contact(id, actors, &body, &motion)?;
        let horn_draw =
            self.setup.rules.combat_version >= 4 && id == ActorId(1) && other.category == 16;
        if horn_draw && !rider_outer {
            random();
        }
        self.source_contact_sound_draws(id, actors, &body, other, &mut random);
        self.respond_contact(id, actors, (&mut body, other), c, tick, events)?;
        if horn_draw && rider_outer {
            random();
        }
        Ok(true)
    }
    #[cfg(test)]
    pub(crate) fn source_scenery_bike_contact(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        events: &mut Vec<Effect>,
    ) -> Result<()> {
        let mut body = self.collision_body(id, actors);
        let other = body;
        self.respond_contact(id, actors, (&mut body, &other), c, tick, events)
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

#[cfg(test)]
#[path = "recovery_rail_tests.rs"]
mod recovery_rail_tests;
