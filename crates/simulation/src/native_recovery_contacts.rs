//! Controlled crowded recovery contact callbacks; no terrain/motion scheduler claim.
use super::*;
use serde_json::{Value, json};

fn crowded_fixture(version: u8) -> Simulation {
    let mut c = fixtures::riding();
    c.specs
        .iter_mut()
        .find(|s| s.id == c.selected)
        .unwrap()
        .condition = 150;
    let mut e = encounter::Setup::authored(&c).unwrap();
    e.topology = topology::Topology::straight(512000);
    e.rules.combat_version = 4;
    e.rules.collision_version = 2;
    e.rules.contact_walk_version = version;
    e.rules.collision = Some(collision::Profile {
        mounted: [7680, 64, 8960],
        fallen: [7680, 64, 2048],
        mass: 256,
        elasticity: 192,
        rider: Some(collision::RiderProfile {
            dimensions: [6400, 16, 7680],
            mass: 25,
            elasticity: 192,
        }),
    });
    e.rules.body.recovery_version = 2;
    let seed = e.seeds.remove(0);
    for i in 0..4 {
        let mut next = seed.clone();
        next.id = ActorId(i + 1);
        next.distance = 2688 + i as i64 * 15;
        next.lateral = (i as i32 - 1) * 2000;
        next.speed = 0;
        e.seeds.push(next);
    }
    let mut sim = Simulation::with_encounters(1, c.clone(), e).unwrap();
    let w = sim.encounter.as_mut().unwrap();
    for (id, u) in &mut w.units {
        let b = &mut sim
            .actors
            .get_mut(id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        b.contact_velocity = Some([0; 3]);
        assert!(u.recovery.begin_live_source(
            *id,
            b,
            &c,
            u.recovery.bike_location,
            recovery::Impact {
                reason: 1,
                rider_velocity: Some([0; 3]),
                bike_velocity: Some([0; 3]),
                ..Default::default()
            },
            &w.setup.topology
        ));
        b.height = 0;
        u.recovery.bike_location.height = 0;
        u.previous_location = u.recovery.bike_location;
        let body = u.recovery.body.as_mut().unwrap();
        body.motion.distance_q8 -= 10;
        body.location.progress -= 10;
        body.motion.lateral += 1000;
        body.location.lateral += 1000;
        body.motion.height = 0;
        body.location.height = 0;
        body.motion.contact_velocity = Some([0; 3]);
        body.previous_location = Some(body.location);
        body.animation.set(c.recovery_animation(), 0, 35);
        body.animation.queued = None;
    }
    sim
}

#[test]
fn source_contact_history_keeps_converged_body_before_its_bike() {
    // Native controlled case98: four fallen riders. Body4 converges onto its
    // bike on the first walk; the next stable sort must retain body-before-bike.
    let mut observed = vec![];
    for version in [1, 2] {
        let mut sim = crowded_fixture(version);
        let c = sim.handling.as_ref().unwrap().clone();
        for tick in 1..=2 {
            let w = sim.encounter.as_mut().unwrap();
            let rng = &mut sim.gameplay_rng;
            w.contacts(&mut sim.actors, &c, tick, &mut vec![], || rng.next())
                .unwrap();
        }
        let w = sim.encounter.as_ref().unwrap();
        observed.push(w.units[&ActorId(3)].previous_location.lateral);
        for (id, u) in &w.units {
            assert_eq!(u.recovery.incident, 1);
            assert_eq!(u.recovery.body.as_ref().unwrap().owner, *id);
        }
        let state = serde_json::to_value(w).unwrap();
        assert_eq!(state.get("source_contact_order").is_some(), version == 2);
        sim.despawn(ActorId(4)).unwrap();
        let state = serde_json::to_value(sim.encounter.as_ref().unwrap()).unwrap();
        if let Some(order) = state.get("source_contact_order") {
            assert!(
                !order
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|key| [8, 9].contains(&key.as_u64().unwrap()))
            );
            assert!(order.as_array().unwrap().len() <= 6);
        }
    }
    assert_eq!(observed, [3040, 3120]);
}

#[test]
fn source_remount_contact_distinguishes_ignored_entry_from_a_new_fall() {
    for completed in [false, true] {
        let mut sim = crowded_fixture(2);
        let c = sim.handling.as_ref().unwrap().clone();
        let id = ActorId(1);
        let w = sim.encounter.as_mut().unwrap();
        let u = w.units.get_mut(&id).unwrap();
        u.recovery.phase = recovery::Phase::Mounting;
        let body = u.recovery.body.as_mut().unwrap();
        body.animation
            .set(c.recovery_animation(), 0, if completed { 2 } else { 41 });
        body.animation.selected = Some((41, 0));
        u.recovery.bike_animation.set(c.recovery_animation(), 0, 48);
        u.recovery.bike_animation.selected = Some((48, 0));
        u.previous_location.progress = 2817;
        let bike = &mut sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        bike.speed = 11000;
        bike.lateral = 0;
        u.recovery.bike_location.lateral = 0;
        u.previous_location.lateral = 0;
        bike.contact_velocity = Some([11000, 0, 0]);
        w.source_prepare_recovery(&mut sim.actors, &c, id, 1, &mut vec![])
            .unwrap();
        assert_eq!(
            w.units[&id].recovery.phase == recovery::Phase::Mounted,
            completed
        );
        let mut wall = collision::Body {
            position: [2688, 0, 0],
            previous: [2688, 0, 0],
            half: [0, 16384, 32768],
            velocity: [0; 3],
            speed: 0,
            mass: 0x1000000,
            category: 1,
            flags: 0,
            impulse: [0; 3],
        };
        let rng = &mut sim.gameplay_rng;
        w.source_object_contact_with_random(
            id,
            &mut sim.actors,
            &mut wall,
            &c,
            1,
            &mut vec![],
            || rng.next(),
        )
        .unwrap();
        let recovery = &w.units[&id].recovery;
        assert_eq!(recovery.incident, if completed { 2 } else { 1 });
        assert_eq!(recovery.body.as_ref().unwrap().owner, id);
        assert_eq!(
            recovery.body.as_ref().unwrap().animation.selected,
            Some((41, 0))
        );
        assert_eq!(recovery.bike_animation.code, if completed { 0 } else { 48 });
        if completed {
            assert_eq!(recovery.bike_animation.queued, Some(0));
        }
    }
}

#[test]
fn source_body2_first_quiet_move_retains_the_lifted_previous_height() {
    let c = fixtures::riding();
    let lifted = c.spec().suspension;
    let mut e = encounter::Setup::authored(&c).unwrap();
    e.rules.combat_version = 4;
    e.rules.body.recovery_version = 2;
    e.rules.bike_contacts = false;
    let mut sim = Simulation::with_encounters(1, c.clone(), e).unwrap();
    let w = sim.encounter.as_mut().unwrap();
    w.hit(
        &mut sim.actors,
        &c,
        ActorId(1),
        recovery::Impact {
            reason: 1,
            ..Default::default()
        },
        0,
        true,
        &mut vec![],
    )
    .unwrap();
    assert_eq!(
        w.units[&ActorId(1)]
            .recovery
            .body
            .as_ref()
            .unwrap()
            .previous_location
            .unwrap()
            .height,
        0
    );
    sim.advance().unwrap();
    assert_eq!(
        sim.encounter.as_ref().unwrap().units[&ActorId(1)]
            .previous_location
            .height,
        lifted
    );
}

pub(super) fn animation(a: &animation::Player) -> Value {
    json!([a.code, a.frame, a.queued.unwrap_or(-1), a.loops, a.selected])
}
fn motion(b: &handling::Bike, previous: topology::Location) -> Value {
    let v = b
        .contact_velocity
        .unwrap_or([b.speed, b.lateral_speed, b.vertical_speed]);
    json!([
        b.distance_q8,
        b.lateral,
        b.height,
        previous.progress,
        previous.lateral,
        previous.height,
        b.speed,
        b.lateral_speed,
        b.vertical_speed,
        v[0],
        v[1],
        v[2]
    ])
}
pub(super) fn state(sim: &Simulation) -> Value {
    let w = sim.encounter.as_ref().unwrap();
    let rows: Vec<_> = w
        .units
        .iter()
        .map(|(id, u)| {
            let b = &sim.actors[id].view.rider.as_ref().unwrap().bike;
            let body = u.recovery.body.as_ref().map(|body| {
                json!([
                    motion(
                        &body.motion,
                        body.previous_location.unwrap_or(body.location)
                    ),
                    animation(&body.animation)
                ])
            });
            let phase = match u.recovery.phase {
                recovery::Phase::Mounted => 0,
                recovery::Phase::Sliding => 1,
                recovery::Phase::Mounting => 3,
                _ => 2,
            };
            json!([
                motion(b, u.previous_location),
                b.lean,
                b.condition,
                phase,
                u.fighter.stats.health,
                u.fighter.stats.maximum,
                animation(&u.recovery.bike_animation),
                body
            ])
        })
        .collect();
    json!([sim.gameplay_rng.0, rows])
}
fn camera(sim: &Simulation) -> i64 {
    let state = sim.encounter.as_ref().unwrap().scenery.as_ref().unwrap();
    let follow = state.frame_follow.or(state.pending_camera).unwrap();
    (follow.progress
        - 256
        - if state.frame_follow.is_some() {
            i64::from(state.camera_back)
        } else {
            0
        })
    .max(0)
}
#[test]
#[ignore = "requires private native crowded recovery callbacks"]
fn source_recovery_contacts_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_RECOVERY_CONTACTS").unwrap()).unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut c = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    c.motion_version = 3;
    c.start_sample = 0;
    c.specs
        .iter_mut()
        .find(|s| s.id == c.selected)
        .unwrap()
        .condition = 150;
    c.ground = vec![handling::Ground::flat(); 2000];
    for g in &mut c.ground {
        g.left = -5000;
        g.right = 5000;
        g.left_shoulder = -5512;
        g.right_shoulder = 5512;
    }
    let mut failures = vec![];
    let mut pre_failures = vec![];
    let mut actual = vec![];
    let mut rows = 0;
    for case in native.as_array().unwrap() {
        let input = &case["input"];
        let n = |i: usize| input[i].as_i64().unwrap() as i32;
        let mut e = encounter::Setup::authored(&c).unwrap();
        e.topology = topology::Topology::straight(512000);
        e.rules.combat_version = 4;
        e.rules.collision_version = 2;
        e.rules.contact_walk_version = if std::env::var_os("RR_RECOVERY_CONTACTS_LEGACY").is_some()
        {
            1
        } else {
            2
        };
        e.rules.bike_contacts = true;
        e.rules.scenery_version = 1;
        e.rules.scenery_start_ready = true;
        e.rules.collision = Some(collision::Profile::from_exe(&exe).unwrap());
        e.rules.body = recovery::BodyLaw::from_exe(&exe).unwrap();
        e.rules.body.motion_version = 1;
        e.rules.body.recovery_version = 2;
        e.rules.finish_progress = Some(500000);
        let mut template = e.seeds[0].clone();
        e.seeds.clear();
        template.stats = combat::Stats {
            strength: 7,
            base: 2500,
            maximum: 2500,
            health: 2500,
        };
        for i in 0..n(2) {
            let mut seed = template.clone();
            seed.id = ActorId(i as u64 + 1);
            seed.distance = i64::from(2688 + i * 15);
            seed.lateral = n(5) * (i - 1) * if n(6) != 0 { 8000 } else { 2000 };
            seed.speed = 0; // Signed velocities are installed after validated construction.
            e.seeds.push(seed);
        }
        let mut sim = Simulation::with_encounters(1, c.clone(), e).unwrap();
        for i in 0..n(2) {
            let id = ActorId(i as u64 + 1);
            let speed = if i % 2 != 0 { n(4) } else { -n(4) / 2 };
            let w = sim.encounter.as_mut().unwrap();
            let u = w.units.get_mut(&id).unwrap();
            let r = sim
                .actors
                .get_mut(&id)
                .unwrap()
                .view
                .rider
                .as_mut()
                .unwrap();
            r.bike.speed = speed;
            r.bike.contact_velocity = Some([speed, 0, 0]);
            if i < n(3) {
                assert!(u.recovery.begin_live_source(
                    id,
                    &mut r.bike,
                    &c,
                    u.recovery.bike_location,
                    recovery::Impact {
                        reason: 1,
                        rider_velocity: Some([0; 3]),
                        bike_velocity: Some([0; 3]),
                        ..Default::default()
                    },
                    &w.setup.topology
                ));
                r.bike.height = 0;
                u.recovery.bike_location.height = 0;
                u.previous_location = u.recovery.bike_location;
                let body = u.recovery.body.as_mut().unwrap();
                body.motion.distance_q8 =
                    i64::from(2688 + i * 15 + if n(6) != 0 { 20 } else { -10 });
                body.location.progress = body.motion.distance_q8;
                body.motion.lateral = r.bike.lateral + n(5) * if n(6) != 0 { -6000 } else { 1000 };
                body.location.lateral = body.motion.lateral;
                body.motion.height = 0;
                body.location.height = 0;
                body.previous_location = Some(body.location);
                body.motion.speed = speed;
                body.motion.lateral_speed = 0;
                body.motion.vertical_speed = 0;
                body.motion.contact_velocity = Some([speed, 0, 0]);
                body.animation.set(c.recovery_animation(), 0, n(1) as i8);
                body.animation.queued = None;
                u.recovery.phase = match n(1) {
                    35 => recovery::Phase::Sliding,
                    36 => recovery::Phase::GettingUp,
                    37 => recovery::Phase::Standing,
                    38 => recovery::Phase::Walking,
                    _ => recovery::Phase::Mounting,
                };
            }
        }
        let cameras_enabled = case.get("pre_camera").is_some();
        if cameras_enabled {
            let w = sim.encounter.as_mut().unwrap();
            w.source_movement_end();
            w.scenery_prepare(true).unwrap();
        }
        let pre_camera = camera(&sim);
        let pre = state(&sim);
        if pre != case["pre"] {
            pre_failures.push(json!({"input":input,"native":case["pre"],"rust":pre}));
        }
        if cameras_enabled && case["pre_camera"] != pre_camera {
            pre_failures.push(
                json!({"input":input,"native_camera":case["pre_camera"],"rust_camera":pre_camera}),
            );
        }
        let mut observed = vec![];
        let mut cameras = vec![];
        for (pass, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
            let w = sim.encounter.as_mut().unwrap();
            if cameras_enabled {
                w.source_movement_end();
            }
            let rng = &mut sim.gameplay_rng;
            w.contacts(&mut sim.actors, &c, pass as u64 + 1, &mut vec![], || {
                rng.next()
            })
            .unwrap();
            if cameras_enabled {
                w.scenery_prepare(true).unwrap();
            }
            let got = state(&sim);
            let actual_camera = camera(&sim);
            rows += 1;
            if &got != expected {
                failures.push(json!({"input":input,"pass":pass+1,"native":expected,"rust":got}));
            }
            if cameras_enabled && case["cameras"][pass] != actual_camera {
                failures.push(json!({"input":input,"pass":pass+1,"native_camera":case["cameras"][pass],"rust_camera":actual_camera}));
            }
            observed.push(got);
            cameras.push(actual_camera);
            for (id, unit) in &sim.encounter.as_ref().unwrap().units {
                if let Some(body) = &unit.recovery.body {
                    assert_eq!(body.owner, *id);
                    assert_eq!(unit.recovery.incident, 1);
                }
            }
        }
        actual.push(json!({"input":input,"pre":pre,"steps":observed,"pre_camera":pre_camera,"cameras":cameras}));
    }
    let out =
        std::path::PathBuf::from(std::env::var_os("RR_NATIVE_RECOVERY_CONTACTS_OUTPUT").unwrap());
    std::fs::write(out.join("rust.json"), serde_json::to_vec(&actual).unwrap()).unwrap();
    std::fs::write(out.join("recovery-contacts-comparison.json"),serde_json::to_vec_pretty(&json!({
        "outcome":if failures.is_empty()&&pre_failures.is_empty(){"passed"}else{"failed"},
        "rows":rows,"cases":native.as_array().unwrap().len(),"mismatches":failures.len(),
        "pre_mismatches":pre_failures.len(),"first_divergence":failures.first(),
        "first_pre_divergence":pre_failures.first(),"scope":"Controlled phase poses and four native mixed bike/half callback walks; no motion/terrain scheduler parity"})).unwrap()).unwrap();
    std::fs::write(
        out.join("failures.json"),
        serde_json::to_vec(&failures).unwrap(),
    )
    .unwrap();
    std::fs::write(
        out.join("pre-failures.json"),
        serde_json::to_vec(&pre_failures).unwrap(),
    )
    .unwrap();
    assert!(
        pre_failures.is_empty(),
        "{} initial pose differences",
        pre_failures.len()
    );
    assert!(
        failures.is_empty(),
        "{} callback differences",
        failures.len()
    );
}
