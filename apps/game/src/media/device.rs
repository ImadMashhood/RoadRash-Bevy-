//! Main-thread CPAL/Rodio ownership; device errors retire voices and retry without touching gameplay.
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use rr_media::{
    audio::{Bus, Levels, Pcm},
    cues::{LoopId, Voice},
};
use std::{
    collections::BTreeMap,
    num::{NonZeroU16, NonZeroU32},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Clone)]
struct Samples {
    pcm: Arc<Pcm>,
    at: usize,
}
impl Iterator for Samples {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let value = *self.pcm.samples.get(self.at)?;
        self.at += 1;
        Some(f32::from(value) / 32768.)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.pcm.samples.len() - self.at;
        (n, Some(n))
    }
}
impl Source for Samples {
    fn current_span_len(&self) -> Option<usize> {
        Some(if self.at >= self.pcm.samples.len() {
            0
        } else {
            self.pcm.samples.len()
        })
    }
    fn channels(&self) -> NonZeroU16 {
        NonZeroU16::new(self.pcm.channels).unwrap()
    }
    fn sample_rate(&self) -> NonZeroU32 {
        NonZeroU32::new(self.pcm.rate).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_micros(self.pcm.duration_us()))
    }
}
struct Owned {
    sound: String,
    player: Player,
    bus: Bus,
    gain: f32,
    priority: u8,
}
pub(super) struct Device {
    output: Option<MixerDeviceSink>,
    loops: BTreeMap<LoopId, Owned>,
    shots: Vec<Owned>,
    pub music: Option<Player>,
    pub movie: Option<Player>,
    pub movie_offset: u64,
    failed: Arc<AtomicBool>,
    retry: Instant,
    pub status: String,
    pub resets: u64,
    pub starts: u64,
    pub peak_voices: usize,
}
impl Default for Device {
    fn default() -> Self {
        Self {
            output: None,
            loops: BTreeMap::new(),
            shots: vec![],
            music: None,
            movie: None,
            movie_offset: 0,
            failed: Arc::new(AtomicBool::new(false)),
            retry: Instant::now(),
            status: "idle".into(),
            resets: 0,
            starts: 0,
            peak_voices: 0,
        }
    }
}
impl Device {
    pub fn connected(&self) -> bool {
        self.output.is_some()
    }
    pub fn voices(&self) -> usize {
        self.loops.len()
            + self.shots.len()
            + usize::from(self.music.is_some())
            + usize::from(self.movie.is_some())
    }
    fn retire(&mut self) {
        self.loops.clear();
        self.shots.clear();
        self.music = None;
        self.movie = None;
        self.output = None;
        self.resets += 1;
    }
    pub fn update_device(&mut self, enabled: bool) {
        if !enabled {
            if self.output.is_some() {
                self.retire();
            }
            self.status = "disabled".into();
            return;
        }
        if self.failed.swap(false, Ordering::Relaxed) {
            self.retire();
            self.retry = Instant::now() + Duration::from_secs(2);
            self.status = "device lost; retrying".into();
        }
        if self.output.is_none() && Instant::now() >= self.retry {
            let flag = self.failed.clone();
            let result = DeviceSinkBuilder::from_default_device()
                .map(|b| {
                    b.with_error_callback(move |_| {
                        flag.store(true, Ordering::Relaxed);
                    })
                })
                .and_then(|b| b.open_sink_or_fallback());
            match result {
                Ok(mut output) => {
                    output.log_on_drop(false);
                    self.output = Some(output);
                    self.status = "connected".into();
                }
                Err(e) => {
                    self.status = format!("unavailable: {e}");
                    self.retry = Instant::now() + Duration::from_secs(2);
                }
            }
        }
    }
    fn play(&mut self, pcm: Arc<Pcm>, looped: bool, offset: u64) -> Option<Player> {
        let output = self.output.as_ref()?;
        let player = Player::connect_new(output.mixer());
        player.set_volume(0.);
        player.pause();
        let channels = usize::from(pcm.channels);
        let at = ((u128::from(offset) * u128::from(pcm.rate) / 1_000_000) as usize)
            .saturating_mul(channels)
            .min(pcm.samples.len());
        let source = Samples { pcm, at };
        if looped {
            player.append(source.repeat_infinite());
        } else {
            player.append(source);
        }
        self.starts += 1;
        Some(player)
    }
    pub fn race(&mut self, desired: &BTreeMap<LoopId, Voice>, sounds: &BTreeMap<String, Arc<Pcm>>) {
        self.loops.retain(|id, owned| {
            desired
                .get(id)
                .is_some_and(|voice| voice.name == owned.sound)
        });
        for (id, voice) in desired {
            if !self.loops.contains_key(id)
                && let Some(pcm) = sounds.get(&voice.name)
                && let Some(player) = self.play(pcm.clone(), true, 0)
            {
                self.loops.insert(
                    *id,
                    Owned {
                        sound: voice.name.clone(),
                        player,
                        bus: voice.bus,
                        gain: voice.gain,
                        priority: voice.priority,
                    },
                );
            }
            if let Some(owned) = self.loops.get_mut(id) {
                owned.player.set_speed(f32::from(voice.pitch) / 256.);
                owned.gain = voice.gain;
            }
        }
    }
    pub fn effect(&mut self, pcm: Arc<Pcm>, priority: u8) {
        self.shots.retain(|v| !v.player.empty());
        if self.shots.len() >= 16 {
            let (at, low) = self
                .shots
                .iter()
                .enumerate()
                .min_by_key(|(_, s)| s.priority)
                .map(|(i, s)| (i, s.priority))
                .unwrap();
            if low > priority {
                return;
            }
            self.shots.remove(at);
        }
        if let Some(player) = self.play(pcm, false, 0) {
            self.shots.push(Owned {
                sound: String::new(),
                player,
                bus: Bus::Effects,
                gain: 0.18,
                priority,
            });
        }
    }
    pub fn start_music(&mut self, pcm: Arc<Pcm>) {
        self.music = None;
        self.music = self.play(pcm, false, 0);
    }
    pub fn start_movie(&mut self, pcm: Arc<Pcm>, offset: u64) {
        self.movie = None;
        self.movie_offset = offset;
        self.movie = self.play(pcm, false, offset);
    }
    pub fn stop_race(&mut self) {
        self.loops.clear();
        self.shots.clear();
    }
    pub fn stop_movie(&mut self) {
        self.movie = None;
        self.movie_offset = 0;
    }
    /// The sum of requested gains is normalized to 0.85, leaving headroom even for simultaneous peaks.
    pub fn mix(
        &mut self,
        levels: Levels,
        effects: bool,
        music: bool,
        paused: bool,
        movie_paused: bool,
    ) {
        self.shots.retain(|v| !v.player.empty());
        let sum = self
            .loops
            .values()
            .chain(self.shots.iter())
            .map(|v| {
                v.gain
                    * levels.gain(v.bus)
                    * if v.bus == Bus::Effects && !effects {
                        0.
                    } else {
                        1.
                    }
            })
            .sum::<f32>()
            + if self.music.is_some() && music {
                0.35 * levels.gain(Bus::Music)
            } else {
                0.
            }
            + if self.movie.is_some() {
                0.5 * levels.gain(Bus::Video)
            } else {
                0.
            };
        let normalize = if sum > 0.85 { 0.85 / sum } else { 1. };
        for v in self.loops.values().chain(self.shots.iter()) {
            v.player.set_volume(
                v.gain
                    * levels.gain(v.bus)
                    * normalize
                    * if v.bus == Bus::Effects && !effects {
                        0.
                    } else {
                        1.
                    },
            );
            if paused {
                v.player.pause();
            } else {
                v.player.play();
            }
        }
        if let Some(p) = &self.music {
            p.set_volume(if music {
                0.35 * levels.gain(Bus::Music) * normalize
            } else {
                0.
            });
            if paused {
                p.pause();
            } else {
                p.play();
            }
        }
        if let Some(p) = &self.movie {
            p.set_volume(0.5 * levels.gain(Bus::Video) * normalize);
            if movie_paused {
                p.pause();
            } else {
                p.play();
            }
        }
        self.peak_voices = self.peak_voices.max(self.voices());
    }
    pub fn movie_position(&self) -> Option<u64> {
        self.movie
            .as_ref()
            .filter(|p| !p.empty())
            .map(|p| self.movie_offset + p.get_pos().as_micros() as u64)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pcm_iterator_uses_shared_bounded_samples_and_frame_alignment() {
        let pcm = Arc::new(Pcm {
            rate: 8000,
            channels: 2,
            samples: vec![i16::MIN, 0, i16::MAX, 0],
        });
        let mut stream = Samples {
            pcm: pcm.clone(),
            at: 0,
        };
        assert_eq!(stream.next(), Some(-1.));
        assert_eq!(stream.channels().get(), 2);
        assert_eq!(stream.clone().collect::<Vec<_>>().len(), 3);
        assert_eq!(Arc::strong_count(&pcm), 2);
    }
    #[test]
    fn offline_mixer_consumes_pcm_with_nonzero_output_and_advancing_position() {
        let (mixer, mut output) =
            rodio::mixer::mixer(NonZeroU16::new(2).unwrap(), NonZeroU32::new(48000).unwrap());
        let player = Player::connect_new(&mixer);
        let pcm = Arc::new(Pcm {
            rate: 8000,
            channels: 1,
            samples: vec![16384; 8000],
        });
        player.append(Samples { pcm, at: 0 });
        let samples: Vec<_> = output.by_ref().take(96000).collect();
        assert!(samples.iter().any(|s| s.abs() > 0.1));
        assert!(player.get_pos() > Duration::from_millis(500));
        assert!(player.empty());
    }
    #[test]
    fn software_mixer_caps_simultaneous_peaks_and_repeated_pause_keeps_position() {
        let (mixer, mut output) =
            rodio::mixer::mixer(NonZeroU16::new(2).unwrap(), NonZeroU32::new(8000).unwrap());
        let mut d = Device::default();
        let pcm = Arc::new(Pcm {
            rate: 8000,
            channels: 1,
            samples: vec![32767; 16000],
        });
        for _ in 0..16 {
            let player = Player::connect_new(&mixer);
            player.set_volume(0.);
            player.append(Samples {
                pcm: pcm.clone(),
                at: 0,
            });
            d.shots.push(Owned {
                sound: String::new(),
                player,
                bus: Bus::Effects,
                gain: 0.18,
                priority: 70,
            });
        }
        d.mix(Levels::default(), true, true, false, false);
        let peak = output.by_ref().take(1600).fold(0f32, |p, s| p.max(s.abs()));
        assert!(peak > 0.5 && peak <= 0.851);
        let before = d.shots[0].player.get_pos();
        d.mix(Levels::default(), true, true, true, false);
        let _: Vec<_> = output.by_ref().take(1600).collect();
        let held = d.shots[0].player.get_pos();
        assert!(held <= before + Duration::from_millis(6));
        d.mix(Levels::default(), true, true, false, false);
        let _: Vec<_> = output.by_ref().take(1600).collect();
        assert!(d.shots[0].player.get_pos() > held);
        assert_eq!(d.voices(), 16);
    }
    #[test]
    fn disabled_device_is_safe_and_retires_all_ownership() {
        let mut d = Device::default();
        d.update_device(false);
        assert!(!d.connected());
        assert_eq!(d.voices(), 0);
        assert_eq!(d.status, "disabled");
    }
    #[test]
    fn nitro_sample_change_retires_the_previous_engine_loop_even_with_the_same_owner() {
        let (mixer, _) =
            rodio::mixer::mixer(NonZeroU16::new(2).unwrap(), NonZeroU32::new(8000).unwrap());
        let mut d = Device::default();
        d.loops.insert(
            LoopId::Engine,
            Owned {
                sound: "ENGRAT".into(),
                player: Player::connect_new(&mixer),
                bus: Bus::Engine,
                gain: 0.25,
                priority: 80,
            },
        );
        let mut desired = BTreeMap::from([(
            LoopId::Engine,
            Voice {
                name: "ENGRAT".into(),
                bus: Bus::Engine,
                pitch: 256,
                gain: 0.25,
                priority: 80,
            },
        )]);
        d.race(&desired, &BTreeMap::new());
        assert_eq!(d.loops.len(), 1);
        desired.get_mut(&LoopId::Engine).unwrap().name = "ENGNITRO".into();
        d.race(&desired, &BTreeMap::new());
        assert!(
            d.loops.is_empty(),
            "an unavailable replacement must not leave the wrong engine sample playing"
        );
    }
}
