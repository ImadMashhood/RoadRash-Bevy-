//! CLI-only output boundary; simulation ticks never use filesystem APIs.
use std::path::{Component, Path, PathBuf};
pub fn output(data: &Path, out: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let input = data.canonicalize()?;
    if out.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("output parent traversal".into());
    }
    let absolute = std::path::absolute(out)?;
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("output ancestor")?;
    }
    let suffix = absolute.strip_prefix(ancestor)?;
    let resolved = if suffix.as_os_str().is_empty() {
        ancestor.canonicalize()?
    } else {
        ancestor.canonicalize()?.join(suffix)
    };
    if resolved.starts_with(&input) || input.starts_with(&resolved) {
        return Err("output overlaps immutable source data".into());
    }
    Ok(resolved)
}
