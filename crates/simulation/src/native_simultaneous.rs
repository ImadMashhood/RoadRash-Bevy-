//! Opt-in live-target multi-rider combat differential tests.
use crate::{
    combat::*,
    encounter::{Seed, Setup},
    *,
};
use serde_json::{Value, json};
#[test]
#[ignore = "requires private live-list native combat trajectories"]
fn native_simultaneous_combat() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_SIMULTANEOUS_EXPECTED").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut c = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    c.ground = vec![handling::Ground::flat(); 2000];
    c.specs
        .iter_mut()
        .find(|s| s.id == c.selected)
        .unwrap()
        .condition = 150;
    let pe_bytes = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let pe = rr_formats::pe::SupportedExe::read(&pe_bytes).unwrap();
    let steal = std::array::from_fn(|i| {
        rr_formats::binary::u32le(pe.read_va(0x464538 + i as u32 * 4, 4).unwrap(), 0).unwrap() as i8
    });
    let profile = std::env::var("RR_SIMULTANEOUS_PROFILE")
        .unwrap()
        .parse::<u8>()
        .unwrap();
    let mut cases = vec![];
    for case in expected.as_array().unwrap() {
        let n = |key: &str| case[key].as_i64().unwrap() as i32;
        let count = n("count");
        let weapon = n("weapon");
        let kind = n("kind");
        let side = n("side");
        let mode = n("mode");
        let mut seed = n("seed") as u32;
        let mut setup = Setup::authored(&c).unwrap();
        setup.rules.combat_version = profile;
        setup.rules.steal_codes = Some(steal);
        setup.rules.bike_contacts = false;
        setup.rules.obstacles.clear();
        setup.seeds.clear();
        for a in 0..count {
            let mut s = Seed::player();
            s.id = ActorId(a as u64 + 1);
            s.distance = 1000;
            s.stats.strength = 1;
            if mode == 6 && a == 0 {
                s.stats.health = 64;
            }
            setup.seeds.push(s);
            let carried = if mode == 4 {
                if a == 1 { 1 } else { 0 }
            } else if weapon == 3 {
                0
            } else {
                weapon
            };
            if carried > 0 {
                setup.inventory.items.insert(
                    WeaponId(a as u64 + 1),
                    Item {
                        kind: if carried == 1 {
                            animation::Weapon::Club
                        } else {
                            animation::Weapon::Chain
                        },
                        owner: Some(ActorId(a as u64 + 1)),
                    },
                );
            }
        }
        let mut sim = Simulation::with_encounters(1, c.clone(), setup).unwrap();
        if mode == 5 {
            let mut cop = c.clone();
            cop.animation.banks[0] = cop.animation.banks[1].clone();
            cop.animation.views[0] = cop.animation.views[1].clone();
            sim.encounter
                .as_mut()
                .unwrap()
                .configs
                .insert(ActorId(count as u64), cop);
        }
        let mut rows = vec![];
        for tick in 0..96u64 {
            let world = sim.encounter.as_mut().unwrap();
            for a in 0..count {
                let id = ActorId(a as u64 + 1);
                let mut gap = if a == 0 {
                    0
                } else if a % 2 == 1 {
                    16
                } else {
                    -16
                };
                if mode == 1 || (mode == 7 && tick >= 24) {
                    gap = 0;
                }
                if mode == 2 && a == 1 {
                    gap = if tick < 12 {
                        64
                    } else if tick < 48 {
                        63
                    } else {
                        64
                    };
                }
                if mode == 3 && a == count - 1 {
                    gap = if tick < 24 { -8 } else { 1000 };
                }
                let lateral = side * ((a % 3) - 1) * 16000;
                let height = if mode == 2 && a == 2 {
                    if tick < 24 { 12288 } else { 12287 }
                } else {
                    0
                };
                let actor = sim.actors.get_mut(&id).unwrap();
                let r = actor.view.rider.as_mut().unwrap();
                r.bike.distance_q8 = 1000 + i64::from(gap);
                r.bike.lateral = lateral;
                r.bike.height = height;
                r.bike.speed = 100;
                r.bike.lean = 0;
                let u = world.units.get_mut(&id).unwrap();
                u.recovery.bike_location.progress = r.bike.distance_q8;
                u.recovery.bike_location.lateral = lateral;
                u.recovery.bike_location.height = height;
                let held = if mode == 4 {
                    if a == 1 {
                        tick >= 10
                    } else {
                        !(12..40).contains(&tick)
                    }
                } else {
                    tick == 0 || mode == 0 || mode == 1 || tick.is_multiple_of(40)
                };
                actor.input = Command {
                    attack: held.then_some(Request {
                        kind: if weapon == 3 {
                            AttackKind::Kick
                        } else if kind == 1 {
                            AttackKind::Backhand
                        } else {
                            AttackKind::Punch
                        },
                        side: Side::Auto,
                    }),
                    ..Default::default()
                };
            }
            let mut events = vec![];
            let captured: Vec<_> = sim
                .actors
                .iter()
                .map(|(id, a)| {
                    let b = &a.view.rider.as_ref().unwrap().bike;
                    (*id, b.distance_q8, b.lateral, b.height, b.speed, b.lean)
                })
                .collect();
            if profile < 4 {
                // Legacy initiation lives in movement. Freeze already detached
                // actors while using that adapter: this probe schedules fight
                // callbacks, not crash physics/animation callbacks.
                let detached: Vec<_> = world
                    .units
                    .iter()
                    .filter(|(_, u)| u.recovery.phase != recovery::Phase::Mounted)
                    .map(|(id, u)| (*id, u.clone(), sim.actors[id].view.rider.clone()))
                    .collect();
                world
                    .movement(&mut sim.actors, &c, tick, &[], &mut events, || 0)
                    .unwrap();
                for (id, unit, rider) in detached {
                    world.units.insert(id, unit);
                    sim.actors.get_mut(&id).unwrap().view.rider = rider;
                }
                for (id, progress, lateral, height, speed, lean) in captured {
                    let r = sim
                        .actors
                        .get_mut(&id)
                        .unwrap()
                        .view
                        .rider
                        .as_mut()
                        .unwrap();
                    r.bike.distance_q8 = progress;
                    r.bike.lateral = lateral;
                    r.bike.height = height;
                    r.bike.speed = speed;
                    r.bike.lean = lean;
                    let u = world.units.get_mut(&id).unwrap();
                    u.recovery.bike_location.progress = progress;
                    u.recovery.bike_location.lateral = lateral;
                    u.recovery.bike_location.height = height;
                }
            }
            world
                .source_combat_probe_tick(
                    &mut sim.actors,
                    &c,
                    tick,
                    |_, _| {
                        seed = seed.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
                        (seed >> 16) & 0x7fff
                    },
                    &mut events,
                )
                .unwrap_or_else(|error| panic!("simultaneous case {} count {count} weapon {weapon} kind {kind} side {side} mode {mode} tick {tick}: {error:?}; actors {:?}", n("id"), sim.actors.iter().map(|(id,a)|(*id,a.view.rider.as_ref().unwrap().animation.clone())).collect::<Vec<_>>()));
            world.inventory.validate().unwrap();
            for id in world.units.keys() {
                let carried = world
                    .inventory
                    .carried(*id)
                    .map_or(animation::Weapon::Fists, |(_, w)| w);
                assert_eq!(
                    sim.actors[id].view.rider.as_ref().unwrap().weapon,
                    carried,
                    "case {} tick {tick}: carried weapon projection",
                    n("id")
                );
            }
            let mut row = vec![tick as i64, i64::from(seed)];
            for a in 0..count {
                let id = ActorId(a as u64 + 1);
                let u = &world.units[&id];
                let r = sim.actors[&id].view.rider.as_ref().unwrap();
                let animation = u
                    .recovery
                    .body
                    .as_ref()
                    .map_or(&r.animation, |b| &b.animation);
                row.extend([
                    i64::from(u.fighter.stats.maximum),
                    i64::from(u.fighter.stats.health),
                    i64::from(r.bike.condition),
                    i64::from(r.bike.speed),
                    i64::from(r.bike.lean),
                    u.fighter.hit_until as i64,
                    u.fighter.steal_until as i64,
                    r.weapon as i64,
                    i64::from(animation.code),
                    animation.frame as i64,
                    i64::from(animation.queued.unwrap_or(-1)),
                    i64::from(animation.loops),
                    i64::from(u.recovery.body.is_some()),
                    u.recovery
                        .body
                        .as_ref()
                        .map_or(-99, |b| i64::from(b.animation.code)),
                    i64::from(
                        u.recovery
                            .body
                            .as_ref()
                            .is_some_and(|b| b.animation.selected.is_some()),
                    ),
                    i64::from(u.recovery.bike_animation.selected.is_some()),
                ]);
            }
            rows.push(row);
        }
        let mut out = case.clone();
        out["rows"] = json!(rows);
        cases.push(out);
    }
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("RR_SIMULTANEOUS_OUTPUT").unwrap())
            .join("rust.json"),
        serde_json::to_vec(&cases).unwrap(),
    )
    .unwrap();
}
