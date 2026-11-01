//! Authored multi-user filesystem acceptance. Run only with isolated XDG roots.
use rr_session::{
    career::Career,
    catalog::Catalog,
    persistence::{self, disk, paths::UserPaths, preferences::Preferences},
    text::Language,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .ok_or("new report path required")?;
    let paths = UserPaths::system()?;
    let cat = Catalog::authored();
    let requirements = persistence::Requirements::base(&cat);
    for slot in 0..5 {
        let path = paths.career(slot)?;
        if path.exists() {
            return Err("Use a clean test user profile".into());
        }
        let mut career = Career::new(&cat, slot + 1)?;
        career.alias = format!("User slot {slot}");
        let bytes = persistence::encode(&career, &cat, &requirements)?;
        disk::write(
            &path,
            None,
            &bytes,
            |b| persistence::decode(b, &cat, &requirements).map(|_| ()),
            disk::Fault::None,
        )?;
        assert_eq!(
            persistence::decode(&disk::read(&path)?, &cat, &requirements)?.career,
            career
        );
    }
    let mut prefs = Preferences::default();
    prefs.settings.language = Language::French;
    prefs.settings.text_scale = 125;
    prefs.settings.high_contrast = true;
    let settings = paths.settings();
    if settings.exists() {
        return Err("Use clean settings".into());
    }
    disk::write(
        &settings,
        None,
        &prefs.encode()?,
        |b| Preferences::decode(b).map(|_| ()),
        disk::Fault::None,
    )?;
    assert_eq!(
        Preferences::decode(&disk::read(&settings)?)?.0.settings,
        prefs.settings
    );
    let uid = std::fs::read_to_string("/proc/self/status")?
        .lines()
        .find(|l| l.starts_with("Uid:"))
        .ok_or("uid")?
        .to_owned();
    let report = serde_json::json!({"outcome":"passed","uid":uid,"slots":5,"settings_roundtrip":true,"data":paths.data,"config":paths.config});
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?
        .write_all(&serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
