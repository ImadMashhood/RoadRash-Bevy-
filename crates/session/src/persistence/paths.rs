use crate::Result;
use std::path::{Path, PathBuf};
pub fn slot_path(base: &Path, slot: u8) -> Result<PathBuf> {
    if slot > 4 {
        return Err("Career slot must be 0..4".into());
    }
    if slot == 0 {
        return Ok(base.to_path_buf());
    }
    let mut name = base.file_stem().ok_or("Career slot basename")?.to_owned();
    name.push(format!("-slot-{slot}.json"));
    Ok(base.with_file_name(name))
}
#[derive(Clone, Debug)]
pub struct UserPaths {
    pub data: PathBuf,
    pub config: PathBuf,
}
impl UserPaths {
    pub fn profile(root: &Path) -> Self {
        Self {
            data: root.join("data"),
            config: root.join("config"),
        }
    }
    pub fn system() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            let base =
                std::env::var_os("APPDATA").ok_or("APPDATA unavailable; use --profile-root")?;
            let root = PathBuf::from(base).join("RoadRash-Rust");
            return Ok(Self {
                data: root.clone(),
                config: root,
            });
        }
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var_os("HOME").ok_or("HOME unavailable; use --profile-root")?;
            let root = PathBuf::from(home).join("Library/Application Support/RoadRash-Rust");
            return Ok(Self {
                data: root.clone(),
                config: root,
            });
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or("HOME unavailable; use --profile-root")?;
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(".local/share"));
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(".config"));
            Ok(Self {
                data: data.join("roadrash-rust"),
                config: config.join("roadrash-rust"),
            })
        }
    }
    pub fn career(&self, slot: u8) -> Result<PathBuf> {
        slot_path(&self.data.join("careers/default.json"), slot)
    }
    pub fn settings(&self) -> PathBuf {
        self.config.join("settings.json")
    }
}
