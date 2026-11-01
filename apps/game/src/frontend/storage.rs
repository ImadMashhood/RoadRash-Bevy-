use super::*;
pub(super) fn save(world: &World, shell: &Shell) -> Result<(), String> {
    let c = shell
        .frontend
        .career
        .as_ref()
        .ok_or("No Big Game career to save.")?;
    let bytes = c.encode()?;
    let options = world.resource::<LaunchOptions>();
    let p = std::path::absolute(&options.career_file).map_err(|e| e.to_string())?;
    if p.extension().is_none_or(|e| e != "json")
        || options
            .career_file
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("career output requires .json without parent traversal".into());
    }
    let mut ancestor = p.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("career output ancestor")?;
    }
    let resolved = ancestor
        .canonicalize()
        .map_err(|e| e.to_string())?
        .join(p.strip_prefix(ancestor).map_err(|e| e.to_string())?);
    if let Some(data) = &options.data_root {
        let data = data.canonicalize().map_err(|e| e.to_string())?;
        if resolved.starts_with(&data) || data.starts_with(&resolved) {
            return Err("career path overlaps immutable source".into());
        }
    }
    let p = resolved;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Phase 16 snapshot bridge. Phase 17 extends disk migration/recovery/backup contracts.
    let tmp = p.with_extension("json.new");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}
pub(super) fn flush_recording(world: &mut World) {
    let snapshot = world.remove_resource::<Race>();
    if let Some(race) = snapshot {
        if let Some(mut recorder) = world.get_resource_mut::<crate::race_recording::Recorder>() {
            recorder.finish(&race.0);
        }
        world.insert_resource(race);
    }
}
