//! Composition root: ordered input/fixed simulation/snapshot adapter and optional deterministic playback.
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use rr_presentation::{
    ClassicPresentationPlugin, PresentationStage, PresentedSnapshot, VisualAssets,
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
struct Previous(rr_simulation::Snapshot);
fn options() -> Result<LaunchOptions, String> {
    let mut options = LaunchOptions {
        render_rate: "60".into(),
        ..Default::default()
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {}", arg.to_string_lossy()))?;
        match arg.to_str() {
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
                println!(
                    "Validated install: {} files, generation {}",
                    install.files, install.generation
                );
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
            .find(|e| e.kind == rr_content::Kind::Rider && !e.preview_variants.is_empty())
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
    App::new()
        .insert_resource(visual)
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
                        title: "RoadRash Rust — classic foundation".into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(Time::<Fixed>::from_hz(f64::from(TICKS_PER_SECOND)))
        .init_resource::<Race>()
        .init_resource::<Previous>()
        .init_resource::<InputCommand>()
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
            Update,
            (interpolate.in_set(PresentationStage::Prepare), smoke_test),
        )
        .run();
}
fn live(driver: Res<Driver>) -> bool {
    driver.0.is_none()
}
fn sample_input(keys: Res<ButtonInput<KeyCode>>, mut command: ResMut<InputCommand>) {
    let left = keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA);
    let right = keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD);
    command.0 = Command {
        throttle: keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW),
        brake: keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS),
        steer: i8::from(right) - i8::from(left),
        ..Default::default()
    };
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
fn interpolate(driver: Res<Driver>, time: Res<Time<Fixed>>, mut view: ResMut<PresentedSnapshot>) {
    if driver.0.is_none() {
        view.2 = time.overstep_fraction();
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
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    *frame += 1;
    if *frame == 30
        && let Some(path) = &options.capture
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    }
    if options.smoke_frames.is_some_and(|limit| *frame >= limit) {
        println!("foundation smoke completed after {} frames", *frame);
        exit.write(AppExit::Success);
    }
}
