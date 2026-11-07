//! Opt-in private acceptance telemetry. Simulation never reads these observations.
use super::*;
use bevy::{
    diagnostic::DiagnosticsStore,
    render::{
        Render, RenderApp, RenderSystems, diagnostic::RenderDiagnosticsPlugin,
        render_asset::RenderAssets, renderer::RenderAdapterInfo, texture::GpuImage,
    },
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Instant,
};
const MAX_TIMING_SAMPLES: usize = 500_000;

#[derive(Resource)]
pub(super) struct LaunchStart(pub Instant);
#[derive(Default)]
struct GpuData {
    images: usize,
    bytes: u64,
    adapter: String,
}
#[derive(Resource, Clone, Default)]
struct Gpu(Arc<Mutex<GpuData>>);
#[derive(Default)]
struct Segment {
    frames: u64,
    wall: Vec<f64>,
    cpu: Vec<f64>,
    prepare: Vec<f64>,
    upload: Vec<f64>,
    diagnostics: BTreeMap<String, Vec<f64>>,
}
#[derive(Resource, Default)]
struct Metrics {
    segments: BTreeMap<String, Segment>,
    window: Option<serde_json::Value>,
    decoded_pixels: BTreeMap<String, serde_json::Value>,
    tour_index: usize,
    cooldown: u32,
    last_memory_second: u64,
    last_tick: u64,
    last_seed: Option<u32>,
    live_ticks: u64,
    paused_frames: u64,
    loads: Vec<serde_json::Value>,
    frames: u64,
    wall_ms: Vec<f64>,
    cpu_ms: Vec<f64>,
    prepare_ms: Vec<f64>,
    upload_ms: Vec<f64>,
    diagnostics: BTreeMap<String, Vec<f64>>,
    last: BTreeMap<String, String>,
    samples: Vec<serde_json::Value>,
    first_frame_ms: Option<f64>,
    restarts: u32,
    pilot_calls: u64,
    pilot_cpu_ms: Vec<f64>,
    driver_cpu_ms: Vec<f64>,
    driver: acceptance::Driver,
    config: Option<(
        rr_simulation::riding::Configuration,
        rr_simulation::encounter::Setup,
    )>,
}
pub(super) struct PerformancePlugin;
impl Plugin for PerformancePlugin {
    fn build(&self, app: &mut App) {
        let gpu = Gpu::default();
        if app.world().resource::<LaunchOptions>().benchmark_pilot {
            app.insert_resource(bevy::winit::WinitSettings {
                focused_mode: bevy::winit::UpdateMode::Continuous,
                unfocused_mode: bevy::winit::UpdateMode::Continuous,
            });
        }
        app.insert_resource(gpu.clone())
            .init_resource::<Metrics>()
            .add_plugins(RenderDiagnosticsPlugin)
            .add_systems(Update, tour.before(PresentationStage::Prepare))
            .add_systems(Update, sample.after(PresentationStage::Draw))
            .add_systems(Last, report);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(gpu)
                .add_systems(Render, gpu_memory.after(RenderSystems::PrepareAssets));
        }
    }
}
fn gpu_memory(assets: Res<RenderAssets<GpuImage>>, info: Res<RenderAdapterInfo>, gpu: Res<Gpu>) {
    let mut data = gpu.0.lock().unwrap_or_else(|e| e.into_inner());
    data.images = 0;
    data.bytes = 0;
    for (_, image) in assets.iter() {
        data.images += 1;
        let d = &image.texture_descriptor;
        let bytes = u64::from(d.format.block_copy_size(None).unwrap_or(4));
        for mip in 0..d.mip_level_count {
            data.bytes += u64::from((d.size.width >> mip).max(1))
                * u64::from((d.size.height >> mip).max(1))
                * u64::from(d.size.depth_or_array_layers)
                * bytes;
        }
    }
    if data.adapter.is_empty() {
        data.adapter = format!("{:?}", **info);
    }
}
fn sample(world: &mut World) {
    world.resource_scope(|world,mut metrics:Mut<Metrics>| {
        metrics.frames+=1;
        if metrics.window.is_none() {
            metrics.window=world.query_filtered::<&Window,With<bevy::window::PrimaryWindow>>()
                .iter(world).next().map(|w|serde_json::json!({"logical":[w.width(),w.height()],"physical":[w.physical_width(),w.physical_height()],"scale_factor":w.scale_factor()}));
        }
        let course=world.resource::<LaunchOptions>().course.clone().unwrap_or_else(||"authored".into());
        let key=if world.resource::<LaunchOptions>().benchmark_tour {format!("{course}/lap{}",metrics.tour_index/5+1)}else{course};
        let race=world.resource::<Race>();let tick=race.0.tick().0;let seed=race.0.initial_seed();
        metrics.live_ticks += if metrics.last_seed==Some(seed) {tick.saturating_sub(metrics.last_tick)}else{tick};
        metrics.last_tick=tick;metrics.last_seed=Some(seed);
        if world.resource::<PauseState>().paused {metrics.paused_frames+=1;}
        if !metrics.decoded_pixels.contains_key(&key)
            && let Some(source)=world.get_resource::<CoursePresentation>() {
            metrics.decoded_pixels.insert(key.clone(),serde_json::to_value(source.scene.assets.pixel_residency()).unwrap());
        }
        metrics.segments.entry(key.clone()).or_default().frames += 1;
        let wall=world.resource::<Time<Real>>().delta().as_secs_f64()*1000.;
        let elapsed=world.resource::<LaunchStart>().0.elapsed().as_secs_f64();
        if metrics.first_frame_ms.is_none() {metrics.first_frame_ms=Some(elapsed*1000.);}
        let settled = metrics.segments[&key].frames > 60 && metrics.cooldown == 0;
        metrics.cooldown = metrics.cooldown.saturating_sub(1);
        // Each course owns its timing coverage. A full aggregate must not starve
        // a later course; aggregate summaries retain their separate bounded prefix.
        let aggregate = metrics.wall_ms.len() < MAX_TIMING_SAMPLES;
        if settled && metrics.segments[&key].wall.len() < MAX_TIMING_SAMPLES {
            if aggregate { metrics.wall_ms.push(wall); }
            metrics.segments.get_mut(&key).unwrap().wall.push(wall);
            if let Some(timing)=world.get_resource::<rr_presentation::RenderTiming>() {
                if aggregate {
                    metrics.cpu_ms.push(timing.cpu_draw_ms);
                    metrics.prepare_ms.push(timing.cpu_prepare_ms);
                    metrics.upload_ms.push(timing.cpu_upload_ms);
                }
                let segment = metrics.segments.get_mut(&key).unwrap();
                segment.cpu.push(timing.cpu_draw_ms);
                segment.prepare.push(timing.cpu_prepare_ms);
                segment.upload.push(timing.cpu_upload_ms);
            }
            if let Some(store)=world.get_resource::<DiagnosticsStore>() {
                for diagnostic in store.iter() {
                    if let Some(measurement)=diagnostic.measurement() {
                        let path=diagnostic.path().to_string();let stamp=format!("{:?}",measurement.time);
                        if metrics.last.get(&path)!=Some(&stamp) {
                            metrics.last.insert(path.clone(),stamp);
                            metrics.segments.get_mut(&key).unwrap().diagnostics.entry(path.clone()).or_default().push(measurement.value);
                            if aggregate { metrics.diagnostics.entry(path).or_default().push(measurement.value); }
                        }
                    }
                }
            }
        }
        if elapsed as u64 > metrics.last_memory_second && metrics.samples.len()<7200 {
            metrics.last_memory_second = elapsed as u64;
            let rss=std::fs::read_to_string("/proc/self/status").ok().and_then(|s|s.lines().find(|l|l.starts_with("VmRSS:")).map(str::to_string));
            let images=world.get_resource::<Assets<Image>>().map_or(0,Assets::len);
            let gpu=world.resource::<Gpu>().0.lock().unwrap_or_else(|e|e.into_inner());
            let restarts=metrics.restarts;
            let live_ticks=metrics.live_ticks;
            metrics.samples.push(serde_json::json!({"elapsed_s":elapsed,"course":key,"simulation_tick":tick,"live_ticks":live_ticks,"entities":world.entities().len(),"cpu_images":images,
                "gpu_images":gpu.images,"gpu_image_bytes":gpu.bytes,"rss":rss,"restarts":restarts}));
        }
        if world.resource::<LaunchOptions>().benchmark_seconds.is_some_and(|seconds| elapsed>=f64::from(seconds)) {
            world.write_message(AppExit::Success);
        }
    });
}
fn tour(world: &mut World) {
    let options = world.resource::<LaunchOptions>();
    if !options.benchmark_tour {
        return;
    }
    let elapsed = world.resource::<LaunchStart>().0.elapsed().as_secs_f64();
    let segment = f64::from(options.benchmark_seconds.unwrap()) / 10.;
    let index = ((elapsed / segment) as usize).min(9);
    if index == world.resource::<Metrics>().tour_index {
        return;
    }
    let started = Instant::now();
    let mut launch = options.clone();
    launch.course = Some(rr_session::catalog::COURSES[index % 5].0.into());
    launch.branch_mask = 0;
    let course = match course_view(&launch, world.resource::<VisualAssets>()) {
        Ok(Some(course)) => course,
        _ => {
            world.write_message(AppExit::error());
            return;
        }
    };
    let seed = world.resource::<Metrics>().restarts + 2;
    let race = build_race(&launch, Some(&course), seed, None);
    world.insert_resource(course);
    world.insert_resource(race);
    *world.resource_mut::<LaunchOptions>() = launch;
    world.resource_mut::<Previous>().0 = Default::default();
    world.resource_mut::<frontend::Shell>().session = None;
    let mut metrics = world.resource_mut::<Metrics>();
    metrics.tour_index = index;
    metrics.cooldown = 60;
    metrics.restarts += 1;
    metrics.config = None;
    metrics.driver = acceptance::Driver::default();
    metrics.loads.push(serde_json::json!({"course_index":index,"elapsed_s":elapsed,"load_ms":started.elapsed().as_secs_f64()*1000.}));
}
pub(super) fn pilot(world: &mut World) {
    if !world.resource::<LaunchOptions>().benchmark_pilot {
        return;
    }
    let started = Instant::now();
    world.resource_scope(|world, mut metrics: Mut<Metrics>| {
        let snapshot = world.resource::<Race>().0.snapshot();
        let ended = snapshot.actors.first().is_some_and(|a| {
            a.finished
                || a.encounter
                    .as_ref()
                    .is_some_and(|u| u.recovery.outcome.is_some())
        });
        if ended {
            let options = world.resource::<LaunchOptions>();
            let race = build_race(
                options,
                world.get_resource::<CoursePresentation>(),
                metrics.restarts + 2,
                None,
            );
            world.insert_resource(race);
            world.resource_mut::<frontend::Shell>().session = None;
            world.resource_mut::<Previous>().0 = Default::default();
            metrics.restarts += 1;
            metrics.config = None;
            metrics.driver = acceptance::Driver::default();
        }
        if metrics.config.is_none() {
            let race = world.resource::<Race>();
            if let Ok(record) = race.0.recording_setup(race.0.initial_seed())
                && let (Some(c), Some(e)) = (record.handling, record.encounter)
            {
                metrics.config = Some((c, e));
            }
        }
        let Metrics {
            config,
            driver,
            driver_cpu_ms,
            ..
        } = &mut *metrics;
        if let Some((c, e)) = config {
            let snapshot = world.resource::<Race>().0.snapshot();
            let command_started = Instant::now();
            world.resource_mut::<InputCommand>().0 = driver.command(&snapshot, c, e);
            if driver_cpu_ms.len() < MAX_TIMING_SAMPLES {
                driver_cpu_ms.push(command_started.elapsed().as_secs_f64() * 1000.);
            }
        }
        metrics.pilot_calls += 1;
        if metrics.pilot_cpu_ms.len() < MAX_TIMING_SAMPLES {
            metrics
                .pilot_cpu_ms
                .push(started.elapsed().as_secs_f64() * 1000.);
        }
    });
}
fn distribution(values: &[f64]) -> serde_json::Value {
    let mut v: Vec<_> = values.iter().copied().filter(|v| v.is_finite()).collect();
    v.sort_by(f64::total_cmp);
    if v.is_empty() {
        return serde_json::Value::Null;
    }
    let percentile = |p: f64| v[((v.len() - 1) as f64 * p).ceil() as usize];
    serde_json::json!({"count":v.len(),"p50":percentile(0.5),"p95":percentile(0.95),"p99":percentile(0.99),"max":v.last()})
}
fn report(
    mut exit: MessageReader<AppExit>,
    options: Res<LaunchOptions>,
    metrics: Res<Metrics>,
    gpu: Res<Gpu>,
    start: Res<LaunchStart>,
) {
    if exit.read().next().is_none() {
        return;
    }
    let Some(path) = &options.performance_report else {
        return;
    };
    let gpu = gpu.0.lock().unwrap_or_else(|e| e.into_inner());
    let diagnostics: BTreeMap<_, _> = metrics
        .diagnostics
        .iter()
        .map(|(key, value)| (key, distribution(value)))
        .collect();
    let binary = std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| rr_content::hash(&b));
    let segments: BTreeMap<_,_> = metrics.segments.iter().map(|(key, s)| {
        let gpu: BTreeMap<_,_>=s.diagnostics.iter().map(|(k,v)|(k,distribution(v))).collect();
        (key,serde_json::json!({"frames":s.frames,"warmup_frames_excluded":60,
            "timing_samples_truncated": s.wall.len() < s.frames.saturating_sub(60) as usize,
            "wall_frame_ms":distribution(&s.wall),"cpu_raster_ms":distribution(&s.cpu),
            "cpu_course_prepare_ms":distribution(&s.prepare),"cpu_surface_upload_ms":distribution(&s.upload),"render_diagnostics":gpu}))
    }).collect();
    let mut report = serde_json::json!({"schema_version":2,"actual_window":metrics.window,"decoded_texture_bytes_by_role":metrics.decoded_pixels,"background_benchmark_continuous":options.benchmark_pilot,"course_segments":segments,"course_loads":metrics.loads,"tour":options.benchmark_tour,"tour_laps":if options.benchmark_tour {2}else{0},"binary_sha256":binary,"native_only":!options.enhanced,
        "vsync":options.vsync,"course":options.course,"level":options.level,"bike":options.bike,"seed_policy":"initial seed 1; restart seeds increment",
        "duration_s":start.0.elapsed().as_secs_f64(),"first_frame_ms":metrics.first_frame_ms,"frames":metrics.frames,
        "warmup_frames_excluded":60,"timing_sample_limit_per_segment":MAX_TIMING_SAMPLES,"aggregate_timing_sample_limit":MAX_TIMING_SAMPLES,
        "aggregate_timing_samples_truncated": metrics.segments.values().map(|s|s.frames.saturating_sub(60)).sum::<u64>() > metrics.wall_ms.len() as u64,
        "wall_frame_ms":distribution(&metrics.wall_ms),"cpu_raster_ms":distribution(&metrics.cpu_ms),
        "render_diagnostics":diagnostics,"gpu_adapter":gpu.adapter,"gpu_image_bytes_scope":"wgpu texture descriptors and mip extents; driver allocations, nonimage buffers and compressed block rounding excluded",
        "memory_samples":metrics.samples,"race_and_course_restarts":metrics.restarts,"live_simulation_ticks":metrics.live_ticks,"paused_frames":metrics.paused_frames,"simulation_seconds":metrics.live_ticks as f64/60.});
    let observer = serde_json::json!({
        "benchmark_pilot_policy": if !options.benchmark_pilot { "disabled" } else if std::env::var_os("RR_DRIVER_LEGACY").is_some() { "legacy proportional controls (RR_DRIVER_LEGACY)" } else { "default cached route planner with 60-tick prediction" },
        "benchmark_pilot_cpu_scope":"Elapsed CPU time around each FixedUpdate pilot callback, including snapshot/config/restart work. Driver::command is a nested subset; do not add its percentile to whole callback. Includes thread descheduling, excludes simulation advance and raster.",
        "benchmark_pilot_calls":metrics.pilot_calls,"benchmark_pilot_cpu_ms":distribution(&metrics.pilot_cpu_ms),"benchmark_driver_command_cpu_ms":distribution(&metrics.driver_cpu_ms),
        "benchmark_pilot_cpu_sample_limit":MAX_TIMING_SAMPLES,"benchmark_pilot_cpu_samples_truncated":metrics.pilot_calls > metrics.pilot_cpu_ms.len() as u64,
        "cpu_course_prepare_ms":distribution(&metrics.prepare_ms),"cpu_surface_upload_ms":distribution(&metrics.upload_ms),
        "cpu_course_prepare_scope":"Course draw system scene selection, camera and owned snapshot preparation before draw_parallel; excludes other Bevy/application systems.",
        "cpu_surface_upload_scope":"Direct main-world upload_rgba asset update only; excludes Bevy render-asset extraction, GPU transfers, sprite retargeting and post-draw frame cleanup."
    });
    report
        .as_object_mut()
        .expect("performance report object")
        .extend(
            observer
                .as_object()
                .expect("observer timing object")
                .clone(),
        );
    let result = (|| -> Result<(), String> {
        let path = rr_session::persistence::disk::guarded(path, options.data_root.as_deref())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .and_then(|mut file| file.write_all(&serde_json::to_vec_pretty(&report).unwrap()))
            .map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        eprintln!("Performance report: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_course_keeps_timing_coverage_after_aggregate_fills() {
        let mut world = World::new();
        world.insert_resource(LaunchOptions {
            course: Some("CANYN".into()),
            benchmark_tour: true,
            ..Default::default()
        });
        world.insert_resource(LaunchStart(Instant::now()));
        world.insert_resource(Time::<Real>::default());
        world.insert_resource(Race::default());
        world.insert_resource(PauseState::default());
        let mut metrics = Metrics {
            tour_index: 9,
            wall_ms: vec![1.; MAX_TIMING_SAMPLES],
            ..Default::default()
        };
        metrics.segments.insert(
            "CANYN/lap2".into(),
            Segment {
                frames: 60,
                ..Default::default()
            },
        );
        world.insert_resource(metrics);
        sample(&mut world);
        let metrics = world.resource::<Metrics>();
        assert_eq!(metrics.wall_ms.len(), MAX_TIMING_SAMPLES);
        assert_eq!(metrics.segments["CANYN/lap2"].wall.len(), 1);
        assert_eq!(metrics.segments["CANYN/lap2"].frames, 61);
    }
}
