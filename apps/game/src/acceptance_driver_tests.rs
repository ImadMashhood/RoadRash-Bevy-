use super::*;
use rr_simulation::{
    population::{Agent, AgentState, Kind},
    topology::{Location, Path as RoadPath, Portal, Topology},
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn cached_forecasts_keep_advance_error_pose_and_stationary_crossings() {
    let record = Replay::read(include_bytes!(
        "../../../docs/fixtures/encounter-replay.json"
    ))
    .unwrap();
    let mut snapshot = Playback::new(record).unwrap().simulation.snapshot();
    assert!(snapshot.actors.len() >= 2);
    snapshot.actors.truncate(2);
    let other = &mut snapshot.actors[1];
    other.encounter.as_mut().unwrap().recovery.bike_location = Location::default();
    other.rider.as_mut().unwrap().bike.speed = 25_600;
    other.rider.as_mut().unwrap().bike.lateral = 12_345;
    let mut ignored = other.clone();
    ignored.id = ActorId(999);
    ignored
        .encounter
        .as_mut()
        .unwrap()
        .recovery
        .bike_location
        .path = 999;
    ignored.rider = None;
    snapshot.actors.push(ignored);
    let road = |target| RoadPath {
        length: 1,
        branch: 0,
        selected_start: None,
        portals: vec![Portal {
            end: true,
            target,
            target_end: false,
        }],
        rises: vec![],
    };
    // An intentionally cyclic producer-error case: advance changes path before
    // returning traversal-bound failure. The old planner kept that exact pose.
    let topology = Topology {
        paths: BTreeMap::from([(0, road(1)), (1, road(0))]),
    };
    let agent = |id, kind, state, direction| Agent {
        rider: None,
        cruise_speed: None,
        id,
        kind,
        state,
        location: Location::default(),
        previous: Location::default(),
        speed: 25_600,
        direction,
        family: None,
        selector: 0,
        subtype: 0,
        born: 0,
        state_since: 0,
        hit_count: 0,
        projected: None,
        siren: false,
        animation: Default::default(),
        frame: 0,
    };
    snapshot.road_users = vec![
        agent(1, Kind::Vehicle, AgentState::Cruise, 1),
        agent(2, Kind::Vehicle, AgentState::Crossing, 1),
        agent(3, Kind::Pedestrian, AgentState::Cruise, -1),
    ];
    let mut expected = Location::default();
    assert!(topology.advance(&mut expected, 500, None).is_err());
    assert_eq!(expected.path, 1);
    let forecast = Forecast::new(&snapshot, &topology, 0, 5);
    assert_eq!(forecast.riders, vec![(expected, 12_345)]);
    assert_eq!(
        forecast.road_users,
        vec![expected, Location::default(), Location::default()]
    );
}

#[test]
#[ignore = "requires private completed current career and original recorded command oracle"]
fn cached_driver_matches_every_recorded_career_command() {
    let root =
        std::path::PathBuf::from(std::env::var("RR_DRIVER_CAMPAIGN").expect("RR_DRIVER_CAMPAIGN"));
    let output = std::path::PathBuf::from(
        std::env::var("RR_DRIVER_EQUIVALENCE_OUTPUT").expect("RR_DRIVER_EQUIVALENCE_OUTPUT"),
    );
    for variable in [
        "RR_DRIVER_CENTER",
        "RR_DRIVER_SPEED",
        "RR_DRIVER_LANE",
        "RR_DRIVER_LEGACY",
    ] {
        assert!(
            std::env::var_os(variable).is_none(),
            "default policy required: {variable}"
        );
    }
    let campaign: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("campaign.json")).unwrap()).unwrap();
    assert_eq!(campaign["completed"], true);
    assert_eq!(campaign["real_commands_only"], true);
    assert_eq!(campaign["native_only"], true);
    let races = campaign["races"].as_array().unwrap();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let checked = std::sync::Mutex::new(vec![]);
    std::thread::scope(|scope| {
        for _ in 0..4.min(races.len()) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(row) = races.get(index) else { break };
                let path = root.join(row["replay"].as_str().unwrap());
                let bytes = std::fs::read(path).unwrap();
                let record = Replay::read(&bytes).unwrap();
                let config = record.handling.as_ref().unwrap().clone();
                let encounter = record.encounter.as_ref().unwrap().clone();
                let mut simulation = Playback::new(record.clone()).unwrap().simulation;
                let mut driver = Driver::default();
                let mut cursor = 0;
                let mut expected = Command::default();
                let mut phases = BTreeMap::<String, u64>::new();
                let mut routes = BTreeSet::new();
                for tick in 0..record.ticks {
                    while let Some(command) = record.commands.get(cursor).filter(|c| c.tick.0 == tick) {
                        assert_eq!(command.actor, ActorId(1));
                        expected = command.input;
                        cursor += 1;
                    }
                    let snapshot = simulation.snapshot();
                    let unit = snapshot.actors[0].encounter.as_ref().unwrap();
                    *phases.entry(format!("{:?}", unit.recovery.phase)).or_default() += 1;
                    routes.insert(unit.source_route);
                    let actual = driver.command(&snapshot, &config, &encounter);
                    assert_eq!(actual, expected, "case {index}, tick {tick}, course {}, level {}", row["course"], row["level"]);
                    simulation.advance().unwrap();
                }
                let hash = simulation.state_hash();
                assert_eq!(Some(&hash), record.expected_hash.as_ref());
                assert_eq!(hash, row["state_sha256"].as_str().unwrap());
                checked.lock().unwrap().push(serde_json::json!({"attempt":index+1,"course":row["course"],"level":row["level"],"ticks":record.ticks,"commands_equal":record.ticks,"state_sha256":hash,"replay_sha256":rr_content::hash(&bytes),"recovery_phases":phases,"source_routes":routes}));
                eprintln!("driver oracle {}/{} passed", index + 1, races.len());
            });
        }
    });
    let mut rows = checked.into_inner().unwrap();
    rows.sort_by_key(|r| r["attempt"].as_u64().unwrap());
    let cases: BTreeSet<_> = rows
        .iter()
        .map(|r| (r["course"].as_str().unwrap(), r["level"].as_u64().unwrap()))
        .collect();
    assert_eq!(cases.len(), 25);
    let total: u64 = rows.iter().map(|r| r["ticks"].as_u64().unwrap()).sum();
    let receipt = serde_json::json!({"outcome":"passed","scope":"Every optimized default-driver command compared with the immutable pre-optimization genuine current-career command stream, then original replay commands advanced the unchanged simulation; all final canonical hashes verified.","races":rows.len(),"course_level_cases":cases.len(),"command_comparisons":total,"rows":rows});
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap()
        .write_all(&serde_json::to_vec_pretty(&receipt).unwrap())
        .unwrap();
}
