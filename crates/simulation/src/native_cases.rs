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

#[test]
#[ignore = "requires private native separator observations"]
fn native_separation_transactions() {
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let mut rows = vec![];
    for row in expected["separation"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let read = |offset: usize, category: u32| collision::Body {
            position: std::array::from_fn(|i| n(offset + i)),
            previous: std::array::from_fn(|i| n(offset + 3 + i)),
            half: std::array::from_fn(|i| n(offset + 6 + i)),
            velocity: std::array::from_fn(|i| n(offset + 9 + i)),
            speed: n(offset + 9),
            mass: n(offset + 12),
            category,
            flags: 0,
            impulse: [0; 3],
        };
        let mut a = read(3, n(1) as u32);
        let mut b = read(16, n(2) as u32);
        collision::separate(&mut a, &mut b);
        let mut output: Vec<i32> = (0..29).map(n).collect();
        for body in [a, b] {
            output.extend(body.position);
            output.extend(body.previous);
            output.extend(body.impulse);
            output.push(i32::from(body.flags));
        }
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("separation-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native complete bike contact observations"]
fn native_bike_contact_transactions() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut rows = vec![];
    for row in expected["bike_contacts"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(1000000);
        setup.rules.combat_version = 2;
        setup.rules.collision_version = 2;
        setup.rules.collision = Some(profile.clone());
        setup.rules.obstacles.clear();
        setup.rules.bike_contacts = false;
        setup.seeds[0].distance = 2560;
        let mut other = Seed::player();
        other.id = ActorId(2);
        other.distance = 2580;
        setup.seeds.push(other);
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        for (index, offset) in [(0, 7), (1, 20)] {
            let id = ActorId(index + 1);
            let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
            unit.previous_location = topology::Location {
                progress: i64::from(n(offset + 3)),
                lateral: n(offset + 4),
                height: n(offset + 5),
                ..Default::default()
            };
            unit.recovery.bike_location.progress = i64::from(n(offset));
            unit.recovery.bike_location.lateral = n(offset + 1);
            unit.recovery.bike_location.height = n(offset + 2);
            let bike = &mut sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            bike.distance_q8 = i64::from(n(offset));
            bike.lateral = n(offset + 1);
            bike.height = n(offset + 2);
            bike.speed = n(offset + 9);
            bike.lateral_speed = n(offset + 10);
            bike.vertical_speed = n(offset + 11);
            bike.condition = n(6);
            bike.lean = 500;
            bike.crashed = if index == 0 { n(5) != 0 } else { n(4) != 0 };
        }
        let world = sim.encounter.as_mut().unwrap();
        world
            .source_pair_contact(
                ActorId(1),
                ActorId(2),
                &mut sim.actors,
                &config,
                100,
                &mut sim.effects,
            )
            .unwrap();
        let mut output: Vec<i32> = (0..33).map(n).collect();
        for id in [ActorId(1), ActorId(2)] {
            let unit = &world.units[&id];
            let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
            let p = world
                .setup
                .topology
                .projected(unit.recovery.bike_location)
                .unwrap() as i32;
            output.extend([
                p,
                bike.lateral,
                bike.height,
                bike.speed,
                bike.lateral_speed,
                bike.vertical_speed,
                bike.lean,
                bike.condition,
                i32::from(bike.crashed),
                unit.fighter.stats.maximum,
                unit.fighter.stats.health,
                unit.recovery
                    .body
                    .as_ref()
                    .map_or(-1, |b| i32::from(b.animation.code)),
                unit.recovery.body.as_ref().map_or(0, |b| b.motion.speed),
                unit.recovery.body.as_ref().map_or(0, |b| b.motion.height),
                unit.recovery
                    .body
                    .as_ref()
                    .map_or(0, |b| b.motion.vertical_speed),
            ]);
        }
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("bike_contacts-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native complete car contact observations"]
fn native_car_contact_transactions() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut rows = vec![];
    for row in expected["car_contacts"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(1000000);
        setup.rules.combat_version = 2;
        setup.rules.collision_version = 2;
        setup.rules.collision = Some(profile.clone());
        setup.rules.obstacles.clear();
        setup.rules.bike_contacts = false;
        setup.seeds[0].distance = 2560;
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        let id = ActorId(1);
        let offset = 7;
        let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
        unit.previous_location = topology::Location {
            progress: i64::from(n(offset + 3)),
            lateral: n(offset + 4),
            height: n(offset + 5),
            ..Default::default()
        };
        unit.recovery.bike_location.progress = i64::from(n(offset));
        unit.recovery.bike_location.lateral = n(offset + 1);
        unit.recovery.bike_location.height = n(offset + 2);
        let bike = &mut sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.distance_q8 = i64::from(n(offset));
        bike.lateral = n(offset + 1);
        bike.height = n(offset + 2);
        bike.speed = n(offset + 9);
        bike.lateral_speed = n(offset + 10);
        bike.vertical_speed = n(offset + 11);
        bike.condition = n(6);
        bike.lean = 500;
        bike.crashed = n(5) != 0;
        let offset = 20;
        let mut car = collision::Body {
            position: std::array::from_fn(|i| n(offset + i)),
            previous: std::array::from_fn(|i| n(offset + 3 + i)),
            half: std::array::from_fn(|i| n(offset + 6 + i)),
            velocity: std::array::from_fn(|i| n(offset + 9 + i)),
            speed: n(4).abs(),
            mass: n(offset + 12),
            category: 16,
            flags: 0,
            impulse: [0; 3],
        };
        let world = sim.encounter.as_mut().unwrap();
        world
            .source_object_contact(
                id,
                &mut sim.actors,
                &mut car,
                &config,
                100,
                &mut sim.effects,
            )
            .unwrap();
        let mut output: Vec<i32> = (0..33).map(n).collect();
        let unit = &world.units[&id];
        let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
        let p = world
            .setup
            .topology
            .projected(unit.recovery.bike_location)
            .unwrap() as i32;
        output.extend([
            p,
            bike.lateral,
            bike.height,
            bike.speed,
            bike.lateral_speed,
            bike.vertical_speed,
            bike.lean,
            bike.condition,
            i32::from(bike.crashed),
            unit.fighter.stats.maximum,
            unit.fighter.stats.health,
            unit.recovery
                .body
                .as_ref()
                .map_or(-1, |b| i32::from(b.animation.code)),
            unit.recovery.body.as_ref().map_or(0, |b| b.motion.speed),
            unit.recovery.body.as_ref().map_or(0, |b| b.motion.height),
            unit.recovery
                .body
                .as_ref()
                .map_or(0, |b| b.motion.vertical_speed),
        ]);
        output.extend(car.position);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("car_contacts-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native complete static contact observations"]
fn native_static_contact_transactions() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut rows = vec![];
    for row in expected["static_contacts"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(1000000);
        setup.rules.combat_version = 2;
        setup.rules.collision_version = 2;
        setup.rules.collision = Some(profile.clone());
        setup.rules.obstacles.clear();
        setup.rules.bike_contacts = false;
        setup.seeds[0].distance = 2560;
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        let id = ActorId(1);
        let offset = 7;
        let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
        unit.previous_location = topology::Location {
            progress: i64::from(n(offset + 3)),
            lateral: n(offset + 4),
            height: n(offset + 5),
            ..Default::default()
        };
        unit.recovery.bike_location.progress = i64::from(n(offset));
        unit.recovery.bike_location.lateral = n(offset + 1);
        unit.recovery.bike_location.height = n(offset + 2);
        let bike = &mut sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.distance_q8 = i64::from(n(offset));
        bike.lateral = n(offset + 1);
        bike.height = n(offset + 2);
        bike.speed = n(offset + 9);
        bike.lateral_speed = n(offset + 10);
        bike.vertical_speed = n(offset + 11);
        bike.condition = n(6);
        bike.lean = 500;
        bike.crashed = n(5) != 0;
        let offset = 20;
        let mut car = collision::Body {
            position: std::array::from_fn(|i| n(offset + i)),
            previous: std::array::from_fn(|i| n(offset + 3 + i)),
            half: std::array::from_fn(|i| n(offset + 6 + i)),
            velocity: std::array::from_fn(|i| n(offset + 9 + i)),
            speed: 0,
            mass: n(offset + 12),
            category: 1,
            flags: 0,
            impulse: [0; 3],
        };
        let world = sim.encounter.as_mut().unwrap();
        world
            .source_object_contact(
                id,
                &mut sim.actors,
                &mut car,
                &config,
                100,
                &mut sim.effects,
            )
            .unwrap();
        let mut output: Vec<i32> = (0..33).map(n).collect();
        let unit = &world.units[&id];
        let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
        let p = world
            .setup
            .topology
            .projected(unit.recovery.bike_location)
            .unwrap() as i32;
        output.extend([
            p,
            bike.lateral,
            bike.height,
            bike.speed,
            bike.lateral_speed,
            bike.vertical_speed,
            bike.lean,
            bike.condition,
            i32::from(bike.crashed),
            unit.fighter.stats.maximum,
            unit.fighter.stats.health,
            unit.recovery
                .body
                .as_ref()
                .map_or(-1, |b| i32::from(b.animation.code)),
            unit.recovery.body.as_ref().map_or(0, |b| b.motion.speed),
            unit.recovery.body.as_ref().map_or(0, |b| b.motion.height),
            unit.recovery
                .body
                .as_ref()
                .map_or(0, |b| b.motion.vertical_speed),
        ]);
        output.extend(car.position);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("static_contacts-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native frame-box observations"]
fn native_scenery_boxes() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let bytes = std::fs::read(data.join("DATA/FAMILIES.RSC")).unwrap();
    let directory = rr_formats::resource::Directory::read(&bytes).unwrap();
    let mut rows = vec![];
    for resource in directory.entries.iter().filter(|r| r.tag == "FAM ") {
        let payload =
            &bytes[resource.offset as usize..(resource.offset + resource.length) as usize];
        let unpacked = rr_formats::resource::unpack(payload).unwrap();
        let family = rr_formats::family::Family::read_unpacked(&unpacked).unwrap();
        for node in family.nodes.iter().filter(|n| n.tag == 0x5252414e) {
            let boxes =
                rr_formats::animation_boxes::read(&unpacked, node.offset as usize, 2).unwrap();
            let mut row = vec![
                i64::from(resource.id),
                i64::from(node.offset),
                boxes.len() as i64,
            ];
            row.extend(boxes.into_iter().flatten().map(i64::from));
            row.push(
                rr_formats::animation_boxes::texture_flags(&unpacked, node.offset as usize, 2)
                    .unwrap()
                    .map_or(-1, i64::from),
            );
            rows.push(row);
        }
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("scenery_boxes-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native building-wall observations"]
fn native_building_walls() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let topology = topology::Topology::straight(1000000);
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut rows = vec![];
    for row in expected["building_walls"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let current = building_wall::Boundary {
            surface: None,
            path: 0,
            building: true,
            sample: 10,
            road: [-500, 500],
            outer: [-750, 750],
            selectors: [n(9) as u8; 2],
            flags: n(7) as u8,
        };
        let previous = building_wall::Boundary {
            sample: 9,
            selectors: [n(8) as u8; 2],
            flags: n(6) as u8,
            ..current.clone()
        };
        let lateral = n(1) * (750 * 256 - 7680 + n(2) * 256);
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology.clone();
        setup.rules.combat_version = 2;
        setup.rules.collision_version = 2;
        setup.rules.collision = Some(profile.clone());
        setup.rules.building_boundaries = vec![previous, current];
        setup.rules.obstacles.clear();
        setup.rules.bike_contacts = false;
        setup.seeds[0].distance = i64::from(2560 + n(3));
        setup.seeds[0].lateral = lateral;
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        let unit = sim
            .encounter
            .as_mut()
            .unwrap()
            .units
            .get_mut(&ActorId(1))
            .unwrap();
        unit.previous_location.progress = i64::from(2304 + n(3));
        let bike = &mut sim
            .actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.speed = n(4);
        bike.lateral_speed = n(5);
        bike.lean = 500;
        let world = sim.encounter.as_mut().unwrap();
        world
            .source_building_contact(ActorId(1), &mut sim.actors, &config, 100, &mut sim.effects)
            .unwrap();
        let bike = &sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().bike;
        let state = &world.units[&ActorId(1)].recovery;
        let mut output: Vec<i32> = (0..10).map(n).collect();
        output.extend([
            bike.distance_q8 as i32,
            bike.lateral,
            bike.speed,
            bike.lateral_speed,
            bike.lean,
            bike.height,
            bike.vertical_speed,
            i32::from(bike.crashed),
            state
                .body
                .as_ref()
                .map_or(-1, |b| i32::from(b.animation.code)),
            state
                .body
                .as_ref()
                .map_or(0, |b| b.motion.distance_q8 as i32),
            state.body.as_ref().map_or(0, |b| b.motion.lateral),
            state.body.as_ref().map_or(0, |b| b.motion.height),
            state.body.as_ref().map_or(0, |b| b.motion.vertical_speed),
        ]);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("building_walls-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native coupled recovery observations"]
fn native_recovery_trajectories() {
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
    config.ground = vec![handling::Ground::flat(); 10000];
    config.start_sample = 0;
    config.motion_version = 2;
    let mut topology = topology::Topology::straight(2560000);
    let mut law =
        recovery::BodyLaw::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    law.motion_version = 1;
    law.recovery_version = 1;
    let mut active = None;
    let mut state = recovery::State::default();
    let mut bike = handling::Bike::new(config.spec(), &config.tables).unwrap();
    let mut rows = vec![];
    for row in expected["recovery_trajectory"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        if active != Some(n(0)) {
            active = Some(n(0));
            topology.paths.get_mut(&0).unwrap().rises = vec![n(4) as i8; 10000];
            for ground in &mut config.ground {
                ground.rise = n(4);
            }
            bike = handling::Bike::new(config.spec(), &config.tables).unwrap();
            bike.speed = n(1);
            bike.condition = 150;
            bike.distance_q8 = 2560;
            bike.height = n(5);
            bike.previous_rise = n(4);
            let at = topology::Location {
                progress: 2560,
                height: n(5),
                ..Default::default()
            };
            state = recovery::State::mounted(ActorId(1), at);
            state.begin_source(
                ActorId(1),
                &mut bike,
                &config,
                at,
                recovery::Impact {
                    reason: n(2) as u8,
                    ..Default::default()
                },
                &topology,
            );
        }
        if n(3) == 1 && n(7) != 0 {
            bike.contact = 0;
            bike.air_lateral_speed = n(7) * 1800;
        }
        state
            .step_with_random(
                &mut bike,
                &config,
                &topology,
                &law,
                recovery::Assists::default(),
                None,
                n(3) as u32,
                || n(6) as u32,
            )
            .unwrap();
        let crash = match state.phase {
            recovery::Phase::Mounted => 0,
            recovery::Phase::Sliding => 1,
            recovery::Phase::Mounting => 3,
            _ => 2,
        };
        let mut output: Vec<i32> = (0..8).map(n).collect();
        output.extend([
            bike.distance_q8 as i32,
            bike.lateral,
            bike.height,
            bike.speed,
            bike.vertical_speed,
            bike.contact,
            i32::from(state.body.is_some()),
            crash,
        ]);
        if let Some(body) = &state.body {
            output.extend([
                body.motion.distance_q8 as i32,
                body.motion.lateral,
                body.motion.height,
                body.motion.speed,
                body.motion.vertical_speed,
                body.motion.contact,
                body.location.heading,
                i32::from(body.animation.code),
                body.animation.frame as i32,
                i32::from(body.animation.loops),
            ]);
        } else {
            output.extend([0, 0, 0, 0, 0, 0, 0, -1, 0, 0]);
        }
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("recovery_trajectory-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native detached contact observations"]
fn native_half_contact_responses() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let profile = collision::Profile::from_exe(&exe).unwrap();
    let rider = profile.rider.as_ref().unwrap();
    let library =
        animation::Library::read(&exe, &std::fs::read(data.join("DATA/GLOBAL.RSC")).unwrap())
            .unwrap();
    let mut rows = vec![];
    for row in expected["half_contacts"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mut body = collision::Body {
            position: [2560, 0, 100],
            previous: [2560, 0, 100],
            half: [
                rider.dimensions[1],
                rider.dimensions[0],
                rider.dimensions[2],
            ],
            velocity: [n(3), 0, n(5)],
            speed: n(3),
            mass: rider.mass,
            category: 8,
            flags: 7,
            impulse: [1000, -2000, 500],
        };
        let other = collision::Body {
            position: [2580, 6000, 0],
            previous: [2580, 6000, 0],
            half: [64, n(9), n(10)],
            velocity: [n(4), 0, 0],
            speed: n(4),
            mass: 256,
            category: n(1) as u32,
            flags: 0,
            impulse: [0; 3],
        };
        let mut motion = collision::Motion {
            speed: n(3),
            lateral: 0,
            vertical: n(5),
            lean: 0,
            steering_gain: 0,
            elasticity: rider.elasticity,
            crashed: true,
        };
        let mut control = collision::HalfControl {
            state: n(2) as u8,
            animation: n(6) as i8,
            queued: None,
            reset_animation: false,
        };
        let mut player = animation::Player::default();
        player.set(&library, 0, n(6) as i8);
        player.loops = 2;
        collision::respond_half(
            &mut body,
            &other,
            &mut motion,
            &mut control,
            n(7) != 0 && n(4) < 10,
            || n(8) as u32,
        );
        if control.reset_animation {
            player.set(&library, 0, control.animation);
            player.queued = control.queued;
        }
        let mut output: Vec<i32> = (0..11).map(n).collect();
        output.extend(body.position);
        output.extend([
            motion.speed,
            motion.vertical,
            i32::from(control.state),
            i32::from(player.code),
            i32::from(player.queued.unwrap_or(-1)),
            player.frame as i32,
            i32::from(player.loops),
            i32::from(body.flags),
        ]);
        output.extend(body.impulse);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("half_contacts-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native bike/body pair observations"]
fn native_proxy_pairs() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut rows = vec![];
    for row in expected["proxy_pairs"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(1000000);
        setup.rules.collision_version = 2;
        setup.rules.combat_version = 2;
        setup.rules.collision = Some(profile.clone());
        setup.rules.obstacles.clear();
        setup.rules.bike_contacts = false;
        setup.seeds[0].distance = 2560;
        let mut second = Seed::player();
        second.id = ActorId(2);
        second.distance = 2580;
        setup.seeds.push(second);
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        let owners = [ActorId(1), ActorId(if n(3) != 0 { 1 } else { 2 })];
        let roles = [n(1) != 0, n(2) != 0];
        for index in 0..2 {
            if !roles[index] {
                continue;
            }
            let id = owners[index];
            let world = sim.encounter.as_mut().unwrap();
            let unit = world.units.get_mut(&id).unwrap();
            let rider = sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            unit.recovery.begin_source(
                id,
                &mut rider.bike,
                &config,
                unit.recovery.bike_location,
                recovery::Impact::default(),
                &world.setup.topology,
            );
            unit.recovery.phase = match n(4) {
                1 => recovery::Phase::Sliding,
                2 => recovery::Phase::Standing,
                _ => recovery::Phase::Mounting,
            };
            let body = unit.recovery.body.as_mut().unwrap();
            body.animation.set(
                &config.animation,
                0,
                match n(4) {
                    1 => 33,
                    2 => 37,
                    _ => 41,
                },
            );
            body.animation.queued = None;
            rider.bike.height = 0;
            rider.bike.vertical_speed = 0;
            rider.bike.condition = 150;
        }
        for index in 0..2 {
            let id = owners[index];
            let offset = 9 + index * 13;
            let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
            let at = topology::Location {
                progress: i64::from(n(offset)),
                lateral: n(offset + 1),
                height: n(offset + 2),
                ..Default::default()
            };
            let previous = topology::Location {
                progress: i64::from(n(offset + 3)),
                lateral: n(offset + 4),
                height: n(offset + 5),
                ..Default::default()
            };
            let motion = if roles[index] {
                let body = unit.recovery.body.as_mut().unwrap();
                body.location = at;
                body.previous_location = Some(previous);
                &mut body.motion
            } else {
                unit.recovery.bike_location = at;
                unit.previous_location = previous;
                &mut sim
                    .actors
                    .get_mut(&id)
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap()
                    .bike
            };
            motion.distance_q8 = at.progress;
            motion.lateral = at.lateral;
            motion.height = at.height;
            motion.speed = n(offset + 9);
            motion.lateral_speed = n(offset + 10);
            motion.vertical_speed = n(offset + 11);
            motion.lean = if roles[index] { 0 } else { 500 };
            motion.condition = 150;
        }
        let world = sim.encounter.as_mut().unwrap();
        let objects = [
            encounter::ContactObject {
                owner: owners[0],
                detached: roles[0],
            },
            encounter::ContactObject {
                owner: owners[1],
                detached: roles[1],
            },
        ];
        let mut seed = n(8) as u32;
        world
            .source_proxy_pair(
                (objects[0], objects[1]),
                &mut sim.actors,
                &config,
                (100, &mut sim.effects),
                || {
                    seed = seed.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
                    (seed >> 16) & 0x7fff
                },
            )
            .unwrap();
        let mut output: Vec<i32> = (0..35).map(n).collect();
        for index in 0..2 {
            let id = owners[index];
            let unit = &world.units[&id];
            let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
            let motion = if roles[index] {
                &unit.recovery.body.as_ref().unwrap().motion
            } else {
                bike
            };
            let crash = match unit.recovery.phase {
                recovery::Phase::Mounted => 0,
                recovery::Phase::Sliding => 1,
                recovery::Phase::Mounting => 3,
                _ => 2,
            };
            output.extend([
                motion.distance_q8 as i32,
                motion.lateral,
                motion.height,
                motion.speed,
                motion.lateral_speed,
                motion.vertical_speed,
                motion.lean,
                bike.condition,
                unit.fighter.stats.maximum,
                unit.fighter.stats.health,
                crash,
                unit.recovery
                    .body
                    .as_ref()
                    .map_or(-1, |b| i32::from(b.animation.code)),
                unit.recovery
                    .body
                    .as_ref()
                    .map_or(-1, |b| i32::from(b.animation.queued.unwrap_or(-1))),
            ]);
        }
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("proxy_pairs-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native detached boundary observations"]
fn native_body_building_walls() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let width = profile.rider.as_ref().unwrap().dimensions[0];
    let mut rows = vec![];
    for row in expected["body_building_walls"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let current = building_wall::Boundary {
            surface: None,
            path: 0,
            building: true,
            sample: 10,
            road: [-500, 500],
            outer: [-750, 750],
            selectors: [n(9) as u8; 2],
            flags: n(7) as u8,
        };
        let previous = building_wall::Boundary {
            sample: 9,
            selectors: [n(8) as u8; 2],
            flags: n(6) as u8,
            ..current.clone()
        };
        let lateral = n(1) * (750 * 256 - width + n(2) * 256);
        let mut motion = collision::Motion {
            speed: n(4),
            lateral: n(5),
            vertical: 0,
            lean: 500,
            steering_gain: 0,
            elasticity: 0,
            crashed: false,
        };
        let hit = building_wall::resolve_actor(
            &current,
            &previous,
            2560 + n(3),
            lateral,
            width,
            &mut motion,
            8,
        );
        let mut output: Vec<i32> = (0..10).map(n).collect();
        output.extend([
            if hit.restore_progress {
                2304 + n(3)
            } else {
                2560 + n(3)
            },
            hit.lateral,
            motion.speed,
            motion.lateral,
            motion.lean,
            0,
            0,
            hit.visual_heading.unwrap_or(0),
        ]);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("body_building_walls-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native complete fight sequences"]
fn native_fight_sequences() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut c = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    c.start_sample = 0;
    c.ground = vec![handling::Ground::flat(); 2000];
    c.specs
        .iter_mut()
        .find(|s| s.id == c.selected)
        .unwrap()
        .condition = 150;
    let mut active = None;
    let mut sim = Simulation::with_encounters(1, c.clone(), Setup::authored(&c).unwrap()).unwrap();
    let mut rows = vec![];
    for row in expected["fight_sequences"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        if active != Some(n(0)) {
            active = Some(n(0));
            let mut setup = Setup::authored(&c).unwrap();
            setup.rules.combat_version = 3;
            setup.rules.bike_contacts = false;
            setup.rules.obstacles.clear();
            setup.seeds[0].distance = 1000;
            setup.seeds[0].stats.strength = 1;
            let mut target = Seed::player();
            target.id = ActorId(2);
            target.distance = 1000;
            target.stats.strength = 1;
            setup.seeds.push(target);
            if n(3) == 1 || n(3) == 2 {
                setup.inventory.items.insert(
                    WeaponId(1),
                    Item {
                        kind: if n(3) == 1 {
                            animation::Weapon::Club
                        } else {
                            animation::Weapon::Chain
                        },
                        owner: Some(ActorId(1)),
                    },
                );
            }
            sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
        }
        let tick = n(6) as u64;
        let gap = if (n(5) == 1 && tick < 12) || (n(5) == 2 && (12..40).contains(&tick)) {
            64
        } else {
            0
        };
        let lateral = (if n(1) == 1 { 1 } else { -1 })
            * if n(5) == 3 && tick >= 12 {
                -18000
            } else {
                18000
            };
        let world = sim.encounter.as_mut().unwrap();
        for (id, actor) in &mut sim.actors {
            let rider = actor.view.rider.as_mut().unwrap();
            rider.bike.distance_q8 = 1000 + if id.0 == 2 { gap } else { 0 };
            rider.bike.lateral = if id.0 == 2 { lateral } else { 0 };
            rider.bike.speed = 0;
            rider.bike.height = 0;
            rider.bike.lean = 0;
            let u = world.units.get_mut(id).unwrap();
            u.recovery.bike_location.progress = rider.bike.distance_q8;
            u.recovery.bike_location.lateral = rider.bike.lateral;
            u.recovery.bike_location.height = 0;
            actor.input = Command::default();
        }
        let input = tick == 0 || n(4) == 1 || (n(4) == 2 && tick.is_multiple_of(40));
        sim.actors.get_mut(&ActorId(1)).unwrap().input.attack = input.then_some(Request {
            kind: if n(3) == 3 {
                AttackKind::Kick
            } else if n(2) == 1 {
                AttackKind::Backhand
            } else {
                AttackKind::Punch
            },
            side: Side::Auto,
        });
        let mut events = vec![];
        world
            .movement(&mut sim.actors, &c, tick, &[], &mut events, || 0)
            .unwrap();
        let mut seed = n(7) as u32;
        world
            .combat(
                &mut sim.actors,
                &c,
                tick,
                |_, _| {
                    seed = seed.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
                    (seed >> 16) & 0x7fff
                },
                &mut events,
            )
            .unwrap();
        let target = &world.units[&ActorId(2)].fighter;
        let attacker = sim.actors[&ActorId(1)].view.rider.as_ref().unwrap();
        let mut output: Vec<i32> = (0..8).map(n).collect();
        output.extend([
            target.stats.maximum,
            target.stats.health,
            sim.actors[&ActorId(2)]
                .view
                .rider
                .as_ref()
                .unwrap()
                .bike
                .condition,
            target.hit_until as i32,
            i32::from(attacker.animation.code),
            attacker.animation.frame as i32,
        ]);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("fight_sequences-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native crash stamina observations"]
fn native_recovery_regeneration() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut c = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    c.start_sample = 0;
    c.ground = vec![handling::Ground::flat(); 2000];
    let mut rows = vec![];
    for row in expected["recovery_regeneration"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mut setup = Setup::authored(&c).unwrap();
        setup.rules.body =
            recovery::BodyLaw::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap())
                .unwrap();
        setup.rules.body.motion_version = 1;
        setup.rules.body.recovery_version = 1;
        setup.rules.combat_version = 3;
        setup.rules.bike_contacts = false;
        setup.seeds[0].distance = 2560;
        setup.seeds[0].speed = 5000;
        let id = ActorId(if n(0) == 1 { 1 } else { 2 });
        if id.0 == 2 {
            let mut other = setup.seeds[0].clone();
            other.id = id;
            setup.seeds.push(other);
        }
        let mut sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
        let world = sim.encounter.as_mut().unwrap();
        let unit = world.units.get_mut(&id).unwrap();
        let bike = &mut sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        unit.recovery.begin_source(
            id,
            bike,
            &c,
            unit.recovery.bike_location,
            recovery::Impact::default(),
            &world.setup.topology,
        );
        if n(1) != 1 {
            unit.recovery.phase = if n(1) == 2 {
                recovery::Phase::GettingUp
            } else {
                recovery::Phase::Mounting
            };
            let body = unit.recovery.body.as_mut().unwrap();
            body.animation
                .set(&c.animation, 0, if n(1) == 2 { 36 } else { 41 });
            body.animation.queued = Some(if n(1) == 2 { 37 } else { 2 });
        }
        unit.fighter.stats.health = n(2);
        world
            .movement(&mut sim.actors, &c, 1, &[], &mut vec![], || 0)
            .unwrap();
        rows.push(vec![
            n(0),
            n(1),
            n(2),
            world.units[&id].fighter.stats.health,
        ]);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("recovery_regeneration-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native contact-scan observations"]
fn native_contact_detection() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut rows = vec![];
    for row in expected["contact_detection"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(1000000);
        setup.rules.combat_version = 3;
        setup.rules.collision_version = 2;
        let mut profile = profile.clone();
        profile.rider = None;
        setup.rules.collision = Some(profile);
        setup.rules.obstacles.clear();
        setup.seeds[0].distance = 2560;
        let mut other = Seed::player();
        other.id = ActorId(2);
        other.distance = 2560;
        setup.seeds.push(other);
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        for id in [ActorId(1), ActorId(2)] {
            let first = id.0 == 1;
            let position = if first {
                [n(12), n(13), n(14)]
            } else {
                [2560, 0, 0]
            };
            let previous = if first {
                [n(9), n(10), n(11)]
            } else {
                [2560, 0, 0]
            };
            let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
            unit.previous_location = topology::Location {
                progress: i64::from(previous[0]),
                lateral: previous[1],
                height: previous[2],
                ..Default::default()
            };
            unit.recovery.bike_location = topology::Location {
                progress: i64::from(position[0]),
                lateral: position[1],
                height: position[2],
                ..Default::default()
            };
            let bike = &mut sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            bike.distance_q8 = i64::from(position[0]);
            bike.lateral = position[1];
            bike.height = position[2];
            bike.speed = if first { n(5) } else { 0 };
            bike.lateral_speed = 0;
            bike.vertical_speed = 0;
            bike.condition = 150;
            bike.crashed = n(1) != 0;
        }
        sim.encounter
            .as_mut()
            .unwrap()
            .contacts(&mut sim.actors, &config, 100, &mut sim.effects, || 3)
            .unwrap();
        let hit = sim.effects.iter().any(|e| e.kind == EffectKind::Contact);
        let mut output: Vec<i32> = (0..15).map(n).collect();
        output.push(i32::from(hit));
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("contact_detection-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native static-query observations"]
fn native_static_detection() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut rows = vec![];
    for row in expected["static_detection"].as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let raw_bottom = if n(2) == 7 { 12288 } else { 0 };
        let top = if n(2) == 7 { 16384 } else { 4096 };
        let bottom = match n(16) {
            1 => top - 8192,
            2 => top - 16383,
            _ => raw_bottom,
        };
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(1000000);
        setup.rules.combat_version = 3;
        setup.rules.collision_version = 2;
        let mut profile = profile.clone();
        profile.rider = None;
        setup.rules.collision = Some(profile);
        setup.rules.bike_contacts = false;
        setup.seeds[0].distance = 2688;
        setup.rules.obstacles = vec![encounter::Obstacle {
            id: 1,
            path: 0,
            progress: 2688,
            half_length: 1,
            left: -1024,
            right: 1024,
            height: top,
            kind: encounter::ObstacleKind::Solid,
            source_box: Some(encounter::StaticBox {
                bounds: [-1024, bottom, 1024, top],
                lateral: 0,
                query_bounds: (n(16) == 1 || n(16) == 2).then_some([-1024, raw_bottom, 1024, top]),
                scenery: None,
            }),
        }];
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        let id = ActorId(1);
        let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
        unit.previous_location = topology::Location {
            progress: i64::from(n(10)),
            lateral: n(11),
            height: n(12),
            ..Default::default()
        };
        unit.recovery.bike_location = topology::Location {
            progress: i64::from(n(13)),
            lateral: n(14),
            height: n(15),
            ..Default::default()
        };
        let bike = &mut sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.distance_q8 = i64::from(n(13));
        bike.lateral = n(14);
        bike.height = n(15);
        bike.speed = n(5);
        bike.lateral_speed = 0;
        bike.vertical_speed = 0;
        bike.condition = 150;
        bike.crashed = n(1) != 0;
        sim.encounter
            .as_mut()
            .unwrap()
            .contacts(&mut sim.actors, &config, 100, &mut sim.effects, || 3)
            .unwrap();
        let hit = sim.effects.iter().any(|e| e.kind == EffectKind::Contact);
        let mut output: Vec<i32> = (0..17).map(n).collect();
        output.push(i32::from(hit));
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("static_detection-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private native source-scenery observations"]
fn source_scenery_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_SCENERY").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    config.motion_version = 2;
    config.ground.fill(handling::Ground::flat());
    let profile =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let legacy = std::env::var("RR_SCENERY_LEGACY").is_ok_and(|v| v == "1");
    let mut rows = vec![];
    for row in expected.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        if n(1) == 11 {
            let mut setup = Setup::authored(&config).unwrap();
            setup.topology = topology::Topology::straight(1000000);
            setup.rules.combat_version = 3;
            setup.rules.collision_version = 2;
            setup.rules.scenery_version = u8::from(!legacy);
            setup.rules.bike_contacts = false;
            setup.rules.collision = Some(profile.clone());
            setup.seeds[0].distance = 12800;
            if !legacy {
                setup.rules.obstacles = vec![encounter::Obstacle {
                    id: 1,
                    path: 0,
                    progress: 81 * 256,
                    half_length: 1,
                    left: 1776 * 256 - 1024,
                    right: 1776 * 256 + 1024,
                    height: 4096,
                    kind: encounter::ObstacleKind::Solid,
                    source_box: Some(encounter::StaticBox {
                        bounds: [1776 * 256 - 1024, 0, 1776 * 256 + 1024, 4096],
                        lateral: 1776 * 256,
                        query_bounds: None,
                        scenery: Some(crate::scenery::BoxRules {
                            class: 16,
                            road_mask: 3,
                            sample: 81,
                            selector: 0,
                            ordinal: 0,
                            box_count: 1,
                            texture_flags: None,
                        }),
                    }),
                }];
            }
            let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
            for frame in 0..=n(2) {
                let camera = if frame <= 120 {
                    50 + frame
                } else {
                    290 - frame
                };
                let world = sim.encounter.as_mut().unwrap();
                world
                    .units
                    .get_mut(&ActorId(1))
                    .unwrap()
                    .recovery
                    .bike_location
                    .progress = i64::from(camera * 256);
                world
                    .contacts(
                        &mut sim.actors,
                        &config,
                        frame as u64,
                        &mut sim.effects,
                        || 3,
                    )
                    .unwrap();
                world.scenery_prepare(true).unwrap();
            }
            rows.push(vec![
                n(0),
                11,
                n(2),
                n(3),
                i32::from(
                    sim.encounter
                        .as_ref()
                        .unwrap()
                        .scenery
                        .as_ref()
                        .is_some_and(|s| s.loaded.contains(&(0, 81))),
                ),
            ]);
            continue;
        }
        if n(1) == 9 {
            let mut setup = Setup::authored(&config).unwrap();
            setup.topology = topology::Topology::straight(1000000);
            setup.rules.combat_version = 3;
            setup.rules.collision_version = 2;
            setup.rules.scenery_version = u8::from(!legacy);
            setup.rules.bike_contacts = false;
            let mut p = profile.clone();
            p.rider = None;
            setup.rules.collision = Some(p);
            setup.seeds[0].distance = 12800;
            setup.seeds[0].speed = 30000;
            let mut other = Seed::player();
            other.id = ActorId(2);
            other.distance = 20864;
            setup.seeds.push(other);
            setup.rules.obstacles = if legacy {
                vec![]
            } else {
                vec![encounter::Obstacle {
                    id: 1,
                    path: 0,
                    progress: 20864,
                    half_length: 1,
                    left: -1024,
                    right: 1024,
                    height: 4096,
                    kind: encounter::ObstacleKind::Solid,
                    source_box: Some(encounter::StaticBox {
                        bounds: [-1024, 0, 1024, 4096],
                        lateral: 0,
                        query_bounds: None,
                        scenery: Some(crate::scenery::BoxRules {
                            class: 16,
                            road_mask: 3,
                            sample: 81,
                            selector: 0,
                            ordinal: 0,
                            box_count: 1,
                            texture_flags: None,
                        }),
                    }),
                }]
            };
            let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
            for _ in 0..n(2) {
                sim.advance().unwrap();
            }
            let state = sim.encounter.as_ref().unwrap().scenery.as_ref();
            rows.push(vec![
                n(0),
                9,
                n(2),
                sim.actors[&ActorId(1)]
                    .view
                    .rider
                    .as_ref()
                    .unwrap()
                    .bike
                    .distance_q8 as i32,
                state.and_then(|s| s.pending_camera).map_or(
                    sim.actors[&ActorId(1)]
                        .view
                        .rider
                        .as_ref()
                        .unwrap()
                        .bike
                        .distance_q8 as i32,
                    |c| c.progress as i32,
                ),
                i32::from(state.is_some_and(|s| s.loaded.contains(&(0, 81)))),
            ]);
            continue;
        }
        let sample = if n(1) == 4 {
            40
        } else if n(1) == 5 || n(1) == 8 {
            138
        } else {
            10
        };
        let local_position = if n(1) == 5 || n(1) == 8 {
            2688
        } else {
            sample * 256 + 128
        };
        let global_position = sample * 256 + 128;
        let path_id = u32::from(n(1) == 5 || n(1) == 8);
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(1000000);
        if path_id == 1 {
            let mut path = setup.topology.paths[&0].clone();
            path.selected_start = Some(32768);
            setup.topology.paths.insert(1, path);
        }
        setup.rules.combat_version = 3;
        setup.rules.collision_version = 2;
        setup.rules.scenery_version = u8::from(!legacy);
        setup.rules.scenery_start_ready = !legacy && (n(1) == 6 || n(1) == 7 || n(1) == 10);
        let mut p = profile.clone();
        p.rider = None;
        setup.rules.collision = Some(p);
        setup.rules.bike_contacts = false;
        setup.rules.obstacles.clear();
        setup.seeds[0].distance = if path_id == 1 { 2688 } else { global_position };
        let classes = if n(1) == 2 {
            vec![16, 3]
        } else {
            vec![n(2) as u8]
        };
        for (order, class) in classes.into_iter().filter(|_| !legacy).enumerate() {
            for index in 0..n(5) {
                let mut bounds = if index == 0 {
                    [-1024, 0, 1024, 4096]
                } else {
                    [4096, 0, 6144, 4096]
                };
                bounds[0] += n(3) * 256;
                bounds[2] += n(3) * 256;
                setup.rules.obstacles.push(encounter::Obstacle {
                    id: setup.rules.obstacles.len() as u64 + 1,
                    path: path_id,
                    progress: local_position,
                    half_length: 1,
                    left: bounds[0],
                    right: bounds[2],
                    height: 4096,
                    kind: encounter::ObstacleKind::Solid,
                    source_box: Some(encounter::StaticBox {
                        bounds,
                        lateral: n(3) * 256,
                        query_bounds: None,
                        scenery: Some(crate::scenery::BoxRules {
                            class,
                            road_mask: if n(1) == 8 { 3 } else { 1 },
                            sample: sample as u32,
                            selector: if order == 1 { 9 } else { 7 },
                            ordinal: (order * 4096 + index as usize) as u32,
                            box_count: n(5) as u16,
                            texture_flags: (n(1) != 5 && n(1) != 8).then_some(n(4) as u32),
                        }),
                    }),
                });
            }
        }
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        if (n(1) == 5 || n(1) == 8) && !legacy {
            let state = sim.encounter.as_mut().unwrap().scenery.as_mut().unwrap();
            let mut flags = vec![0; 128];
            flags[10] = n(4) as u32;
            if n(1) == 8 {
                state.slot_flags.insert(1, flags);
                state.slot_flags.insert(0, vec![0; 128]);
                state.object_roads.insert(2, 1);
            } else {
                state.slot_flags.insert(0, flags);
            }
        }
        if n(9) > 1 && n(1) != 7 && n(1) != 10 && !legacy {
            sim.encounter
                .as_mut()
                .unwrap()
                .scenery
                .as_mut()
                .unwrap()
                .pending_camera = Some(topology::Location {
                path: path_id,
                progress: if path_id == 1 {
                    2688
                } else {
                    i64::from(n(10).max(0))
                },
                ..Default::default()
            });
        }
        let id = ActorId(1);
        let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
        unit.previous_location = topology::Location {
            path: path_id,
            progress: local_position,
            lateral: n(6),
            ..Default::default()
        };
        unit.recovery.bike_location = unit.previous_location;
        let bike = &mut sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.distance_q8 = global_position;
        bike.lateral = n(6);
        bike.speed = n(7);
        bike.crashed = n(8) != 0;
        bike.condition = 150;
        bike.lean = 384;
        bike.drive = 256;
        bike.lateral_speed = 256;
        bike.contact_velocity = Some([n(7), 0, 0]);
        sim.encounter
            .as_mut()
            .unwrap()
            .contacts(&mut sim.actors, &config, 100, &mut sim.effects, || 3)
            .unwrap();
        if n(1) == 10 {
            sim.encounter
                .as_mut()
                .unwrap()
                .source_scenery_bike_contact(id, &mut sim.actors, &config, 100, &mut sim.effects)
                .unwrap();
        }
        let initial_query = sim
            .encounter
            .as_ref()
            .unwrap()
            .active_contacts
            .iter()
            .any(|(a, _)| *a == id.0);
        if n(1) == 7 || n(1) == 10 {
            sim.tick = Tick(101);
            for age in 1..=n(9) {
                if n(10) == 1 && age == 3 {
                    let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
                    unit.recovery.bike_location.progress += 256;
                    sim.actors
                        .get_mut(&id)
                        .unwrap()
                        .view
                        .rider
                        .as_mut()
                        .unwrap()
                        .bike
                        .distance_q8 += 256;
                }
                sim.advance().unwrap();
            }
        }
        if n(11) != 0 {
            let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
            unit.recovery.bike_location.progress = local_position;
            unit.recovery.bike_location.lateral = n(6);
            let bike = &mut sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            bike.distance_q8 = global_position;
            bike.lateral = n(6);
            sim.encounter
                .as_mut()
                .unwrap()
                .contacts(&mut sim.actors, &config, 101, &mut sim.effects, || 3)
                .unwrap();
        }
        let world = sim.encounter.as_ref().unwrap();
        let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
        let mut output: Vec<i32> = (0..12).map(n).collect();
        output.extend([
            bike.distance_q8 as i32,
            bike.lateral,
            bike.height,
            bike.speed,
            bike.lean,
            bike.lateral_speed,
            bike.condition,
            bike.scenery_timer,
            bike.drive,
        ]);
        output.extend([
            i32::from(if n(1) == 7 || n(1) == 10 {
                initial_query
            } else {
                world.active_contacts.iter().any(|(a, _)| *a == id.0)
            }),
            bike.scenery_mark.as_ref().map_or(0, |m| m.sample as i32),
            bike.scenery_mark
                .as_ref()
                .map_or(0, |m| i32::from(m.selector)),
            bike.scenery_mark
                .as_ref()
                .map_or(0, |m| i32::from(m.display_frame)),
        ]);
        output.push(bike.scenery_mark.as_ref().map_or(0, |m| i32::from(m.frame)));
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("scenery-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private source BORR placement observations"]
fn native_borr_slots() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let mut rows = vec![];
    for (index, stem) in ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"]
        .into_iter()
        .enumerate()
    {
        let mut course = rr_course::Course::read(
            &std::fs::read(data.join(format!("DATA/COURSES/{stem}.CRS"))).unwrap(),
        )
        .unwrap();
        for section in &mut course.sections {
            section.scenery.clear();
        }
        let placements = rr_course::scenery::place(&course, 1, 1).unwrap();
        for slot in placements.instances.iter().filter(|p| p.kind == "roadside") {
            let lateral = slot.lateral.unwrap();
            let path = &course.sections[slot.section as usize].samples[slot.sample as usize];
            if -path.width[0] < lateral && lateral < path.width[1] {
                rows.push(vec![
                    index as i64,
                    i64::from(slot.section),
                    i64::from(slot.sample),
                    i64::from((lateral - 224) * 256),
                    i64::from((lateral + 224) * 256),
                ]);
            }
        }
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("borr_slots-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires actual native mixed contact-walk observations"]
fn source_contact_walk_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_WALK").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    config.ground = vec![handling::Ground::flat(); 2000];
    let mut rows = vec![];
    for row in expected.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::straight(1_000_000);
        setup.rules.contact_walk_version = u8::from(std::env::var_os("RR_WALK_LEGACY").is_none());
        setup.rules.combat_version = 3;
        setup.rules.collision_version = 2;
        let mut profile =
            collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap())
                .unwrap();
        let actor_height = profile.mounted[2];
        if n(2) < 12 {
            profile.rider = None;
        }
        setup.rules.collision = Some(profile);
        setup.rules.obstacles.clear();
        if n(4) != 0 {
            setup.rules.obstacles.push(encounter::Obstacle {
                id: 1,
                path: 0,
                progress: 2688,
                half_length: 1,
                left: -1024,
                right: 1024,
                height: 4096,
                kind: encounter::ObstacleKind::Solid,
                source_box: Some(encounter::StaticBox {
                    bounds: [-1024, 0, 1024, 4096],
                    lateral: 0,
                    query_bounds: None,
                    scenery: None,
                }),
            });
        }
        setup.seeds.clear();
        for i in 0..n(1) {
            let mut seed = Seed::player();
            seed.id = ActorId(i as u64 + 1);
            seed.distance = i64::from(
                2688 + (i - 1)
                    * if n(2) < 4 {
                        20
                    } else if n(2) < 8 {
                        120
                    } else {
                        0
                    },
            );
            seed.lateral = if n(2) % 4 == 0 {
                0
            } else {
                (i - 1) * if n(2) % 4 == 1 { 1500 } else { 6000 }
            };
            seed.speed = if i == 0 { n(3) } else { 5000 };
            setup.seeds.push(seed);
        }
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        for i in 0..n(1) {
            let id = ActorId(i as u64 + 1);
            let bike = &mut sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            bike.height = if n(2) >= 20 {
                if i == 2 { 19200 } else { 0 }
            } else if n(2) >= 16 {
                0
            } else if n(2) >= 8 {
                i * (actor_height * 2 / (n(1) - 1) + n(2) % 4 - 1)
            } else if n(2) % 4 == 3 {
                i * 1000
            } else {
                0
            };
            bike.condition = 150;
            bike.contact_velocity = Some([bike.speed, 0, 0]);
            let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
            unit.recovery.bike_location.height = bike.height;
            unit.previous_location = unit.recovery.bike_location;
            unit.previous_location.progress -= if n(2) >= 16 {
                if i == 0 { 129 } else { 0 }
            } else {
                i64::from(n(3) / 256)
            };
        }
        if n(2) >= 12 && n(2) < 16 {
            let id = ActorId(1);
            let world = sim.encounter.as_mut().unwrap();
            let unit = world.units.get_mut(&id).unwrap();
            let bike = &mut sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            unit.recovery.begin_source(
                id,
                bike,
                &config,
                unit.recovery.bike_location,
                recovery::Impact::default(),
                &world.setup.topology,
            );
            bike.height = 0;
            bike.vertical_speed = 0;
            let body = unit.recovery.body.as_mut().unwrap();
            body.motion.distance_q8 = bike.distance_q8 + 10;
            body.motion.lateral = bike.lateral;
            body.motion.height = 0;
            body.motion.speed = n(3);
            body.motion.vertical_speed = 0;
            body.motion.lateral_speed = 0;
            body.motion.contact_velocity = Some([n(3), 0, 0]);
            body.location = unit.recovery.bike_location;
            body.location.progress += 10;
            body.location.height = 0;
            body.previous_location = Some(body.location);
            body.animation.set(&config.animation, 0, 33);
        }
        let mut rng = 3u32;
        sim.encounter
            .as_mut()
            .unwrap()
            .contacts(&mut sim.actors, &config, 100, &mut sim.effects, || {
                rng = rng.wrapping_mul(214013).wrapping_add(2531011);
                (rng >> 16) & 32767
            })
            .unwrap();
        let mut output: Vec<i32> = (0..5).map(n).collect();
        for i in 0..n(1) {
            let id = ActorId(i as u64 + 1);
            let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
            let u = &sim.encounter.as_ref().unwrap().units[&id];
            output.extend([
                bike.distance_q8 as i32,
                bike.lateral,
                bike.height,
                bike.speed,
                bike.lateral_speed,
                bike.vertical_speed,
                bike.condition,
                u.fighter.stats.maximum,
                u.fighter.stats.health,
                i32::from(u.recovery.phase != recovery::Phase::Mounted),
            ]);
        }
        if n(2) >= 12 {
            let body = sim.encounter.as_ref().unwrap().units[&ActorId(1)]
                .recovery
                .body
                .as_ref()
                .map(|b| &b.motion);
            output.extend(body.map_or([0; 6], |m| {
                [
                    m.distance_q8 as i32,
                    m.lateral,
                    m.height,
                    m.speed,
                    m.lateral_speed,
                    m.vertical_speed,
                ]
            }));
        }
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("collision-walk-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires actual native mounted collision trajectories"]
fn source_contact_motion_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_COLLISION_MOTION").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let base = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let mut c = base.clone();
    let mut sim = Simulation::with_encounters(1, c.clone(), Setup::authored(&c).unwrap()).unwrap();
    let mut active = None;
    let mut rows = vec![];
    for row in expected.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        if active != Some(n(0)) {
            active = Some(n(0));
            c = base.clone();
            c.selected = c.specs[n(1) as usize].id;
            c.motion_version = if std::env::var_os("RR_MOTION_LEGACY").is_some() {
                2
            } else {
                3
            };
            c.ground = vec![
                handling::Ground {
                    left: -5000,
                    right: 5000,
                    ..handling::Ground::flat()
                };
                2000
            ];
            for (j, g) in c.ground.iter_mut().enumerate() {
                g.left_shoulder = -5250;
                g.right_shoulder = 5250;
                if n(3) != 0 && (54..58).contains(&j) {
                    g.rise = 50;
                }
                if n(3) == 2 && (58..62).contains(&j) {
                    g.rise = -50;
                }
            }
            let mut setup = Setup::authored(&c).unwrap();
            setup.topology = topology::Topology::straight(1_000_000);
            setup.rules.combat_version = 3;
            setup.rules.collision_version = 2;
            setup.rules.contact_walk_version = 1;
            let mut profile =
                collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap())
                    .unwrap();
            profile.rider = None;
            setup.rules.collision = Some(profile);
            setup.rules.obstacles.clear();
            setup.seeds[0].distance = 12800;
            setup.seeds[0].speed = n(16);
            if n(4) != 0 {
                setup.rules.obstacles.push(encounter::Obstacle {
                    id: 1,
                    path: 0,
                    progress: 60 * 256 + 128,
                    half_length: 1,
                    left: n(17) * 256 - 1024,
                    right: n(17) * 256 + 1024,
                    height: if n(4) == 2 { 16384 } else { 4096 },
                    kind: encounter::ObstacleKind::Solid,
                    source_box: Some(encounter::StaticBox {
                        bounds: [
                            n(17) * 256 - 1024,
                            if n(4) == 2 { 12288 } else { 0 },
                            n(17) * 256 + 1024,
                            if n(4) == 2 { 16384 } else { 4096 },
                        ],
                        lateral: n(17) * 256,
                        query_bounds: None,
                        scenery: None,
                    }),
                });
            }
            sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
            sim.actors
                .get_mut(&ActorId(1))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike
                .condition = 150;
            if n(16) == 0 {
                let rider = sim
                    .actors
                    .get_mut(&ActorId(1))
                    .unwrap()
                    .view
                    .rider
                    .as_mut()
                    .unwrap();
                for tick in 1..=3000 {
                    rider
                        .step_with_combat_profile(
                            &c,
                            Command {
                                throttle: true,
                                ..Default::default()
                            },
                            100 + tick,
                            3,
                        )
                        .unwrap();
                }
                rider.bike.distance_q8 = 12800;
            }
        }
        let id = ActorId(1);
        let tick = n(5) as u64;
        let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
        let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
        unit.previous_location = unit.recovery.bike_location;
        unit.previous_location.lateral = bike.lateral;
        unit.previous_location.height = bike.height;
        let rider = sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap();
        rider
            .step_with_combat_profile(
                &c,
                Command {
                    throttle: true,
                    steer: if tick <= 20 { n(2) as i8 } else { 0 },
                    ..Default::default()
                },
                100 + tick as u32,
                3,
            )
            .unwrap();
        let bike = &rider.bike;
        let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
        unit.recovery.bike_location.progress = bike.distance_q8;
        unit.recovery.bike_location.lateral = bike.lateral;
        unit.recovery.bike_location.height = bike.height;
        sim.encounter
            .as_mut()
            .unwrap()
            .contacts(&mut sim.actors, &c, 100 + tick, &mut sim.effects, || 3)
            .unwrap();
        let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
        let unit = &sim.encounter.as_ref().unwrap().units[&id];
        let mut output: Vec<i32> = (0..6).map(n).collect();
        output.extend([
            bike.distance_q8 as i32,
            bike.lateral,
            bike.height,
            bike.speed,
            bike.lateral_speed,
            bike.vertical_speed,
            bike.drive,
            bike.lean,
            bike.condition,
            i32::from(unit.recovery.phase != recovery::Phase::Mounted),
        ]);
        output.push(n(16));
        output.push(n(17));
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("collision-motion-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires private source topology inventory"]
fn source_topology_identity_inventory() {
    let root = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    for stem in ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"] {
        let c = rr_course::Course::read(
            &std::fs::read(root.join(format!("DATA/COURSES/{stem}.CRS"))).unwrap(),
        )
        .unwrap();
        let mut identities = std::collections::BTreeMap::new();
        let mut conflicts = 0;
        let mut offset_conflicts = 0;
        let mut index_conflicts = 0;
        let mut lengths = std::collections::BTreeSet::new();
        for mask in 0..1usize << c.splits().len() {
            let r = c
                .route(
                    &(0..c.splits().len())
                        .map(|i| ((mask >> i) & 1) as u8)
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            lengths.insert(r.points.len());
            for (i, p) in r.points.iter().enumerate() {
                let key = (p.node, p.local);
                let value = (i, p.offset);
                if let Some(old) = identities.insert(key, value) {
                    if old != value {
                        conflicts += 1;
                    }
                    if old.0 != value.0 {
                        index_conflicts += 1;
                    }
                    if old.1 != value.1 {
                        offset_conflicts += 1;
                    }
                }
            }
        }
        eprintln!(
            "{stem}: nodesamples={} conflicts={conflicts} index={index_conflicts} offset={offset_conflicts} lengths={lengths:?}",
            identities.len()
        );
    }
}

#[test]
#[ignore = "requires actual native source fork/ground observations"]
fn source_contact_topology_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_COLLISION_TOPOLOGY").unwrap()).unwrap(),
    )
    .unwrap();
    let mut rows = vec![];
    for row in expected.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let stem = ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"][n(1) as usize];
        let course = rr_course::Course::read(
            &std::fs::read(data.join(format!("DATA/COURSES/{stem}.CRS"))).unwrap(),
        )
        .unwrap();
        let choices = (0..course.splits().len())
            .map(|bit| {
                if bit == n(3) as usize {
                    0
                } else {
                    ((n(2) >> bit) & 1) as u8
                }
            })
            .collect::<Vec<_>>();
        let route = course.route(&choices).unwrap();
        let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
        let mut setup = Setup::authored(&config).unwrap();
        setup.topology = topology::Topology::from_course(&course, &route, 0).unwrap();
        setup.seeds[0].distance = i64::from(n(4)) * 256 - 1;
        setup.seeds[0].lateral = n(6) * 256 + n(5);
        setup.seeds[0].speed = 512;
        setup.rules.obstacles.clear();
        setup.rules.combat_version = 3;
        setup.rules.collision_version = 2;
        setup.rules.scenery_version = 1;
        setup.rules.contact_walk_version = 1;
        setup.rules.collision = Some(
            collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap())
                .unwrap(),
        );
        let legacy = std::env::var_os("RR_ROUTE_LEGACY").is_some();
        if !legacy {
            setup.rules.source_roads =
                Some(source_roads::Roads::from_course(&course, &route).unwrap());
        }
        let mut sim = Simulation::with_encounters(1, config.clone(), setup).unwrap();
        let world = sim.encounter.as_mut().unwrap();
        world
            .prepare_source_roads(&sim.actors, &config, 100)
            .unwrap();
        let id = ActorId(1);
        let at = &mut world.units.get_mut(&id).unwrap().recovery.bike_location;
        world
            .setup
            .topology
            .advance(at, 2, world.navigation.get(&id).copied())
            .unwrap();
        let at = *at;
        let (half, offset, left, right) = if let Some(roads) = &world.setup.rules.source_roads {
            let profile = &roads.profiles[usize::from(world.units[&id].source_route.unwrap())];
            let identity = profile
                .identity(at.path, (at.progress >> 8) as u32)
                .unwrap();
            let ground = roads
                .ground(world.units[&id].source_route.unwrap())
                .unwrap()[n(4) as usize];
            (
                i32::from(profile.choices[n(3) as usize]),
                identity[2],
                ground.left,
                ground.right,
            )
        } else {
            let p = &route.points[n(4) as usize];
            let g = config.ground[n(4) as usize];
            (i32::from(choices[n(3) as usize]), p.offset, g.left, g.right)
        };
        let mut output: Vec<i32> = (0..7).map(n).collect();
        output.extend([
            n(4) * 256 + 1,
            half,
            at.path as i32,
            (at.progress >> 8) as i32,
            offset,
            left,
            right,
        ]);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("collision-topology-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires native held-steering fork trajectories"]
fn source_fork_motion_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_COLLISION_FORK_MOTION").unwrap()).unwrap(),
    )
    .unwrap();
    let courses = ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"].map(|stem| {
        rr_course::Course::read(
            &std::fs::read(data.join(format!("DATA/COURSES/{stem}.CRS"))).unwrap(),
        )
        .unwrap()
    });
    let route = courses[0]
        .route(&vec![0; courses[0].splits().len()])
        .unwrap();
    let mut config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    config.motion_version = 3;
    let collision =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut bases = std::collections::BTreeMap::new();
    let mut active = None;
    let mut rows = vec![];
    let mut sim =
        Simulation::with_encounters(1, config.clone(), Setup::authored(&config).unwrap()).unwrap();
    for row in expected.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        if active != Some(n(0)) {
            active = Some(n(0));
            let mask = (n(2) & !(1 << n(3))) as u8;
            let key = (n(1), mask);
            let (configuration, base) = bases.entry(key).or_insert_with(|| {
                let course = &courses[n(1) as usize];
                let choices = (0..course.splits().len())
                    .map(|bit| (mask >> bit) & 1)
                    .collect::<Vec<_>>();
                let route = course.route(&choices).unwrap();
                let roads = source_roads::Roads::from_course(course, &route).unwrap();
                let mut c = config.clone();
                c.ground = roads.ground(mask).unwrap();
                let mut setup = Setup::authored(&c).unwrap();
                setup.topology = topology::Topology::from_course(course, &route, 0).unwrap();
                setup.rules.finish_progress = Some(200_000_000);
                setup.rules.obstacles.clear();
                setup.rules.combat_version = 3;
                setup.rules.collision_version = 2;
                setup.rules.scenery_version = 1;
                setup.rules.contact_walk_version = 1;
                setup.rules.collision = Some(collision.clone());
                setup.rules.source_roads = Some(roads);
                (c, setup)
            });
            let mut setup = base.clone();
            let roads = setup.rules.source_roads.as_ref().unwrap();
            let (&incoming, fork) = roads
                .forks
                .iter()
                .find(|(_, fork)| fork.bit == n(3) as u8)
                .unwrap();
            let profile = &roads.profiles[usize::from(mask)];
            let sample = profile.selected[&incoming] + roads.ground[&incoming].len() as u32;
            let edge = profile.identity(fork.halves[0], 0).unwrap()[2]
                + roads.ground[&fork.halves[0]][0].right;
            setup.seeds[0].distance = i64::from(sample) * 256 - 180;
            setup.seeds[0].lateral = edge * 256 + n(4);
            setup.seeds[0].speed = n(6);
            sim = Simulation::with_encounters(1, configuration.clone(), setup).unwrap();
            sim.actors
                .get_mut(&ActorId(1))
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike
                .condition = 150;
        }
        sim.step(Command {
            steer: n(5) as i8,
            ..Default::default()
        })
        .unwrap();
        let world = sim.encounter.as_ref().unwrap();
        let unit = &world.units[&ActorId(1)];
        let bike = &sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().bike;
        let profile = &world.setup.rules.source_roads.as_ref().unwrap().profiles
            [usize::from(unit.source_route.unwrap())];
        let at = unit.recovery.bike_location;
        let identity = profile
            .identity(at.path, (at.progress >> 8) as u32)
            .unwrap();
        let mut output: Vec<i32> = (0..8).map(n).collect();
        output.extend([
            bike.distance_q8 as i32,
            bike.lateral,
            bike.height,
            bike.speed,
            i32::from(profile.choices[n(3) as usize]),
            at.path as i32,
            (at.progress >> 8) as i32,
            identity[2],
        ]);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("collision-fork-motion-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires actual native scenery queries at branch offsets"]
fn source_branch_scenery_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_BRANCH_SCENERY").unwrap()).unwrap(),
    )
    .unwrap();
    let courses = ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"].map(|stem| {
        rr_course::Course::read(
            &std::fs::read(data.join(format!("DATA/COURSES/{stem}.CRS"))).unwrap(),
        )
        .unwrap()
    });
    let route = courses[0]
        .route(&vec![0; courses[0].splits().len()])
        .unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let collision =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut bases = std::collections::BTreeMap::new();
    let mut rows = vec![];
    for row in expected.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let (c, base) = bases.entry(n(1)).or_insert_with(|| {
            let course = &courses[n(1) as usize];
            let route = course.route(&vec![0; course.splits().len()]).unwrap();
            let roads = source_roads::Roads::from_course(course, &route).unwrap();
            let mut c = config.clone();
            c.ground = roads.ground(0).unwrap();
            let mut setup = Setup::authored(&c).unwrap();
            setup.topology = topology::Topology::from_course(course, &route, 0).unwrap();
            setup.rules.combat_version = 3;
            setup.rules.collision_version = 2;
            setup.rules.contact_walk_version = 1;
            setup.rules.scenery_version = 1;
            setup.rules.scenery_start_ready = true;
            setup.rules.collision = Some(collision.clone());
            setup.rules.obstacles.clear();
            setup.rules.source_roads = Some(roads);
            (c, setup)
        });
        let mut setup = base.clone();
        setup.seeds[0].source_route = Some(n(2) as u8);
        setup.seeds[0].distance = i64::from(n(6)) * 256 + 128;
        setup.seeds[0].lateral = n(9) * 256;
        setup.seeds[0].speed = 5000;
        setup.seeds[0].location = Some(topology::Location {
            path: n(7) as u32,
            progress: i64::from(n(8)) * 256 + 128,
            lateral: n(9) * 256,
            height: n(5),
            heading: 0,
        });
        setup.rules.obstacles.push(encounter::Obstacle {
            id: 1,
            path: n(7) as u32,
            progress: i64::from(n(8)) * 256 + 128,
            half_length: 1,
            left: -1024,
            right: 1024,
            height: 4096,
            kind: if n(4) == 3 {
                encounter::ObstacleKind::Decorative
            } else {
                encounter::ObstacleKind::Solid
            },
            source_box: Some(encounter::StaticBox {
                bounds: [-1024, 0, 1024, 4096],
                lateral: 0,
                query_bounds: None,
                scenery: Some(scenery::BoxRules {
                    class: n(4) as u8,
                    road_mask: 3,
                    sample: n(8) as u32,
                    selector: 7,
                    ordinal: 0,
                    box_count: 1,
                    texture_flags: None,
                }),
            }),
        });
        let mut sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
        let bike = &mut sim
            .actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.condition = 150;
        bike.contact_velocity = Some([5000, 0, 0]);
        sim.encounter
            .as_mut()
            .unwrap()
            .contacts(&mut sim.actors, c, 100, &mut sim.effects, || 3)
            .unwrap();
        let unit = &sim.encounter.as_ref().unwrap().units[&ActorId(1)];
        let bike = &sim.actors[&ActorId(1)].view.rider.as_ref().unwrap().bike;
        let mut output: Vec<i32> = (0..10).map(n).collect();
        output.extend([
            bike.distance_q8 as i32,
            bike.lateral,
            bike.height,
            bike.speed,
            bike.lateral_speed,
            bike.vertical_speed,
            bike.condition,
            bike.scenery_timer,
            unit.fighter.stats.maximum,
            unit.fighter.stats.health,
            i32::from(unit.recovery.phase != recovery::Phase::Mounted),
            bike.scenery_mark.as_ref().map_or(0, |m| m.sample as i32),
        ]);
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("collision-branch-scenery-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires pinned native per-route finish ownership observations"]
fn source_topology_finishes_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_TOPOLOGY_FINISHES").unwrap()).unwrap(),
    )
    .unwrap();
    let courses = ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"].map(|stem| {
        rr_course::Course::read(
            &std::fs::read(data.join(format!("DATA/COURSES/{stem}.CRS"))).unwrap(),
        )
        .unwrap()
    });
    let mut contexts = std::collections::BTreeMap::new();
    let mut rows = vec![];
    for row in expected.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let course = &courses[n(0) as usize];
        let roads = contexts.entry((n(0), n(2))).or_insert_with(|| {
            let route = course.route(&vec![0; course.splits().len()]).unwrap();
            let topology = topology::Topology::from_course(course, &route, 50).unwrap();
            let mut roads = source_roads::Roads::from_course(course, &route).unwrap();
            roads.set_finishes(course, n(2) as u8, &topology).unwrap();
            roads.validate(&topology).unwrap();
            (roads, topology)
        });
        let origin = roads.0.origin(&roads.1).unwrap();
        let finish = roads.0.profiles[n(1) as usize].finish_progress.unwrap();
        let location = roads.0.location(n(1) as u8, finish, origin, 0, 0).unwrap();
        rows.push(vec![
            n(0),
            n(1),
            n(2),
            ((finish + origin) / 256) as i32,
            location.path as i32,
            (location.progress / 256) as i32,
        ]);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("topology-finishes-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires actual native callback-driven fork displacements"]
fn source_fork_contact_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_FORK_CONTACT").unwrap()).unwrap(),
    )
    .unwrap();
    let courses = ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"].map(|stem| {
        rr_course::Course::read(
            &std::fs::read(data.join(format!("DATA/COURSES/{stem}.CRS"))).unwrap(),
        )
        .unwrap()
    });
    let route = courses[0]
        .route(&vec![0; courses[0].splits().len()])
        .unwrap();
    let config = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let collision =
        collision::Profile::from_exe(&std::fs::read(data.join("ROADRASH.EXE")).unwrap()).unwrap();
    let mut bases = std::collections::BTreeMap::new();
    let mut rows = vec![];
    for row in expected.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap() as i32;
        let mask = (n(1) & !(1 << n(2))) as u8;
        let (c, base) = bases.entry((n(0), mask)).or_insert_with(|| {
            let course = &courses[n(0) as usize];
            let route = course
                .route(
                    &(0..course.splits().len())
                        .map(|bit| (mask >> bit) & 1)
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            let roads = source_roads::Roads::from_course(course, &route).unwrap();
            let mut c = config.clone();
            c.ground = roads.ground(mask).unwrap();
            let mut setup = Setup::authored(&c).unwrap();
            setup.topology = topology::Topology::from_course(course, &route, 0).unwrap();
            setup.rules.combat_version = 3;
            setup.rules.collision_version = 2;
            setup.rules.contact_walk_version = 1;
            setup.rules.scenery_version = 1;
            setup.rules.collision = Some(collision.clone());
            setup.rules.obstacles.clear();
            setup.rules.source_roads = Some(roads);
            (c, setup)
        });
        let mut setup = base.clone();
        setup.seeds[0].distance = i64::from(n(4)) * 256 - 40;
        setup.seeds[0].lateral = n(5) * 256 + n(3);
        setup.seeds[0].speed = 30000;
        let mut second = Seed::player();
        second.id = ActorId(2);
        second.distance = i64::from(n(4)) * 256 - 5;
        second.lateral = n(5) * 256 + n(3);
        setup.seeds.push(second);
        let mut sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
        for i in 0..2 {
            let id = ActorId(i + 1);
            let bike = &mut sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap()
                .bike;
            bike.condition = 150;
            bike.contact_velocity = Some([bike.speed, 0, 0]);
            if i == 0 {
                let unit = sim.encounter.as_mut().unwrap().units.get_mut(&id).unwrap();
                unit.previous_location.progress -= 117;
                unit.previous.progress -= 117;
            }
        }
        sim.encounter
            .as_mut()
            .unwrap()
            .contacts(&mut sim.actors, c, 100, &mut sim.effects, || 3)
            .unwrap();
        let mut output: Vec<i32> = (0..6).map(n).collect();
        let world = sim.encounter.as_ref().unwrap();
        for i in 0..2 {
            let id = ActorId(i + 1);
            let unit = &world.units[&id];
            let bike = &sim.actors[&id].view.rider.as_ref().unwrap().bike;
            let profile = &world.setup.rules.source_roads.as_ref().unwrap().profiles
                [usize::from(unit.source_route.unwrap())];
            output.extend([
                i32::from(profile.choices[n(2) as usize]),
                unit.recovery.bike_location.path as i32,
                (unit.recovery.bike_location.progress >> 8) as i32,
                bike.distance_q8 as i32,
                bike.lateral,
                bike.height,
                bike.speed,
                bike.lateral_speed,
                bike.vertical_speed,
                bike.condition,
                i32::from(unit.recovery.phase != recovery::Phase::Mounted),
            ]);
        }
        rows.push(output);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("collision-fork-contact-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}
