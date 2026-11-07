//! Private exact police AI transactions against the pinned native decision call.
use super::*;
use population::police_ai::{AiState, Context, ObjectKey, Relations, RiderView, Road, ScanObject};
use serde_json::{Value, json};
#[test]
#[ignore = "requires pinned native source police decision transactions"]
fn source_police_ai_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let config = population::police::Config::read(&exe).unwrap();
    let native: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_POLICE_AI").unwrap()).unwrap(),
    )
    .unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let base = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    let configs: Vec<_> = (0..5)
        .map(|level| config.configuration(&base, level).unwrap())
        .collect();
    let mut result = vec![];
    for case in native.as_array().unwrap() {
        let input = &case["input"];
        let n = |i: usize| input[i].as_i64().unwrap() as i32;
        let level = n(1) as u8;
        let c = &configs[usize::from(level)];
        let meta = case.get("meta");
        let own_id = ActorId(meta.map_or(2, |m| m[1].as_u64().unwrap() + 1));
        let initial_code = meta.map_or(2, |m| m[8].as_i64().unwrap() as i8);
        let initial = &case["initial"];
        let i = |at: usize| initial[at].as_i64().unwrap() as i32;
        let (words, _) = config.pursuit(level, 0, 0).unwrap();
        let mut ai =
            AiState::from_words(&words, n(9) as u8, level, animation::Weapon::Club).unwrap();
        ai.initialized = true;
        ai.target = Some(meta.map_or(ActorId(1), |m| ActorId(m[0].as_u64().unwrap() + 1)));
        ai.ahead = (i(4) >= 0).then(|| ActorId(i(4) as u64 + 1));
        ai.behind = (i(5) >= 0).then(|| ActorId(i(5) as u64 + 1));
        for (off, value) in [
            (0x160, n(3)),
            (
                0x174,
                if n(3) == 6 {
                    0
                } else if n(3) <= 2 {
                    1
                } else if n(3) == 5 {
                    2
                } else {
                    3
                },
            ),
            (0x188, n(7)),
            (0x198, 13760),
            (0x194, 0),
            (0x19c, 100),
            (0x1a4, i(2)),
            (0x1a0, i(3)),
            (0x178, i(6)),
            (0x17c, i(7)),
        ] {
            ai.set(off, value);
        }
        ai.words[0x164 / 4] = (ai.words[0x164 / 4] & !255) | 1;
        for at in 0..30 {
            ai.words[at] = initial[8][at].as_i64().unwrap() as i32;
            ai.words[0x78 / 4 + at] = initial[9][at].as_i64().unwrap() as i32;
        }
        let mut relation = Relations::from_words(&config.personalities[0]).unwrap();
        for at in 0..30 {
            relation.current[at] = initial[10][at].as_i64().unwrap() as i32;
            relation.target[at] = initial[11][at].as_i64().unwrap() as i32;
        }
        let road = Road {
            center: 0,
            left: -500,
            right: 500,
            lanes: [512, 512],
        };
        let length = i(1);
        let target = RiderView {
            id: ActorId(1),
            position: 12800 + n(4),
            lateral: n(5),
            speed: 5000,
            captured_speed: 5000,
            half_lateral: n(5),
            length,
            crash: n(8),
            present: true,
            hit_until: 0,
            index: relation.index,
            road,
            relations: Some(relation.clone()),
        };
        let own = RiderView {
            id: ActorId(2),
            position: 12800,
            lateral: 0,
            speed: n(6),
            captured_speed: n(6),
            half_lateral: 0,
            length,
            crash: 0,
            present: true,
            hit_until: 0,
            index: ai.index(),
            road,
            relations: None,
        };
        let mut riders = std::collections::BTreeMap::from([(target.id, target), (own.id, own)]);
        let mut relations = std::collections::BTreeMap::from([(ActorId(1), relation.clone())]);
        if let Some(roster) = case.get("roster") {
            riders.clear();
            relations.clear();
            for row in roster.as_array().unwrap() {
                let v = |at: usize| row[at].as_i64().unwrap() as i32;
                let id = ActorId(v(0) as u64 + 1);
                let mut r = Relations {
                    index: v(13) as u8,
                    current: [0; 30],
                    target: [0; 30],
                    reaction: [0; 15],
                };
                for at in 0..30 {
                    r.current[at] = row[14][at].as_i64().unwrap() as i32;
                    r.target[at] = row[15][at].as_i64().unwrap() as i32;
                }
                for at in 0..15 {
                    r.reaction[at] = row[16][at].as_i64().unwrap() as i32;
                }
                relations.insert(id, r.clone());
                riders.insert(
                    id,
                    RiderView {
                        id,
                        position: v(1),
                        lateral: v(2),
                        speed: v(3),
                        captured_speed: v(4),
                        length: v(5),
                        crash: v(6),
                        present: v(7) != 0,
                        hit_until: v(8) as u32,
                        half_lateral: v(10),
                        index: r.index,
                        road,
                        relations: Some(r),
                    },
                );
            }
        }
        let mut objects = vec![];
        for (order, r) in riders.values().enumerate() {
            for kind in [4, 8] {
                objects.push(ScanObject {
                    key: ObjectKey {
                        actor: Some(r.id),
                        agent: None,
                        kind,
                    },
                    position: r.position,
                    lateral: r.lateral,
                    speed: r.speed,
                    present: true,
                    road,
                    order: 0x10000000 + order as u64 * 2 + u64::from(kind == 8),
                });
            }
        }
        if let Some(roster) = case.get("roster") {
            objects.clear();
            for row in roster.as_array().unwrap() {
                let v = |at: usize| row[at].as_i64().unwrap() as i32;
                let index = v(0) as u64;
                let id = ActorId(index + 1);
                for kind in [4u8, 8] {
                    let half = kind == 8;
                    objects.push(ScanObject {
                        key: ObjectKey {
                            actor: Some(id),
                            agent: None,
                            kind,
                        },
                        position: v(if half { 9 } else { 1 }),
                        lateral: v(if half { 10 } else { 2 }),
                        speed: v(if half { 11 } else { 4 }),
                        present: v(if half { 12 } else { 7 }) != 0,
                        road,
                        order: 0x10000000 + index * 2 + u64::from(half),
                    });
                }
            }
            for row in case["non_riders"].as_array().unwrap() {
                let v = |at: usize| row[at].as_i64().unwrap();
                objects.push(ScanObject {
                    key: ObjectKey {
                        actor: None,
                        agent: Some(1),
                        kind: v(0) as u8,
                    },
                    position: v(1) as i32,
                    lateral: v(2) as i32,
                    speed: v(3) as i32,
                    present: true,
                    road,
                    order: v(4) as u64,
                });
            }
        }
        objects.sort_by_key(|o| (o.position, o.order));
        let bend = n(2);
        let curve = i32::from(config.atan[bend.unsigned_abs() as usize])
            * if bend < 0 { -16384 } else { 16384 };
        let ctx = Context {
            before_wrap: std::env::var_os("RR_POLICE_AI_BEFORE_WRAP").is_some(),
            own: own_id,
            tick: input.get(12).map_or(100, |v| v.as_u64().unwrap()) as u32,
            busted: false,
            riders: &riders,
            objects: &objects,
            current_road: road,
            next_road: road,
            curve_current: curve,
            curve_next: curve,
            curve_limit: curve,
            eagerness: config.overtake_eagerness[usize::from(level)],
            place: meta.map_or(0, |m| m[2].as_i64().unwrap() as i32),
        };
        let mut bike = handling::Bike::new(c.spec(), &c.tables).unwrap();
        bike.distance_q8 = 12800;
        bike.speed = n(6);
        bike.contact = meta.map_or(37, |m| m[5].as_i64().unwrap() as i32);
        bike.ai_roll = Some(meta.map_or(0, |m| m[6].as_i64().unwrap() as i32));
        bike.skid = meta.map_or(0, |m| m[4].as_u64().unwrap() as u8);
        bike.lean = n(10);
        bike.previous_lean = 0;
        bike.gear = i(0) as usize;
        let mut seed = n(11) as u32;
        let mut draw = || {
            seed = seed.wrapping_mul(214013).wrapping_add(2531011);
            (seed >> 16) & 32767
        };
        let legacy = std::env::var_os("RR_POLICE_AI_LEGACY").is_some();
        let legacy_point = if legacy {
            assert!(
                meta.is_none(),
                "Frozen scaffold baseline is the two-rider decision contract"
            );
            let distance = n(4);
            let chasing = distance.abs() < 8192;
            let gap = distance.abs() >> 4;
            let off = (gap * 4096 + 14000).min(28000);
            let target_x = if chasing {
                if n(5) < 0 && n(5) + off < 500 * 256 {
                    n(5) + off
                } else {
                    n(5) - off
                }
            } else {
                0
            };
            let shown = (bike.speed >> 12) + (bike.speed >> 7) + (bike.speed >> 9);
            let d = (distance >> 4) + 1;
            let mut desired = if chasing {
                if d > 500 {
                    i(2)
                } else if d <= -500 {
                    i(3)
                } else {
                    ((if d < 0 { d >> 3 } else { d }) + ((5000 >> 12) + (5000 >> 7) + (5000 >> 9)))
                        .clamp(i(3), i(2))
                }
            } else {
                i(2)
            };
            desired = desired.clamp(shown - 5, shown + 5);
            if chasing {
                if n(8) == 1 {
                    desired = 0;
                } else if n(8) != 0 {
                    desired = desired.min(20);
                }
            }
            let angle = riding::atan2(distance.max(960), target_x / 256);
            let signed = if angle > 127 { angle - 256 } else { angle };
            let axis = (signed * 32767 / 32).clamp(-32767, 32767);
            bike.source_drive_mode(
                c.spec(),
                &c.tables,
                if shown < desired {
                    1
                } else if shown > desired {
                    2
                } else {
                    0
                },
            );
            bike.source_steer(
                c.spec(),
                &c.tables,
                ((i64::from(axis) * 0x200000) / 32767) as i32,
                0,
            );
            let mut legacy_code = 2;
            if chasing && distance.abs() < 480 && n(12) % 30 == 0 {
                let code = c.animation.attacks[usize::from(n(5) > 0)][0][1];
                let mut p = animation::Player::default();
                p.set(&c.animation, 0, code);
                legacy_code = p.code;
            }
            Some((target_x, desired, legacy_code))
        } else {
            None
        };
        let carried = match meta.map_or(1, |m| m[9].as_i64().unwrap()) {
            0 => animation::Weapon::Fists,
            2 => animation::Weapon::Chain,
            _ => animation::Weapon::Club,
        };
        let decision = if legacy {
            population::police_ai::Decision {
                request: None,
                target: None,
            }
        } else {
            ai.tick(&ctx, &mut bike, c, initial_code, carried, &mut draw)
                .unwrap()
        };
        let (mut code, mut back, mut right) = (initial_code, false, false);
        if let Some(request) = decision.request {
            back = request.kind == combat::AttackKind::Backhand;
            right = request.side == combat::Side::Right;
            code = c.animation.attacks[usize::from(right)][usize::from(back)]
                [usize::from(ai.attack_weapon)];
            let mut player = animation::Player::default();
            player.set(&c.animation, 0, code);
            code = player.code;
            if code != initial_code {
                let relation = relations.get_mut(&decision.target.unwrap()).unwrap();
                let index = usize::from(ai.index()) * 3;
                if relation.target[index + 1] - relation.target[index] + relation.target[index + 2]
                    == relation.current[index + 1] - relation.current[index]
                        + relation.current[index + 2]
                {
                    relation.react(ai.index(), 1).unwrap();
                }
                relation.react(ai.index(), 0).unwrap();
            }
        }
        let target_index = |id: Option<ActorId>| id.map_or(-1, |id| id.0 as i32 - 1);
        let over = ai
            .overtake
            .and_then(|key| objects.iter().find(|o| o.key == key));
        relation = relations[&ActorId(1)].clone();
        let mut values = json!([
            seed,
            ai.get(0x160),
            target_index(ai.target),
            ai.get(0x174),
            ai.get(0x194),
            ai.get(0x198),
            ai.get(0x19c),
            ai.get(0x1a4),
            ai.get(0x1a0),
            ai.get(0x188),
            target_index(ai.ahead),
            target_index(ai.behind),
            ai.get(0x178),
            ai.get(0x17c),
            over.map_or(0, |o| o.key.kind),
            over.map_or(-1, |o| target_index(o.key.actor)),
            over.map_or(0, |o| o.position),
            bike.force,
            bike.drive,
            bike.lean,
            bike.previous_lean,
            bike.lateral_speed,
            bike.speed,
            ai.attack_weapon,
            code,
            i32::from(back),
            i32::from(right),
            config.stamina().maximum - 100
                + match ctx.riders[&own_id].crash {
                    0 => 1,
                    1 => 0,
                    _ => 2,
                },
            ai.words[0x164 / 4] & 255,
            &ai.words[0x138 / 4..0x160 / 4],
            &ai.words[..30],
            relation.current,
            relation.target
        ]);
        if let Some((x, speed, legacy_code)) = legacy_point {
            let row = values.as_array_mut().unwrap();
            row[4] = json!(x);
            row[6] = json!(speed);
            row[24] = json!(legacy_code);
        }
        if meta.is_some() {
            values.as_array_mut().unwrap().push(json!(
                relations
                    .iter()
                    .filter(|(id, _)| **id != own_id)
                    .map(|(_, r)| r.current)
                    .collect::<Vec<_>>()
            ));
        }
        result.push(values);
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("police-ai-rust.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires source native raw-lane geometry inventory"]
fn source_police_ai_geometry_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let mut rows = vec![];
    for (ci, stem) in ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"]
        .into_iter()
        .enumerate()
    {
        let course = rr_course::Course::read(
            &std::fs::read(data.join(format!("DATA/COURSES/{stem}.CRS"))).unwrap(),
        )
        .unwrap();
        let route = course.route(&vec![0; course.splits().len()]).unwrap();
        let roads = source_roads::Roads::from_course(&course, &route).unwrap();
        for (path, lanes) in roads.lanes_q8.unwrap() {
            for (local, lane) in lanes.into_iter().enumerate() {
                rows.push(json!([ci, path, local, lane[0], lane[1]]));
            }
        }
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        out.join("police-ai-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
}
