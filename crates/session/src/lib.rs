//! Session authority and frontend domain. No Bevy entities, file handles or renderer references.
#![forbid(unsafe_code)]
pub mod career;
pub mod catalog;
pub mod flow;
pub mod race;
pub type Result<T> = std::result::Result<T, String>;
pub mod playback;
