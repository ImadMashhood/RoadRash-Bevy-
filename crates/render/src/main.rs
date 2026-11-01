use rr_render::{camera::*, scene::SceneOptions, source::SourceAssets, *};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn load(install: &Path, catalog: &Path, stem: &str, level: u8, mask: u32) -> Result<Scene> {
    let active = rr_import::active_install(install)?;
    let course = rr_course::Course::read(&fs::read(
        active.data_root.join(format!("DATA/COURSES/{stem}.CRS")),
    )?)?;
    let assets = SourceAssets::load(&active.data_root, catalog, stem, &course)?;
    Scene::new(
        course,
        assets,
        SceneOptions {
            course: stem.into(),
            level,
            branch_mask: mask,
            seed: 1,
        },
    )
}
fn frame(a: &[String]) -> Result<()> {
    let stem = &a[3];
    if !rr_course::export::COURSES.iter().any(|c| c.0 == stem) {
        return Err("course must be CITY/HIWAY/MEDLY/NAPA/CANYN".into());
    }
    let sample: f64 = a[4].parse()?;
    let mask = a.get(6).map(|v| v.parse()).transpose()?.unwrap_or(0);
    let level = a.get(7).map(|v| v.parse()).transpose()?.unwrap_or(1);
    let framing = match a.get(8).map(String::as_str).unwrap_or("classic") {
        "classic" => Framing::Classic,
        "wide" => Framing::Wide,
        _ => return Err("framing must be classic|wide".into()),
    };
    let spec = CameraSpec::new(framing, false);
    let scene = load(Path::new(&a[1]), Path::new(&a[2]), stem, level, mask)?;
    let (frame, receipt) = scene.draw(
        &spec,
        &View {
            position_q8: sample * 256.,
            ..View::default()
        },
        &rr_simulation::Simulation::default().snapshot(),
        false,
    )?;
    let out = suite::output_path(Path::new(&a[1]), &PathBuf::from(&a[5]))?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, frame.image.png()?)?;
    fs::write(
        out.with_extension("json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"render":receipt,"notes":scene.notes,"source_sha256":scene.course.source_sha256}),
        )?,
    )?;
    println!("{}", out.display());
    Ok(())
}
fn read_png(path: &str) -> Result<rr_formats::pixels::Rgba> {
    if fs::metadata(path)?.len() > 64 * 1024 * 1024 {
        return Err("diff input file bound".into());
    }
    Ok(rr_formats::pixels::Rgba::from_png(&fs::read(path)?)?)
}
fn diff(a: &[String]) -> Result<()> {
    let out = std::path::absolute(&a[3])?;
    if out == std::path::absolute(&a[1])? || out == std::path::absolute(&a[2])? {
        return Err("diff output must not overwrite inputs".into());
    }
    let (count, diff) = suite::image_diff(&read_png(&a[1])?, &read_png(&a[2])?)?;
    fs::write(out, diff.png()?)?;
    println!("{count} differing pixels; mask=none");
    if count != 0 {
        return Err("image regression mismatch (diff retained)".into());
    }
    Ok(())
}
fn authored(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let mut hashes = std::collections::BTreeMap::new();
    for name in fixture::CASES {
        let scene = fixture::scene(name)?;
        let (frame, _) = scene.draw(
            &CameraSpec::default(),
            &View {
                position_q8: 50. * 256.,
                ..Default::default()
            },
            &rr_simulation::Simulation::default().snapshot(),
            false,
        )?;
        fs::write(out.join(format!("{name}.png")), frame.image.png()?)?;
        hashes.insert(name, suite::hash(&frame.image.pixels));
    }
    fs::write(
        out.join("goldens.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"schema_version":1,"fixture":"authored-render-fixture-v1","rgba_sha256":hashes}),
        )?,
    )?;
    Ok(())
}
fn run() -> Result<()> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    match a.first().map(String::as_str){
        Some("frame")if(6..=9).contains(&a.len())=>frame(&a),
        Some("suite")if a.len()==4=>suite::run(Path::new(&a[1]),Path::new(&a[2]),Path::new(&a[3])),
        Some("diff")if a.len()==4=>diff(&a),
        Some("authored")if a.len()==2=>authored(Path::new(&a[1])),
        _=>Err("usage: rr-render frame <install> <catalog> <COURSE> <sample> <out.png> [mask] [level] [classic|wide] | suite <install> <catalog> <out> | diff <expected.png> <actual.png> <diff.png> | authored <output>".into()),
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
