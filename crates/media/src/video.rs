//! Local-only FFmpeg adapter. Dimensions, duration, subprocess time and buffered frames are bounded.
use crate::{
    Result,
    audio::{MAX_INPUT, Pcm},
};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Info {
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub frames: u64,
    pub duration_us: u64,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub audio_rate: u32,
    pub audio_channels: u16,
}
impl Info {
    pub fn validate(&self) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || self.width > 1920
            || self.height > 1080
            || self.fps_num == 0
            || self.fps_den == 0
            || u64::from(self.fps_num) > 60 * u64::from(self.fps_den)
            || self.frames == 0
            || self.frames > 36000
            || self.duration_us == 0
            || self.duration_us > 600_000_000
            || !matches!(self.video_codec.as_str(), "cinepak" | "rawvideo")
            || !(1..=2).contains(&self.audio_channels)
            || !(8000..=192000).contains(&self.audio_rate)
        {
            return Err("Unsupported/oversized AVI stream layout (Cinepak/raw, <=1920x1080, <=60 fps, <=10 min)".into());
        }
        if self
            .audio_codec
            .as_ref()
            .is_some_and(|c| !matches!(c.as_str(), "adpcm_ima_wav" | "pcm_u8" | "pcm_s16le"))
        {
            return Err("Unsupported AVI audio codec".into());
        }
        Ok(())
    }
    pub fn target_frame(&self, elapsed_us: u64) -> u64 {
        ((u128::from(elapsed_us) * u128::from(self.fps_num))
            / (1_000_000 * u128::from(self.fps_den)))
        .min(u128::from(self.frames - 1)) as u64
    }
}
pub fn resolve(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Media requires a relative local catalog path".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let p = root
        .join(relative)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !p.starts_with(root) {
        return Err("Media escapes imported source root".into());
    }
    input(&p)?;
    Ok(p)
}
fn input(path: &Path) -> Result<PathBuf> {
    let p = path.canonicalize().map_err(|e| e.to_string())?;
    let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > 256 * 1024 * 1024 {
        return Err("Media input must be a regular file <=256 MiB".into());
    }
    Ok(p)
}
fn bounded(command: &mut Command, cap: usize, timeout: Duration) -> Result<Vec<u8>> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Media decoder unavailable: {e}"))?;
    let stdout = child.stdout.take().ok_or("Decoder stdout")?;
    let (tx, rx) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        let mut bytes = vec![];
        let result = stdout
            .take(cap as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
            .map_err(|e| e.to_string());
        let _ = tx.send(result);
    });
    let start = Instant::now();
    let mut output = None;
    let result = loop {
        if output.is_none() {
            match rx.try_recv() {
                Ok(bytes) => match bytes {
                    Ok(bytes) if bytes.len() <= cap => output = Some(bytes),
                    Ok(_) => break Err("Decoder output exceeds configured bound".into()),
                    Err(e) => break Err(e),
                },
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    break Err("Decoder output channel closed".into());
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    break Err(format!("Media decoder rejected input ({status})"));
                }
                if let Some(bytes) = output.take() {
                    break Ok(bytes);
                }
            }
            Ok(None) => {}
            Err(e) => break Err(e.to_string()),
        }
        if start.elapsed() > timeout {
            break Err("Media decoder timed out".into());
        }
        thread::sleep(Duration::from_millis(2));
    };
    if result.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    let _ = reader.join();
    result
}
pub fn probe(path: &Path) -> Result<Info> {
    let path = input(path)?;
    let raw=bounded(Command::new("ffprobe").args(["-v","error","-protocol_whitelist","file,pipe","-show_entries","stream=codec_name,codec_type,width,height,r_frame_rate,nb_frames,sample_rate,channels:format=duration","-of","json"]).arg(path),65536,Duration::from_secs(5))?;
    let value: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let streams = value["streams"].as_array().ok_or("AVI streams missing")?;
    if streams.len() > 4 {
        return Err("Too many AVI streams".into());
    }
    let video = streams
        .iter()
        .find(|s| s["codec_type"] == "video")
        .ok_or("AVI video stream missing")?;
    let audio = streams.iter().find(|s| s["codec_type"] == "audio");
    let ratio = video["r_frame_rate"]
        .as_str()
        .ok_or("AVI frame rate missing")?;
    let (num, den) = ratio.split_once('/').ok_or("AVI frame rate rational")?;
    let seconds = value["format"]["duration"]
        .as_str()
        .ok_or("AVI duration missing")?
        .parse::<f64>()
        .map_err(|e| e.to_string())?;
    if !seconds.is_finite() || !(0.0..=600.0).contains(&seconds) {
        return Err("AVI duration bounds".into());
    }
    let info = Info {
        width: video["width"]
            .as_u64()
            .ok_or("AVI width")?
            .try_into()
            .map_err(|_| "AVI width overflow")?,
        height: video["height"]
            .as_u64()
            .ok_or("AVI height")?
            .try_into()
            .map_err(|_| "AVI height overflow")?,
        fps_num: num.parse().map_err(|_| "AVI rate numerator")?,
        fps_den: den.parse().map_err(|_| "AVI rate denominator")?,
        frames: video["nb_frames"]
            .as_str()
            .ok_or("AVI frame count missing")?
            .parse()
            .map_err(|_| "AVI frame count")?,
        duration_us: (seconds * 1_000_000.).round() as u64,
        video_codec: video["codec_name"].as_str().ok_or("AVI codec")?.into(),
        audio_codec: audio.and_then(|a| a["codec_name"].as_str().map(str::to_string)),
        audio_rate: audio
            .and_then(|a| a["sample_rate"].as_str())
            .unwrap_or("22050")
            .parse()
            .map_err(|_| "AVI audio rate")?,
        audio_channels: audio
            .and_then(|a| a["channels"].as_u64())
            .unwrap_or(2)
            .try_into()
            .map_err(|_| "AVI audio channels")?,
    };
    info.validate()?;
    Ok(info)
}
pub fn audio(path: &Path, info: &Info) -> Result<Option<Pcm>> {
    info.validate()?;
    if info.audio_codec.is_none() {
        return Ok(None);
    }
    let path = input(path)?;
    let raw = bounded(
        Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-nostdin",
                "-threads",
                "1",
                "-protocol_whitelist",
                "file,pipe",
                "-i",
            ])
            .arg(path)
            .args([
                "-map",
                "0:a:0",
                "-vn",
                "-acodec",
                "pcm_s16le",
                "-f",
                "s16le",
                "pipe:1",
            ]),
        MAX_INPUT,
        Duration::from_secs(20),
    )?;
    if raw.len() % 2 != 0 {
        return Err("Incomplete decoded PCM sample".into());
    }
    let sound = Pcm {
        rate: info.audio_rate,
        channels: info.audio_channels,
        samples: raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|s| i16::from_le_bytes([s[0], s[1]]))
            .collect(),
    };
    sound.validate()?;
    Ok(Some(sound))
}
#[derive(Debug)]
pub struct Frame {
    pub index: u64,
    pub rgba: Vec<u8>,
}
pub enum Poll {
    Waiting,
    Frame(Frame),
    Ended,
    Failed(String),
}
pub struct Stream {
    pub info: Info,
    receiver: Mutex<Receiver<Result<Frame>>>,
    child: Arc<Mutex<Child>>,
    cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    pending: Option<Frame>,
    last_progress: Instant,
    ended: bool,
}
impl Stream {
    pub fn open(path: &Path, info: Info) -> Result<Self> {
        info.validate()?;
        let path = input(path)?;
        let mut child = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-nostdin",
                "-threads",
                "1",
                "-protocol_whitelist",
                "file,pipe",
                "-i",
            ])
            .arg(path)
            .args([
                "-map", "0:v:0", "-an", "-pix_fmt", "rgba", "-vsync", "0", "-f", "rawvideo",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut stdout = child.stdout.take().ok_or("Video decoder stdout")?;
        let child = Arc::new(Mutex::new(child));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, receiver) = mpsc::sync_channel(3);
        let stop = cancel.clone();
        let process = child.clone();
        let length = info.width as usize * info.height as usize * 4;
        let count = info.frames;
        let worker = thread::spawn(move || {
            for index in 0..count {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let mut rgba = vec![0; length];
                if let Err(e) = stdout.read_exact(&mut rgba) {
                    let _ =
                        tx.try_send(Err(format!("Truncated/rejected video frame {index}: {e}")));
                    break;
                }
                let mut item = Ok(Frame { index, rgba });
                loop {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    match tx.try_send(item) {
                        Ok(()) => break,
                        Err(mpsc::TrySendError::Full(v)) => {
                            item = v;
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => {
                            stop.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                }
            }
            let mut process = process.lock().unwrap_or_else(|e| e.into_inner());
            let _ = process.kill();
            let _ = process.wait();
        });
        Ok(Self {
            info,
            receiver: Mutex::new(receiver),
            child,
            cancel,
            worker: Some(worker),
            pending: None,
            last_progress: Instant::now(),
            ended: false,
        })
    }
    /// Select the newest decoded frame not later than the clock; old frames are dropped.
    pub fn process_id(&self) -> u32 {
        self.child.lock().unwrap_or_else(|e| e.into_inner()).id()
    }
    pub fn poll(&mut self, target: u64) -> Poll {
        let mut selected = None;
        if let Some(frame) = self.pending.take() {
            if frame.index > target {
                self.pending = Some(frame);
                return Poll::Waiting;
            }
            selected = Some(frame);
        }
        for _ in 0..64 {
            match self
                .receiver
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .try_recv()
            {
                Ok(Ok(frame)) => {
                    self.last_progress = Instant::now();
                    if frame.index > target {
                        self.pending = Some(frame);
                        break;
                    }
                    selected = Some(frame);
                }
                Ok(Err(e)) => return Poll::Failed(e),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.ended = true;
                    break;
                }
            }
        }
        if let Some(frame) = selected {
            return Poll::Frame(frame);
        }
        if self.ended {
            Poll::Ended
        } else if self.last_progress.elapsed() > Duration::from_secs(10) {
            Poll::Failed("Video decoder made no progress for ten seconds".into())
        } else {
            Poll::Waiting
        }
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        {
            let mut child = self.child.lock().unwrap_or_else(|e| e.into_inner());
            let _ = child.kill();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Clock {
    elapsed_us: u64,
    paused: bool,
    remainder_ns: u64,
}
impl Clock {
    pub fn elapsed_us(&self) -> u64 {
        self.elapsed_us
    }
    pub fn paused(&self) -> bool {
        self.paused
    }
    pub fn set_paused(&mut self, p: bool) {
        self.paused = p;
    }
    pub fn advance(&mut self, delta_us: u64) {
        if !self.paused {
            self.elapsed_us = self.elapsed_us.saturating_add(delta_us);
        }
    }
    pub fn advance_ns(&mut self, delta_ns: u64) {
        if !self.paused {
            let total = delta_ns.saturating_add(self.remainder_ns);
            self.remainder_ns = total % 1000;
            self.elapsed_us = self.elapsed_us.saturating_add(total / 1000);
        }
    }
    pub fn synchronize_audio(&mut self, position_us: u64) {
        if !self.paused {
            self.elapsed_us = position_us;
            self.remainder_ns = 0;
        }
    }
}

/// Enumerate original root/result and one-level showroom clips without following
/// symlinks outside the immutable imported video root.
pub fn catalog(data: &Path) -> Result<Vec<String>> {
    let root = data
        .join("VIDEO")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let mut names = vec![];
    for entry in std::fs::read_dir(&root).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let paths = if path.is_dir() {
            std::fs::read_dir(path)
                .map_err(|e| e.to_string())?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .collect()
        } else {
            vec![path]
        };
        for path in paths {
            if path
                .extension()
                .is_none_or(|e| !e.eq_ignore_ascii_case("avi"))
            {
                continue;
            }
            let relative = path.strip_prefix(&root).map_err(|e| e.to_string())?;
            resolve(&root, relative)?;
            let name = relative
                .with_extension("")
                .to_str()
                .ok_or("Non-UTF-8 video identity")?
                .replace('\\', "/");
            if !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_- /".contains(&b))
                || name.len() > 80
            {
                return Err("Unsupported video identity".into());
            }
            names.push(name);
            if names.len() > 128 {
                return Err("Video catalog limit".into());
            }
        }
    }
    names.sort();
    Ok(names)
}
