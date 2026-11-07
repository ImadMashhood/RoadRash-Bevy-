use super::*;
#[cfg(test)]
mod source4_tests {
    use super::*;
    fn field() -> (Configuration, Simulation) {
        let c = fixtures::riding();
        let mut setup = Setup::authored(&c).unwrap();
        setup.rules.combat_version = 4;
        setup.rules.bike_contacts = false;
        setup.seeds.clear();
        for id in 1..=3 {
            let mut seed = Seed::player();
            seed.id = ActorId(id);
            seed.distance = 1000;
            seed.lateral = (id as i32 - 2) * 10000;
            seed.stats.strength = 1;
            setup.seeds.push(seed);
        }
        (c.clone(), Simulation::with_encounters(1, c, setup).unwrap())
    }
    #[test]
    fn source4_detached_target_uses_body_clip_instead_of_stale_mounted_reaction() {
        let (c, mut sim) = field();
        let world = sim.encounter.as_mut().unwrap();
        world
            .hit(
                &mut sim.actors,
                &c,
                ActorId(1),
                Impact {
                    reason: 4,
                    ..Default::default()
                },
                0,
                true,
                &mut vec![],
            )
            .unwrap();
        world
            .units
            .get_mut(&ActorId(1))
            .unwrap()
            .fighter
            .stats
            .health = 100;
        world
            .units
            .get_mut(&ActorId(1))
            .unwrap()
            .recovery
            .body
            .as_mut()
            .unwrap()
            .animation
            .code = 35;
        // The source fall starts above melee height. This fixture observes
        // the detached rider after landing, while the old mounted clip stays9.
        let body = world
            .units
            .get_mut(&ActorId(1))
            .unwrap()
            .recovery
            .body
            .as_mut()
            .unwrap();
        body.motion.height = 0;
        body.location.height = 0;
        sim.actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .animation
            .code = 9;
        let request = Request {
            kind: AttackKind::Punch,
            side: Side::Left,
        };
        let definition =
            Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 4)
                .unwrap();
        let r = sim
            .actors
            .get_mut(&ActorId(2))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap();
        r.animation.set(&c.animation, 0, definition.code);
        r.animation.frame = 1;
        r.animation.selected = Some((definition.code as usize, 1));
        world.units.get_mut(&ActorId(2)).unwrap().fighter.swing = Some(Swing {
            repeat: false,
            request,
            target: Some(ActorId(1)),
            weapon: animation::Weapon::Fists,
            started: 0,
            definition,
            resolved: false,
            swish: false,
        });
        world
            .source_combat_probe_tick(&mut sim.actors, &c, 1, |_, _| 1, &mut vec![])
            .unwrap();
        assert!(
            world.units[&ActorId(1)].fighter.stats.health < 100,
            "Detached clip must use body: target={:?}, body={:?}, swing={:?}, health={}",
            world.source_combat_target(ActorId(2), &sim.actors),
            world.source_combat_half_position(ActorId(1), &sim.actors),
            world.units[&ActorId(2)].fighter.swing,
            world.units[&ActorId(1)].fighter.stats.health
        );
    }
    #[test]
    fn source4_impact_scan_uses_the_previous_half_in_equal_position_list_order() {
        let (_, sim) = field();
        let world = sim.encounter.as_ref().unwrap();
        assert_eq!(
            world.source_combat_target(ActorId(3), &sim.actors),
            Some(ActorId(2))
        );
        let at = world.position(ActorId(3), &sim.actors).unwrap();
        let historical = combat::target(
            ActorId(3),
            at,
            world
                .units
                .keys()
                .map(|id| (*id, world.position(*id, &sim.actors).unwrap())),
        );
        assert_eq!(
            historical,
            Some(ActorId(1)),
            "historical profile's ID tie is deliberately retained separately"
        );
    }
    #[test]
    fn source4_nonplayer_failed_repeat_consumes_the_native_second_draw_and_queues_reaction() {
        let (c, mut sim) = field();
        let request = Request {
            kind: AttackKind::Punch,
            side: Side::Left,
        };
        let definition =
            Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 4)
                .unwrap();
        let tick = (0..definition.duration)
            .find(|age| definition.active(u64::from(*age)))
            .unwrap() as u64;
        sim.actors
            .get_mut(&ActorId(2))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .animation
            .set(&c.animation, 0, definition.code);
        let world = sim.encounter.as_mut().unwrap();
        world.units.get_mut(&ActorId(2)).unwrap().fighter.swing = Some(Swing {
            repeat: false,
            request,
            target: Some(ActorId(1)),
            weapon: animation::Weapon::Fists,
            started: 0,
            definition,
            resolved: false,
            swish: false,
        });
        let mut draws = vec![];
        world
            .combat(
                &mut sim.actors,
                &c,
                tick,
                |_, site| {
                    draws.push(site);
                    if draws.len() == 1 { 1 } else { 2 }
                },
                &mut vec![],
            )
            .unwrap();
        assert_eq!(draws, ["attack_chain", "attack_chain_fallback"]);
        assert_eq!(
            sim.actors[&ActorId(2)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .animation
                .queued,
            Some(6)
        );
        assert_eq!(world.units[&ActorId(1)].fighter.stats.health, 2436);
    }
    #[test]
    fn source4_detached_half_keeps_target_identity_separate_from_its_quiet_bike() {
        let (c, mut sim) = field();
        let world = sim.encounter.as_mut().unwrap();
        world
            .hit(
                &mut sim.actors,
                &c,
                ActorId(1),
                Impact {
                    reason: 4,
                    ..Default::default()
                },
                0,
                true,
                &mut vec![],
            )
            .unwrap();
        sim.actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .distance_q8 = 100000;
        world
            .units
            .get_mut(&ActorId(1))
            .unwrap()
            .recovery
            .bike_location
            .progress = 100000;
        let body = world
            .units
            .get_mut(&ActorId(1))
            .unwrap()
            .recovery
            .body
            .as_mut()
            .unwrap();
        body.location.progress = 999;
        body.motion.distance_q8 = 999;
        body.projected = Some(999);
        assert_eq!(
            world.source_combat_target(ActorId(2), &sim.actors),
            Some(ActorId(3)),
            "same-position next half precedes a one-unit-behind body"
        );
        world
            .units
            .get_mut(&ActorId(3))
            .unwrap()
            .recovery
            .bike_location
            .progress = 1005;
        sim.actors
            .get_mut(&ActorId(3))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .distance_q8 = 1005;
        assert_eq!(
            world.source_combat_target(ActorId(2), &sim.actors),
            Some(ActorId(1))
        );
    }
}

#[cfg(test)]
mod source4_transaction_tests {
    use super::*;
    use crate::combat::{Item, WeaponId};
    fn fixture() -> (Configuration, Simulation) {
        let c = fixtures::riding();
        let mut setup = Setup::authored(&c).unwrap();
        setup.rules.combat_version = 4;
        setup.rules.bike_contacts = false;
        setup.seeds.clear();
        for id in 1..=3 {
            let mut seed = Seed::player();
            seed.id = ActorId(id);
            seed.distance = 1000;
            seed.lateral = (id as i32 - 2) * 10000;
            seed.stats.strength = 1;
            setup.seeds.push(seed);
        }
        (c.clone(), Simulation::with_encounters(1, c, setup).unwrap())
    }
    #[test]
    fn source4_kick_clip_alias_keeps_the_native_signed_backhand_speed_transfer() {
        let (c, mut sim) = fixture();
        let request = Request {
            kind: AttackKind::Kick,
            side: Side::Left,
        };
        let definition =
            Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 4)
                .unwrap();
        let tick = (0..definition.duration)
            .find(|age| definition.active(u64::from(*age)))
            .unwrap() as u64;
        sim.actors
            .get_mut(&ActorId(2))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .animation
            .set(&c.animation, 0, definition.code);
        let world = sim.encounter.as_mut().unwrap();
        world.units.get_mut(&ActorId(2)).unwrap().fighter.swing = Some(Swing {
            repeat: false,
            request,
            target: Some(ActorId(1)),
            weapon: animation::Weapon::Fists,
            started: 0,
            definition,
            resolved: false,
            swish: false,
        });
        world
            .combat(&mut sim.actors, &c, tick, |_, _| 0, &mut vec![])
            .unwrap();
        assert_eq!(
            sim.actors[&ActorId(1)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .bike
                .speed,
            -300
        );
        assert_eq!(
            sim.actors[&ActorId(2)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .bike
                .speed,
            300
        );
        assert_eq!(
            sim.actors[&ActorId(1)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .bike
                .condition,
            99
        );
    }
    #[test]
    fn source4_equal_positions_retain_the_previous_physical_half_sort_order() {
        let (_, mut sim) = fixture();
        sim.actors
            .get_mut(&ActorId(3))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .distance_q8 = 999;
        sim.encounter
            .as_mut()
            .unwrap()
            .units
            .get_mut(&ActorId(3))
            .unwrap()
            .recovery
            .bike_location
            .progress = 999;
        let world = sim.encounter.as_mut().unwrap();
        world.refresh_source_combat_order(&sim.actors);
        assert_eq!(
            world.source_combat_order[&ActorId(1)],
            [ActorId(3), ActorId(1), ActorId(2)]
        );
        sim.actors
            .get_mut(&ActorId(3))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .distance_q8 = 1000;
        world
            .units
            .get_mut(&ActorId(3))
            .unwrap()
            .recovery
            .bike_location
            .progress = 1000;
        world.refresh_source_combat_order(&sim.actors);
        assert_eq!(
            world.source_combat_target(ActorId(1), &sim.actors),
            Some(ActorId(3))
        );
    }
    #[test]
    fn source4_held_attack_restarts_in_its_own_think_after_an_earlier_actor_hit() {
        let (c, mut sim) = fixture();
        let definition = Definition::from_library_version(
            &c.animation,
            Request {
                kind: AttackKind::Punch,
                side: Side::Right,
            },
            animation::Weapon::Fists,
            4,
        )
        .unwrap();
        let first = (0..definition.duration)
            .find(|age| definition.active(u64::from(*age)))
            .unwrap() as u64;
        for actor in sim.actors.values_mut() {
            actor.input.attack = Some(Request {
                kind: AttackKind::Punch,
                side: Side::Auto,
            });
        }
        for tick in 0..=first {
            sim.encounter
                .as_mut()
                .unwrap()
                .source_combat_probe_tick(&mut sim.actors, &c, tick, |_, _| 1, &mut vec![])
                .unwrap();
        }
        let world = sim.encounter.as_ref().unwrap();
        assert_eq!(
            world.units[&ActorId(2)]
                .fighter
                .swing
                .as_ref()
                .unwrap()
                .started,
            first
        );
        assert_ne!(
            sim.actors[&ActorId(2)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .animation
                .code,
            9
        );
        assert_eq!(world.units[&ActorId(2)].fighter.stats.health, 2437);
    }
    #[test]
    fn source4_removing_a_windup_target_drops_one_owned_weapon_and_cannot_duplicate_a_hit() {
        let (c, mut sim) = fixture();
        let request = Request {
            kind: AttackKind::Punch,
            side: Side::Left,
        };
        let definition =
            Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 4)
                .unwrap();
        let tick = (0..definition.duration)
            .find(|age| definition.active(u64::from(*age)))
            .unwrap() as u64;
        sim.actors
            .get_mut(&ActorId(2))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .animation
            .set(&c.animation, 0, definition.code);
        let world = sim.encounter.as_mut().unwrap();
        world.inventory.items.insert(
            WeaponId(1),
            Item {
                kind: animation::Weapon::Club,
                owner: Some(ActorId(1)),
            },
        );
        world.units.get_mut(&ActorId(2)).unwrap().fighter.swing = Some(Swing {
            repeat: false,
            request,
            target: Some(ActorId(1)),
            weapon: animation::Weapon::Fists,
            started: 0,
            definition,
            resolved: false,
            swish: false,
        });
        world.remove(ActorId(1));
        sim.actors.remove(&ActorId(1));
        world
            .combat(
                &mut sim.actors,
                &c,
                tick,
                |_, _| panic!("removed target invented an impact draw"),
                &mut vec![],
            )
            .unwrap();
        assert_eq!(world.inventory.items[&WeaponId(1)].owner, None);
        assert_eq!(world.units[&ActorId(3)].fighter.stats.health, 2500);
    }
}

#[cfg(test)]
mod source4_fallback_theft_tests {
    use super::*;
    use crate::combat::{Item, WeaponId};
    fn field() -> (Configuration, Simulation) {
        let c = fixtures::riding();
        let mut setup = Setup::authored(&c).unwrap();
        setup.rules.combat_version = 4;
        setup.rules.bike_contacts = false;
        setup.seeds.clear();
        for id in 1..=3 {
            let mut s = Seed::player();
            s.id = ActorId(id);
            s.distance = 1000;
            s.lateral = (id as i32 - 2) * 10000;
            s.stats.strength = 1;
            setup.seeds.push(s);
        }
        (c.clone(), Simulation::with_encounters(1, c, setup).unwrap())
    }
    #[test]
    fn source4_unavailable_attack_resolves_its_clip_and_rejects_another_weapon_fallback() {
        let (mut c, mut sim) = field();
        let code = c.animation.attacks[0][0][0] as usize;
        let fallback = c.animation.attacks[0][0][1];
        c.animation.banks[0][code] = None;
        c.animation.fallback[code] = fallback;
        c.validate().unwrap();
        let request = Request {
            kind: AttackKind::Punch,
            side: Side::Left,
        };
        assert_eq!(
            Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 3)
                .unwrap()
                .code,
            code as i8
        );
        assert_eq!(
            Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 4)
                .unwrap()
                .code,
            fallback
        );
        sim.actors.get_mut(&ActorId(2)).unwrap().input.attack = Some(request);
        for tick in 0..20 {
            sim.encounter
                .as_mut()
                .unwrap()
                .source_combat_probe_tick(
                    &mut sim.actors,
                    &c,
                    tick,
                    |_, _| panic!("unsupported fists fallback must not hit as a club"),
                    &mut vec![],
                )
                .unwrap();
        }
        assert_eq!(
            sim.encounter.as_ref().unwrap().units[&ActorId(1)]
                .fighter
                .stats
                .health,
            2500
        );
    }
    #[test]
    fn source4_valid_same_weapon_fallback_can_land_without_restarting_the_requested_backhand() {
        let (mut c, mut sim) = field();
        let requested = c.animation.attacks[0][1][0] as usize;
        let actual = c.animation.attacks[0][0][0];
        c.animation.banks[0][requested] = None;
        c.animation.fallback[requested] = actual;
        c.validate().unwrap();
        let request = Request {
            kind: AttackKind::Backhand,
            side: Side::Left,
        };
        let def =
            Definition::from_library_version(&c.animation, request, animation::Weapon::Fists, 4)
                .unwrap();
        let first = (0..def.duration)
            .find(|age| def.active(u64::from(*age)))
            .unwrap() as u64;
        sim.actors.get_mut(&ActorId(2)).unwrap().input.attack = Some(request);
        for tick in 0..=first {
            if tick == 1 {
                sim.actors.get_mut(&ActorId(2)).unwrap().input.attack = None;
            }
            sim.encounter
                .as_mut()
                .unwrap()
                .source_combat_probe_tick(&mut sim.actors, &c, tick, |_, _| 1, &mut vec![])
                .unwrap();
        }
        assert_eq!(
            sim.encounter.as_ref().unwrap().units[&ActorId(1)]
                .fighter
                .stats
                .health,
            2436
        );
        assert_eq!(
            sim.actors[&ActorId(1)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .bike
                .speed,
            0,
            "normal fallback clip does not inherit backhand speed transfer"
        );
    }
    #[test]
    fn source4_stolen_attack_clip_retains_its_clock_across_a_held_backhand_command() {
        let (c, mut sim) = field();
        let world = sim.encounter.as_mut().unwrap();
        let club = c.animation.attacks[0][0][1];
        let mut codes = [2; 8];
        codes[1] = club;
        codes[5] = club;
        world.setup.rules.steal_codes = Some(codes);
        world.inventory.items.insert(
            WeaponId(1),
            Item {
                kind: animation::Weapon::Club,
                owner: Some(ActorId(1)),
            },
        );
        let strike = Request {
            kind: AttackKind::Punch,
            side: Side::Left,
        };
        let definition =
            Definition::from_library_version(&c.animation, strike, animation::Weapon::Fists, 4)
                .unwrap();
        let tick = (0..definition.duration)
            .find(|age| definition.active(u64::from(*age)))
            .unwrap() as u64;
        for (id, request, weapon, started) in [
            (
                ActorId(1),
                Request {
                    kind: AttackKind::Punch,
                    side: Side::Right,
                },
                animation::Weapon::Club,
                tick,
            ),
            (ActorId(2), strike, animation::Weapon::Fists, 0),
        ] {
            let definition =
                Definition::from_library_version(&c.animation, request, weapon, 4).unwrap();
            let rider = sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            rider.weapon = weapon;
            rider.animation.set(&c.animation, 0, definition.code);
            world.units.get_mut(&id).unwrap().fighter.swing = Some(Swing {
                repeat: false,
                request,
                target: Some(if id == ActorId(1) {
                    ActorId(2)
                } else {
                    ActorId(1)
                }),
                weapon,
                started,
                definition,
                resolved: false,
                swish: false,
            });
        }
        world
            .combat(&mut sim.actors, &c, tick, |_, _| 3, &mut vec![])
            .unwrap();
        assert_eq!(world.inventory.items[&WeaponId(1)].owner, Some(ActorId(2)));
        assert_eq!(
            world.units[&ActorId(2)]
                .fighter
                .swing
                .as_ref()
                .unwrap()
                .started,
            tick
        );
        sim.actors.get_mut(&ActorId(2)).unwrap().input.attack = Some(Request {
            kind: AttackKind::Backhand,
            side: Side::Left,
        });
        world
            .source_combat_probe_tick(&mut sim.actors, &c, tick + 1, |_, _| 1, &mut vec![])
            .unwrap();
        assert_eq!(
            world.units[&ActorId(2)]
                .fighter
                .swing
                .as_ref()
                .unwrap()
                .started,
            tick
        );
        assert_eq!(
            sim.actors[&ActorId(2)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .animation
                .code,
            club
        );
        world.inventory.validate().unwrap();
    }
}

#[path = "fighting_road_tests.rs"]
mod physical_roads;
