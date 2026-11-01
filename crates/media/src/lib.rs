//! Bounded source audio, local codec processes and presentation-only media timing.
#![forbid(unsafe_code)]
pub mod audio;
pub mod cues;
pub mod video;
pub type Result<T> = std::result::Result<T, String>;
