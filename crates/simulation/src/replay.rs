use crate::*;
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
        file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        Self::read(&bytes)
    }
    pub fn read(bytes: &[u8]) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        if bytes.len() > 4 * 1024 * 1024 {
            return Err("replay size limit".into());
        }
        let replay: Self = serde_json::from_slice(bytes)?;
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
    limit: u64,
    debt: u64,
}
impl Playback {
    pub fn new(replay: Replay) -> Result<Self> {
        let mut simulation = Simulation::with_schema(
            replay.seed,
            replay.route.unwrap_or_else(Route::straight),
            replay.schema_version,
        )?;
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
            limit: replay.ticks,
            debt: 0,
        })
    }
    pub fn frame(&mut self, nanoseconds: u64) -> Result<()> {
        if nanoseconds > 1_000_000_000 {
            return Err(Error::Invalid("frame duration limit"));
        }
        self.frames += 1;
        self.debt += nanoseconds * u64::from(TICKS_PER_SECOND);
        while self.debt >= 1_000_000_000 && !self.done() {
            self.previous = self.current.clone();
            self.simulation.advance()?;
            self.current = self.simulation.snapshot();
            if self.current.tick.is_multiple_of(30) || self.current.tick == self.limit {
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
pub struct Report {
    pub simulation_schema: u32,
    pub rate: String,
    pub ticks: u64,
    pub presentation_frames: u64,
    pub state_sha256: String,
    pub final_snapshot: Snapshot,
    pub tick_hashes: Vec<(u64, String)>,
}
pub fn run(replay: Replay, rate: &str) -> std::result::Result<Report, Box<dyn std::error::Error>> {
    let expected = replay.expected_hash.clone();
    let mut playback = Playback::new(replay)?;
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
    })
}
