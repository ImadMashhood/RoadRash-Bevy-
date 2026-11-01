use crate::*;
pub const MAX_REPLAY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replay {
    pub schema_version: u32,
    pub seed: u32,
    pub ticks: u64,
    #[serde(default)]
    pub commands: Vec<QueuedCommand>,
    pub route: Option<Route>,
    pub expected_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handling: Option<riding::Configuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encounter: Option<encounter::Setup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population: Option<population::Setup>,
}
impl Replay {
    pub fn read_file(
        path: &std::path::Path,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        use std::io::Read;
        let file = std::fs::File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err("replay must be a regular file".into());
        }
        let mut bytes = vec![];
        file.take(MAX_REPLAY_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        Self::read(&bytes)
    }
    pub fn read(bytes: &[u8]) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        if bytes.len() > MAX_REPLAY_BYTES {
            return Err("replay size limit".into());
        }
        let replay: Self = serde_json::from_slice(bytes)?;
        if replay.schema_version < 6 && bytes.len() > 4 * 1024 * 1024 {
            return Err("legacy replay size limit".into());
        }
        if !(1..=SIMULATION_SCHEMA).contains(&replay.schema_version)
            || replay.ticks == 0
            || replay.ticks > 1_000_000
            || replay.commands.len() > 65536
        {
            return Err("replay schema/duration/command count".into());
        }
        if replay.commands.iter().any(|c| c.tick.0 >= replay.ticks) {
            return Err("command outside replay duration".into());
        }
        if (replay.schema_version >= 3) != replay.handling.is_some() {
            return Err("schema-3 replay requires handling configuration".into());
        }
        if (replay.schema_version >= 4) != replay.encounter.is_some() {
            return Err("schema-4 encounter setup required".into());
        }
        if (replay.schema_version >= 5) != replay.population.is_some() {
            return Err("schema-5 population required".into());
        }
        if replay.schema_version == 1 && replay.commands.iter().any(|c| c.input.steer_q15.is_some())
        {
            return Err("analog steering requires replay schema 2".into());
        }
        Ok(replay)
    }
}
pub struct Playback {
    pub simulation: Simulation,
    pub previous: Snapshot,
    pub current: Snapshot,
    pub frames: u64,
    pub checkpoints: Vec<(u64, String)>,
    pub checkpoint_every_tick: bool,
    pub forensic: Vec<ForensicTick>,
    pub observations: Vec<(Snapshot, Vec<crate::combat::Effect>)>,
    limit: u64,
    debt: u64,
}
impl Playback {
    pub fn new(replay: Replay) -> Result<Self> {
        let requested_schema = replay.schema_version;
        let mut simulation = if let Some(config) = replay.handling {
            if replay.schema_version < 3 {
                return Err(Error::Invalid("handling requires schema 3"));
            }
            if replay.schema_version >= 5 {
                Simulation::with_population(
                    replay.seed,
                    config,
                    replay
                        .encounter
                        .ok_or(Error::Invalid("missing encounter setup"))?,
                    replay
                        .population
                        .ok_or(Error::Invalid("missing population setup"))?,
                )?
            } else if replay.schema_version == 4 {
                Simulation::with_encounters(
                    replay.seed,
                    config,
                    replay
                        .encounter
                        .ok_or(Error::Invalid("missing encounter setup"))?,
                )?
            } else {
                Simulation::with_handling(replay.seed, config)?
            }
        } else {
            Simulation::with_schema(
                replay.seed,
                replay.route.unwrap_or_else(Route::straight),
                replay.schema_version,
            )?
        };
        if simulation.schema_version() != requested_schema {
            return Err(Error::Invalid(
                "replay schema does not match enabled setup features",
            ));
        }
        for command in replay.commands {
            simulation.queue(command)?;
        }
        let current = simulation.snapshot();
        Ok(Self {
            simulation,
            previous: current.clone(),
            current,
            frames: 0,
            checkpoints: vec![],
            checkpoint_every_tick: false,
            forensic: vec![],
            observations: vec![],
            limit: replay.ticks,
            debt: 0,
        })
    }
    pub fn frame(&mut self, nanoseconds: u64) -> Result<()> {
        if nanoseconds > 1_000_000_000 {
            return Err(Error::Invalid("frame duration limit"));
        }
        self.observations.clear();
        self.frames += 1;
        self.debt += nanoseconds * u64::from(TICKS_PER_SECOND);
        while self.debt >= 1_000_000_000 && !self.done() {
            self.previous = self.current.clone();
            self.simulation.advance()?;
            self.current = self.simulation.snapshot();
            self.observations
                .push((self.current.clone(), self.simulation.effects().to_vec()));
            if self.checkpoint_every_tick {
                if self.forensic.len() >= 100_000 {
                    return Err(Error::Invalid("forensic tick bound"));
                }
                let digest = |v: &serde_json::Value| {
                    format!(
                        "{:x}",
                        sha2::Sha256::digest(serde_json::to_vec(v).expect("trace values"))
                    )
                };
                let population = self.simulation.population();
                self.forensic.push(ForensicTick {
                    tick: self.current.tick,
                    rng: [self.simulation.spawn_rng.0, self.simulation.gameplay_rng.0],
                    scheduler_sha256: digest(&serde_json::json!({"brains":population.map(|p| &p.brains),"controllers":population.map(|p| &p.controllers),"timers":population.map(|p| &p.timers)})),
                    motion_sha256: digest(&serde_json::json!(self.current.actors.iter().map(|a|(&a.id,&a.rider,&a.encounter)).collect::<Vec<_>>())),
                });
            }
            if self.checkpoint_every_tick
                || self.current.tick.is_multiple_of(30)
                || self.current.tick == self.limit
            {
                self.checkpoints
                    .push((self.current.tick, self.simulation.state_hash()));
            }
            self.debt -= 1_000_000_000;
        }
        Ok(())
    }
    pub fn done(&self) -> bool {
        self.simulation.tick.0 >= self.limit
    }
    pub fn alpha(&self) -> f32 {
        (self.debt.min(1_000_000_000) as f64 / 1_000_000_000.0) as f32
    }
}
pub fn frame_ns(rate: &str, frame: u64) -> std::result::Result<u64, &'static str> {
    match rate {
        "30" => Ok(1_000_000_000 / 30),
        "60" => Ok(1_000_000_000 / 60),
        "144" => Ok(1_000_000_000 / 144),
        "irregular" => Ok([6_944_444, 33_333_333, 11_111_111, 16_666_667][frame as usize % 4]),
        _ => Err("rate must be 30, 60, 144 or irregular"),
    }
}
#[derive(Serialize)]
pub struct ForensicTick {
    pub tick: u64,
    pub rng: [u32; 2],
    pub scheduler_sha256: String,
    pub motion_sha256: String,
}
#[derive(Serialize)]
pub struct Report {
    pub simulation_schema: u32,
    pub rate: String,
    pub ticks: u64,
    pub presentation_frames: u64,
    pub state_sha256: String,
    pub final_snapshot: Snapshot,
    pub tick_hashes: Vec<(u64, String)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forensic: Vec<ForensicTick>,
}
pub fn run(replay: Replay, rate: &str) -> std::result::Result<Report, Box<dyn std::error::Error>> {
    run_with_tick_hashes(replay, rate, false)
}
/// Opt-in forensic mode resolves a checkpoint mismatch to an exact simulation tick.
pub fn run_with_tick_hashes(
    replay: Replay,
    rate: &str,
    every_tick: bool,
) -> std::result::Result<Report, Box<dyn std::error::Error>> {
    let expected = replay.expected_hash.clone();
    if every_tick && replay.ticks > 100_000 {
        return Err("forensic mode supports at most 100000 ticks per run".into());
    }
    let mut playback = Playback::new(replay)?;
    playback.checkpoint_every_tick = every_tick;
    while !playback.done() {
        playback.frame(frame_ns(rate, playback.frames)?)?;
    }
    let hash = playback.simulation.state_hash();
    if expected.is_some_and(|e| e != hash) {
        return Err("replay state hash mismatch".into());
    }
    Ok(Report {
        simulation_schema: playback.simulation.schema_version(),
        rate: rate.into(),
        ticks: playback.simulation.tick.0,
        presentation_frames: playback.frames,
        state_sha256: hash,
        final_snapshot: playback.current,
        tick_hashes: playback.checkpoints,
        forensic: playback.forensic,
    })
}
