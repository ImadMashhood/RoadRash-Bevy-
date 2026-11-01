//! Integer fixed-step driver for repeatable session-level acceptance (including countdown).
use crate::{
    Result,
    race::{Entry, Mode, RaceIdentity, RaceSession, Summary},
};
use rr_simulation::{
    TICKS_PER_SECOND,
    replay::{Playback, Replay, frame_ns},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
#[derive(Serialize)]
pub struct Report {
    pub rate: String,
    pub frames: u64,
    pub session_ticks: u64,
    pub simulation_ticks: u64,
    pub state_sha256: String,
    pub result_sha256: String,
    pub summary: Summary,
    pub checkpoints: Vec<(u64, String)>,
}
pub fn run(replay: Replay, rate: &str, effects: bool) -> Result<Report> {
    let limit = replay.ticks;
    let seed = replay.seed;
    let mut sim = Playback::new(replay).map_err(|e| e.to_string())?.simulation;
    sim.set_effects_enabled(effects);
    let identity = RaceIdentity {
        id: 1,
        mode: Mode::Thrash,
        course: 1,
        level: 1,
        bike: 0,
        seed,
        roster: sim
            .snapshot()
            .actors
            .iter()
            .map(|a| Entry {
                actor: a.id,
                name: format!("Rider {}", a.id.0),
            })
            .collect(),
    };
    let mut session = RaceSession::new(
        identity,
        sim.encounters().map(|w| w.setup.topology.clone()),
        &sim.snapshot(),
        180,
        None,
    )?;
    let mut frames = 0;
    let mut debt = 0;
    let mut session_ticks = 0;
    let mut checkpoints = vec![];
    while session.summary().is_none() {
        let duration = frame_ns(rate, frames).map_err(str::to_string)?;
        frames += 1;
        debt += duration * u64::from(TICKS_PER_SECOND);
        while debt >= 1_000_000_000 && session.summary().is_none() {
            debt -= 1_000_000_000;
            session_ticks += 1;
            if session.permit_tick() {
                sim.advance().map_err(|e| e.to_string())?;
                session.observe(&sim.snapshot());
                if sim.tick().0.is_multiple_of(30)
                    || session.summary().is_some()
                    || sim.tick().0 >= limit
                {
                    checkpoints.push((sim.tick().0, sim.state_hash()));
                }
                if sim.tick().0 >= limit && session.summary().is_none() {
                    session.quit(&sim.snapshot());
                }
            }
        }
    }
    let summary = session.summary().unwrap().clone();
    summary.validate()?;
    let bytes = serde_json::to_vec(&summary).map_err(|e| e.to_string())?;
    Ok(Report {
        rate: rate.into(),
        frames,
        session_ticks,
        simulation_ticks: sim.tick().0,
        state_sha256: sim.state_hash(),
        result_sha256: format!("{:x}", Sha256::digest(bytes)),
        summary,
        checkpoints,
    })
}
