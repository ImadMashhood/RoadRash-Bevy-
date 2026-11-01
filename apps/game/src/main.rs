//! Composition root: ordered input/fixed simulation/snapshot adapter and optional deterministic playback.
mod input_adapter;
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use input_adapter::{Bindings, key_code};
use rr_input::{Action, Context};
use rr_presentation::{
    ClassicPresentationPlugin, CoursePresentation, PresentationStage, PresentedSnapshot,
    VisualAssets,
};
use rr_simulation::{
    Command, SIMULATION_SCHEMA, Simulation, TICKS_PER_SECOND,
    replay::{Playback, Replay, Report, frame_ns},
};
use std::{num::NonZeroU8, path::PathBuf};
#[derive(Resource, Default)]
struct Race(Simulation);
#[derive(Resource, Default)]
struct LaunchOptions {
    smoke_frames: Option<u32>,
    capture: Option<PathBuf>,
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
        ..Default::default()
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {}", arg.to_string_lossy()))?;
        match arg.to_str() {
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
    let playback = options
        .replay
        .take()
        .map(Playback::new)
        .transpose()
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        });
    let (visual, asset_root) = visuals(&options).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let course = course_view(&options, &visual).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
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
    let mut app = App::new();
    if let Some(course) = course {
        app.insert_resource(course);
    }
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
        .add_systems(PreUpdate, (playback_frame, pause_input))
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
fn active_actions(
    bindings: &Bindings,
    context: Context,
    pressed: impl Fn(KeyCode) -> bool,
) -> std::collections::BTreeSet<Action> {
    bindings.0.active(
        context,
        bindings.0.contexts[&context]
            .values()
            .flatten()
            .copied()
            .filter(|key| pressed(key_code(*key))),
    )
}
fn sample_input(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    mut command: ResMut<InputCommand>,
) {
    let actions = active_actions(&bindings, Context::Race, |key| keys.pressed(key));
    let left = actions.contains(&Action::SteerLeft);
    let right = actions.contains(&Action::SteerRight);
    command.0 = Command {
        throttle: actions.contains(&Action::Accelerate),
        brake: actions.contains(&Action::Brake),
        steer: i8::from(right) - i8::from(left),
        ..Default::default()
    };
}
fn pause_input(
    driver: Res<Driver>,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    mut pause: ResMut<PauseState>,
    mut command: ResMut<InputCommand>,
    mut window: Single<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    if driver.0.is_some() {
        return;
    }
    let actions = active_actions(&bindings, Context::Race, |key| keys.pressed(key));
    let was_paused = pause.paused;
    if !window.focused
        || (!pause.paused
            && active_actions(&bindings, Context::Race, |key| keys.just_pressed(key))
                .contains(&Action::Pause))
    {
        pause.paused = true;
        pause.release_required = true;
    }
    if pause.paused {
        command.0 = Command::default();
        if !actions.iter().any(|a| {
            matches!(
                a,
                Action::Accelerate | Action::Brake | Action::SteerLeft | Action::SteerRight
            )
        }) {
            pause.release_required = false;
        }
        if window.focused
            && !pause.release_required
            && active_actions(&bindings, Context::Paused, |key| keys.just_pressed(key))
                .contains(&Action::Resume)
        {
            pause.paused = false;
        }
    }
    if pause.paused != was_paused {
        window.title = if pause.paused {
            format!(
                "RoadRash Rust — paused (resume: {:?})",
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
    mut exit: MessageWriter<AppExit>,
) {
    previous.0 = race.0.snapshot();
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
        view.2 = if pause.paused {
            1.
        } else {
            time.overstep_fraction()
        };
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
            simulation_schema: SIMULATION_SCHEMA,
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
    if *frame == 30
        && let Some(path) = &options.capture
    {
        let metadata = serde_json::json!({"requested_frame":*frame,"simulation_tick":view.0.tick,"interpolation_alpha":view.2,"window_size":options.window_size.unwrap_or((960,720)),"camera":course.as_ref().map(|c|&c.camera),"source_start_q8":course.as_ref().map(|c|c.start_q8),"course":course.as_ref().map(|c|&c.scene.options.course),"level":options.level,"branch_mask":options.branch_mask,"texture_override_active":options.enhanced ^ options.switch_tick.is_some_and(|t|view.0.tick>=t)});
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
            .add_systems(Update, sample_input);
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
            .add_systems(Update, pause_input);
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
    }
}
