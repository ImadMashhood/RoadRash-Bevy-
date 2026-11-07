//! Composition root: ordered input/fixed simulation/snapshot adapter and optional deterministic playback.
mod acceptance;
mod frontend;
mod gamepad_calibration;
mod input_adapter;
mod media;
mod performance;
mod population_cues;
mod race_recording;
#[cfg(test)]
mod source_combat_art;
#[cfg(test)]
mod source_recovery_art;
mod source_roads;
mod source_scenery;
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use input_adapter::{Bindings, LiveInput, Router, poll_input, tick_command};
use rr_input::{Action, Context};
use rr_presentation::{
    ClassicPresentationPlugin, CoursePresentation, PresentationStage, PresentedSnapshot,
    VisualAssets,
};
use rr_simulation::{
    Command, Simulation, TICKS_PER_SECOND,
    replay::{Playback, Replay, Report, frame_ns},
};
use std::{num::NonZeroU8, path::PathBuf};
#[derive(Resource, Default)]
struct Race(Simulation);
#[derive(Resource, Default, Clone)]
struct LaunchOptions {
    frontend: Option<bool>,
    career_file: PathBuf,
    save_base: PathBuf,
    settings_file: PathBuf,
    explicit: std::collections::BTreeSet<String>,
    ui_script: Option<PathBuf>,
    condition_percent: u16,
    smoke_frames: Option<u32>,
    capture: Option<PathBuf>,
    capture_frame: u32,
    replay: Option<Replay>,
    render_rate: String,
    report: Option<PathBuf>,
    catalog: Option<PathBuf>,
    asset: Option<String>,
    override_image: Option<PathBuf>,
    enhanced: bool,
    switch_tick: Option<u64>,
    expected_hash: Option<String>,
    data_root: Option<PathBuf>,
    course: Option<String>,
    level: u8,
    branch_mask: u32,
    framing: rr_render::Framing,
    integer_scale: bool,
    camera_sample: Option<f64>,
    window_size: Option<(u32, u32)>,
    bindings: Option<PathBuf>,
    calibration: Option<PathBuf>,
    bike: u32,
    rider_color: u8,
    practice_riders: u8,
    opponents: u8,
    record_replay: Option<PathBuf>,
    fast_recovery: bool,
    rescue_recovery: bool,
    effects: bool,
    audio: bool,
    video: bool,
    media_report: Option<PathBuf>,
    functional_check: Option<PathBuf>,
    performance_report: Option<PathBuf>,
    benchmark_seconds: Option<u32>,
    benchmark_pilot: bool,
    benchmark_tour: bool,
    vsync: bool,
}
#[derive(Resource, Default)]
struct Driver(Option<Playback>);
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum FixedStage {
    Input,
    Simulation,
    Snapshot,
}
#[derive(Resource, Default)]
struct InputCommand(Command);
#[derive(Resource, Default)]
struct PauseState {
    paused: bool,
    release_required: bool,
}
#[derive(Resource, Default)]
struct Previous(rr_simulation::Snapshot);
fn options() -> Result<LaunchOptions, String> {
    let paths = rr_session::persistence::paths::UserPaths::system().unwrap_or_else(|_| {
        rr_session::persistence::paths::UserPaths::profile(std::path::Path::new("local/user"))
    });
    let mut options = LaunchOptions {
        condition_percent: 100,
        career_file: paths.career(0)?,
        save_base: paths.career(0)?,
        settings_file: paths.settings(),
        render_rate: "60".into(),
        level: 1,
        bike: 1,
        opponents: 14,
        effects: true,
        audio: true,
        video: true,
        capture_frame: 30,
        ..Default::default()
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {}", arg.to_string_lossy()))?;
        if let Some(name) = arg.to_str() {
            options.explicit.insert(name.into());
        }
        match arg.to_str() {
            Some("--ui-script") => options.ui_script = Some(value.into()),
            Some("--profile-root") => {
                let paths =
                    rr_session::persistence::paths::UserPaths::profile(&PathBuf::from(value));
                options.career_file = paths.career(0)?;
                options.save_base = paths.career(0)?;
                options.settings_file = paths.settings();
            }
            Some("--settings-file") => options.settings_file = value.into(),
            Some("--career-file") => {
                options.career_file = value.into();
                options.save_base = options.career_file.clone();
            }
            Some("--vsync") => {
                options.vsync = match value.to_str() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("vsync must be on/off".into()),
                }
            }
            Some("--performance-report") => options.performance_report = Some(value.into()),
            Some("--benchmark-seconds") => {
                let seconds = value
                    .to_string_lossy()
                    .parse::<u32>()
                    .map_err(|_| "benchmark-seconds must be 1..7200")?;
                if !(1..=7200).contains(&seconds) {
                    return Err("benchmark-seconds must be 1..7200".into());
                }
                options.benchmark_seconds = Some(seconds);
            }
            Some("--benchmark-tour") => {
                options.benchmark_tour = match value.to_str() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("benchmark-tour must be on/off".into()),
                }
            }
            Some("--benchmark-pilot") => {
                options.benchmark_pilot = match value.to_str() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("benchmark-pilot must be on/off".into()),
                }
            }
            Some("--functional-check") => options.functional_check = Some(value.into()),
            Some("--media-report") => options.media_report = Some(value.into()),
            Some("--videos") => {
                options.video = match value.to_str() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("videos must be on/off".into()),
                }
            }
            Some("--audio") => {
                options.audio = match value.to_str() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("audio must be on/off".into()),
                }
            }
            Some("--frontend") => {
                options.frontend = Some(match value.to_str() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("frontend must be on/off".into()),
                })
            }
            Some("--record-replay") => options.record_replay = Some(value.into()),
            Some("--opponents") => {
                options.opponents = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "opponents must be 0..14")?;
                if options.opponents > 14 {
                    return Err("opponents must be 0..14".into());
                }
            }
            Some("--practice-riders") => {
                options.practice_riders = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "practice riders must be 0..14")?;
                if options.practice_riders > 14 {
                    return Err("practice riders must be 0..14".into());
                }
            }
            Some("--fast-recovery" | "--rescue-recovery" | "--effects") => {
                let enabled = match value.to_str() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("setting must be on/off".into()),
                };
                match arg.to_str() {
                    Some("--effects") => options.effects = enabled,
                    Some("--fast-recovery") => options.fast_recovery = enabled,
                    _ => options.rescue_recovery = enabled,
                }
            }
            Some("--bike") => {
                options.bike = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid bike ID")?;
            }
            Some("--rider-color") => {
                options.rider_color = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid rider color")?;
            }
            Some("--calibration") => options.calibration = Some(value.into()),
            Some("--bindings") => options.bindings = Some(value.into()),
            Some("--smoke-frames") => {
                let count: u32 = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid frame count")?;
                if count < 120 {
                    return Err("smoke test needs at least 120 frames".into());
                }
                options.smoke_frames = Some(count);
            }
            Some("--capture-frame") => {
                options.capture_frame = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid capture frame")?;
                if options.capture_frame == 0 {
                    return Err("capture frame must be positive".into());
                }
            }
            Some("--capture") => options.capture = Some(value.into()),
            Some("--install-root") => {
                let install =
                    rr_import::active_install(&PathBuf::from(value)).map_err(|e| e.to_string())?;
                options.data_root = Some(install.data_root.clone());
                println!(
                    "Validated install: {} files, generation {}",
                    install.files, install.generation
                );
            }
            Some("--course") => options.course = Some(value.to_string_lossy().to_ascii_uppercase()),
            Some("--level") => {
                options.level = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid level")?;
                if !(1..=5).contains(&options.level) {
                    return Err("level must be 1..5".into());
                }
            }
            Some("--branch-mask") => {
                options.branch_mask = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid branch mask")?
            }
            Some("--camera-sample") => {
                let n: f64 = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid camera sample")?;
                if !n.is_finite() || n < 0. {
                    return Err("camera sample must be finite/nonnegative".into());
                }
                options.camera_sample = Some(n)
            }
            Some("--framing") => {
                options.framing = match value.to_str() {
                    Some("classic") => rr_render::Framing::Classic,
                    Some("wide") => rr_render::Framing::Wide,
                    _ => return Err("framing must be classic|wide".into()),
                }
            }
            Some("--integer-scale") => {
                options.integer_scale = match value.to_str() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("integer-scale must be on|off".into()),
                }
            }
            Some("--window-size") => {
                let text = value.to_string_lossy();
                let (w, h) = text
                    .split_once('x')
                    .ok_or("window-size must be WIDTHxHEIGHT")?;
                let w: u32 = w.parse().map_err(|_| "invalid window width")?;
                let h: u32 = h.parse().map_err(|_| "invalid window height")?;
                if !(320..=3840).contains(&w) || !(240..=2160).contains(&h) {
                    return Err("window dimensions out of bounds".into());
                }
                options.window_size = Some((w, h));
            }
            Some("--replay") => {
                let replay = Replay::read_file(&PathBuf::from(value)).map_err(|e| e.to_string())?;
                options.expected_hash = replay.expected_hash.clone();
                options.replay = Some(replay);
            }
            Some("--catalog") => options.catalog = Some(value.into()),
            Some("--asset") => options.asset = Some(value.to_string_lossy().into_owned()),
            Some("--enhanced-image") => options.override_image = Some(value.into()),
            Some("--texture-mode") => {
                options.enhanced = match value.to_str() {
                    Some("native") => false,
                    Some("enhanced") => true,
                    _ => return Err("texture-mode must be native or enhanced".into()),
                }
            }
            Some("--switch-textures-at") => {
                options.switch_tick = Some(
                    value
                        .to_string_lossy()
                        .parse()
                        .map_err(|_| "invalid texture switch tick")?,
                )
            }
            Some("--render-hz") => {
                options.render_rate = value.to_string_lossy().into_owned();
                frame_ns(&options.render_rate, 0).map_err(str::to_string)?;
            }
            Some("--report") => options.report = Some(value.into()),
            _ => return Err(format!("unknown argument: {}", arg.to_string_lossy())),
        }
    }
    if options.settings_file.exists() {
        match rr_session::persistence::disk::read(&options.settings_file)
            .and_then(|b| rr_session::persistence::preferences::Preferences::decode(&b))
        {
            Ok((p, warnings)) => {
                for w in warnings {
                    eprintln!("Preferences: {w}");
                }
                if !options.explicit.contains("--vsync") {
                    options.vsync = p.settings.vsync;
                }
                if !options.explicit.contains("--videos") {
                    options.video = p.settings.videos;
                }
                if !options.explicit.contains("--effects") {
                    options.effects = p.settings.effects;
                }
                if !options.explicit.contains("--texture-mode") {
                    options.enhanced = p.settings.enhanced;
                }
                if !options.explicit.contains("--framing") {
                    options.framing = if p.settings.wide {
                        rr_render::Framing::Wide
                    } else {
                        rr_render::Framing::Classic
                    };
                }
                if !options.explicit.contains("--career-file") && p.active_slot != 0 {
                    options.career_file = rr_session::persistence::paths::slot_path(
                        &options.save_base,
                        p.active_slot,
                    )?;
                }
            }
            Err(e) => eprintln!("Preferences retained without applying invalid data: {e}"),
        }
    }
    if (options.benchmark_pilot || options.benchmark_seconds.is_some())
        && options.performance_report.is_none()
    {
        return Err("benchmark options require --performance-report".into());
    }
    if options.benchmark_tour
        && (!options.benchmark_pilot || options.benchmark_seconds.is_none_or(|s| s < 25))
    {
        return Err("benchmark-tour requires imported pilot and at least 25 seconds".into());
    }
    if options.benchmark_pilot && options.data_root.is_none() {
        return Err("benchmark-pilot requires a verified imported game".into());
    }
    Ok(options)
}
fn visuals(options: &LaunchOptions) -> Result<(VisualAssets, String), Box<dyn std::error::Error>> {
    let Some(root) = &options.catalog else {
        return Ok((VisualAssets::default(), "assets".into()));
    };
    let catalog = rr_content::Catalog::read(&root.join("catalog.json"))?;
    let id = if let Some(id) = &options.asset {
        rr_content::AssetId::new(id.clone())?
    } else {
        catalog
            .entries
            .iter()
            .find(|e| {
                e.kind == rr_content::Kind::Rider
                    && !e.preview_variants.is_empty()
                    && (options.data_root.is_none() || e.frame == Some(2))
            })
            .ok_or("no nonempty rider asset")?
            .id
            .clone()
    };
    let pack = rr_content::Pack {
        name: "game-launch".into(),
        references: vec![rr_content::Reference {
            from: id.clone(),
            to: id.clone(),
            expected_kind: rr_content::Kind::Rider,
        }],
        ..Default::default()
    };
    let resolved = catalog.resolve(&[pack])?;
    let entry = &resolved[&id];
    let native = entry
        .preview_variants
        .first()
        .ok_or("selected asset has no image")?
        .clone();
    if !rr_content::blob_valid(root, &native) {
        return Err(format!("game-launch: {} has missing/corrupt preview {native}", id.0).into());
    }
    let enhanced = if let Some(path) = &options.override_image {
        let rgba = rr_formats::pixels::Rgba::from_png(&std::fs::read(path)?)?;
        Some(rr_content::put_blob(root, &rgba.png()?, "png")?)
    } else {
        None
    };
    println!("Resolved semantic asset {} from {}", id.0, entry.source);
    Ok((
        VisualAssets {
            native: Some(native),
            enhanced,
            size: entry.logical_size,
            initially_enhanced: options.enhanced,
            switch_tick: options.switch_tick,
        },
        root.join("objects")
            .canonicalize()?
            .to_string_lossy()
            .into_owned(),
    ))
}

fn course_view(
    options: &LaunchOptions,
    visual: &VisualAssets,
) -> Result<Option<CoursePresentation>, Box<dyn std::error::Error>> {
    let (Some(data), Some(catalog)) = (&options.data_root, &options.catalog) else {
        if options.course.is_some() {
            return Err("--course requires --install-root and --catalog".into());
        }
        return Ok(None);
    };
    let stem = options.course.as_deref().unwrap_or("CITY");
    if !rr_course::export::COURSES.iter().any(|c| c.0 == stem) {
        return Err("unknown course (CITY/HIWAY/MEDLY/NAPA/CANYN)".into());
    }
    let course = rr_course::Course::read(&std::fs::read(
        data.join(format!("DATA/COURSES/{stem}.CRS")),
    )?)?;
    let mut assets = rr_render::source::SourceAssets::load(data, catalog, stem, &course)?;
    if let Some(default) = &assets.animations {
        assets.palettes.insert(0, default.clone());
    }
    if let Some(foot) = &assets.police_foot_rider {
        assets.palettes.insert(6, foot.clone());
    }
    for color in std::iter::once(options.rider_color)
        .chain(
            (0..if options.replay.is_none() && options.practice_riders == 0 {
                options.opponents
            } else {
                options.practice_riders
            })
                .map(|i| (i + 1) % 15),
        )
        .chain(
            options
                .replay
                .iter()
                .filter_map(|r| r.encounter.as_ref())
                .flat_map(|s| s.seeds.iter().map(|s| s.color)),
        )
    {
        if color != 0 && !assets.palettes.contains_key(&color) {
            assets.palettes.insert(
                color,
                std::sync::Arc::new(rr_render::rider::RiderAssets::load(data, color)?),
            );
        }
    }
    if options.rider_color != 0 {
        assets.animations = assets.palettes.get(&options.rider_color).cloned();
    }
    if options.asset.is_some() {
        let blob = visual.native.as_ref().ok_or("no selected rider")?;
        assets.rider.texture = rr_render::raster::Texture::new(
            rr_formats::pixels::Rgba::from_png(&std::fs::read(
                catalog.join("objects").join(blob),
            )?)?,
            false,
        )?;
        let entries = rr_content::Catalog::read(&catalog.join("catalog.json"))?;
        let entry = entries
            .entries
            .iter()
            .find(|e| Some(&e.id.0) == options.asset.as_ref())
            .ok_or("selected rider metadata")?;
        let (logical, pivot) = rr_render::source::rider_registration(
            &std::fs::read(data.join("DATA/GLOBAL.RSC"))?,
            entry.frame.ok_or("selected rider frame")?,
        )?;
        assets.rider.logical = logical;
        assets.rider.pivot = pivot;
    }
    if let Some(path) = &options.override_image {
        assets.set_override(path)?
    }
    let scene = rr_render::Scene::new(
        course,
        assets,
        rr_render::SceneOptions {
            course: stem.into(),
            level: options.level,
            branch_mask: options.branch_mask,
            seed: 1,
        },
    )?;
    let start_q8 = options.camera_sample.unwrap_or(50.) * 256.;
    if start_q8 >= scene.route.len_q8() as f64 {
        return Err("camera sample beyond selected route".into());
    }
    println!(
        "Classic course {stem}, level {}, branch {}: {} source samples; {} resource notes",
        options.level,
        options.branch_mask,
        scene.route.points.len(),
        scene.notes.len()
    );
    Ok(Some(CoursePresentation {
        scene: std::sync::Arc::new(scene),
        camera: rr_render::CameraSpec::new(options.framing, options.integer_scale),
        start_q8,
    }))
}
fn build_race(
    options: &LaunchOptions,
    course: Option<&CoursePresentation>,
    seed: u32,
    character: Option<u8>,
) -> Race {
    if let (Some(data), Some(course)) = (&options.data_root, course) {
        let mut config = rr_simulation::riding::Configuration::load(
            data,
            &course.scene.route,
            options.bike,
            (course.start_q8 / 256.) as u32,
        )
        .unwrap_or_else(|e| {
            eprintln!("handling startup: {e}");
            std::process::exit(2)
        });
        config.motion_version = 3;
        for (g, p) in config.ground.iter_mut().zip(&course.scene.route.points) {
            g.kind = source_roads::ground_kind(
                &course.scene.terrain[p.section as usize][p.local as usize],
                g.flags,
            );
        }
        // Derived specification 16 isolates the player's carried damage. Opponents
        // continue selecting the original 1..15 specs, and replays capture this setup.
        if options.condition_percent < 100 {
            let mut derived = config.spec().clone();
            derived.id = 16;
            derived.condition =
                (derived.condition * i32::from(options.condition_percent) / 100).max(1);
            config.specs.push(derived);
            config.selected = 16;
        }
        let exe = std::fs::read(data.join("ROADRASH.EXE")).expect("verified executable");
        let mut setup = rr_simulation::encounter::Setup::source(
            &config,
            &course.scene.course,
            &course.scene.route,
            &exe,
            options.level - 1,
            if options.practice_riders > 0 {
                options.practice_riders
            } else {
                options.opponents
            },
        )
        .unwrap_or_else(|e| {
            eprintln!("encounter startup: {e}");
            std::process::exit(2)
        });
        source_roads::import(&mut setup, &course.scene).expect("validated physical source roads");
        let mut building_offsets = std::collections::BTreeMap::new();
        for p in &course.scene.route.points {
            building_offsets.insert((p.node, p.local), p.offset);
            if let Some(dual) = &p.dual {
                building_offsets.insert((dual.node, dual.local), dual.offset);
            }
        }
        for (path, node) in course
            .scene
            .course
            .nodes
            .iter()
            .filter(|(_, n)| n.kind == rr_course::NodeKind::Road)
        {
            for sample in 0..node.length as usize {
                let offset = if setup.rules.source_roads.is_some() {
                    0
                } else {
                    building_offsets
                        .get(&(*path, sample as u32))
                        .copied()
                        .unwrap_or(0)
                };
                setup.rules.building_boundaries.push(source_roads::boundary(
                    &course.scene,
                    *path,
                    sample,
                    offset,
                ));
            }
        }
        setup
            .rules
            .building_boundaries
            .sort_by_key(|b| (b.path, b.sample));
        // Physical source boxes are independent of rendered sprite size, pivot
        // and any future texture override. A frame with no boxes is not solid.
        setup.rules.obstacles.retain(|o| {
            !matches!(
                o.kind,
                rr_simulation::encounter::ObstacleKind::Solid
                    | rr_simulation::encounter::ObstacleKind::Decorative
            )
        });
        source_scenery::import(&mut setup, &course.scene, options.level)
            .expect("validated source scenery boxes");
        for (path_id, node) in &course.scene.course.nodes {
            if node.kind != rr_course::NodeKind::Road {
                continue;
            }
            let section = node.link as usize;
            for local in 0..node.length as usize {
                let sample = &course.scene.course.sections[section].samples[local];
                for side in 0..2 {
                    if let Some(profile) = course.scene.buildings[section][local][side] {
                        let bounds = profile.lateral_bounds(sample, side);
                        let start = local as i64 * 256 + i64::from(profile.start);
                        let end = local as i64 * 256 + i64::from(profile.end);
                        setup
                            .rules
                            .obstacles
                            .push(rr_simulation::encounter::Obstacle {
                                source_box: None,
                                id: setup.rules.obstacles.len() as u64 + 1,
                                path: *path_id,
                                progress: (start + end) / 2,
                                half_length: ((end - start + 1) / 2) as i32,
                                left: (bounds[0]
                                    + if setup.rules.source_roads.is_some() {
                                        0
                                    } else {
                                        building_offsets
                                            .get(&(*path_id, local as u32))
                                            .copied()
                                            .unwrap_or(0)
                                    })
                                    * 256,
                                right: (bounds[1]
                                    + if setup.rules.source_roads.is_some() {
                                        0
                                    } else {
                                        building_offsets
                                            .get(&(*path_id, local as u32))
                                            .copied()
                                            .unwrap_or(0)
                                    })
                                    * 256,
                                height: profile.height * 256,
                                kind: rr_simulation::encounter::ObstacleKind::Building,
                            });
                    }
                }
            }
        }
        setup.rules.collision_version = 2;
        setup.rules.contact_walk_version = 2;
        setup.rules.scenery_version = 1;
        setup.rules.scenery_start_ready = true;
        setup.rules.collision = Some(
            rr_simulation::collision::Profile::from_exe(&exe).expect("verified collision physics"),
        );
        setup.rules.combat_version = 4;
        let pe = rr_formats::pe::SupportedExe::read(&exe).expect("verified interaction tables");
        setup.rules.steal_codes = Some(std::array::from_fn(|i| {
            rr_formats::binary::u32le(
                pe.read_va(0x464538 + i as u32 * 4, 4)
                    .expect("steal animation table"),
                0,
            )
            .expect("steal code") as i8
        }));
        setup.rules.body.motion_version = 1;
        setup.rules.body.recovery_version = 2;
        setup
            .rules
            .body
            .enable_source_curvature(&exe)
            .expect("verified source recovery angles");
        setup.seeds[0].color = options.rider_color;
        setup.rules.assists.fast_recovery = options.fast_recovery;
        setup.rules.assists.rescue_unreachable = options.rescue_recovery;
        config.actions = Some(
            rr_simulation::actions::Config::read(
                &std::fs::read(data.join("DATA/BIKESPEC.RSC")).expect("verified bike data"),
                options.bike,
            )
            .expect("validated player actions"),
        );
        let mut sim = if options.practice_riders > 0 {
            if let Some(character) = character {
                apply_character(&exe, character, options.level - 1, &mut setup);
            }
            Simulation::with_population(
                seed,
                config,
                setup,
                rr_simulation::population::Setup {
                    traffic: false,
                    periodic_police: false,
                    ..Default::default()
                },
            )
            .unwrap()
        } else {
            let mut population = rr_simulation::population::Setup::source(
                &config,
                &mut setup,
                &course.scene.course,
                &exe,
                options.level - 1,
                course.scene.assets.vehicles.len() as u16,
            )
            .unwrap_or_else(|e| {
                eprintln!("population startup: {e}");
                std::process::exit(2)
            });
            population.functional_traffic = true;
            population.tactics_version = 1;
            population.police = Some(
                rr_simulation::population::police::Config::read(&exe)
                    .expect("validated native police tables"),
            );
            population.vehicle_masks = course.scene.assets.vehicle_masks.clone();
            population.pedestrian_appearances = course.scene.assets.pedestrian_appearances.clone();
            let requested = population.spawns.len();
            population.spawns.retain(|s| {
                s.kind != rr_simulation::population::Kind::Pedestrian
                    || s.family
                        .and_then(|id| course.scene.assets.families.get(&id))
                        .and_then(|f| {
                            f.pedestrian_frames
                                .get(&s.selector)
                                .and_then(|frames| frames.iter().find_map(Option::as_ref))
                        })
                        .is_some()
            });
            if requested != population.spawns.len() {
                println!(
                    "Population: {} unsupported pedestrian appearances retained as research requests, excluded from live collisions",
                    requested - population.spawns.len()
                );
            }
            if let Some(character) = character {
                apply_character(&exe, character, options.level - 1, &mut setup);
            }
            Simulation::with_population(seed, config, setup, population).unwrap()
        };
        sim.set_effects_enabled(options.effects);
        Race(sim)
    } else {
        Race::default()
    }
}
fn apply_character(
    exe: &[u8],
    character: u8,
    level: u8,
    setup: &mut rr_simulation::encounter::Setup,
) {
    let catalog = rr_session::catalog::Catalog::from_exe(exe).expect("verified career catalog");
    let c = catalog.character(character).expect("validated character");
    let pe = rr_formats::pe::SupportedExe::read(exe).unwrap();
    let word = |va| rr_formats::binary::u32le(pe.read_va(va, 4).unwrap(), 0).unwrap() as i32;
    let row = 0x463c78 + u32::from(c.personality) * 0x40;
    let seed = &mut setup.seeds[0];
    seed.stats = rr_simulation::combat::Stats {
        strength: word(row + 0x30),
        base: word(row + 0x34),
        maximum: word(row + 0x34),
        health: word(row + 0x34),
    };
    seed.color = c.personality;
    let weapon = word(row + (6 + u32::from(level)) * 4);
    setup
        .inventory
        .items
        .retain(|_, item| item.owner != Some(rr_simulation::ActorId(1)));
    if (1..=2).contains(&weapon) {
        setup.inventory.items.insert(
            rr_simulation::combat::WeaponId(100),
            rr_simulation::combat::Item {
                kind: if weapon == 1 {
                    rr_simulation::animation::Weapon::Club
                } else {
                    rr_simulation::animation::Weapon::Chain
                },
                owner: Some(rr_simulation::ActorId(1)),
            },
        );
    }
}
fn main() {
    let launch_started = std::time::Instant::now();
    let mut options = options().unwrap_or_else(|m| {
        eprintln!("{m}");
        std::process::exit(2)
    });
    if let Some(output) = &options.functional_check {
        if let Err(error) = acceptance::run(&options, output) {
            eprintln!("Functional acceptance failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    if options.benchmark_pilot {
        options.frontend = Some(false);
        options.video = false;
    }
    let perf_enabled = options.performance_report.is_some();
    let present_mode = if options.vsync {
        bevy::window::PresentMode::AutoVsync
    } else {
        bevy::window::PresentMode::AutoNoVsync
    };
    let mut playback = options
        .replay
        .clone()
        .map(Playback::new)
        .transpose()
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        });
    if let Some(playback) = &mut playback {
        playback.simulation.set_effects_enabled(options.effects);
    }
    let (visual, asset_root) = visuals(&options).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let course = course_view(&options, &visual).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let race = if playback.is_none() {
        build_race(&options, course.as_ref(), 1, None)
    } else {
        Race::default()
    };
    let window_size = options.window_size.unwrap_or((960, 720));
    let bindings = options
        .bindings
        .as_ref()
        .map(|path| rr_input::ActionMap::read(path))
        .transpose()
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        })
        .unwrap_or_else(|| {
            rr_session::persistence::disk::read(&options.settings_file)
                .and_then(|b| rr_session::persistence::preferences::Preferences::decode(&b))
                .map(|(p, _)| p.bindings)
                .unwrap_or_default()
        });
    if options.calibration.is_none() {
        let path = PathBuf::from("local/controller-calibration.json");
        if path.exists() {
            options.calibration = Some(path);
        }
    }
    let profiles = options
        .calibration
        .as_ref()
        .map(|path| rr_input::calibration::CalibrationFile::read(path))
        .transpose()
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        })
        .unwrap_or_default();
    let recorder = race_recording::Recorder::new(
        if playback.is_none() {
            options.record_replay.clone()
        } else {
            None
        },
        &race.0,
        options.data_root.as_deref(),
    )
    .unwrap_or_else(|e| {
        eprintln!("recording startup: {e}");
        std::process::exit(2)
    });
    let shell = frontend::Shell::new(&options, &race.0, playback.is_some());
    let mut app = App::new();
    app.insert_resource(performance::LaunchStart(launch_started));
    app.insert_resource(shell)
        .add_plugins(frontend::FrontendPlugin)
        .add_plugins(media::MediaPlugin);
    app.insert_resource(recorder)
        .init_resource::<population_cues::Warnings>()
        .add_systems(Last, race_recording::save)
        .add_systems(
            Update,
            population_cues::update.after(PresentationStage::Prepare),
        );
    app.insert_resource(input_adapter::CalibrationProfiles(profiles));
    input_adapter::install_calibration(&mut app);
    if let Some(course) = course {
        app.insert_resource(course);
    }
    app.insert_resource(race);
    app.insert_resource(visual)
        .insert_resource(Bindings(bindings))
        .insert_resource(options)
        .insert_resource(Driver(playback))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_root,
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode,
                        title: "RoadRash Rust — classic renderer".into(),
                        resolution: (window_size.0, window_size.1).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(Time::<Fixed>::from_hz(f64::from(TICKS_PER_SECOND)))
        .init_resource::<Race>()
        .init_resource::<Previous>()
        .init_resource::<InputCommand>()
        .init_resource::<PauseState>()
        .init_resource::<LiveInput>()
        .init_resource::<Router>()
        .add_plugins(ClassicPresentationPlugin)
        .configure_sets(
            FixedUpdate,
            (
                FixedStage::Input,
                FixedStage::Simulation,
                FixedStage::Snapshot,
            )
                .chain(),
        )
        .add_systems(
            FixedUpdate,
            (sample_input, performance::pilot)
                .chain()
                .in_set(FixedStage::Input)
                .run_if(live),
        )
        .add_systems(
            FixedUpdate,
            advance.in_set(FixedStage::Simulation).run_if(live),
        )
        .add_systems(
            FixedUpdate,
            (publish, media::capture)
                .chain()
                .in_set(FixedStage::Snapshot)
                .run_if(live),
        )
        .add_systems(PreUpdate, playback_frame)
        .add_systems(
            PreUpdate,
            poll_input
                .after(bevy::input::InputSystems)
                .run_if(physical_input),
        )
        .add_systems(
            PreUpdate,
            pause_input
                .after(poll_input)
                .run_if(physical_input)
                .run_if(frontend::pause_controls),
        )
        .add_systems(
            Update,
            (
                interpolate.in_set(PresentationStage::Prepare),
                smoke_test.after(PresentationStage::Draw),
            ),
        );
    if perf_enabled {
        app.add_plugins(performance::PerformancePlugin);
    }
    app.run();
}
fn live(
    driver: Res<Driver>,
    pause: Res<PauseState>,
    shell: Option<Res<frontend::Shell>>,
    media: Option<Res<media::Media>>,
) -> bool {
    driver.0.is_none()
        && !pause.paused
        && shell.is_none_or(|s| s.race_active())
        && media::not_playing(media)
}
fn physical_input(driver: Res<Driver>, options: Res<LaunchOptions>) -> bool {
    driver.0.is_none() && !options.benchmark_pilot
}
fn sample_input(input: Res<LiveInput>, mut command: ResMut<InputCommand>, race: Option<Res<Race>>) {
    command.0 = tick_command(&input.0);
    if let Some(race) = race.filter(|race| race.0.schema_version() >= 3) {
        use rr_simulation::animation::Pose;
        command.0.pose = if input.0.buttons.punch {
            Some(Pose::PunchLeft)
        } else if input.0.buttons.kick {
            Some(Pose::KickLeft)
        } else if input.0.buttons.backhand {
            Some(Pose::BackhandLeft)
        } else {
            None
        };
        if race.0.schema_version() >= 6 {
            command.0.nitro = input.0.buttons.nitro;
            command.0.dismount = input.0.buttons.dismount;
            command.0.lean = input.0.buttons.lean;
        }
        if race.0.schema_version() >= 4 {
            command.0.attack = command
                .0
                .pose
                .take()
                .and_then(rr_simulation::combat::Request::from_pose)
                .map(|mut request| {
                    request.side = rr_simulation::combat::Side::Auto;
                    request
                });
            let drive = &input.0.drive;
            if !drive.neutral() {
                command.0.walk = Some(rr_simulation::recovery::Walk {
                    longitudinal: i8::from(command.0.throttle) - i8::from(command.0.brake),
                    lateral: command
                        .0
                        .steer_q15
                        .unwrap_or(i16::from(command.0.steer))
                        .signum() as i8,
                });
            }
        }
    }
}
fn pause_input(
    driver: Res<Driver>,
    input: Res<LiveInput>,
    bindings: Res<Bindings>,
    mut pause: ResMut<PauseState>,
    mut command: ResMut<InputCommand>,
    mut window: Single<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    if driver.0.is_some() {
        return;
    }
    let was_paused = pause.paused;
    if !window.focused || input.0.active_disconnected || (!pause.paused && input.0.pause_pressed) {
        pause.paused = true;
        pause.release_required = true;
    }
    if pause.paused {
        command.0 = Command::default();
        if input.0.drive.neutral() && input.0.buttons.neutral() {
            pause.release_required = false;
        }
        if was_paused
            && window.focused
            && !pause.release_required
            && input.0.drive.neutral()
            && input.0.buttons.neutral()
            && input.0.resume_pressed
            && !input.0.active_disconnected
        {
            pause.paused = false;
        }
    }
    if pause.paused != was_paused {
        window.title = if pause.paused {
            format!(
                "RoadRash Rust — paused (Start or {:?} to resume)",
                bindings.0.contexts[&Context::Paused][&Action::Resume]
            )
        } else {
            "RoadRash Rust — classic renderer".into()
        };
    }
}
fn failure(message: impl std::fmt::Display, exit: &mut MessageWriter<AppExit>) {
    eprintln!("{message}");
    exit.write(AppExit::Error(NonZeroU8::new(1).unwrap()));
}
fn advance(
    command: Res<InputCommand>,
    mut race: ResMut<Race>,
    mut previous: ResMut<Previous>,
    recorder: Option<ResMut<race_recording::Recorder>>,
    mut shell: Option<ResMut<frontend::Shell>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(shell) = shell.as_mut()
        && let Some(session) = shell.session.as_mut()
        && !session.permit_tick()
    {
        return;
    }
    previous.0 = race.0.snapshot();
    if let Some(mut recorder) = recorder {
        recorder.command(race.0.tick().0, command.0);
    }
    if let Err(e) = race.0.step(command.0) {
        failure(e, &mut exit);
    }
    if let Some(shell) = shell.as_mut() {
        frontend::observe_social(shell, &race.0);
    }
    if let Some(shell) = shell.as_mut()
        && let Some(session) = shell.session.as_mut()
    {
        session.observe(&race.0.snapshot());
    }
}
fn publish(race: Res<Race>, previous: Res<Previous>, mut view: ResMut<PresentedSnapshot>) {
    view.0 = race.0.snapshot();
    view.1 = previous.0.clone();
}
fn interpolate(
    driver: Res<Driver>,
    pause: Res<PauseState>,
    time: Res<Time<Fixed>>,
    mut view: ResMut<PresentedSnapshot>,
) {
    if driver.0.is_none() {
        if pause.paused {
            // Freeze the owned visual pair too: resuming on a frame with
            // no fixed tick must not interpolate back to a pre-pause pose.
            view.1 = view.0.clone();
            view.2 = 1.;
        } else {
            view.2 = time.overstep_fraction();
        }
    }
}
fn playback_frame(
    mut driver: ResMut<Driver>,
    options: Res<LaunchOptions>,
    mut view: ResMut<PresentedSnapshot>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(playback) = &mut driver.0 else {
        return;
    };
    if playback.done() {
        return;
    }
    let duration = frame_ns(&options.render_rate, playback.frames).unwrap();
    if let Err(e) = playback.frame(duration) {
        failure(e, &mut exit);
        return;
    }
    view.0 = playback.current.clone();
    view.1 = playback.previous.clone();
    view.2 = playback.alpha();
    if playback.done() {
        let report = Report {
            simulation_schema: playback.simulation.schema_version(),
            rate: options.render_rate.clone(),
            ticks: playback.current.tick,
            presentation_frames: playback.frames,
            state_sha256: playback.simulation.state_hash(),
            final_snapshot: playback.current.clone(),
            tick_hashes: playback.checkpoints.clone(),
            forensic: vec![],
        };
        if options
            .expected_hash
            .as_ref()
            .is_some_and(|h| *h != report.state_sha256)
        {
            failure("replay hash mismatch", &mut exit);
            return;
        }
        if let Some(path) = &options.report {
            let result = serde_json::to_vec_pretty(&report)
                .map_err(|e| e.to_string())
                .and_then(|b| std::fs::write(path, b).map_err(|e| e.to_string()));
            if let Err(e) = result {
                failure(e, &mut exit);
                return;
            }
        }
        println!(
            "Rendered replay: {} frames, hash {}",
            report.presentation_frames, report.state_sha256
        );
        exit.write(AppExit::Success);
    }
}
type SmokeQueries<'w, 's> = (
    Query<'w, 's, &'static bevy::text::TextLayoutInfo, With<frontend::MenuEntity>>,
    Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
);
fn smoke_test(
    options: Res<LaunchOptions>,
    mut frame: Local<u32>,
    view: Res<PresentedSnapshot>,
    course: Option<Res<CoursePresentation>>,
    (shell, media): (Option<Res<frontend::Shell>>, Option<Res<media::Media>>),
    (menu_layout, windows): SmokeQueries,
    (mut commands, mut exit): (Commands, MessageWriter<AppExit>),
) {
    *frame += 1;
    if *frame == options.capture_frame
        && let Some(path) = &options.capture
    {
        let menu_sizes: Vec<_> = menu_layout
            .iter()
            .map(|l| [l.size.x / l.scale_factor, l.size.y / l.scale_factor])
            .collect();
        let metadata = serde_json::json!({"actual_window":windows.iter().next().map(|w|serde_json::json!({"logical":[w.width(),w.height()],"physical":[w.physical_width(),w.physical_height()],"scale_factor":w.scale_factor()})),"menu_logical_sizes":menu_sizes,"media":media.as_ref().map(|m|&m.metrics),"frontend":shell.as_ref().map(|s|serde_json::json!({"screen":s.frontend.screen,"phase":s.session.as_ref().map(|s|s.phase()),"summary":s.frontend.summary})),"calibration_profile":options.calibration,"requested_frame":*frame,"simulation_tick":view.0.tick,"interpolation_alpha":view.2,"window_size":options.window_size.unwrap_or((960,720)),"camera":course.as_ref().map(|c|&c.camera),"source_start_q8":course.as_ref().map(|c|c.start_q8),"course":course.as_ref().map(|c|&c.scene.options.course),"level":options.level,"branch_mask":options.branch_mask,"texture_override_active":options.enhanced ^ options.switch_tick.is_some_and(|t|view.0.tick>=t)});
        if let Err(e) = serde_json::to_vec_pretty(&metadata)
            .map_err(|e| e.to_string())
            .and_then(|b| {
                std::fs::write(path.with_extension("capture.json"), b).map_err(|e| e.to_string())
            })
        {
            failure(e, &mut exit);
            return;
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    }
    if options.smoke_frames.is_some_and(|limit| *frame >= limit) {
        println!("foundation smoke completed after {} frames", *frame);
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    #[test]
    fn remapped_controls_reach_tick_commands_and_opposed_steering_cancels() {
        let mut bindings = rr_input::ActionMap::default();
        bindings
            .rebind(
                Context::Race,
                Action::Accelerate,
                vec![rr_input::Key::Space],
            )
            .unwrap();
        let mut app = App::new();
        app.insert_resource(Bindings(bindings))
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<InputCommand>()
            .init_resource::<LiveInput>()
            .init_resource::<Router>()
            .add_systems(Update, (poll_input, sample_input).chain());
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyW);
        }
        app.update();
        assert!(!app.world().resource::<InputCommand>().0.throttle);
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Space);
            keys.press(KeyCode::ArrowLeft);
            keys.press(KeyCode::ArrowRight);
        }
        app.update();
        let command = app.world().resource::<InputCommand>().0;
        assert!(command.throttle);
        assert_eq!(command.steer, 0);
    }
    #[test]
    fn focus_pause_requires_neutral_controls_and_explicit_resume() {
        let mut app = App::new();
        app.init_resource::<Bindings>()
            .init_resource::<Driver>()
            .init_resource::<PauseState>()
            .init_resource::<InputCommand>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<LiveInput>()
            .init_resource::<Router>()
            .init_resource::<PresentedSnapshot>()
            .init_resource::<Time<Fixed>>()
            .add_systems(Update, (poll_input, pause_input, interpolate).chain());
        {
            let mut view = app.world_mut().resource_mut::<PresentedSnapshot>();
            view.0.distance_mm = 100;
            view.1.distance_mm = 90;
        }
        let window = app
            .world_mut()
            .spawn((
                Window {
                    focused: false,
                    ..Default::default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();
        app.world_mut().resource_mut::<InputCommand>().0.throttle = true;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        assert!(app.world().resource::<PauseState>().paused);
        assert!(!app.world().resource::<InputCommand>().0.throttle);
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert!(app.world().resource::<PauseState>().paused);
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyW);
            keys.clear();
        }
        app.update();
        assert!(app.world().resource::<PauseState>().paused);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();
        assert!(!app.world().resource::<PauseState>().paused);
        let view = app.world().resource::<PresentedSnapshot>();
        assert_eq!(view.0.distance_mm, 100);
        assert_eq!(
            view.1.distance_mm, 100,
            "resume without a fixed tick must not rewind visual interpolation"
        );
    }
}

#[cfg(test)]
mod gamepad_pause_tests {
    use super::*;
    use bevy::input::{
        InputPlugin, InputSystems,
        gamepad::{
            GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent,
            RawGamepadEvent,
        },
    };
    fn button(app: &mut App, id: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                id, button, value,
            )));
        app.update();
    }
    #[test]
    fn start_pauses_once_disconnect_stays_paused_and_resume_requires_neutral() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin))
            .init_resource::<Bindings>()
            .init_resource::<Router>()
            .init_resource::<LiveInput>()
            .init_resource::<Driver>()
            .init_resource::<InputCommand>()
            .init_resource::<PauseState>()
            .add_systems(
                PreUpdate,
                (poll_input, pause_input).chain().after(InputSystems),
            );
        app.world_mut().spawn((
            Window {
                focused: true,
                ..Default::default()
            },
            bevy::window::PrimaryWindow,
        ));
        let id = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(GamepadConnectionEvent::new(
            id,
            GamepadConnection::Connected {
                name: "Authored pause test".into(),
                vendor_id: None,
                product_id: None,
            },
        ));
        app.update();
        button(&mut app, id, GamepadButton::Start, 1.);
        assert!(
            app.world().resource::<PauseState>().paused,
            "same Start edge must not immediately resume"
        );
        app.update();
        assert!(app.world().resource::<PauseState>().paused);
        button(&mut app, id, GamepadButton::Start, 0.);
        button(&mut app, id, GamepadButton::South, 1.);
        button(&mut app, id, GamepadButton::Start, 1.);
        assert!(
            app.world().resource::<PauseState>().paused,
            "holding throttle blocks resume even after a previous neutral frame"
        );
        button(&mut app, id, GamepadButton::South, 0.);
        button(&mut app, id, GamepadButton::Start, 0.);
        button(&mut app, id, GamepadButton::Start, 1.);
        assert!(!app.world().resource::<PauseState>().paused);
        app.world_mut().resource_mut::<InputCommand>().0.throttle = true;
        app.world_mut().write_message(GamepadConnectionEvent::new(
            id,
            GamepadConnection::Disconnected,
        ));
        app.update();
        assert!(app.world().resource::<PauseState>().paused);
        assert!(!app.world().resource::<InputCommand>().0.throttle);
        app.update();
        assert!(app.world().resource::<PauseState>().paused);
        app.world_mut().write_message(GamepadConnectionEvent::new(
            id,
            GamepadConnection::Connected {
                name: "Reconnected authored pad".into(),
                vendor_id: None,
                product_id: None,
            },
        ));
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                id,
                GamepadButton::South,
                1.,
            )));
        app.update();
        assert!(app.world().resource::<PauseState>().paused);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();
        assert!(!app.world().resource::<PauseState>().paused);
        assert!(
            !tick_command(&app.world().resource::<LiveInput>().0).throttle,
            "held-on-reconnect input cannot latch throttle after keyboard resume"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyP);
        app.update();
        assert!(!tick_command(&app.world().resource::<LiveInput>().0).throttle);
        button(&mut app, id, GamepadButton::South, 0.);
        button(&mut app, id, GamepadButton::South, 1.);
        assert!(tick_command(&app.world().resource::<LiveInput>().0).throttle);
    }
}
