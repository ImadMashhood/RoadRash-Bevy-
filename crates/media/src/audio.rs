use crate::Result;
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};
pub const MAX_INPUT: usize = 128 * 1024 * 1024;
pub const MAX_SAMPLES: usize = 64 * 1024 * 1024;
#[derive(Clone, Debug)]
pub struct Pcm {
    pub rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}
impl Pcm {
    pub fn validate(&self) -> Result<()> {
        if !(1..=2).contains(&self.channels)
            || !(8000..=192000).contains(&self.rate)
            || self.samples.is_empty()
            || self.samples.len() > MAX_SAMPLES
            || !self
                .samples
                .len()
                .is_multiple_of(usize::from(self.channels))
        {
            return Err("Invalid/oversized PCM layout".into());
        }
        Ok(())
    }
    pub fn duration_us(&self) -> u64 {
        self.samples.len() as u64 * 1_000_000 / u64::from(self.rate) / u64::from(self.channels)
    }
    pub fn wav(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let n = self.samples.len() * 2;
        let mut b = vec![];
        b.extend(b"RIFF");
        b.extend(((n + 36) as u32).to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(self.channels.to_le_bytes());
        b.extend(self.rate.to_le_bytes());
        b.extend((self.rate * u32::from(self.channels) * 2).to_le_bytes());
        b.extend((self.channels * 2).to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend((n as u32).to_le_bytes());
        for s in &self.samples {
            b.extend(s.to_le_bytes());
        }
        Ok(b)
    }
}
pub fn read_file(path: &Path) -> Result<Pcm> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if !f.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Audio input must be a regular file".into());
    }
    let mut bytes = vec![];
    f.take(MAX_INPUT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    read_wav(&bytes)
}
pub fn read_wav(b: &[u8]) -> Result<Pcm> {
    let range = |at: usize, n: usize| {
        b.get(at..at.checked_add(n).ok_or("WAV offset overflow")?)
            .ok_or("Truncated WAV".to_string())
    };
    let word = |at| -> Result<u16> {
        let v = range(at, 2)?;
        Ok(u16::from_le_bytes([v[0], v[1]]))
    };
    let dword = |at| -> Result<u32> {
        let v = range(at, 4)?;
        Ok(u32::from_le_bytes(v.try_into().unwrap()))
    };
    if b.len() > MAX_INPUT || range(0, 4)? != b"RIFF" || range(8, 4)? != b"WAVE" {
        return Err("Expected bounded RIFF/WAVE audio".into());
    }
    let end = 8usize
        .checked_add(dword(4)? as usize)
        .ok_or("RIFF overflow")?;
    if end > b.len() || end < 12 {
        return Err("Invalid RIFF extent".into());
    }
    let mut at = 12;
    let mut format = None;
    let mut data = None;
    while at < end {
        let header = range(at, 8)?;
        let size = dword(at + 4)? as usize;
        let payload = at + 8;
        let next = payload
            .checked_add(size + (size & 1))
            .ok_or("WAV chunk overflow")?;
        if next > end {
            return Err("WAV chunk exceeds RIFF extent".into());
        }
        if &header[..4] == b"fmt " {
            if format.is_some() || size < 16 {
                return Err("Duplicate/short WAV format".into());
            }
            format = Some(payload);
        }
        if &header[..4] == b"data" {
            if data.is_some() {
                return Err("Duplicate WAV data".into());
            }
            data = Some((payload, size));
        }
        at = next;
    }
    let fmt = format.ok_or("Missing WAV format")?;
    let (start, size) = data.ok_or("Missing WAV samples")?;
    let channels = word(fmt + 2)?;
    let rate = dword(fmt + 4)?;
    let bits = word(fmt + 14)?;
    if !(8000..=192000).contains(&rate)
        || size > MAX_SAMPLES * usize::from(bits.max(8) / 8)
        || word(fmt)? != 1
        || !matches!(bits, 8 | 16)
        || !(1..=2).contains(&channels)
        || word(fmt + 12)? != channels * (bits / 8)
        || dword(fmt + 8)? != rate * u32::from(channels) * u32::from(bits / 8)
    {
        return Err(
            "Unsupported or inconsistent PCM WAV format (use AVI codec adapter for IMA)".into(),
        );
    }
    let raw = range(start, size)?;
    let samples = if bits == 8 {
        raw.iter().map(|v| (i16::from(*v) - 128) * 256).collect()
    } else {
        if size % 2 != 0 {
            return Err("Partial 16-bit WAV sample".into());
        }
        raw.as_chunks::<2>()
            .0
            .iter()
            .map(|v| i16::from_le_bytes([v[0], v[1]]))
            .collect()
    };
    let pcm = Pcm {
        rate,
        channels,
        samples,
    };
    pcm.validate()?;
    Ok(pcm)
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rpm {
    pub current: i32,
    pub previous_speed: i32,
}
impl Rpm {
    /// Scalar primitive matching pinned bike.cpp; this state belongs to presentation.
    pub fn update(&mut self, ratio: i32, speed: i32, height: i32, crashed: bool) -> i32 {
        if self.previous_speed == speed && speed != 0 {
            return self.current;
        }
        let next = if height < 101 || speed == 0 {
            self.previous_speed = speed;
            let raw = ratio.wrapping_mul(speed);
            let target = (raw.wrapping_add((raw >> 31) & 255) >> 8).wrapping_add(8192);
            if !crashed || self.current < 8193 {
                target.clamp(
                    self.current.saturating_sub(8192),
                    self.current.saturating_add(8192),
                )
            } else {
                self.current.saturating_sub(4096)
            }
        } else {
            self.current.saturating_add(4096).min(65535)
        };
        self.current = next;
        next
    }
}
pub fn pitch_q8(frequency: i32, rate: u32) -> u16 {
    ((i64::from(frequency.clamp(100, 100000)) * 256 + i64::from(rate) / 2) / i64::from(rate.max(1)))
        .clamp(1, 4096) as u16
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bus {
    Engine,
    Effects,
    Music,
    Video,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Levels {
    pub master: u8,
    pub engine: u8,
    pub effects: u8,
    pub music: u8,
}
impl Default for Levels {
    fn default() -> Self {
        Self {
            master: 100,
            engine: 80,
            effects: 80,
            music: 80,
        }
    }
}
impl Levels {
    pub fn gain(self, bus: Bus) -> f32 {
        let sub = match bus {
            Bus::Engine => self.engine,
            Bus::Effects => self.effects,
            Bus::Music | Bus::Video => self.music,
        };
        f32::from(self.master.min(100)) * f32::from(sub.min(100)) / 10000.
    }
}
