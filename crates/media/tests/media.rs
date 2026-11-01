use rr_media::{audio::*, cues::*, video::*};
use rr_simulation::{
    ActorId,
    combat::{Effect, EffectKind},
    replay::{Playback, Replay, frame_ns},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
fn pcm() -> Pcm {
    Pcm {
        rate: 8000,
        channels: 1,
        samples: vec![-32768, -1, 0, 1, 32767],
    }
}
#[test]
fn pcm_roundtrip_edges_and_every_truncated_prefix_are_bounded() {
    let wave = pcm().wav().unwrap();
    assert_eq!(read_wav(&wave).unwrap().samples, pcm().samples);
    for n in 0..wave.len() {
        assert!(read_wav(&wave[..n]).is_err());
    }
    let mut malformed = wave.clone();
    malformed[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(read_wav(&malformed).is_err());
}
#[test]
fn unsigned_eight_bit_source_samples_decode_exactly() {
    let mut wave = pcm().wav().unwrap();
    wave[4..8].copy_from_slice(&42u32.to_le_bytes());
    wave[28..32].copy_from_slice(&8000u32.to_le_bytes());
    wave[32..34].copy_from_slice(&1u16.to_le_bytes());
    wave[34..36].copy_from_slice(&8u16.to_le_bytes());
    wave[40..44].copy_from_slice(&5u32.to_le_bytes());
    wave.truncate(44);
    wave.extend([0, 127, 128, 129, 255, 0]);
    assert_eq!(
        read_wav(&wave).unwrap().samples,
        vec![-32768, -256, 0, 256, 32512]
    );
}
#[test]
fn malformed_chunks_duplicate_formats_and_bad_layouts_do_not_panic() {
    let base = pcm().wav().unwrap();
    for at in [4, 16, 20, 22, 24, 28, 32, 34, 40] {
        let mut b = base.clone();
        for v in b.iter_mut().skip(at).take(2) {
            *v = 255;
        }
        assert!(std::panic::catch_unwind(|| read_wav(&b)).is_ok());
    }
    let mut b = base.clone();
    b.extend_from_slice(&base[12..36]);
    let len = b.len() as u32 - 8;
    b[4..8].copy_from_slice(&len.to_le_bytes());
    assert!(read_wav(&b).is_err());
    assert!(
        Pcm {
            rate: 0,
            channels: 0,
            samples: vec![]
        }
        .wav()
        .is_err()
    );
}
#[test]
fn engine_rpm_idle_gear_change_crash_and_airborne_have_bounded_transitions() {
    let mut rpm = Rpm::default();
    assert_eq!(rpm.update(256, 0, 0, false), 8192);
    assert_eq!(rpm.update(1024, 10000, 0, false), 16384);
    let previous = rpm.current;
    assert_eq!(rpm.update(4096, 10000, 0, false), previous);
    assert_eq!(rpm.update(1024, 10001, 0, true), previous - 4096);
    let old = rpm.current;
    assert_eq!(rpm.update(1024, 10002, 101, false), old + 4096);
    assert_eq!(pitch_q8(22050, 22050), 256);
    assert_eq!(pitch_q8(-1000, 22050), 1);
    assert!(pitch_q8(999999, 8000) <= 4096);
}
#[test]
fn independent_buses_mute_restore_without_cross_bus_changes() {
    let levels = Levels {
        master: 100,
        engine: 50,
        effects: 25,
        music: 80,
    };
    assert_eq!(levels.gain(Bus::Engine), 0.5);
    assert_eq!(levels.gain(Bus::Effects), 0.25);
    let muted = Levels { music: 0, ..levels };
    assert_eq!(muted.gain(Bus::Music), 0.);
    assert_eq!(muted.gain(Bus::Engine), 0.5);
    assert_eq!(
        Levels {
            master: 0,
            ..levels
        }
        .gain(Bus::Video),
        0.
    );
}
#[test]
fn music_advances_wraps_skips_missing_and_stops_when_all_absent() {
    let mut available = [false; 14];
    assert_eq!(next_track(0, &available), None);
    available[2] = true;
    available[13] = true;
    assert_eq!(next_track(0, &available), Some(2));
    assert_eq!(next_track(3, &available), Some(13));
    assert_eq!(next_track(14, &available), Some(2));
    assert_eq!(ARTISTS.iter().map(|a| a.2).sum::<usize>(), 14);
}
#[test]
fn effects_are_deduplicated_bounded_and_prioritized_under_simultaneous_impacts() {
    let replay = Replay::read(include_bytes!(
        "../../../docs/fixtures/encounter-replay.json"
    ))
    .unwrap();
    let snapshot = Playback::new(replay).unwrap().current;
    let mut p = Planner::default();
    let events: Vec<_> = (1..=1000)
        .map(|id| Effect {
            tick: id,
            actor: ActorId(id),
            other: None,
            kind: if id % 2 == 0 {
                EffectKind::Crash
            } else {
                EffectKind::Swing
            },
        })
        .collect();
    p.capture(&snapshot, &events, 256, 0, true, true);
    assert!(p.events.len() <= 128);
    assert!(p.events.iter().all(|e| e.priority == 100));
    let n = p.events.len();
    p.capture(&snapshot, &events[900..], 256, 0, true, true);
    assert_eq!(p.events.len(), n);
}
#[test]
fn siren_ownership_pauses_resumes_and_cleans_on_exit_without_rng() {
    use rr_simulation::{population::*, topology::*};
    let replay = Replay::read(include_bytes!(
        "../../../docs/fixtures/encounter-replay.json"
    ))
    .unwrap();
    let mut sim = Playback::new(replay).unwrap().simulation;
    let mut s = sim.snapshot();
    let mut users = World::new(Setup::default());
    let id = users
        .spawn(
            &Spawn {
                id: 1,
                location: Location {
                    progress: 1000,
                    ..Default::default()
                },
                kind: Kind::Police,
                family: None,
                selector: 0,
                subtype: 0,
                speed: 0,
                direction: 1,
            },
            0,
            &Topology::straight(10000),
        )
        .unwrap()
        .unwrap();
    users.agents.get_mut(&id).unwrap().siren = true;
    s.road_users = users.agents.into_values().collect();
    let hash = sim.state_hash();
    let mut planner = Planner::default();
    for _ in 0..20 {
        planner.capture(&s, &[], 256, 0, true, true);
        assert!(planner.loops.contains_key(&LoopId::Siren(id)));
        planner.capture(&s, &[], 256, 0, false, true);
        assert!(planner.loops.is_empty());
    }
    assert_eq!(hash, sim.state_hash());
    sim.step(Default::default()).unwrap();
}
#[test]
fn replay_cue_timestamps_are_equal_at_all_frame_schedules_and_never_change_authority() {
    let replay = Replay::read(include_bytes!(
        "../../../docs/fixtures/encounter-replay.json"
    ))
    .unwrap();
    let mut baseline = None;
    for rate in ["30", "60", "144", "irregular"] {
        let mut playback = Playback::new(replay.clone()).unwrap();
        let mut p = Planner::default();
        let mut tape = vec![];
        while !playback.done() {
            playback
                .frame(frame_ns(rate, playback.frames).unwrap())
                .unwrap();
            for (s, e) in &playback.observations {
                let hash = playback.simulation.state_hash();
                p.capture(s, e, 1024, 0, true, true);
                assert_eq!(hash, playback.simulation.state_hash());
                while let Some(event) = p.events.pop_front() {
                    tape.push((event.tick, event.actor, event.name));
                }
            }
        }
        assert!(!tape.is_empty());
        let value = (playback.simulation.state_hash(), tape);
        if let Some(expected) = &baseline {
            assert_eq!(&value, expected);
        } else {
            baseline = Some(value);
        }
    }
}
#[test]
fn clock_long_playback_pause_and_fractional_refresh_do_not_drift() {
    let mut stalled = Clock::default();
    stalled.advance_ns(5_000_000_000);
    assert_eq!(stalled.elapsed_us(), 5_000_000);
    stalled.set_paused(true);
    stalled.advance_ns(5_000_000_000);
    assert_eq!(stalled.elapsed_us(), 5_000_000);
    for rate in ["30", "60", "144", "irregular"] {
        let mut clock = Clock::default();
        let mut total = 0;
        for frame in 0..100000 {
            let ns = frame_ns(rate, frame).unwrap();
            clock.advance_ns(ns);
            total += ns;
            if total >= 600_000_000_000 {
                break;
            }
        }
        assert_eq!(clock.elapsed_us(), total / 1000);
        let before = clock.elapsed_us();
        clock.set_paused(true);
        for _ in 0..100 {
            clock.advance_ns(1_000_000_000);
        }
        assert_eq!(clock.elapsed_us(), before);
        clock.set_paused(false);
        clock.synchronize_audio(10_000);
        assert_eq!(clock.elapsed_us(), 10_000);
    }
}
fn fixture() -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "rr-video-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("authored.avi");
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-nostdin",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=0.5:size=16x16:rate=10",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.5:sample_rate=8000",
            "-c:v",
            "rawvideo",
            "-pix_fmt",
            "bgr24",
            "-c:a",
            "pcm_s16le",
            "-shortest",
        ])
        .arg(&path)
        .status()
        .expect("FFmpeg is a documented test/runtime dependency");
    assert!(status.success());
    (root, path)
}
#[test]
fn bounded_avi_probe_samples_and_stream_frames_match_authored_fixture() {
    let (root, path) = fixture();
    let info = probe(&path).unwrap();
    assert_eq!((info.width, info.height, info.frames), (16, 16, 5));
    let pcm = audio(&path, &info).unwrap().unwrap();
    assert_eq!(pcm.rate, 8000);
    assert_eq!(pcm.samples.len(), 4000);
    let mut stream = Stream::open(&path, info).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = None;
    loop {
        match stream.poll(u64::MAX) {
            Poll::Frame(f) => {
                assert_eq!(f.rgba.len(), 16 * 16 * 4);
                last = Some(f.index);
            }
            Poll::Ended => break,
            Poll::Failed(e) => panic!("{e}"),
            Poll::Waiting => {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    assert_eq!(last, Some(4));
    drop(stream);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn immediate_skip_reaps_decoder_and_rejects_escape_paths() {
    let (root, path) = fixture();
    let info = probe(&path).unwrap();
    let stream = Stream::open(&path, info).unwrap();
    let pid = stream.process_id();
    drop(stream);
    #[cfg(target_os = "linux")]
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    assert!(resolve(&root, std::path::Path::new("../outside.avi")).is_err());
    assert!(probe(std::path::Path::new("https://invalid/video.avi")).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn result_video_families_cover_every_native_variant_and_progression_without_gameplay_rng() {
    for (outcome, prefix, count) in [
        (MovieResult::Win, "WIN", 6),
        (MovieResult::Lose, "LOSE", 10),
        (MovieResult::Busted, "BUSTED", 6),
        (MovieResult::Wreck, "WRECK", 6),
    ] {
        for draw in 0..100u32 {
            assert_eq!(
                result_movie(outcome, false, false, draw),
                Some(format!("{prefix}{}", draw % count + 1))
            );
        }
    }
    assert_eq!(
        result_movie(MovieResult::Win, true, false, 5),
        Some("LEVEL6".into())
    );
    assert_eq!(
        result_movie(MovieResult::Win, true, true, 5),
        Some("FINALWIN".into())
    );
    assert_eq!(result_movie(MovieResult::Quiet, false, false, 5), None);
}
