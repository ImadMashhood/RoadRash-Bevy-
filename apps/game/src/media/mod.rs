//! Bevy media plugin. Snapshots/events are inputs; sound/video never write authoritative simulation.
mod device;
mod movie;
#[cfg(test)]
mod tests;
use crate::{Driver, LaunchOptions, PauseState, Race, frontend::Shell};
use bevy::prelude::*;
use device::Device;
use rr_media::{
    audio::{Levels, Pcm},
    cues::{EFFECTS, ENGINE_VARIANTS, Planner, TRACKS, next_track},
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    time::Instant,
};
type BankResult = (BTreeMap<String, Arc<Pcm>>, Vec<String>);
struct Inbox<T>(std::sync::Mutex<Receiver<T>>);
impl<T> Inbox<T> {
    fn new(rx: Receiver<T>) -> Self {
        Self(std::sync::Mutex::new(rx))
    }
    fn try_recv(&self) -> Result<T, TryRecvError> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).try_recv()
    }
}
#[derive(Resource, Default)]
pub(crate) struct Media {
    requested_track: Option<usize>,
    pub(crate) metrics: serde_json::Value,
    sounds: BTreeMap<String, Arc<Pcm>>,
    specs: Vec<rr_simulation::handling::BikeSpec>,
    bank: Option<Inbox<BankResult>>,
    planner: Planner,
    last_key: Option<u64>,
    desired_track: Option<usize>,
    loaded_track: Option<(usize, Arc<Pcm>)>,
    loading_track: Option<(usize, Inbox<rr_media::Result<Pcm>>)>,
    available: [bool; 14],
    last_tick: u64,
    pub(crate) cue_trace: Vec<rr_media::cues::Event>,
    pub(crate) diagnostics: Vec<String>,
    pub(crate) video_request: Option<String>,
    pub(crate) skip_video: bool,
    pub(crate) movie: Option<movie::Playing>,
    movie_loading: Option<Inbox<rr_media::Result<movie::Prepared>>>,
    pending_video: Option<String>,
    movie_target: Option<rr_session::flow::Screen>,
    seen_result: Option<(u64, rr_session::race::Outcome)>,
    pub(crate) video_frames: u64,
    pub(crate) video_drops: u64,
    pub(crate) movies_completed: u64,
    pub(crate) movies_skipped: u64,
    last_rgb_mean: f64,
}
pub struct MediaPlugin;
impl Plugin for MediaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Media>()
            .insert_non_send(Device::default())
            .add_systems(Startup, (load_bank, movie::setup))
            .add_systems(Last, report)
            .add_systems(
                Update,
                update.after(rr_presentation::PresentationStage::Prepare),
            );
    }
}
pub(crate) fn not_playing(media: Option<Res<Media>>) -> bool {
    media.is_none_or(|m| m.movie.is_none() && m.movie_target.is_none())
}
fn load_bank(options: Res<LaunchOptions>, mut media: ResMut<Media>) {
    if options.video
        && options.smoke_frames.is_none()
        && options.ui_script.is_none()
        && options.replay.is_none()
        && options.frontend.unwrap_or(options.course.is_none())
    {
        media.video_request = Some("INTRO".into());
    }
    let Some(root) = options.data_root.clone() else {
        return;
    };
    media.specs = std::fs::read(root.join("DATA/BIKESPEC.RSC"))
        .ok()
        .and_then(|b| rr_simulation::handling::BikeSpec::read(&b).ok())
        .unwrap_or_default();
    if let Some(config) = options.replay.as_ref().and_then(|r| r.handling.as_ref()) {
        media.specs = config.specs.clone();
    }
    for (i, name) in TRACKS.iter().enumerate() {
        media.available[i] = root.join(format!("AUDIO/MUSIC/{name}.RRA")).is_file();
    }
    let (tx, rx) = mpsc::sync_channel(1);
    media.bank = Some(Inbox::new(rx));
    std::thread::spawn(move || {
        let mut sounds = BTreeMap::new();
        let mut missing = vec![];
        let mut budget = 0usize;
        for name in EFFECTS.into_iter().chain(ENGINE_VARIANTS) {
            if sounds.contains_key(name) {
                continue;
            }
            let result = rr_media::video::resolve(
                &root,
                std::path::Path::new(&format!("AUDIO/EFFECTS/{name}.WAV")),
            )
            .and_then(|p| rr_media::audio::read_file(&p));
            match result {
                Ok(pcm) => {
                    budget += pcm.samples.len() * 2;
                    if budget > 64 * 1024 * 1024 {
                        missing.push("Effect bank exceeds 64 MiB budget".into());
                        break;
                    }
                    sounds.insert(name.into(), Arc::new(pcm));
                }
                Err(e) => missing.push(format!("{name}: {e}")),
            }
        }
        let _ = tx.send((sounds, missing));
    });
}
fn ratio(media: &Media, snapshot: &rr_simulation::Snapshot, bike: u32) -> i32 {
    let gear = snapshot
        .actors
        .iter()
        .find(|a| a.id.0 == 1)
        .and_then(|a| a.rider.as_ref())
        .map_or(0, |r| r.bike.gear);
    media
        .specs
        .iter()
        .find(|s| s.id == bike)
        .and_then(|s| s.gears.get(gear))
        .map_or(256, |g| g.ratio)
}
pub(crate) fn capture(
    race: Res<Race>,
    options: Res<LaunchOptions>,
    shell: Option<Res<Shell>>,
    mut media: ResMut<Media>,
) {
    let snapshot = race.0.snapshot();
    let key = shell
        .as_ref()
        .and_then(|s| s.session.as_ref())
        .map(|s| s.identity().id);
    if media.last_key != key {
        media.planner.reset();
        media.last_key = key;
    }
    let ratio = ratio(&media, &snapshot, options.bike);
    media.planner.capture(
        &snapshot,
        race.0.effects(),
        ratio,
        options.bike.saturating_sub(1) as u8,
        true,
        shell
            .as_ref()
            .map_or(options.effects, |s| s.frontend.settings.effects),
    );
    media.last_tick = snapshot.tick;
}
fn update(world: &mut World) {
    let Some(mut device) = world.remove_non_send::<Device>() else {
        return;
    };
    world.resource_scope(|world, mut media: Mut<Media>| {
        let options = world.resource::<LaunchOptions>().clone();
        let front = world.resource::<Shell>().frontend.clone();
        let paused = world.resource::<PauseState>().paused;
        if let Some(rx) = &media.bank
            && let Ok((sounds, missing)) = rx.try_recv()
        {
            media.sounds = sounds;
            media.diagnostics.extend(missing);
            media.bank = None;
        }
        device.update_device(options.audio && options.data_root.is_some());
        let replay_observations = world
            .resource::<Driver>()
            .0
            .as_ref()
            .map(|d| d.observations.clone())
            .unwrap_or_default();
        for (snapshot, effects) in replay_observations {
            if snapshot.tick <= media.last_tick {
                continue;
            }
            let ratio = ratio(&media, &snapshot, options.bike);
            media.planner.capture(
                &snapshot,
                &effects,
                ratio,
                options.bike.saturating_sub(1) as u8,
                true,
                front.settings.effects,
            );
            media.last_tick = snapshot.tick;
        }
        let live = world.resource::<Driver>().0.is_some()
            || world.resource::<Shell>().race_active()
            || (world.resource::<Shell>().session.is_some()
                && matches!(
                    front.screen,
                    rr_session::flow::Screen::Paused | rr_session::flow::Screen::QuitConfirm
                ));
        let race_playing = live && media.movie.is_none() && media.movie_target.is_none();
        if race_playing {
            device.race(&media.planner.loops, &media.sounds);
        } else {
            device.stop_race();
            media.planner.loops.clear();
        }
        while let Some(event) = media.planner.events.pop_front() {
            if race_playing && let Some(pcm) = media.sounds.get(&event.name) {
                device.effect(pcm.clone(), event.priority);
            }
            if media.cue_trace.len() < 8192 {
                media.cue_trace.push(event);
            }
        }
        // Music belongs to explicit jukebox selection or an active race; leaving either stops it.
        let selection = if matches!(
            front.screen,
            rr_session::flow::Screen::Jukebox | rr_session::flow::Screen::Tracks
        ) {
            front.settings.track.map(usize::from)
        } else if race_playing {
            Some(0)
        } else {
            None
        };
        if selection != media.requested_track {
            media.requested_track = selection;
            media.desired_track = selection;
            media.loaded_track = None;
            device.music = None;
        }
        if let Some((index, rx)) = &media.loading_track {
            match rx.try_recv() {
                Ok(result) => {
                    let index = *index;
                    media.loading_track = None;
                    match result {
                        Ok(pcm) => {
                            if media.desired_track == Some(index) {
                                media.loaded_track = Some((index, Arc::new(pcm)));
                            }
                        }
                        Err(e) => {
                            media.available[index] = false;
                            media.diagnostics.push(e);
                            media.desired_track = next_track(index + 1, &media.available);
                        }
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    media.loading_track = None;
                    media.desired_track = None;
                }
            }
        }
        if media.movie.is_some() || media.movie_target.is_some() {
            device.music = None;
        } else if let Some(index) = media.desired_track {
            if let Some((loaded, pcm)) = &media.loaded_track
                && *loaded == index
            {
                if device.music.as_ref().is_some_and(|p| p.empty()) {
                    media.desired_track = next_track(index + 1, &media.available);
                    media.loaded_track = None;
                    device.music = None;
                } else if device.music.is_none() {
                    device.start_music(pcm.clone());
                }
            }
            if media.loaded_track.is_none() && media.loading_track.is_none() {
                if let Some(index) = next_track(index, &media.available)
                    && let Some(root) = &options.data_root
                {
                    media.desired_track = Some(index);
                    let path = root.join(format!("AUDIO/MUSIC/{}.RRA", TRACKS[index]));
                    let (tx, rx) = mpsc::sync_channel(1);
                    media.loading_track = Some((index, Inbox::new(rx)));
                    std::thread::spawn(move || {
                        let result = rr_media::video::resolve(
                            path.parent()
                                .and_then(|p| p.parent())
                                .and_then(|p| p.parent())
                                .unwrap(),
                            std::path::Path::new(&format!("AUDIO/MUSIC/{}.RRA", TRACKS[index])),
                        )
                        .and_then(|p| rr_media::audio::read_file(&p));
                        let _ = tx.send(result);
                    });
                } else {
                    media.desired_track = None;
                }
            }
        } else {
            device.music = None;
            media.loaded_track = None;
        }
        movie::update(world, &mut media, &mut device);
        let levels = Levels {
            master: front.settings.master_volume,
            engine: front.settings.engine_volume,
            effects: front.settings.effect_volume,
            music: front.settings.music_volume,
        };
        let movie_paused = media.movie.as_ref().is_some_and(|m| m.clock.paused());
        device.mix(
            levels,
            front.settings.effects,
            front.settings.music,
            paused,
            movie_paused,
        );
        media.metrics = serde_json::json!({
            "device": device.status,
            "screen": front.screen,
            "race_paused": paused,
            "master": levels.master,
            "music": levels.music,
            "voices": device.voices(),
            "music_position_us": device.music.as_ref().map(|p| p.get_pos().as_micros() as u64),
            "movie_position_us": device.movie_position(),
            "movie_clock_us": media.movie.as_ref().map(|m| m.clock.elapsed_us()),
            "video_frames": media.video_frames,
            "movie_frame": media.movie.as_ref().and_then(|m| m.last_frame),
            "last_frame_rgb_mean": media.last_rgb_mean,
        });
        if media.diagnostics.len() > 64 {
            let keep = media.diagnostics.len() - 64;
            media.diagnostics.drain(..keep);
        }
    });
    world.insert_non_send(device);
}

fn report(
    mut exits: MessageReader<bevy::app::AppExit>,
    options: Res<LaunchOptions>,
    media: Res<Media>,
    device: NonSend<Device>,
) {
    if exits.read().next().is_none() {
        return;
    }
    let Some(path) = &options.media_report else {
        return;
    };
    let report = serde_json::json!({
        "schema_version": 1,
        "device_status": device.status,
        "device_resets": device.resets,
        "voice_starts": device.starts,
        "peak_voices": device.peak_voices,
        "voices_at_exit": device.voices(),
        "source_effects_loaded": media.sounds.len(),
        "cue_trace": media.cue_trace,
        "video_frames": media.video_frames,
        "video_frames_dropped": media.video_drops,
        "movies_completed": media.movies_completed,
        "movies_skipped": media.movies_skipped,
        "video_active": media.movie.is_some(),
        "diagnostics": media.diagnostics,
    });
    let result = (|| -> Result<(), String> {
        let path = rr_session::persistence::disk::guarded(path, options.data_root.as_deref())?;
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut f| f.write_all(&serde_json::to_vec_pretty(&report).unwrap()))
            .map_err(|e| e.to_string())
    })();
    if let Err(e) = result {
        error!("Media report: {e}");
    }
}
