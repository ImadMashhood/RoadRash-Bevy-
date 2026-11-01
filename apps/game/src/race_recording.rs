//! Optional live-input recording at the application boundary, outside domain ticks.
use bevy::{app::AppExit, prelude::*};
use rr_simulation::{ActorId, Command, QueuedCommand, Simulation, Tick, replay::Replay};
use std::path::{Component, Path, PathBuf};
#[derive(Resource, Default)]
pub struct Recorder {
    pub replay: Option<Replay>,
    path: Option<PathBuf>,
    finished: bool,
}
impl Recorder {
    pub fn new(
        path: Option<PathBuf>,
        simulation: &Simulation,
        data: Option<&Path>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        if path.exists()
            || path.extension().is_none_or(|e| e != "json")
            || path.components().any(|c| matches!(c, Component::ParentDir))
        {
            return Err("recording requires a new .json path without parent traversal".into());
        }
        let absolute = std::path::absolute(&path)?;
        let mut ancestor = absolute.as_path();
        while !ancestor.exists() {
            ancestor = ancestor.parent().ok_or("recording output ancestor")?;
        }
        let resolved = ancestor
            .canonicalize()?
            .join(absolute.strip_prefix(ancestor)?);
        if let Some(data) = data {
            let data = data.canonicalize()?;
            if resolved.starts_with(&data) || data.starts_with(&resolved) {
                return Err("recording output overlaps immutable source".into());
            }
        }
        Ok(Self {
            replay: Some(simulation.recording_setup(1)?),
            path: Some(resolved),
            finished: false,
        })
    }
    pub fn command(&mut self, tick: u64, input: Command) {
        if let Some(replay) = &mut self.replay {
            if replay.commands.len() < 65536 {
                replay.commands.push(QueuedCommand {
                    tick: Tick(tick),
                    actor: ActorId(1),
                    sequence: 0,
                    input,
                });
            } else {
                error!("recording command limit reached");
                self.replay = None;
            }
        }
    }
}
pub fn save(
    mut exits: MessageReader<AppExit>,
    race: Res<super::Race>,
    mut recorder: ResMut<Recorder>,
) {
    if exits.read().next().is_none() || recorder.finished {
        return;
    }
    recorder.finished = true;
    let Some(mut replay) = recorder.replay.take() else {
        return;
    };
    if race.0.tick().0 == 0 {
        return;
    }
    replay.ticks = race.0.tick().0;
    replay.expected_hash = Some(race.0.state_hash());
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let bytes = serde_json::to_vec(&replay)?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err("recording exceeds 4 MiB replay limit".into());
        }
        let path = recorder.path.as_ref().unwrap();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(&bytes)?;
        println!(
            "Recorded {} live commands to {}",
            replay.commands.len(),
            path.display()
        );
        Ok(())
    })();
    if let Err(e) = result {
        error!("recording failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_recording_replays_to_the_same_hash() {
        let root = std::env::temp_dir().join(format!(
            "rr-record-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("session.json");
        let mut simulation = Simulation::default();
        let mut recorder = Recorder::new(Some(path.clone()), &simulation, None).unwrap();
        for tick in 0..120 {
            let command = Command {
                throttle: tick < 80,
                brake: tick >= 80,
                ..Default::default()
            };
            recorder.command(tick, command);
            simulation.step(command).unwrap();
        }
        let hash = simulation.state_hash();
        let mut app = App::new();
        app.add_message::<AppExit>()
            .insert_resource(super::super::Race(simulation))
            .insert_resource(recorder)
            .add_systems(Last, save);
        app.world_mut().write_message(AppExit::Success);
        app.update();
        let replay = Replay::read_file(&path).unwrap();
        assert_eq!(
            rr_simulation::replay::run(replay, "irregular")
                .unwrap()
                .state_sha256,
            hash
        );
        assert!(Recorder::new(Some(path), &Simulation::default(), None).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
