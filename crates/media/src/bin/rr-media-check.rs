//! Private media census and pinned primitive probe. Only metadata receipts are publishable.
use rr_media::{
    audio::{Rpm, pitch_q8, read_file},
    video,
};
use serde_json::json;
use std::path::Path;
fn digest(pcm: &[i16]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for s in pcm {
        for b in s.to_le_bytes() {
            hash = (hash ^ u64::from(b)).wrapping_mul(0x100000001b3);
        }
    }
    format!("{hash:016x}")
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: rr-media-check DATA_ROOT OUTPUT_DIR".into());
    }
    let root = Path::new(&args[0]).canonicalize()?;
    let output = std::path::absolute(&args[1])?;
    if output
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("output must not contain parent traversal".into());
    }
    let mut ancestor = output.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("output ancestor")?;
    }
    let suffix = output.strip_prefix(ancestor)?;
    let resolved = if suffix.as_os_str().is_empty() {
        ancestor.canonicalize()?
    } else {
        ancestor.canonicalize()?.join(suffix)
    };
    if resolved.starts_with(&root) || root.starts_with(&resolved) {
        return Err("output overlaps immutable source".into());
    }
    std::fs::create_dir_all(&resolved)?;
    let mut paths = vec![];
    for dir in ["AUDIO/EFFECTS", "AUDIO/MUSIC"] {
        for entry in std::fs::read_dir(root.join(dir))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "WAV" || e == "RRA") {
                paths.push(path.strip_prefix(&root)?.to_path_buf());
            }
        }
    }
    paths.sort();
    let mut wav = vec![];
    for path in paths {
        let pcm = read_file(&root.join(&path))?;
        wav.push(json!([
            path.to_string_lossy(),
            pcm.rate,
            pcm.channels,
            pcm.samples.len(),
            digest(&pcm.samples)
        ]));
    }
    let mut rpm = vec![];
    for current in [0, 8192, 16000, 40000] {
        for previous_speed in [0, 10000] {
            for ratio in [256, 1024, 4096] {
                for speed in [0, 1, 10000, 30000] {
                    for height in [0, 101] {
                        for crashed in [false, true] {
                            let mut p = Rpm {
                                current,
                                previous_speed,
                            };
                            p.update(ratio, speed, height, crashed);
                            rpm.push(json!([p.current, p.previous_speed]));
                        }
                    }
                }
            }
        }
    }
    let mut pitch = vec![];
    for frequency in [-1, 0, 100, 8192, 16000, 100000, 200000] {
        for rate in [8000, 22050, 44100, 48000] {
            pitch.push(pitch_q8(frequency, rate));
        }
    }
    let mut avi = vec![];
    for name in ["WIN1", "BUSTED1", "WRECK1", "INTRO"] {
        let path = root.join(format!("VIDEO/{name}.AVI"));
        let info = video::probe(&path)?;
        let audio = video::audio(&path, &info)?.ok_or("missing AVI audio")?;
        let mut raw = vec![];
        for s in &audio.samples {
            raw.extend(s.to_le_bytes());
        }
        std::fs::write(resolved.join(format!("rust-{name}.pcm")), raw)?;
        avi.push(json!([
            name,
            info.width,
            info.height,
            info.fps_num,
            info.fps_den,
            info.frames,
            audio.samples.len(),
            digest(&audio.samples)
        ]));
    }
    std::fs::write(
        resolved.join("media-rust.json"),
        serde_json::to_vec(&json!({"wav":wav,"rpm":rpm,"pitch":pitch,"avi":avi}))?,
    )?;
    println!("86 source WAV/RRA files; 384 RPM, 28 pitch and four AVI cases");
    Ok(())
}
