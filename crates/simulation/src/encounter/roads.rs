use super::*;
impl World {
    pub(crate) fn prepare_source_roads(
        &mut self,
        actors: &BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
    ) -> Result<()> {
        self.prepare_source_roads_selected(actors, c, tick, None)
    }
    pub(crate) fn prepare_source_roads_selected(
        &mut self,
        actors: &BTreeMap<ActorId, crate::Actor>,
        c: &Configuration,
        tick: u64,
        only: Option<ActorId>,
    ) -> Result<()> {
        let Some(roads) = &self.setup.rules.source_roads else {
            return Ok(());
        };
        for (&id, unit) in &mut self.units {
            if only.is_some_and(|selected| selected != id) {
                continue;
            }
            let mut index = unit
                .source_route
                .ok_or(Error::Invalid("missing actor source route"))?;
            let profile = roads
                .profiles
                .get(usize::from(index))
                .ok_or(Error::Invalid("actor source route"))?;
            let at = unit.recovery.bike_location;
            let rider = actors[&id].view.rider.as_ref().unwrap();
            let bike = &rider.bike;
            if !profile.selected.contains_key(&at.path) {
                return Err(Error::Invalid("actor outside physical route"));
            }
            if self.road_configs.get(&id) != Some(&index) {
                let mut config = self.configs.get(&id).unwrap_or(c).clone();
                config.ground = roads.ground(index)?;
                config.validate()?;
                self.configs.insert(id, config);
                self.road_configs.insert(id, index);
            }
            let config = &self.configs[&id];
            if !self.road_topologies.contains_key(&index) {
                self.road_topologies
                    .insert(index, roads.topology(index, &self.setup.topology)?);
            }
            let topology = &self.road_topologies[&index];
            if bike.speed > 0
                && at.progress >= self.setup.topology.paths[&at.path].length - 256
                && roads.forks.contains_key(&at.path)
            {
                let crosses = if unit.recovery.phase == Phase::Mounted {
                    let mut predicted = rider.clone();
                    let mut command = actors[&id].input;
                    if unit.drive {
                        command.throttle = true;
                    }
                    if only.is_some() {
                        predicted.source_physics_motion(config);
                    } else {
                        predicted.step_with_combat_profile(
                            config,
                            command,
                            tick as u32,
                            self.setup.rules.combat_version,
                        )?;
                    }
                    predicted.bike.distance_q8 - bike.distance_q8
                        >= self.setup.topology.paths[&at.path].length - at.progress
                } else {
                    let mut motion = bike.clone();
                    let mut location = at;
                    crate::recovery::motion_step_bike(
                        &mut motion,
                        &mut location,
                        config,
                        topology,
                        &self.setup.rules.body,
                    )?;
                    location.path != at.path
                };
                if crosses {
                    let (chosen, portal) =
                        roads.fork(index, at, bike.lateral, self.navigation.get(&id).copied())?;
                    index = chosen;
                    if let Some(portal) = portal {
                        self.navigation.insert(id, portal);
                    }
                    unit.source_route = Some(index);
                }
            }
            if self.road_configs.get(&id) != Some(&index) {
                let mut config = self.configs[&id].clone();
                config.ground = roads.ground(index)?;
                config.validate()?;
                self.configs.insert(id, config);
                self.road_configs.insert(id, index);
            }
            if !self.road_topologies.contains_key(&index) {
                self.road_topologies
                    .insert(index, roads.topology(index, &self.setup.topology)?);
            }
            if let Some(body) = &mut unit.recovery.body {
                let mut body_index = *body.source_route.get_or_insert(index);
                let profile = roads
                    .profiles
                    .get(usize::from(body_index))
                    .ok_or(Error::Invalid("body source route"))?;
                if !profile.selected.contains_key(&body.location.path) {
                    return Err(Error::Invalid("body outside physical route"));
                }
                if self
                    .body_configs
                    .get(&id)
                    .is_none_or(|(old, _)| *old != body_index)
                {
                    let mut config = self.configs[&id].clone();
                    config.ground = roads.ground(body_index)?;
                    self.body_configs.insert(id, (body_index, config));
                }
                let config = &self.body_configs[&id].1;
                if !self.road_topologies.contains_key(&body_index) {
                    self.road_topologies.insert(
                        body_index,
                        roads.topology(body_index, &self.setup.topology)?,
                    );
                }
                let topology = &self.road_topologies[&body_index];
                if body.motion.speed > 0
                    && body.location.progress >= topology.paths[&body.location.path].length - 256
                    && roads.forks.contains_key(&body.location.path)
                {
                    let mut motion = body.motion.clone();
                    let mut location = body.location;
                    let planned = if body.animation.code == 38 {
                        topology.leg(location.path, at.path)
                    } else {
                        None
                    };
                    crate::recovery::motion_step(
                        &mut motion,
                        &mut location,
                        body.location.heading,
                        config,
                        topology,
                        &self.setup.rules.body,
                        planned,
                    )?;
                    if location.path != body.location.path {
                        let (chosen, _) =
                            roads.fork(body_index, body.location, body.motion.lateral, planned)?;
                        body_index = chosen;
                        body.source_route = Some(chosen);
                    }
                }
                if self
                    .body_configs
                    .get(&id)
                    .is_none_or(|(old, _)| *old != body_index)
                {
                    let mut config = self.configs[&id].clone();
                    config.ground = roads.ground(body_index)?;
                    self.body_configs.insert(id, (body_index, config));
                }
                if !self.road_topologies.contains_key(&body_index) {
                    self.road_topologies.insert(
                        body_index,
                        roads.topology(body_index, &self.setup.topology)?,
                    );
                }
            } else {
                self.body_configs.remove(&id);
            }
        }
        Ok(())
    }
    pub(super) fn sync_body_routes(&mut self) {
        if self.setup.rules.source_roads.is_some() {
            for unit in self.units.values_mut() {
                if let Some(body) = &mut unit.recovery.body {
                    body.source_route = body.source_route.or(unit.source_route);
                }
            }
        }
    }
    pub(crate) fn road_identity_object(
        &self,
        object: ContactObject,
        path: u32,
        local: u32,
    ) -> Option<[i32; 4]> {
        let roads = self.setup.rules.source_roads.as_ref()?;
        let unit = self.units.get(&object.owner)?;
        let route = if object.detached {
            unit.recovery
                .body
                .as_ref()?
                .source_route
                .or(unit.source_route)
        } else {
            unit.source_route
        };
        roads
            .profiles
            .get(usize::from(route?))?
            .identity(path, local)
    }
}

#[cfg(test)]
mod incident_tests {
    use super::*;
    fn fixture() -> (Configuration, crate::Simulation) {
        let c = crate::fixtures::riding();
        let mut setup = Setup::authored(&c).unwrap();
        setup.topology = Topology::straight(100000);
        setup.rules.combat_version = 3;
        setup.rules.contact_walk_version = 1;
        setup.rules.collision_version = 2;
        setup.rules.collision = Some(crate::collision::Profile {
            mounted: [10, 4, 10],
            fallen: [10, 4, 4],
            mass: 256,
            elasticity: 192,
            rider: None,
        });
        setup.rules.dimensions = [10, 4, 10];
        setup.rules.obstacles = vec![];
        for (id, progress, left, right) in [
            (1, 950, 100, 200),
            (2, 1050, 100, 200),
            (3, 5050, 100, 200),
            (4, 1050, -200, -100),
        ] {
            setup.rules.obstacles.push(Obstacle {
                id,
                path: 0,
                progress,
                half_length: 50,
                left,
                right,
                height: 10000,
                kind: ObstacleKind::Building,
                source_box: None,
            });
        }
        setup.seeds[0].distance = 980;
        setup.seeds[0].speed = 1000;
        setup.seeds[0].lateral = 100;
        let sim = crate::Simulation::with_encounters(1, c.clone(), setup).unwrap();
        (c, sim)
    }
    fn scrape(
        sim: &mut crate::Simulation,
        c: &Configuration,
        tick: u64,
        progress: i64,
        lateral: i32,
    ) -> usize {
        let id = ActorId(1);
        let bike = &mut sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.distance_q8 = progress;
        bike.lateral = lateral;
        bike.speed = 1000;
        bike.lateral_speed = 2048;
        let u = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
        u.recovery.bike_location = Location {
            progress,
            lateral,
            ..Default::default()
        };
        u.previous_location = Location {
            progress: progress - 5,
            lateral: lateral - 20,
            ..Default::default()
        };
        sim.effects.clear();
        sim.encounter
            .as_mut()
            .unwrap()
            .contacts(&mut sim.actors, c, tick, &mut sim.effects, || 0)
            .unwrap();
        sim.effects
            .iter()
            .filter(|e| e.kind == EffectKind::Contact)
            .count()
    }
    #[test]
    fn adjacent_facade_panels_share_an_incident_but_distant_facades_do_not() {
        let (c, mut sim) = fixture();
        assert_eq!(scrape(&mut sim, &c, 0, 980, 100), 1);
        let condition = sim.actors[&ActorId(1)]
            .view
            .rider
            .as_ref()
            .unwrap()
            .bike
            .condition;
        assert_eq!(scrape(&mut sim, &c, 1, 1020, 100), 0);
        assert_eq!(
            sim.actors[&ActorId(1)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .bike
                .condition,
            condition
        );
        assert_eq!(
            scrape(&mut sim, &c, 2, 5020, 100),
            1,
            "an unrelated panel must not inherit an active scrape merely because it is on the same path"
        );
        assert_eq!(scrape(&mut sim, &c, 3, 5020, 0), 0);
        assert_eq!(
            scrape(&mut sim, &c, 4, 5020, 100),
            1,
            "separation then re-impact creates a new incident"
        );
    }
    #[test]
    fn simultaneous_contacts_keep_independent_incident_keys() {
        let (c, mut sim) = fixture();
        sim.encounter.as_mut().unwrap().setup.rules.dimensions[0] = 150;
        assert_eq!(scrape(&mut sim, &c, 0, 1020, 0), 2);
        let active = &sim.encounter.as_ref().unwrap().active_contacts;
        assert_eq!(active.len(), 2);
        assert!(active.contains(&(2, (1 << 63) | 1)));
        assert!(active.contains(&(2, (1 << 63) | 4)));
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn serialized_route_state_rebuilds_configs_and_topologies_deterministically() {
        let mut c = crate::fixtures::riding();
        c.motion_version = 3;
        let mut setup = Setup::authored(&c).unwrap();
        setup.rules.combat_version = 3;
        setup.rules.collision_version = 2;
        setup.rules.contact_walk_version = 1;
        setup.rules.scenery_version = 1;
        setup.rules.obstacles.clear();
        setup.rules.collision = Some(crate::collision::Profile {
            mounted: [10, 4, 10],
            fallen: [10, 4, 4],
            mass: 256,
            elasticity: 192,
            rider: None,
        });
        setup.rules.source_roads = Some(crate::source_roads::Roads {
            version: 1,
            initial: 0,
            lanes_q8: None,
            ground: BTreeMap::from([(0, c.ground.clone())]),
            forks: BTreeMap::new(),
            profiles: vec![crate::source_roads::Profile {
                finish_progress: None,
                choices: vec![],
                selected: BTreeMap::from([(0, 0)]),
                identities: BTreeMap::from([(
                    0,
                    (0..128).map(|local| [local, local, 0, 3]).collect(),
                )]),
            }],
        });
        let mut restored = crate::Simulation::with_encounters(1, c.clone(), setup.clone()).unwrap();
        let mut original = crate::Simulation::with_encounters(1, c, setup).unwrap();
        for _ in 0..4 {
            original.step(crate::fixtures::gas()).unwrap();
            restored.step(crate::fixtures::gas()).unwrap();
        }
        assert!(!original.encounter.as_ref().unwrap().road_configs.is_empty());
        let bytes = serde_json::to_vec(original.encounter.as_ref().unwrap()).unwrap();
        let decoded: World = serde_json::from_slice(&bytes).unwrap();
        assert!(decoded.configs.is_empty());
        assert!(decoded.road_configs.is_empty());
        assert!(decoded.road_topologies.is_empty());
        assert!(decoded.body_configs.is_empty());
        restored.encounter = Some(decoded);
        for _ in 0..10 {
            original.advance().unwrap();
            restored.advance().unwrap();
            assert_eq!(original.state_hash(), restored.state_hash());
        }
        restored
            .encounter
            .as_mut()
            .unwrap()
            .units
            .get_mut(&ActorId(1))
            .unwrap()
            .source_route = Some(255);
        assert!(restored.advance().is_err());
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    #[test]
    fn mounted_actor_three_and_actor_ones_body_have_distinct_static_incidents() {
        for version in [0, 1] {
            let c = crate::fixtures::riding();
            let mut s = Setup::authored(&c).unwrap();
            s.topology = Topology::straight(100000);
            s.rules.combat_version = 3;
            s.rules.contact_walk_version = version;
            s.rules.collision_version = 2;
            s.rules.collision = Some(crate::collision::Profile {
                mounted: [10, 4, 10],
                fallen: [10, 4, 4],
                mass: 256,
                elasticity: 192,
                rider: Some(crate::collision::RiderProfile {
                    dimensions: [10, 4, 10],
                    mass: 256,
                    elasticity: 192,
                }),
            });
            s.seeds[0].distance = 50;
            for (id, distance) in [(2, 15000), (3, 2688)] {
                let mut seed = Seed::player();
                seed.id = ActorId(id);
                seed.distance = distance;
                s.seeds.push(seed);
            }
            s.rules.obstacles = vec![Obstacle {
                id: 1,
                path: 0,
                progress: 2688,
                half_length: 1,
                left: -1024,
                right: 1024,
                height: 40000,
                kind: ObstacleKind::Solid,
                source_box: Some(StaticBox {
                    bounds: [-1024, 0, 1024, 40000],
                    lateral: 0,
                    query_bounds: None,
                    scenery: None,
                }),
            }];
            let mut sim = crate::Simulation::with_encounters(1, c.clone(), s).unwrap();
            let world = sim.encounter.as_mut().unwrap();
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
            unit.recovery.begin_source(
                ActorId(1),
                bike,
                &c,
                unit.recovery.bike_location,
                Impact::default(),
                &world.setup.topology,
            );
            let body = unit.recovery.body.as_mut().unwrap();
            body.location.progress = 2688;
            body.location.lateral = 0;
            body.motion.distance_q8 = 2688;
            body.motion.lateral = 0;
            body.motion.speed = 0;
            body.motion.lateral_speed = 0;
            body.motion.contact_velocity = Some([0; 3]);
            world
                .contacts(&mut sim.actors, &c, 0, &mut sim.effects, || 0)
                .unwrap();
            let keys = world
                .active_contacts
                .iter()
                .filter(|(_, b)| *b == (1 << 63) | 1)
                .copied()
                .collect::<BTreeSet<_>>();
            assert_eq!(keys.len(), if version == 0 { 1 } else { 2 });
            if version == 1 {
                assert!(keys.contains(&(3, (1 << 63) | 1)));
                assert!(keys.contains(&(6, (1 << 63) | 1)));
            }
        }
    }
}
