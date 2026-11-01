//! One previous-valid backup, unique temporary writes, serialized writers and directory durability.
use super::{MAX_BYTES, Result};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
fn unique(path: &Path, tag: &str) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(format!(
        ".{tag}.{}.{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    p.into()
}
pub fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(suffix);
    p.into()
}
pub fn guarded(path: &Path, source: Option<&Path>) -> Result<PathBuf> {
    if path.extension().is_none_or(|e| e != "json")
        || path.components().any(|c| matches!(c, Component::ParentDir))
    {
        return Err("Save/settings path requires .json without parent traversal".into());
    }
    let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("save path ancestor")?;
    }
    let canonical = ancestor.canonicalize().map_err(|e| e.to_string())?;
    let suffix = absolute.strip_prefix(ancestor).map_err(|e| e.to_string())?;
    let resolved = if suffix.as_os_str().is_empty() {
        canonical
    } else {
        canonical.join(suffix)
    };
    if let Some(source) = source {
        let source = source.canonicalize().map_err(|e| e.to_string())?;
        if resolved.starts_with(&source) || source.starts_with(&resolved) {
            return Err("Save/settings path overlaps immutable source".into());
        }
    }
    Ok(resolved)
}
pub fn read(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Save/settings input must be a regular file".into());
    }
    let mut bytes = vec![];
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("Save/settings input exceeds 64 KiB".into());
    }
    Ok(bytes)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    None,
    AfterCreate,
    AfterWrite,
    AfterFileSync,
    AfterBackup,
    AfterReplace,
    AfterDirectorySync,
}
fn point(fault: Fault, at: Fault) -> Result<()> {
    if fault == at {
        Err(format!("Injected save failure at {at:?}"))
    } else {
        Ok(())
    }
}
struct Lease(PathBuf);
impl Drop for Lease {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|f| f.sync_all())
            .map_err(|e| format!("Directory sync: {e}"))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
fn replace(bytes: &[u8], path: &Path) -> Result<()> {
    let tmp = Temporary(unique(path, "tmp"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp.0)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(&tmp.0, path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn write(
    path: &Path,
    source: Option<&Path>,
    bytes: &[u8],
    validate: impl Fn(&[u8]) -> Result<()>,
    fault: Fault,
) -> Result<()> {
    if bytes.len() > MAX_BYTES {
        return Err("Save/settings output exceeds 64 KiB".into());
    }
    validate(bytes)?;
    let path = guarded(path, source)?;
    let parent = path.parent().ok_or("save parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let lock = sidecar(&path, ".lock");
    let mut lease=OpenOptions::new().write(true).create_new(true).open(&lock).map_err(|e|format!("Save writer unavailable ({}): {e}. If no game is running, see stale-lock recovery in SAVES.md.",lock.display()))?;
    let _lease = Lease(lock);
    writeln!(lease, "{}", std::process::id()).map_err(|e| e.to_string())?;
    let previous = if path.exists() {
        let old = read(&path)?;
        validate(&old).map_err(|e|format!("Current file is invalid and was preserved: {e}. Restore its backup or choose a new slot."))?;
        Some(old)
    } else {
        None
    };
    let tmp = Temporary(unique(&path, "tmp"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp.0)
        .map_err(|e| e.to_string())?;
    point(fault, Fault::AfterCreate)?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    point(fault, Fault::AfterWrite)?;
    file.sync_all().map_err(|e| e.to_string())?;
    point(fault, Fault::AfterFileSync)?;
    drop(file);
    if let Some(old) = previous {
        replace(&old, &sidecar(&path, ".bak"))?;
        sync_dir(parent)?;
    }
    point(fault, Fault::AfterBackup)?;
    std::fs::rename(&tmp.0, &path).map_err(|e| e.to_string())?;
    point(fault, Fault::AfterReplace)?;
    sync_dir(parent)?;
    point(fault, Fault::AfterDirectorySync)?;
    Ok(())
}
/// Explicit recovery keeps the damaged primary as a separate diagnostic file and retains .bak.
pub fn restore(
    path: &Path,
    source: Option<&Path>,
    validate: impl Fn(&[u8]) -> Result<()>,
) -> Result<PathBuf> {
    let path = guarded(path, source)?;
    let backup = read(&sidecar(&path, ".bak"))?;
    validate(&backup)?;
    let parent = path.parent().ok_or("restore parent")?;
    let lock = sidecar(&path, ".lock");
    let _file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| e.to_string())?;
    let _lease = Lease(lock);
    let preserved = unique(&path, "corrupt");
    if path.exists() {
        std::fs::rename(&path, &preserved).map_err(|e| e.to_string())?;
    }
    if let Err(e) = replace(&backup, &path) {
        if preserved.exists() {
            let _ = std::fs::rename(&preserved, &path);
        }
        return Err(e);
    }
    sync_dir(parent)?;
    Ok(preserved)
}
