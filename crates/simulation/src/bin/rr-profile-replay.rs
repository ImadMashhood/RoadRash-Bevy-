//! Opt-in CPU stage probe for a private production replay; emits no source setup.
use rr_simulation::replay::{Playback, Replay};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::Instant,
};
fn distribution(mut values: Vec<f64>) -> serde_json::Value {
    values.sort_by(f64::total_cmp);
    let at = |p: f64| values[((values.len() - 1) as f64 * p).ceil() as usize];
    serde_json::json!({"count":values.len(),"p50":at(0.5),"p95":at(0.95),"p99":at(0.99),"max":values.last()})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: rr-profile-replay REPLAY NEW_OUTPUT_JSON".into());
    }
    let input = PathBuf::from(&args[0]);
    let mut bytes = vec![];
    let file = std::fs::File::open(&input)?;
    if !file.metadata()?.is_file() {
        return Err("profile input must be a regular file".into());
    }
    file.take(rr_simulation::replay::MAX_REPLAY_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let replay = Replay::read(&bytes)?;
    if replay.ticks > 100_000 {
        return Err("CPU stage probe supports at most 100000 ticks".into());
    }
    let (schema, seed, ticks, expected) = (
        replay.schema_version,
        replay.seed,
        replay.ticks,
        replay.expected_hash.clone(),
    );
    let mut simulation = Playback::new(replay)?.simulation;
    let mut steps = Vec::with_capacity(ticks as usize);
    let mut snapshots = Vec::with_capacity(ticks as usize);
    let (mut actors, mut road_users) = (0, 0);
    for _ in 0..ticks {
        let start = Instant::now();
        simulation.advance()?;
        steps.push(start.elapsed().as_secs_f64() * 1000.);
        let start = Instant::now();
        let snapshot = std::hint::black_box(simulation.snapshot());
        snapshots.push(start.elapsed().as_secs_f64() * 1000.);
        actors = actors.max(snapshot.actors.len());
        road_users = road_users.max(snapshot.road_users.len());
    }
    let hash = simulation.state_hash();
    if expected.is_some_and(|value| value != hash) {
        return Err("CPU probe changed replay authority".into());
    }
    let binary = std::fs::read(std::env::current_exe()?)?;
    let report = serde_json::json!({"outcome":"passed","scope":"Instant elapsed time around production Simulation.advance and owned snapshot extraction; includes thread descheduling, excludes input loading/JSON output, Bevy ECS GPU extraction and driver control search. GPU/pass/raster timings are measured separately in the live application.","units":"milliseconds","ticks":ticks,"seed":seed,"simulation_schema":schema,"input_sha256":format!("{:x}",Sha256::digest(&bytes)),"binary_sha256":format!("{:x}",Sha256::digest(binary)),"state_sha256":hash,"max_actors":actors,"max_road_users":road_users,"simulation_step_cpu_ms":distribution(steps),"snapshot_extraction_cpu_ms":distribution(snapshots)});
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    file.write_all(&serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
