//! Composition root: ordered input/fixed simulation/snapshot adapter and optional deterministic playback.
mod gamepad_calibration;
mod input_adapter;
mod population_cues;
mod race_recording;
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
#[derive(Resource, Default)]
struct LaunchOptions {
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
    let mut options = LaunchOptions {
        render_rate: "60".into(),
        level: 1,
        bike: 1,
        opponents: 14,
        effects: true,
        capture_frame: 30,
        ..Default::default()
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {}", arg.to_string_lossy()))?;
        match arg.to_str() {
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
fn main() {
    let mut options = options().unwrap_or_else(|m| {
        eprintln!("{m}");
        std::process::exit(2)
    });
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
        if let (Some(data), Some(course)) = (&options.data_root, &course) {
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
            for (g, p) in config.ground.iter_mut().zip(&course.scene.route.points) {
                g.kind = match course.scene.terrain[p.section as usize][p.local as usize].kind {
                    rr_render::scene::TerrainKind::Wall => 1,
                    rr_render::scene::TerrainKind::Building => 4,
                    _ => 0,
                };
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
            setup.seeds[0].color = options.rider_color;
            setup.rules.assists.fast_recovery = options.fast_recovery;
            setup.rules.assists.rescue_unreachable = options.rescue_recovery;
            let mut sim = if options.practice_riders > 0 {
                Simulation::with_encounters(1, config, setup).unwrap()
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
                population.vehicle_masks = course.scene.assets.vehicle_masks.clone();
                let requested = population.spawns.len();
                population.spawns.retain(|s| {
                    s.kind != rr_simulation::population::Kind::Pedestrian
                        || s.family
                            .and_then(|id| course.scene.assets.families.get(&id))
                            .and_then(|f| {
                                f.resolve(2, usize::from(s.selector), Some(0))
                                    .and_then(|at| f.sprites.get(&at))
                            })
                            .is_some()
                });
                if requested != population.spawns.len() {
                    println!(
                        "Population: {} unsupported pedestrian appearances retained as research requests, excluded from live collisions",
                        requested - population.spawns.len()
                    );
                }
                Simulation::with_population(1, config, setup, population).unwrap()
            };
            sim.set_effects_enabled(options.effects);
            Race(sim)
        } else {
            Race::default()
        }
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
        .unwrap_or_default();
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
    let mut app = App::new();
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
            sample_input.in_set(FixedStage::Input).run_if(live),
        )
        .add_systems(
            FixedUpdate,
            advance.in_set(FixedStage::Simulation).run_if(live),
        )
        .add_systems(
            FixedUpdate,
            publish.in_set(FixedStage::Snapshot).run_if(live),
        )
        .add_systems(PreUpdate, playback_frame)
        .add_systems(
            PreUpdate,
            (poll_input, pause_input)
                .chain()
                .after(bevy::input::InputSystems)
                .run_if(physical_input),
        )
        .add_systems(
            Update,
            (
                interpolate.in_set(PresentationStage::Prepare),
                smoke_test.after(PresentationStage::Draw),
            ),
        )
        .run();
}
fn live(driver: Res<Driver>, pause: Res<PauseState>) -> bool {
    driver.0.is_none() && !pause.paused
}
fn physical_input(driver: Res<Driver>) -> bool {
    driver.0.is_none()
}
fn sample_input(
    input: Res<LiveInput>,
    mut command: ResMut<InputCommand>,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    race: Option<Res<Race>>,
) {
    command.0 = tick_command(&input.0);
    if let Some(race) = race.filter(|race| race.0.schema_version() >= 3) {
        let actions = bindings.0.active(
            Context::Race,
            bindings.0.contexts[&Context::Race]
                .values()
                .flatten()
                .copied()
                .filter(|key| keys.pressed(input_adapter::key_code(*key))),
        );
        use rr_simulation::animation::Pose;
        command.0.pose = if actions.contains(&Action::Punch) {
            Some(Pose::PunchLeft)
        } else if actions.contains(&Action::Kick) {
            Some(Pose::KickLeft)
        } else if actions.contains(&Action::Backhand) {
            Some(Pose::BackhandLeft)
        } else {
            None
        };
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
        if input.0.drive.neutral() {
            pause.release_required = false;
        }
        if was_paused
            && window.focused
            && !pause.release_required
            && input.0.drive.neutral()
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
    mut exit: MessageWriter<AppExit>,
) {
    previous.0 = race.0.snapshot();
    if let Some(mut recorder) = recorder {
        recorder.command(race.0.tick().0, command.0);
    }
    if let Err(e) = race.0.step(command.0) {
        failure(e, &mut exit);
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
fn smoke_test(
    options: Res<LaunchOptions>,
    mut frame: Local<u32>,
    view: Res<PresentedSnapshot>,
    course: Option<Res<CoursePresentation>>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    *frame += 1;
    if *frame == options.capture_frame
        && let Some(path) = &options.capture
    {
        let metadata = serde_json::json!({"calibration_profile":options.calibration,"requested_frame":*frame,"simulation_tick":view.0.tick,"interpolation_alpha":view.2,"window_size":options.window_size.unwrap_or((960,720)),"camera":course.as_ref().map(|c|&c.camera),"source_start_q8":course.as_ref().map(|c|c.start_q8),"course":course.as_ref().map(|c|&c.scene.options.course),"level":options.level,"branch_mask":options.branch_mask,"texture_override_active":options.enhanced ^ options.switch_tick.is_some_and(|t|view.0.tick>=t)});
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
