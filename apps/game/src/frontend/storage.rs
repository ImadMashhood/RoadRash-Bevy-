use super::*;
pub(super) fn save(world: &World, shell: &Shell) -> Result<(), String> {
    let c = shell
        .frontend
        .career
        .as_ref()
        .ok_or("No Big Game career to save.")?;
    let options = world.resource::<LaunchOptions>();
    let requirements = rr_session::persistence::Requirements::base(&shell.catalog);
    let bytes = rr_session::persistence::encode(c, &shell.catalog, &requirements)?;
    rr_session::persistence::disk::write(
        &options.career_file,
        options.data_root.as_deref(),
        &bytes,
        |b| rr_session::persistence::decode(b, &shell.catalog, &requirements).map(|_| ()),
        rr_session::persistence::disk::Fault::None,
    )
}
pub(super) fn load(
    path: &std::path::Path,
    catalog: &Catalog,
) -> Result<rr_session::persistence::Loaded, String> {
    let requirements = rr_session::persistence::Requirements::base(catalog);
    let bytes = rr_session::persistence::disk::read(path)?;
    rr_session::persistence::decode(&bytes, catalog, &requirements)
}
pub(super) fn restore(world: &World, shell: &Shell) -> Result<(), String> {
    let options = world.resource::<LaunchOptions>();
    let required = rr_session::persistence::Requirements::base(&shell.catalog);
    rr_session::persistence::disk::restore(
        &options.career_file,
        options.data_root.as_deref(),
        |b| rr_session::persistence::decode(b, &shell.catalog, &required).map(|_| ()),
    )
    .map(|_| ())
}
#[derive(Resource, Default)]
pub(super) struct PreferenceState {
    last: Option<Vec<u8>>,
}
pub(super) fn preferences(world: &mut World) {
    let options = world.resource::<LaunchOptions>();
    if options.settings_file.as_os_str().is_empty() {
        return;
    }
    let shell = world.resource::<Shell>();
    let mut settings = shell.frontend.settings.clone();
    settings.track = None;
    let p = rr_session::persistence::preferences::Preferences {
        schema_version: 1,
        settings,
        bindings: world.resource::<crate::Bindings>().0.clone(),
        active_slot: shell.active_slot,
    };
    let Ok(bytes) = p.encode() else {
        return;
    };
    let mut state = world.resource_mut::<PreferenceState>();
    if state.last.is_none() {
        state.last = Some(bytes);
        return;
    }
    if state.last.as_ref() == Some(&bytes) {
        return;
    }
    state.last = Some(bytes.clone());
    let options = world.resource::<LaunchOptions>();
    let result = rr_session::persistence::disk::write(
        &options.settings_file,
        options.data_root.as_deref(),
        &bytes,
        |b| rr_session::persistence::preferences::Preferences::decode(b).map(|_| ()),
        rr_session::persistence::disk::Fault::None,
    );
    if let Err(e) = result {
        world.resource_mut::<Shell>().notice = format!("Preferences were not saved: {e}");
    }
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
