use super::*;
impl World {
    pub(super) fn scenery_begin(
        &mut self,
        _actors: &BTreeMap<ActorId, crate::Actor>,
        _c: &Configuration,
        _tick: u64,
    ) -> Result<()> {
        let Some(state) = &mut self.scenery else {
            return Ok(());
        };
        let roads = self.setup.rules.source_roads.as_ref();
        let route = self.units.get(&ActorId(1)).and_then(|u| {
            u.recovery
                .body
                .as_ref()
                .and_then(|b| b.source_route)
                .or(u.source_route)
        });
        let camera_profile = roads.and_then(|r| route.and_then(|i| r.profiles.get(usize::from(i))));
        if let (Some(roads), Some(previous), Some(current)) = (roads, state.source_route, route)
            && previous != current
        {
            let before = roads
                .profiles
                .get(usize::from(previous))
                .ok_or(Error::Invalid("scenery route context"))?;
            let after = camera_profile.ok_or(Error::Invalid("scenery route context"))?;
            let same = |key: &(u32, u32)| {
                before
                    .identity(key.0, key.1)
                    .is_some_and(|old| Some(old) == after.identity(key.0, key.1))
            };
            state.loaded.retain(same);
            if let Some(resident) = &mut state.resident {
                resident.retain(same);
            }
        }
        state.source_route = route;
        state.object_roads.retain(|key, _| {
            self.units
                .get(&ActorId(key / 2))
                .is_some_and(|unit| key % 2 == 0 || unit.recovery.body.is_some())
        });
        if state.object_roads.len() > self.units.len() * 2
            || state.object_roads.values().any(|road| *road > 1)
            || state.camera_back < 0
            || state.camera_back > 900
        {
            return Err(Error::Invalid("source scenery actor cache"));
        }
        for (id, unit) in &self.units {
            for detached in [false, true] {
                let at = if detached {
                    let Some(body) = &unit.recovery.body else {
                        continue;
                    };
                    body.location
                } else {
                    unit.recovery.bike_location
                };
                let key = (at.path, (at.progress >> 8) as u32);
                let index = self
                    .setup
                    .rules
                    .scenery_roads
                    .partition_point(|r| (r.path, r.first) <= key);
                let inherited = state.object_roads.get(&(id.0 * 2)).copied().unwrap_or(0);
                let road = state
                    .object_roads
                    .entry(id.0 * 2 + u64::from(detached))
                    .or_insert(inherited);
                if let Some(profile) = roads.and_then(|r| {
                    (if detached {
                        unit.recovery
                            .body
                            .as_ref()
                            .and_then(|b| b.source_route)
                            .or(unit.source_route)
                    } else {
                        unit.source_route
                    })
                    .and_then(|i| r.profiles.get(usize::from(i)))
                }) {
                    if let Some(identity) = profile.identity(key.0, key.1)
                        && identity[3] != 3
                    {
                        *road = (identity[3] as u8).trailing_zeros() as u8;
                    }
                } else if index > 0 {
                    let span = &self.setup.rules.scenery_roads[index - 1];
                    if span.path == at.path && key.1 <= span.last && span.road_mask != 3 {
                        *road = span.road_mask.trailing_zeros() as u8;
                    }
                }
            }
        }
        if state.slot_flags.len() > 2
            || state.slot_flags.keys().any(|road| *road > 1)
            || state.slot_flags.values().any(|s| s.len() != 128)
            || state.loaded.len() > self.setup.rules.obstacles.len()
            || state
                .resident
                .as_ref()
                .is_some_and(|s| s.len() > self.setup.rules.obstacles.len())
        {
            return Err(Error::Invalid("source scenery state budget"));
        }
        if state
            .pending_camera
            .is_some_and(|p| !self.setup.topology.valid(p))
        {
            return Err(Error::Invalid("source scenery camera"));
        }
        if !self.scenery_index_ready {
            for (index, o) in self
                .setup
                .rules
                .obstacles
                .iter()
                .enumerate()
                .filter(|(_, o)| o.source_box.as_ref().is_some_and(|b| b.scenery.is_some()))
            {
                self.scenery_index
                    .entry((o.path, (o.progress >> 8) as u32))
                    .or_default()
                    .push(index);
            }
            self.scenery_index_ready = true;
        }
        if state
            .loaded
            .iter()
            .chain(state.resident.iter().flatten())
            .any(|key| !self.scenery_index.contains_key(key))
            || state
                .frame_follow
                .is_some_and(|p| !self.setup.topology.valid(p))
        {
            return Err(Error::Invalid("source scenery physical cache"));
        }
        let added = state.apply_pending(&self.setup.topology, &self.scenery_index);
        let mut flag_updates = vec![];
        for key in added {
            for index in &self.scenery_index[&key] {
                let meta = self.setup.rules.obstacles[*index]
                    .source_box
                    .as_ref()
                    .unwrap()
                    .scenery
                    .as_ref()
                    .unwrap();
                if meta.class == 16
                    && let Some(flags) = meta.texture_flags
                {
                    let (sample, mask) = if let Some(profile) = camera_profile {
                        let Some(i) = profile.identity(key.0, key.1) else {
                            continue;
                        };
                        (i[1] as u32, i[3] as u8)
                    } else {
                        (meta.sample, meta.road_mask)
                    };
                    for road in 0..2 {
                        if mask & (1 << road) != 0 {
                            flag_updates.push((road, sample, flags));
                        }
                    }
                }
            }
        }
        flag_updates.sort();
        for (road, sample, flags) in flag_updates {
            state.slot_flags.entry(road).or_insert_with(|| vec![0; 128])[(sample & 127) as usize] =
                flags;
        }
        Ok(())
    }

    pub(crate) fn scenery_prepare(&mut self, finalise: bool) -> Result<()> {
        let Some(state) = &mut self.scenery else {
            return Ok(());
        };
        if let Some(unit) = self.units.get(&ActorId(1)) {
            let back = if unit.recovery.outcome.is_some() {
                (state.camera_back + 3).min(900)
            } else {
                (state.camera_back - 3).max(0)
            };
            let follow = unit
                .recovery
                .body
                .as_ref()
                .map(|b| b.location)
                .or(state.frame_follow)
                .unwrap_or(unit.recovery.bike_location);
            let mut camera = follow;
            let route = unit
                .recovery
                .body
                .as_ref()
                .and_then(|body| body.source_route)
                .or(unit.source_route);
            let topology = if self.setup.rules.contact_walk_version >= 2 {
                route
                    .and_then(|route| self.road_topologies.get(&route))
                    .unwrap_or(&self.setup.topology)
            } else {
                &self.setup.topology
            };
            topology.advance(&mut camera, -i64::from(back), None)?;
            state.pending_camera = Some(camera);
            state.frame_follow = Some(follow);
            if finalise {
                let resident = crate::scenery::State::physical_window(
                    &self.setup.topology,
                    camera,
                    0x4f00,
                    0x3000,
                    &self.scenery_index,
                );
                if let Some(previous) = &state.resident {
                    for rebuilt in resident.difference(previous) {
                        state.loaded.remove(rebuilt);
                    }
                }
                state.resident = Some(resident);
                state.camera_back = back;
                state.frame_follow = None;
            }
        }
        Ok(())
    }
    pub(super) fn scenery_obstacle(
        &self,
        o: &Obstacle,
        _c: &Configuration,
        object: ContactObject,
    ) -> Option<Obstacle> {
        let identity = if self.setup.rules.source_roads.is_some() {
            Some(self.road_identity_object(object, o.path, (o.progress >> 8) as u32)?)
        } else {
            None
        };
        let Some(_meta) = o.source_box.as_ref().and_then(|b| b.scenery.as_ref()) else {
            let mut result = o.clone();
            if let Some(i) = identity {
                result.left += i[2] * 256;
                result.right += i[2] * 256;
            }
            return Some(result);
        };
        let state = self.scenery.as_ref()?;
        if !state.loaded.contains(&(o.path, (o.progress >> 8) as u32)) {
            return None;
        }
        let mut result = o.clone();
        let b = result.source_box.as_mut().unwrap();
        if let Some(i) = identity {
            let offset = i[2] * 256;
            result.left += offset;
            result.right += offset;
            b.lateral += offset;
            for axis in [0, 2] {
                b.bounds[axis] += offset;
                if let Some(bounds) = &mut b.query_bounds {
                    bounds[axis] += offset;
                }
            }
            let m = b.scenery.as_mut().unwrap();
            m.sample = i[1] as u32;
            m.road_mask = i[3] as u8;
        }
        let meta = b.scenery.as_ref().unwrap();
        if meta.class == 16 && meta.box_count == 1 && (b.lateral / 256).unsigned_abs() >= 0x400 {
            let road = if self.setup.rules.source_roads.is_some() && meta.road_mask != 3 {
                u32::from(meta.road_mask.trailing_zeros() as u8)
            } else {
                u32::from(
                    state
                        .object_roads
                        .get(&object.key())
                        .copied()
                        .or_else(|| state.object_roads.get(&(object.owner.0 * 2)).copied())
                        .unwrap_or(0),
                )
            };
            let flags = state
                .slot_flags
                .get(&road)
                .map_or(0, |v| v[(meta.sample & 127) as usize]);
            if flags & 0x4000 == 0 {
                for axis in [0, 2] {
                    b.bounds[axis] = (b.bounds[axis] - b.lateral) * 3 + b.lateral;
                }
                if let Some(bounds) = &mut b.query_bounds {
                    for axis in [0, 2] {
                        bounds[axis] = (bounds[axis] - b.lateral) * 3 + b.lateral;
                    }
                }
            }
        }
        Some(result)
    }
    pub(super) fn scenery_mark(
        &mut self,
        id: ActorId,
        actors: &mut BTreeMap<ActorId, crate::Actor>,
        o: &Obstacle,
        _c: &Configuration,
        tick: u64,
    ) {
        let meta = o.source_box.as_ref().unwrap().scenery.as_ref().unwrap();
        let sample = i64::from(meta.sample);
        let bike = &mut actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        if !bike.crashed && bike.scenery_mark.as_ref().map_or(0, |m| m.sample) != sample {
            if meta.class < 12 {
                bike.scenery_timer = (1 + i32::from(meta.class - 3) % 3) * 60;
            }
            if bike.speed > 1000 {
                bike.scenery_mark = Some(crate::scenery::Mark {
                    sample,
                    selector: meta.selector,
                    tick,
                    frame: 1,
                    display_frame: 1,
                });
            }
            bike.lateral_speed = 0;
            bike.lean = bike.lean.wrapping_add(bike.lean / 2);
        }
        bike.speed = bike.speed.clamp(-1000, 30000);
        bike.vertical_speed = bike.vertical_speed.clamp(-10000, 1536);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remount_discards_detached_road_history_before_a_new_crash() {
        let mut c = crate::fixtures::riding();
        c.motion_version = 2;
        let mut setup = Setup::authored(&c).unwrap();
        setup.rules.combat_version = 3;
        setup.rules.collision_version = 2;
        setup.rules.scenery_version = 1;
        setup.rules.collision = Some(crate::collision::Profile {
            mounted: [7680, 64, 8960],
            fallen: [7680, 64, 2048],
            mass: 256,
            elasticity: 192,
            rider: None,
        });
        let mut sim = crate::Simulation::with_encounters(1, c.clone(), setup).unwrap();
        let world = sim.encounter.as_mut().unwrap();
        let state = world.scenery.as_mut().unwrap();
        state.object_roads.insert(2, 1);
        state.object_roads.insert(3, 0); // old crash, before remount
        world.scenery_begin(&sim.actors, &c, 0).unwrap();
        assert_eq!(world.scenery.as_ref().unwrap().object_roads.get(&3), None);
        let unit = world.units.get_mut(&ActorId(1)).unwrap();
        let bike = &mut sim
            .actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        unit.recovery.body = Some(crate::recovery::Body {
            source_route: None,
            entity: 3,
            owner: ActorId(1),
            location: unit.recovery.bike_location,
            previous_location: None,
            visual_heading: None,
            motion: bike.clone(),
            animation: Default::default(),
            frame: 2,
            mirror: false,
            view_flags: 0,
            roll: 0,
            projected: Some(bike.distance_q8),
            branch: 0,
        });
        world.scenery_begin(&sim.actors, &c, 1).unwrap();
        assert_eq!(
            world.scenery.as_ref().unwrap().object_roads.get(&3),
            Some(&1)
        );
        world.scenery.as_mut().unwrap().object_roads.insert(3, 0);
        world.scenery_begin(&sim.actors, &c, 2).unwrap();
        assert_eq!(
            world.scenery.as_ref().unwrap().object_roads.get(&3),
            Some(&0)
        );
    }
}
