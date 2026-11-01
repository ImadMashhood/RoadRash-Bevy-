//! Opt-in local corpus audit. Emits metadata and hashes, never source frames/audio.
use rr_media::video::{self, Poll};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: rr-video-audit DATA_ROOT NEW_OUTPUT_JSON".into());
    }
    let root = PathBuf::from(&args[0]).canonicalize()?;
    let output = std::path::absolute(&args[1])?;
    let parent = output.parent().ok_or("output parent")?.canonicalize()?;
    if parent.starts_with(&root) {
        return Err("output overlaps source".into());
    }
    let output = parent.join(output.file_name().ok_or("output filename")?);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let mut rows = vec![];
    for name in video::catalog(&root)? {
        let path = video::resolve(&root.join("VIDEO"), &PathBuf::from(format!("{name}.AVI")))?;
        let info = video::probe(&path)?;
        let audio = video::audio(&path, &info)?;
        let audio_frames = audio
            .as_ref()
            .map(|a| a.samples.len() / usize::from(a.channels));
        let mut audio_hash = Sha256::new();
        if let Some(audio) = &audio {
            for sample in &audio.samples {
                audio_hash.update(sample.to_le_bytes());
            }
        }
        let mut stream = video::Stream::open(&path, info.clone())?;
        let pid = stream.process_id();
        let started = Instant::now();
        let mut frames = 0;
        let mut hash = Sha256::new();
        loop {
            match stream.poll(frames) {
                Poll::Frame(frame) => {
                    if frame.index != frames {
                        return Err(format!("{name}: skipped frame").into());
                    }
                    hash.update(&frame.rgba);
                    frames += 1;
                }
                Poll::Ended => break,
                Poll::Failed(e) => return Err(format!("{name}: {e}").into()),
                Poll::Waiting => std::thread::sleep(Duration::from_millis(1)),
            }
            if started.elapsed() > Duration::from_secs(120) {
                return Err(format!("{name}: timeout").into());
            }
        }
        if frames != info.frames {
            return Err(format!("{name}: incomplete decode").into());
        }
        drop(stream);
        if PathBuf::from(format!("/proc/{pid}")).exists() {
            return Err("decoder was not reaped".into());
        }
        println!("{name}: {frames} frames, audio {audio_frames:?}");
        rows.push(serde_json::json!({"name":name,"info":info,"frames_decoded":frames,"rgba_sha256":format!("{:x}",hash.finalize()),"audio_frames":audio_frames,"audio_sha256":format!("{:x}",audio_hash.finalize()),"decoder_reaped":true}));
    }
    serde_json::to_writer_pretty(
        file,
        &serde_json::json!({"outcome":"passed","videos":rows.len(),"rows":rows}),
    )?;
    Ok(())
}
