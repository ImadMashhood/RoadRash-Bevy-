#![forbid(unsafe_code)]
//! Deterministic classic software drawing, presented by Bevy as an RGBA surface.
//! All course/asset loading is outside frame execution; simulation is read-only.
pub mod camera;
pub mod fixture;
pub mod raster;
pub mod scene;
pub mod source;
pub mod suite;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub use camera::{CameraSpec, Framing, View};
pub use scene::{Scene, SceneOptions};

pub mod rider;
