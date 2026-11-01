//! Bounded JSON and complete-file replacement shared by input settings.
use crate::{Error, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(65537).read_to_end(&mut bytes))
        .map_err(|e| Error(format!("{}: {e}", path.display())))?;
    if bytes.len() > 65536 {
        return Err(Error("input settings exceed 64 KiB".into()));
    }
    serde_json::from_slice(&bytes).map_err(|e| Error(e.to_string()))
}
pub fn save<T: Serialize>(value: &T, path: &Path) -> Result<()> {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| Error(e.to_string()))?;
    if bytes.len() > 65535 {
        return Err(Error("input settings exceed 64 KiB".into()));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| Error("input settings need a file name".into()))?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| Error(format!("{}: {e}", path.display())))?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| Error(format!("{}: {e}", path.display())))
}
