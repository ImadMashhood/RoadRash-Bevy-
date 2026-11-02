//! Opt-in private native handler comparisons. No diagnostic entry points ship in the game.
use crate::combat::*;
use crate::encounter::{Seed, Setup};
use crate::*;
use serde_json::{Value, json};

#[test]
#[ignore = "requires supported private data and native handler observations"]
fn native_strike_transactions() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").expect("native data"));
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").expect("native observations"))
            .unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    config.ground = vec![handling::Ground::flat(); 2000];
    config
        .specs
        .iter_mut()
        .find(|s| s.id == config.selected)
        .unwrap()
        .condition = 150;
    let mut rows = vec![];
    for row in expected["strike_transactions"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let side = if n(1) == 1 { Side::Right } else { Side::Left };
        let kind = if n(3) == 3 {
            AttackKind::Kick
        } else if n(2) == 1 {
            AttackKind::Backhand
        } else {
            AttackKind::Punch
        };
        let weapon = match n(3) {
            1 => animation::Weapon::Club,
            2 => animation::Weapon::Chain,
            _ => animation::Weapon::Fists,
        };
        let mut setup = Setup::authored(&config).unwrap();
        setup.rules.combat_version = 2;
        setup.rules.obstacles.clear();
        setup.rules.bike_contacts = false;
        setup.seeds[0].distance = 1000;
        setup.seeds[0].speed = 5000;
        let mut target = Seed::player();
        target.id = ActorId(2);
        target.distance = 1000;
        target.speed = 5000;
        target.lateral = if side == Side::Right { 18000 } else { -18000 };
        target.stats.health = n(6);
        setup.seeds.push(target);
        if weapon != animation::Weapon::Fists {
            setup.inventory.items.insert(
                WeaponId(1),
                Item {
                    kind: weapon,
                    owner: Some(ActorId(1)),
                },
            );
        }
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        for actor in sim.actors.values_mut() {
            let rider = actor.view.rider.as_mut().unwrap();
            rider.animation.set(&config.animation, 0, 2);
        }
        sim.actors
            .get_mut(&ActorId(2))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .condition = n(7);
        if n(5) != 0 {
            let rider = sim
                .actors
                .get_mut(&ActorId(2))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            rider.pose = animation::Pose::HitReact;
            rider.animation.set(&config.animation, 0, 9);
        }
        let world = sim.encounter.as_mut().unwrap();
        world.units.get_mut(&ActorId(2)).unwrap().fighter.hit_until = n(4) as u64;
        let request = Request { kind, side };
        let definition =
            Definition::from_library_version(&config.animation, request, weapon, 2).unwrap();
        let age = (0..definition.duration)
            .find(|age| definition.active(u64::from(*age)))
            .unwrap();
        world.units.get_mut(&ActorId(1)).unwrap().fighter.swing = Some(Swing {
            repeat: false,
            request,
            target: Some(ActorId(2)),
            weapon,
            started: 100 - u64::from(age),
            definition,
            resolved: false,
            swish: false,
        });
        world
            .combat(&mut sim.actors, &config, 100, |_, _| 0, &mut sim.effects)
            .unwrap();
        let target = &world.units[&ActorId(2)];
        let stats = target.fighter.stats;
        let bike = &sim.actors[&ActorId(2)].view.rider.as_ref().unwrap().bike;
        let attacker = &sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().bike;
        let animation = target.recovery.body.as_ref().map_or(
            sim.actors[&ActorId(2)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .animation
                .code,
            |body| body.animation.code,
        );
        rows.push(json!([
            n(0),
            n(1),
            n(2),
            n(3),
            n(4),
            n(5),
            n(6),
            n(7),
            stats.maximum,
            stats.health,
            bike.condition,
            bike.speed,
            bike.lean,
            i32::from(bike.crashed),
            animation,
            target.fighter.hit_until,
            attacker.speed,
            attacker.lean
        ]));
    }
    let out =
        std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").expect("private output"));
    std::fs::write(
        out.join("strike_transactions-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
    assert_eq!(
        rows.len(),
        expected["strike_transactions"].as_array().unwrap().len()
    );
}

#[test]
#[ignore = "requires supported private data and native theft observations"]
fn native_weapon_theft_transactions() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    config.ground = vec![handling::Ground::flat(); 2000];
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let pe = rr_formats::pe::SupportedExe::read(&exe).unwrap();
    let codes = std::array::from_fn(|i| {
        rr_formats::binary::u32le(pe.read_va(0x464538 + i as u32 * 4, 4).unwrap(), 0).unwrap() as i8
    });
    let mut rows = vec![];
    for row in expected["weapon_theft"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let side = if n(0) == 1 { Side::Right } else { Side::Left };
        let target_side = if n(2) == 1 { Side::Right } else { Side::Left };
        let weapon = if n(1) == 1 {
            animation::Weapon::Club
        } else {
            animation::Weapon::Chain
        };
        let mut setup = Setup::authored(&config).unwrap();
        setup.rules.combat_version = 2;
        setup.rules.steal_codes = Some(codes);
        setup.rules.bike_contacts = false;
        setup.rules.obstacles.clear();
        setup.seeds[0].distance = 1000;
        let mut target = Seed::player();
        target.id = ActorId(2);
        target.distance = 1000;
        target.lateral = if side == Side::Right { 18000 } else { -18000 };
        setup.seeds.push(target);
        setup.inventory.items.insert(
            WeaponId(1),
            Item {
                kind: weapon,
                owner: Some(ActorId(2)),
            },
        );
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        for (id, side, carried) in [
            (ActorId(1), side, animation::Weapon::Fists),
            (ActorId(2), target_side, weapon),
        ] {
            let request = Request {
                kind: AttackKind::Punch,
                side,
            };
            let definition =
                Definition::from_library_version(&config.animation, request, carried, 2).unwrap();
            let age = if id == ActorId(1) {
                (0..definition.duration)
                    .find(|a| definition.active(u64::from(*a)))
                    .unwrap()
            } else {
                0
            };
            let rider = sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            rider.animation.set(&config.animation, 0, definition.code);
            rider.weapon = carried;
            if id == ActorId(2) {
                rider.animation.frame = n(3) as usize;
            }
            sim.encounter
                .as_mut()
                .unwrap()
                .units
                .get_mut(&id)
                .unwrap()
                .fighter
                .swing = Some(Swing {
                repeat: false,
                request,
                target: Some(if id == ActorId(1) {
                    ActorId(2)
                } else {
                    ActorId(1)
                }),
                weapon: carried,
                started: 100 - u64::from(age),
                definition,
                resolved: false,
                swish: false,
            });
        }
        let world = sim.encounter.as_mut().unwrap();
        world
            .units
            .get_mut(&ActorId(2))
            .unwrap()
            .fighter
            .steal_until = n(4) as u64;
        world
            .combat(
                &mut sim.actors,
                &config,
                100,
                |_, _| n(5) as u32,
                &mut sim.effects,
            )
            .unwrap();
        let attacker = sim.actors[&ActorId(1)].view.rider.as_ref().unwrap();
        let target = sim.actors[&ActorId(2)].view.rider.as_ref().unwrap();
        let stolen = attacker.weapon != animation::Weapon::Fists;
        rows.push(json!([
            n(0),
            n(1),
            n(2),
            n(3),
            n(4),
            n(5),
            attacker.weapon as u8,
            target.weapon as u8,
            world.units[&ActorId(1)].fighter.steal_until,
            world.units[&ActorId(2)].fighter.stats.health,
            attacker.animation.code,
            target.animation.code,
            if stolen {
                i32::from(attacker.animation.queued.unwrap_or(-1))
            } else {
                -99
            },
            if stolen {
                i32::from(target.animation.queued.unwrap_or(-1))
            } else {
                -99
            }
        ]));
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("weapon_theft-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}
