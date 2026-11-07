//! Actual native static contacts immediately before/after completed mount think.
use super::native_recovery_contacts::state;
use super::*;
use serde_json::{Value, json};

#[test]
#[ignore = "requires private native remount/static callback matrix"]
fn source_remount_contacts_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_REMOUNT_CONTACTS").unwrap()).unwrap(),
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
    c.ground = vec![handling::Ground::flat(); 2000];
    let mut failures = vec![];
    let mut pre_failures = vec![];
    let mut observed = vec![];
    let mut rows = 0;
    for case in native.as_array().unwrap() {
        let input = &case["input"];
        let n = |i: usize| input[i].as_i64().unwrap() as i32;
        let mut e = encounter::Setup::authored(&c).unwrap();
        e.topology = topology::Topology::straight(512000);
        e.rules.combat_version = 4;
        e.rules.collision_version = 2;
        e.rules.contact_walk_version = 2;
        e.rules.collision = Some(collision::Profile::from_exe(&exe).unwrap());
        e.rules.body = recovery::BodyLaw::from_exe(&exe).unwrap();
        e.rules.body.motion_version = 1;
        e.rules.body.recovery_version = 2;
        e.seeds[0].distance = 2688;
        e.seeds[0].speed = n(3);
        e.seeds[0].stats = combat::Stats {
            strength: 7,
            base: 2500,
            maximum: 2500,
            health: 2500,
        };
        let mut sim = Simulation::with_encounters(1, c.clone(), e).unwrap();
        let id = ActorId(1);
        let w = sim.encounter.as_mut().unwrap();
        let u = w.units.get_mut(&id).unwrap();
        let b = &mut sim
            .actors
            .get_mut(&id)
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike;
        b.condition = n(5);
        b.contact_velocity = Some([n(3), 0, 0]);
        assert!(u.recovery.begin_live_source(
            id,
            b,
            &c,
            u.recovery.bike_location,
            recovery::Impact {
                reason: 4,
                rider_velocity: Some([0; 3]),
                bike_velocity: Some([0; 3]),
                ..Default::default()
            },
            &w.setup.topology
        ));
        b.height = 0;
        u.recovery.bike_location.height = 0;
        u.previous_location = u.recovery.bike_location;
        u.previous_location.progress = 2688 - i64::from(n(4) * 129);
        let body = u.recovery.body.as_mut().unwrap();
        body.motion.height = 0;
        body.location.height = 0;
        let mut previous = body.location;
        previous.progress = u.previous_location.progress;
        body.previous_location = Some(previous);
        body.animation.set(c.recovery_animation(), 0, n(1) as i8);
        body.animation.queued = None;
        body.animation.selected = Some((if n(1) == 2 { 41 } else { n(1) } as usize, 0));
        u.recovery.bike_animation.set(c.recovery_animation(), 0, 48);
        u.recovery.bike_animation.queued = None;
        u.recovery.bike_animation.selected = Some((48, 0));
        u.recovery.phase = match n(1) {
            35 => recovery::Phase::Sliding,
            36 => recovery::Phase::GettingUp,
            37 => recovery::Phase::Standing,
            38 => recovery::Phase::Walking,
            _ => recovery::Phase::Mounting,
        };
        let pre = state(&sim);
        if pre != case["pre"] {
            pre_failures.push(json!({"input":input,"native":case["pre"],"rust":pre}));
        }
        let mut stages = vec![];
        for stage in 0..2 {
            let w = sim.encounter.as_mut().unwrap();
            if stage == n(2) {
                let rng = &mut sim.gameplay_rng;
                let mut other = collision::Body {
                    position: [2688, 0, 0],
                    previous: [2688, 0, 0],
                    half: [0, 16384, n(6)],
                    velocity: [0; 3],
                    speed: 0,
                    mass: 0x1000000,
                    category: 1,
                    flags: 0,
                    impulse: [0; 3],
                };
                w.source_object_contact_with_random(
                    id,
                    &mut sim.actors,
                    &mut other,
                    &c,
                    1,
                    &mut vec![],
                    || rng.next(),
                )
                .unwrap();
            } else {
                w.source_prepare_recovery(&mut sim.actors, &c, id, 1, &mut vec![])
                    .unwrap();
            }
            let got = state(&sim);
            let expected = &case["steps"][stage as usize];
            rows += 1;
            if &got != expected {
                failures.push(json!({"input":input,"stage":stage,"native":expected,"rust":got}));
            }
            stages.push(got);
        }
        observed.push(json!({"input":input,"pre":pre,"steps":stages}));
    }
    let out =
        std::path::PathBuf::from(std::env::var_os("RR_NATIVE_REMOUNT_CONTACTS_OUTPUT").unwrap());
    std::fs::write(
        out.join("rust.json"),
        serde_json::to_vec(&observed).unwrap(),
    )
    .unwrap();
    std::fs::write(out.join("remount-contacts-comparison.json"),serde_json::to_vec_pretty(&json!({"outcome":if failures.is_empty()&&pre_failures.is_empty(){"passed"}else{"failed"},"rows":rows,"cases":native.as_array().unwrap().len(),"mismatches":failures.len(),"pre_mismatches":pre_failures.len(),"first_divergence":failures.first(),"first_pre_divergence":pre_failures.first(),"scope":"Actual source static contact and recovery think around controlled mount completion; no full motion scheduler claim"})).unwrap()).unwrap();
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
        "{} initial pose mismatches",
        pre_failures.len()
    );
    assert!(
        failures.is_empty(),
        "{} callback mismatches",
        failures.len()
    );
}
