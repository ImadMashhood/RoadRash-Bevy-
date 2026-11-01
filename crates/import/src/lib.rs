//! Local-only, validated installation with immutable generations and atomic pointer activation.
mod iso;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, ImportError>;
#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
    Validation(Vec<Problem>),
    Cancelled,
    Busy,
}
impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "filesystem error: {e}"),
            Self::Json(e) => write!(f, "manifest error: {e}"),
            Self::Invalid(e) => write!(f, "{e}"),
            Self::Validation(p) => write!(
                f,
                "{} files failed validation; use the supported 1997 disc revision and inspect diagnostics",
                p.len()
            ),
            Self::Cancelled => write!(f, "import cancelled; previous active install preserved"),
            Self::Busy => write!(f, "another import holds the installation lock"),
        }
    }
}
impl std::error::Error for ImportError {}
impl From<std::io::Error> for ImportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for ImportError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequiredFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub files: Vec<RequiredFile>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Problem {
    pub path: String,
    pub kind: String,
    pub detail: String,
}
#[derive(Clone, Debug)]
pub struct Limits {
    pub max_source_bytes: u64,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_directory_bytes: u64,
    pub max_entries: usize,
    pub max_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_source_bytes: 2 * 1024 * 1024 * 1024,
            max_file_bytes: 512 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
            max_directory_bytes: 16 * 1024 * 1024,
            max_entries: 10000,
            max_depth: 32,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub schema_version: u32,
    pub importer_version: String,
    #[serde(default = "legacy_build")]
    pub importer_build_sha256: String,
    pub recipe_version: u32,
    pub manifest_sha256: String,
    pub upstream_revision: String,
    pub source: String,
    pub source_kind: String,
    pub files: Vec<RequiredFile>,
}
fn legacy_build() -> String {
    "legacy-unrecorded".into()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Active {
    pub schema_version: u32,
    pub generation: String,
    pub receipt_sha256: String,
}
#[derive(Debug)]
pub struct Installation {
    pub generation: String,
    pub data_root: PathBuf,
    pub files: usize,
}

pub fn supported_manifest() -> Result<Manifest> {
    Ok(serde_json::from_str(include_str!(
        "../../../docs/baseline/required-files.json"
    ))?)
}
pub(crate) fn check_cancel(cancelled: &impl Fn() -> bool) -> Result<()> {
    if cancelled() {
        Err(ImportError::Cancelled)
    } else {
        Ok(())
    }
}
pub(crate) fn validate_component(s: &str) -> Result<()> {
    if s.is_empty()
        || s == "."
        || s == ".."
        || s.chars()
            .any(|c| c == '/' || c == '\\' || c == ':' || c.is_control())
    {
        Err(ImportError::Invalid(format!(
            "unsafe path component: {s:?}"
        )))
    } else {
        Ok(())
    }
}
fn validate_manifest(m: &Manifest, limits: &Limits) -> Result<()> {
    if m.schema_version != 1 || m.files.is_empty() || m.files.len() > limits.max_entries {
        return Err(ImportError::Invalid("unsupported or empty manifest".into()));
    }
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    for f in &m.files {
        if f.path.starts_with('/') {
            return Err(ImportError::Invalid("absolute manifest path".into()));
        }
        for c in f.path.split('/') {
            validate_component(c)?;
        }
        if !seen.insert(f.path.to_uppercase())
            || f.sha256.len() != 64
            || !f.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(ImportError::Invalid(
                "duplicate path or invalid hash in manifest".into(),
            ));
        }
        total = total
            .checked_add(f.size)
            .ok_or_else(|| ImportError::Invalid("manifest total overflow".into()))?;
        if f.size > limits.max_file_bytes || total > limits.max_total_bytes {
            return Err(ImportError::Invalid("manifest size limit exceeded".into()));
        }
    }
    Ok(())
}
fn no_symlinks(path: &Path) -> Result<()> {
    let absolute = std::path::absolute(path)?;
    let mut current = PathBuf::new();
    for part in absolute.components() {
        if matches!(part, Component::ParentDir) {
            return Err(ImportError::Invalid(
                "parent traversal not supported".into(),
            ));
        }
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(ImportError::Invalid(format!(
                    "symlink path rejected: {}",
                    current.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
pub(crate) fn open_regular(path: &Path) -> Result<File> {
    no_symlinks(path)?;
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(ImportError::Invalid(format!(
            "not a regular file: {}",
            path.display()
        )));
    }
    Ok(File::open(path)?)
}
pub(crate) fn copy_hash(
    input: &mut impl Read,
    output: &mut File,
    length: u64,
    cancelled: &impl Fn() -> bool,
) -> Result<String> {
    let mut remaining = length;
    let mut buffer = [0; 65536];
    let mut digest = Sha256::new();
    while remaining > 0 {
        check_cancel(cancelled)?;
        let wanted = remaining.min(buffer.len() as u64) as usize;
        let n = input.read(&mut buffer[..wanted])?;
        if n == 0 {
            return Err(ImportError::Invalid("source truncated during copy".into()));
        }
        output.write_all(&buffer[..n])?;
        digest.update(&buffer[..n]);
        remaining -= n as u64;
    }
    let mut extra = [0];
    if input.read(&mut extra)? != 0 {
        return Err(ImportError::Invalid("source grew during copy".into()));
    }
    output.sync_all()?;
    Ok(format!("{:x}", digest.finalize()))
}
fn scan_dir(
    root: &Path,
    limits: &Limits,
    cancelled: &impl Fn() -> bool,
) -> Result<BTreeMap<String, PathBuf>> {
    fn walk(
        root: &Path,
        relative: &str,
        depth: usize,
        out: &mut BTreeMap<String, PathBuf>,
        count: &mut usize,
        limits: &Limits,
        cancelled: &impl Fn() -> bool,
    ) -> Result<()> {
        check_cancel(cancelled)?;
        if depth > limits.max_depth {
            return Err(ImportError::Invalid(
                "source directory depth limit exceeded".into(),
            ));
        }
        let mut names = BTreeSet::new();
        for item in fs::read_dir(root)? {
            check_cancel(cancelled)?;
            let item = item?;
            *count += 1;
            if *count > limits.max_entries {
                return Err(ImportError::Invalid("source entry limit exceeded".into()));
            }
            let name = item
                .file_name()
                .into_string()
                .map_err(|_| ImportError::Invalid("non-Unicode source filename".into()))?;
            validate_component(&name)?;
            if !names.insert(name.to_uppercase()) {
                return Err(ImportError::Invalid(format!(
                    "ambiguous case-colliding name: {name}"
                )));
            }
            let ty = item.file_type()?;
            if ty.is_symlink() {
                return Err(ImportError::Invalid(format!(
                    "source symlink rejected: {}",
                    item.path().display()
                )));
            }
            let key = if relative.is_empty() {
                name.to_uppercase()
            } else {
                format!("{relative}/{}", name.to_uppercase())
            };
            if ty.is_dir() {
                walk(&item.path(), &key, depth + 1, out, count, limits, cancelled)?;
            } else if ty.is_file() {
                if item.metadata()?.len() > limits.max_file_bytes {
                    return Err(ImportError::Invalid(format!(
                        "source file exceeds limit: {key}"
                    )));
                }
                out.insert(key, item.path());
            } else {
                return Err(ImportError::Invalid(format!(
                    "special source file rejected: {key}"
                )));
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(root, "", 0, &mut out, &mut 0, limits, cancelled)?;
    Ok(out)
}
fn json_write(path: &Path, value: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(bytes)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn sync_dir(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn sync_tree(root: &Path) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            sync_tree(&entry.path())?;
        }
    }
    sync_dir(root)
}

/// Destination is a user-owned private directory. Imports serialize on an OS lock.
/// Failures before active.json replacement leave the existing generation untouched.
/// Interrupted staging directories are cleaned by the next locked import.
pub fn import(
    source: &Path,
    destination: &Path,
    manifest: &Manifest,
    limits: &Limits,
    cancelled: impl Fn() -> bool,
) -> Result<Installation> {
    validate_manifest(manifest, limits)?;
    check_cancel(&cancelled)?;
    no_symlinks(source)?;
    no_symlinks(destination)?;
    let source = source.canonicalize()?;
    fs::create_dir_all(destination)?;
    let destination = destination.canonicalize()?;
    if destination.starts_with(&source) || source.starts_with(&destination) {
        return Err(ImportError::Invalid(
            "source and destination must not contain one another".into(),
        ));
    }
    let lockpath = destination.join("import.lock");
    no_symlinks(&lockpath)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lockpath)?;
    lock.try_lock_exclusive().map_err(|e| {
        if e.kind() == std::io::ErrorKind::WouldBlock {
            ImportError::Busy
        } else {
            ImportError::Io(e)
        }
    })?;
    for entry in fs::read_dir(&destination)? {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(".rr-active-")
        {
            if !entry.file_type()?.is_file() {
                return Err(ImportError::Invalid(
                    "invalid abandoned activation file".into(),
                ));
            }
            fs::remove_file(entry.path())?;
        }
    }
    let generations = destination.join("installs");
    no_symlinks(&generations)?;
    fs::create_dir_all(&generations)?;
    // Only recognized owned staging directories are eligible for recovery cleanup.
    for entry in fs::read_dir(&generations)? {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(".rr-staging-")
        {
            if entry.file_type()?.is_symlink() {
                return Err(ImportError::Invalid("staging symlink rejected".into()));
            }
            if entry.file_type()?.is_dir() {
                fs::remove_dir_all(entry.path())?;
            }
        }
    }
    let stage = tempfile::Builder::new()
        .prefix(".rr-staging-")
        .tempdir_in(&generations)?;
    let data = stage.path().join("ROADRASH");
    fs::create_dir(&data)?;
    let mut image = if source.is_file() {
        Some(iso::Iso::open(&source, limits, &cancelled)?)
    } else {
        None
    };
    let folder = if image.is_none() {
        Some(scan_dir(&source, limits, &cancelled)?)
    } else {
        None
    };
    let mut problems = vec![];
    for required in &manifest.files {
        check_cancel(&cancelled)?;
        let output = data.join(&required.path);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let key = required.path.to_uppercase();
        let actual_length = if let Some(iso) = &image {
            iso.entries
                .get(&format!("ROADRASH/{key}"))
                .map(|e| e.length)
        } else {
            match folder.as_ref().unwrap().get(&key) {
                None => None,
                Some(path) => match fs::metadata(path) {
                    Ok(metadata) => Some(metadata.len()),
                    Err(error) => {
                        problems.push(Problem {
                            path: required.path.clone(),
                            kind: "unreadable".into(),
                            detail: error.to_string(),
                        });
                        continue;
                    }
                },
            }
        };
        let Some(length) = actual_length else {
            problems.push(Problem {
                path: required.path.clone(),
                kind: "missing".into(),
                detail: "required file not found".into(),
            });
            continue;
        };
        if length != required.size {
            problems.push(Problem {
                path: required.path.clone(),
                kind: "wrong-size".into(),
                detail: format!("expected {}, found {length}", required.size),
            });
            continue;
        }
        let result = (|| -> Result<String> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)?;
            if let Some(iso) = &mut image {
                let entry = iso.entries[&format!("ROADRASH/{key}")].clone();
                iso.copy_file(&entry, &mut file, length, &cancelled)
            } else {
                let mut input = open_regular(&folder.as_ref().unwrap()[&key])?;
                copy_hash(&mut input, &mut file, length, &cancelled)
            }
        })();
        match result {
            Ok(digest) if digest.eq_ignore_ascii_case(&required.sha256) => {}
            Ok(digest) => problems.push(Problem {
                path: required.path.clone(),
                kind: "wrong-hash".into(),
                detail: format!(
                    "expected {}, found {digest}; unsupported or modified revision",
                    required.sha256
                ),
            }),
            Err(ImportError::Cancelled) => return Err(ImportError::Cancelled),
            Err(e) => problems.push(Problem {
                path: required.path.clone(),
                kind: "unreadable".into(),
                detail: e.to_string(),
            }),
        }
    }
    if !problems.is_empty() {
        return Err(ImportError::Validation(problems));
    }
    check_cancel(&cancelled)?;
    let receipt = Receipt {
        schema_version: 1,
        importer_version: env!("CARGO_PKG_VERSION").into(),
        importer_build_sha256: env!("RR_IMPORT_BUILD_SHA256").into(),
        recipe_version: 1,
        manifest_sha256: hash(&serde_json::to_vec(manifest)?),
        upstream_revision: manifest.upstream_revision.clone(),
        source: source.to_string_lossy().into_owned(),
        source_kind: if image.is_some() {
            "iso9660"
        } else {
            "directory"
        }
        .into(),
        files: manifest.files.clone(),
    };
    let receipt_bytes = json_write(&stage.path().join("receipt.json"), &receipt)?;
    sync_tree(&data)?;
    sync_dir(stage.path())?;
    check_cancel(&cancelled)?;
    let generation = format!(
        "install-{}",
        stage
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(".rr-staging-")
    );
    let final_path = generations.join(&generation);
    fs::rename(stage.path(), &final_path)?;
    sync_dir(&generations)?;
    // A cancelled or crashed activation can leave an immutable unreferenced install;
    // it is safe and deliberately not deleted because another consumer could hold it.
    check_cancel(&cancelled)?;
    let active = Active {
        schema_version: 1,
        generation: generation.clone(),
        receipt_sha256: hash(&receipt_bytes),
    };
    let mut pointer = tempfile::Builder::new()
        .prefix(".rr-active-")
        .tempfile_in(&destination)?;
    pointer.write_all(&serde_json::to_vec_pretty(&active)?)?;
    pointer.as_file().sync_all()?;
    let active_path = destination.join("active.json");
    no_symlinks(&active_path)?;
    pointer
        .persist(&active_path)
        .map_err(|e| ImportError::Io(e.error))?;
    sync_dir(&destination)?;
    Ok(Installation {
        generation,
        data_root: final_path.join("ROADRASH"),
        files: manifest.files.len(),
    })
}

/// Resolve an immutable active generation and verify its receipt before use.
pub fn active_install(destination: &Path) -> Result<Installation> {
    no_symlinks(destination)?;
    let active_path = destination.join("active.json");
    let active_file = open_regular(&active_path)?;
    if active_file.metadata()?.len() > 4096 {
        return Err(ImportError::Invalid(
            "oversized active installation record".into(),
        ));
    }
    let active: Active = serde_json::from_reader(active_file)?;
    if active.schema_version != 1 || !active.generation.starts_with("install-") {
        return Err(ImportError::Invalid("invalid active install record".into()));
    }
    validate_component(&active.generation)?;
    let root = destination.join("installs").join(&active.generation);
    let mut bytes = vec![];
    let receipt_file = open_regular(&root.join("receipt.json"))?;
    if receipt_file.metadata()?.len() > 1024 * 1024 {
        return Err(ImportError::Invalid(
            "oversized installation receipt".into(),
        ));
    }
    receipt_file.take(1024 * 1024).read_to_end(&mut bytes)?;
    if hash(&bytes) != active.receipt_sha256 {
        return Err(ImportError::Invalid("active receipt hash mismatch".into()));
    }
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    if receipt.schema_version != 1 {
        return Err(ImportError::Invalid("unsupported receipt schema".into()));
    }
    let data_root = root.join("ROADRASH");
    no_symlinks(&data_root)?;
    if !data_root.is_dir() {
        return Err(ImportError::Invalid("active data directory missing".into()));
    }
    Ok(Installation {
        generation: active.generation,
        data_root,
        files: receipt.files.len(),
    })
}
