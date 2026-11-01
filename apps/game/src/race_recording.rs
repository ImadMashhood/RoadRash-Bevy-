//! Optional live-input recording at the application boundary, outside domain ticks.
use bevy::{app::AppExit, prelude::*};
use rr_simulation::{ActorId, Command, QueuedCommand, Simulation, Tick, replay::Replay};
use std::path::{Component, Path, PathBuf};
#[derive(Resource, Default)]
pub struct Recorder {
    pub replay: Option<Replay>,
    path: Option<PathBuf>,
    finished: bool,
    last: Option<Command>,
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
            replay: Some(simulation.recording_setup(simulation.initial_seed())?),
            path: Some(resolved),
            finished: false,
            last: None,
        })
    }
    pub fn command(&mut self, tick: u64, input: Command) {
        let transient = input.attack.is_some()
            || input.walk.is_some()
            || input.impact.is_some()
            || input.terminal.is_some()
            || input.pose.is_some()
            || input.weapon.is_some()
            || input.damage != 0
            || input.fork.is_some();
        if !transient && self.last == Some(input) {
            return;
        }
        self.last = Some(input);
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
impl Recorder {
    pub fn finish(&mut self, simulation: &Simulation) {
        if self.finished {
            return;
        }
        self.finished = true;
        let Some(mut replay) = self.replay.take() else {
            return;
        };
        if simulation.tick().0 == 0 {
            return;
        }
        replay.ticks = simulation.tick().0;
        replay.expected_hash = Some(simulation.state_hash());
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let bytes = serde_json::to_vec(&replay)?;
            // Enforce version-specific production reader limits before creating output.
            Replay::read(&bytes)?;
            let path = self.path.as_ref().unwrap();
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
    pub fn rebase(
        &mut self,
        simulation: &Simulation,
        data: Option<&Path>,
        race_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.path.clone().map(|p| {
            if p.exists() {
                p.with_file_name(format!(
                    "{}-race-{race_id}.json",
                    p.file_stem().unwrap().to_string_lossy()
                ))
            } else {
                p
            }
        });
        *self = Self::new(path, simulation, data)?;
        Ok(())
    }
}
pub fn save(
    mut exits: MessageReader<AppExit>,
    race: Res<super::Race>,
    mut recorder: ResMut<Recorder>,
) {
    if exits.read().next().is_some() {
        recorder.finish(&race.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steady_controls_coalesce_but_repeated_transient_actions_are_preserved() {
        let mut recorder = Recorder {
            replay: Some(Simulation::default().recording_setup(1).unwrap()),
            ..Default::default()
        };
        let gas = Command {
            throttle: true,
            ..Default::default()
        };
        for tick in 0..36000 {
            recorder.command(tick, gas);
        }
        assert_eq!(recorder.replay.as_ref().unwrap().commands.len(), 1);
        let punch = Command {
            attack: Some(rr_simulation::combat::Request {
                kind: rr_simulation::combat::AttackKind::Punch,
                side: rr_simulation::combat::Side::Auto,
            }),
            ..gas
        };
        for tick in 36000..36010 {
            recorder.command(tick, punch);
        }
        assert_eq!(recorder.replay.as_ref().unwrap().commands.len(), 11);
        recorder.command(36010, Command::default());
        assert_eq!(recorder.replay.as_ref().unwrap().commands.len(), 12);
    }
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
