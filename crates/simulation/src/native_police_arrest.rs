//! Actual find-cop and police-think eligibility comparisons on a supported straight ring.
use super::*;
use serde_json::json;
#[test]
#[ignore = "requires actual native police arrest transactions and owned source data"]
fn source_police_arrest_native() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let native: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_POLICE_ARREST").unwrap()).unwrap(),
    )
    .unwrap();
    let exe = std::fs::read(data.join("ROADRASH.EXE")).unwrap();
    let police = population::police::Config::read(&exe).unwrap();
    let course =
        rr_course::Course::read(&std::fs::read(data.join("DATA/COURSES/CITY.CRS")).unwrap())
            .unwrap();
    let route = course.route(&vec![0; course.splits().len()]).unwrap();
    let mut c = riding::Configuration::load(&data, &route, 1, 0).unwrap();
    c.ground = vec![handling::Ground::flat(); 1024];
    let mut e = encounter::Setup::authored(&c).unwrap();
    e.rules.combat_version = 4;
    e.rules.bike_contacts = false;
    e.seeds[0].distance = 12800;
    for n in 2..=4 {
        let mut seed = encounter::Seed::player();
        seed.id = ActorId(n);
        seed.distance = 12800;
        e.seeds.push(seed);
    }
    let p = population::Setup {
        police: Some(police),
        traffic: false,
        periodic_police: false,
        ..Default::default()
    };
    let mut sim = Simulation::with_population(1, c.clone(), e, p).unwrap();
    sim.encounter
        .as_mut()
        .unwrap()
        .units
        .get_mut(&ActorId(4))
        .unwrap()
        .police = true;
    sim.population.as_mut().unwrap().police = Some(population::police::Live {
        actor: ActorId(4),
        agent: 1,
        lane: 0,
        pursuit: vec![0; 129],
        player_relationship: 0,
        removal_pending: false,
    });
    let mut state = recovery::State::mounted(
        ActorId(1),
        topology::Location {
            progress: 12800,
            ..Default::default()
        },
    );
    let mut bike = handling::Bike::new(c.spec(), &c.tables).unwrap();
    bike.distance_q8 = 12800;
    state.begin_source(
        ActorId(1),
        &mut bike,
        &c,
        state.bike_location,
        recovery::Impact::default(),
        &sim.encounter.as_ref().unwrap().setup.topology,
    );
    let body = state.body.unwrap();
    let mut failures = vec![];
    for row in native.as_array().unwrap() {
        let n = |i: usize| row[i].as_i64().unwrap();
        let e = sim.encounter.as_mut().unwrap();
        let p = sim.population.as_mut().unwrap();
        let player = e.units.get_mut(&ActorId(1)).unwrap();
        player.terminal_request = None;
        player.recovery.outcome = if n(6) != 0 {
            Some(recovery::Outcome::Finished)
        } else {
            None
        };
        player.recovery.phase = match n(2) {
            0 => recovery::Phase::Mounted,
            1 => recovery::Phase::Sliding,
            2 => recovery::Phase::Walking,
            _ => recovery::Phase::Mounting,
        };
        player.recovery.bike_location.progress = 12800;
        player.recovery.body = if n(2) == 0 {
            None
        } else {
            let mut b = body.clone();
            b.location.progress = 12800 + if n(7) == 3 { 100 } else { 0 };
            Some(b)
        };
        let cop = e.units.get_mut(&ActorId(4)).unwrap();
        cop.recovery.phase = match n(5) {
            0 => recovery::Phase::Mounted,
            1 => recovery::Phase::Sliding,
            _ => recovery::Phase::Walking,
        };
        cop.recovery.bike_location.progress = 12800 + n(1);
        p.police.as_mut().unwrap().removal_pending = n(8) == 0;
        sim.actors
            .get_mut(&ActorId(1))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .speed = n(3) as i32;
        sim.actors
            .get_mut(&ActorId(4))
            .unwrap()
            .view
            .rider
            .as_mut()
            .unwrap()
            .bike
            .speed = (n(3) + n(4)) as i32;
        p.police_progression(e, &sim.actors);
        let actual = e.units[&ActorId(1)].terminal_request == Some(recovery::Outcome::Arrested);
        if actual != (n(10) != 0) {
            failures.push(json!({"input":row,"actual_arrest":actual}));
        }
    }
    let out = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_POLICE_ARREST_OUTPUT").unwrap());
    std::fs::write(
        out.join("first-divergences.json"),
        serde_json::to_vec_pretty(&failures).unwrap(),
    )
    .unwrap();
    let receipt = json!({"outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":native.as_array().unwrap().len(),"mismatches":failures.len(),"scope":"Actual native find_cop/police_think result including intervening halves, ties, rider/bike terminal flags, crash/speed/gap gates; native NULL cop half represented by the host pending-unload gate; graph cache/phase integration separate"});
    std::fs::write(
        out.join("police-arrest-comparison.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(
        failures.is_empty(),
        "{} native arrest mismatches",
        failures.len()
    );
}
