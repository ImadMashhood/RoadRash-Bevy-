use super::*;
fn app() -> App {
    let mut app = App::new();
    let options = LaunchOptions {
        frontend: Some(true),
        condition_percent: 100,
        effects: true,
        career_file: std::env::temp_dir().join(format!(
            "rr-career-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )),
        ..Default::default()
    };
    let race = Race::default();
    let shell = Shell::new(&options, &race.0, false);
    app.insert_resource(shell)
        .insert_resource(options)
        .insert_resource(race)
        .init_resource::<PauseState>()
        .init_resource::<Previous>()
        .init_resource::<InputCommand>()
        .init_resource::<PresentedSnapshot>()
        .init_resource::<VisualAssets>()
        .init_resource::<super::super::LiveInput>()
        .init_resource::<super::super::Bindings>()
        .init_resource::<super::super::race_recording::Recorder>();
    app
}
fn start(app: &mut App, mode: Mode, course: u8) {
    apply(app.world_mut(), Act::Begin(mode));
    apply(app.world_mut(), Act::Character(1));
    if mode == Mode::BigGame {
        apply(app.world_mut(), Act::Open(Screen::Courses));
    }
    apply(app.world_mut(), Act::Course(course));
    coordinate(app.world_mut());
}
#[test]
fn actual_bevy_adapter_countdown_pause_restart_and_cleanup_are_bounded() {
    let mut app = app();
    start(&mut app, Mode::Thrash, 1);
    let initial = app.world().resource::<Race>().0.state_hash();
    for i in 0..12 {
        let count = {
            let mut shell = app.world_mut().resource_mut::<Shell>();
            let session = shell.session.as_mut().unwrap();
            for _ in 0..30 {
                assert!(!session.permit_tick());
            }
            session.phase()
        };
        assert!(matches!(count, Phase::Countdown { remaining: 150 }));
        apply(app.world_mut(), Act::Open(Screen::Paused));
        let phase = app
            .world()
            .resource::<Shell>()
            .session
            .as_ref()
            .unwrap()
            .phase();
        for _ in 0..10 {
            coordinate(app.world_mut());
        }
        assert_eq!(
            app.world()
                .resource::<Shell>()
                .session
                .as_ref()
                .unwrap()
                .phase(),
            phase
        );
        assert_eq!(app.world().resource::<Race>().0.state_hash(), initial);
        apply(app.world_mut(), Act::Restart);
        coordinate(app.world_mut());
        assert_eq!(app.world().resource::<Race>().0.state_hash(), initial);
        assert_eq!(app.world().resource::<Race>().0.snapshot().actors.len(), 1);
        assert_eq!(
            app.world()
                .resource::<Shell>()
                .session
                .as_ref()
                .unwrap()
                .phase(),
            Phase::Countdown { remaining: 180 }
        );
        assert_eq!(app.world().resource::<Shell>().frontend.stack_depth(), 0);
        assert!(i < 12);
    }
    apply(app.world_mut(), Act::Main);
    assert!(app.world().resource::<Shell>().session.is_none());
    assert!(
        app.world()
            .resource::<Race>()
            .0
            .snapshot()
            .actors
            .is_empty()
    );
    assert!(
        app.world()
            .resource::<PresentedSnapshot>()
            .0
            .actors
            .is_empty()
    );
    assert!(!app.world().contains_resource::<CoursePresentation>());
}
#[test]
fn every_terminal_outcome_uses_real_adapter_and_results_do_not_hold_world_references() {
    for terminal in [
        rr_simulation::recovery::Outcome::Finished,
        rr_simulation::recovery::Outcome::Wrecked,
        rr_simulation::recovery::Outcome::Arrested,
    ] {
        let mut app = app();
        start(&mut app, Mode::Thrash, 1);
        let replay = rr_simulation::replay::Replay::read(include_bytes!(
            "../../../../docs/fixtures/encounter-replay.json"
        ))
        .unwrap();
        let mut sim = rr_simulation::replay::Playback::new(replay)
            .unwrap()
            .simulation;
        let front = app.world().resource::<Shell>().frontend.clone();
        let cat = app.world().resource::<Shell>().catalog.clone();
        let mut session = Shell::make_session(&front, &cat, &sim, 1);
        for _ in 0..180 {
            session.permit_tick();
        }
        sim.step(rr_simulation::Command {
            terminal: Some(terminal),
            ..Default::default()
        })
        .unwrap();
        session.observe(&sim.snapshot());
        app.world_mut().resource_mut::<Race>().0 = sim;
        app.world_mut().resource_mut::<Shell>().session = Some(session);
        coordinate(app.world_mut());
        assert_eq!(
            app.world().resource::<Shell>().frontend.screen,
            Screen::Results
        );
        let frozen = app.world().resource::<Shell>().frontend.summary.clone();
        apply(app.world_mut(), Act::Open(Screen::Standings));
        coordinate(app.world_mut());
        assert_eq!(app.world().resource::<Shell>().frontend.summary, frozen);
        apply(app.world_mut(), Act::Continue);
        assert_eq!(
            app.world().resource::<Shell>().frontend.screen,
            Screen::Courses
        );
        assert!(
            app.world()
                .resource::<Race>()
                .0
                .snapshot()
                .actors
                .is_empty()
        );
        assert_eq!(app.world().resource::<Shell>().frontend.summary, frozen);
    }
}
#[test]
fn save_load_adapter_preserves_big_game_and_rejects_source_overlap() {
    let mut app = app();
    apply(app.world_mut(), Act::Begin(Mode::BigGame));
    apply(app.world_mut(), Act::Character(2));
    apply(app.world_mut(), Act::Save);
    let path = app.world().resource::<LaunchOptions>().career_file.clone();
    assert!(path.exists());
    let frozen = app.world().resource::<Shell>().frontend.career.clone();
    app.world_mut().resource_mut::<Shell>().frontend.career = None;
    apply(app.world_mut(), Act::Load);
    assert_eq!(app.world().resource::<Shell>().frontend.career, frozen);
    std::fs::remove_file(&path).unwrap();
    let dir = path.with_extension("source-dir");
    std::fs::create_dir_all(&dir).unwrap();
    app.world_mut().resource_mut::<LaunchOptions>().data_root = Some(dir.clone());
    app.world_mut().resource_mut::<LaunchOptions>().career_file = dir.join("nested/career.json");
    apply(app.world_mut(), Act::Save);
    assert!(!dir.join("nested").exists());
    assert!(app.world().resource::<Shell>().notice.contains("overlaps"));
    std::fs::remove_dir(dir).unwrap();
}
#[test]
fn quit_confirmation_cancel_returns_to_pause_and_neutral_resume() {
    let mut app = app();
    start(&mut app, Mode::Thrash, 1);
    apply(app.world_mut(), Act::Open(Screen::Paused));
    apply(app.world_mut(), Act::Open(Screen::QuitConfirm));
    apply(app.world_mut(), Act::Back);
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Paused
    );
    app.world_mut()
        .resource_mut::<super::super::LiveInput>()
        .0
        .drive
        .throttle = true;
    apply(app.world_mut(), Act::Resume);
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Paused
    );
    assert!(
        app.world()
            .resource::<Shell>()
            .notice
            .contains("Release driving controls")
    );
    app.world_mut()
        .resource_mut::<super::super::LiveInput>()
        .0
        .drive
        .throttle = false;
    apply(app.world_mut(), Act::Resume);
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Racing
    );
    assert!(!app.world().resource::<PauseState>().paused);
    assert!(app.world().resource::<Shell>().notice.is_empty());
    apply(app.world_mut(), Act::Open(Screen::Paused));
    assert!(app.world().resource::<Shell>().notice.is_empty());
}

#[test]
fn quote_information_does_not_spend_and_notices_follow_the_current_context() {
    let mut app = app();
    apply(app.world_mut(), Act::Begin(Mode::BigGame));
    apply(app.world_mut(), Act::Character(1));
    apply(app.world_mut(), Act::Open(Screen::Showroom));
    app.world_mut().resource_mut::<Shell>().notice = "Old bike specifications".into();
    apply(app.world_mut(), Act::Bike(1));
    assert!(app.world().resource::<Shell>().notice.is_empty());
    let before = app.world().resource::<Shell>().frontend.career.clone();
    apply(app.world_mut(), Act::Open(Screen::Purchase));
    let shell = app.world().resource::<Shell>();
    assert!(shell.notice.contains("Trade-in:") && shell.notice.contains("Cash available:"));
    assert_eq!(shell.frontend.career, before);
    apply(app.world_mut(), Act::Back);
    assert!(app.world().resource::<Shell>().notice.is_empty());
    apply(app.world_mut(), Act::Back);
    apply(app.world_mut(), Act::Open(Screen::Repair));
    let shell = app.world().resource::<Shell>();
    assert!(shell.notice.contains("Repair cost:") && shell.notice.contains("Cash available:"));
    assert_eq!(shell.frontend.career, before);
}
#[test]
fn source_factory_repeated_courses_bikes_and_carried_damage_drop_all_scene_owners() {
    let (Ok(data), Ok(catalog)) = (
        std::env::var("RR_SESSION_DATA"),
        std::env::var("RR_SESSION_CATALOG"),
    ) else {
        return;
    };
    let native_catalog = Catalog::from_exe(
        &std::fs::read(std::path::Path::new(&data).join("ROADRASH.EXE")).unwrap(),
    )
    .unwrap();
    let mut measurements = vec![];
    for (stem, _) in COURSES {
        let options = LaunchOptions {
            data_root: Some(data.clone().into()),
            catalog: Some(catalog.clone().into()),
            course: Some(stem.into()),
            level: 1,
            bike: 1,
            opponents: 14,
            condition_percent: 50,
            effects: true,
            ..Default::default()
        };
        let course = super::super::course_view(&options, &VisualAssets::default())
            .unwrap()
            .unwrap();
        let owner = std::sync::Arc::downgrade(&course.scene);
        for bike in [1, 8, 15] {
            let mut options = options.clone();
            options.bike = bike;
            let first = super::super::build_race(&options, Some(&course), 1, Some(1));
            let second = super::super::build_race(&options, Some(&course), 1, Some(1));
            assert_eq!(first.0.state_hash(), second.0.state_hash());
            assert_eq!(first.0.snapshot().actors.len(), 15);
            measurements.push(serde_json::json!({"course":stem,"bike":bike,"actors":15,"initial_hash":first.0.state_hash(),"same_seed_hash_matches":first.0.state_hash()==second.0.state_hash(),"scene_owners":std::sync::Arc::strong_count(&course.scene),"rss_status":std::fs::read_to_string("/proc/self/status").ok().and_then(|s|s.lines().find(|l|l.starts_with("VmRSS:")).map(str::to_string))}));
            let setup = first.0.recording_setup(1).unwrap();
            let config = setup.handling.unwrap();
            let full = config
                .specs
                .iter()
                .find(|s| s.id == bike)
                .unwrap()
                .condition;
            let snapshot = first.0.snapshot();
            assert_eq!(
                snapshot.actors[0].rider.as_ref().unwrap().bike.condition,
                (full / 2).max(1)
            );
            let mut front = Frontend::default();
            front.mode = Mode::BigGame;
            front.bike = (bike - 1) as u8;
            let mut session = Shell::make_session(&front, &native_catalog, &first.0, 1);
            assert_eq!(session.condition_capacity(), full);
            session.quit(&snapshot);
            assert_eq!(
                session.summary().unwrap().bike_condition(),
                ((full / 2).max(1) * 100 / full).clamp(0, 100)
            );
            for a in snapshot.actors.iter().skip(1) {
                let traits = &setup.population.as_ref().unwrap().brains[&a.id];
                let original = config.specs.iter().find(|s| s.id == traits.bike).unwrap();
                assert_eq!(a.rider.as_ref().unwrap().bike.condition, original.condition);
            }
        }
        if stem == "CITY" {
            let exe = std::fs::read(std::path::Path::new(&data).join("ROADRASH.EXE")).unwrap();
            let catalog = Catalog::from_exe(&exe).unwrap();
            for c in &catalog.characters {
                let mut options = options.clone();
                options.bike = u32::from(c.bikes[0]) + 1;
                let race = super::super::build_race(&options, Some(&course), 1, Some(c.id));
                let u = &race.0.encounters().unwrap().units[&ActorId(1)];
                assert_eq!(u.color, c.personality);
                assert!(u.fighter.stats.maximum > 0);
            }
        }
        drop(course);
        assert!(owner.upgrade().is_none());
    }
    if let Ok(path) = std::env::var("RR_SESSION_METRICS") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"outcome":"passed","measurements":measurements,"all_scene_weak_owners_expired":true,"source_character_options":8})).unwrap()).unwrap();
    }
}
#[test]
fn returning_to_menu_stops_owned_siren_without_audio_device() {
    let mut app = app();
    use rr_simulation::{population::*, topology::*};
    let mut world = rr_simulation::population::World::new(Setup::default());
    let id = world
        .spawn(
            &Spawn {
                id: 1,
                location: Location {
                    progress: 1000,
                    ..Default::default()
                },
                kind: Kind::Police,
                family: None,
                selector: 0,
                subtype: 0,
                speed: 0,
                direction: 1,
            },
            0,
            &Topology::straight(10000),
        )
        .unwrap()
        .unwrap();
    world.agents.get_mut(&id).unwrap().siren = true;
    app.world_mut()
        .resource_mut::<PresentedSnapshot>()
        .0
        .road_users = world.agents.into_values().collect();
    app.init_resource::<super::super::population_cues::Warnings>();
    app.add_systems(Update, super::super::population_cues::update);
    app.update();
    assert_eq!(
        app.world()
            .resource::<super::super::population_cues::Warnings>()
            .events,
        vec![Cue::SirenStart(id)]
    );
    apply(app.world_mut(), Act::Main);
    app.update();
    assert_eq!(
        app.world()
            .resource::<super::super::population_cues::Warnings>()
            .events,
        vec![Cue::SirenStop(id)]
    );
}
#[test]
fn restart_flushes_recording_then_rebases_a_new_complete_setup() {
    let mut app = app();
    start(&mut app, Mode::Thrash, 1);
    let path = app
        .world()
        .resource::<LaunchOptions>()
        .career_file
        .with_file_name(format!(
            "record-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    let race = app.world_mut().remove_resource::<Race>().unwrap();
    let recorder = crate::race_recording::Recorder::new(Some(path.clone()), &race.0, None).unwrap();
    app.world_mut().insert_resource(race);
    app.world_mut().insert_resource(recorder);
    for tick in 0..4 {
        let input = rr_simulation::Command {
            throttle: true,
            ..Default::default()
        };
        app.world_mut()
            .resource_mut::<crate::race_recording::Recorder>()
            .command(tick, input);
        app.world_mut()
            .resource_mut::<Race>()
            .0
            .step(input)
            .unwrap();
    }
    let hash = app.world().resource::<Race>().0.state_hash();
    apply(app.world_mut(), Act::Open(Screen::Paused));
    apply(app.world_mut(), Act::Restart);
    coordinate(app.world_mut());
    let replay = rr_simulation::replay::Replay::read_file(&path).unwrap();
    assert_eq!(replay.ticks, 4);
    assert_eq!(
        rr_simulation::replay::run(replay, "144")
            .unwrap()
            .state_sha256,
        hash
    );
    assert_eq!(
        app.world()
            .resource::<crate::race_recording::Recorder>()
            .replay
            .as_ref()
            .unwrap()
            .commands
            .len(),
        0
    );
    std::fs::remove_file(path).unwrap();
}
#[test]
fn real_keyboard_menu_edges_and_return_focus_are_owned_by_the_frontend() {
    let mut app = app();
    app.init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, navigate);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    app.update();
    assert_eq!(app.world().resource::<Shell>().focus, 1);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Characters
    );
    assert_eq!(app.world().resource::<Shell>().focus, 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Main
    );
    assert_eq!(app.world().resource::<Shell>().focus, 0);
}
#[test]
fn slots_save_load_and_explicit_backup_restore_use_validated_records() {
    let mut app = app();
    apply(app.world_mut(), Act::Begin(Mode::BigGame));
    apply(app.world_mut(), Act::Character(1));
    apply(app.world_mut(), Act::Open(Screen::Restroom));
    apply(app.world_mut(), Act::SaveAs);
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Saves
    );
    apply(app.world_mut(), Act::ChooseSlot(1));
    let path = app.world().resource::<LaunchOptions>().career_file.clone();
    assert!(path.exists());
    let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(raw["schema_version"], 2);
    let original = app.world().resource::<Shell>().frontend.career.clone();
    app.world_mut()
        .resource_mut::<Shell>()
        .frontend
        .career
        .as_mut()
        .unwrap()
        .cash = 123;
    apply(app.world_mut(), Act::Save);
    let modified = app.world().resource::<Shell>().frontend.career.clone();
    std::fs::write(&path, b"truncated").unwrap();
    apply(app.world_mut(), Act::Load);
    assert_eq!(app.world().resource::<Shell>().frontend.career, modified);
    apply(app.world_mut(), Act::RestoreBackup);
    assert_eq!(app.world().resource::<Shell>().frontend.career, original);
    assert_eq!(app.world().resource::<Shell>().active_slot, 1);
    let base = path.file_name().unwrap().to_string_lossy().to_string();
    for e in std::fs::read_dir(path.parent().unwrap()).unwrap() {
        let p = e.unwrap().path();
        if p.file_name().unwrap().to_string_lossy().starts_with(&base) {
            std::fs::remove_file(p).unwrap();
        }
    }
}
#[test]
fn preference_adapter_persists_bindings_audio_and_graphics_independently_from_career() {
    let mut app = app();
    let path = app
        .world()
        .resource::<LaunchOptions>()
        .career_file
        .with_file_name(format!(
            "rr-pref-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    app.world_mut()
        .resource_mut::<LaunchOptions>()
        .settings_file = path.clone();
    app.init_resource::<storage::PreferenceState>();
    storage::preferences(app.world_mut());
    let career = app.world().resource::<Shell>().frontend.career.clone();
    app.world_mut()
        .resource_mut::<Shell>()
        .frontend
        .settings
        .music_volume = 37;
    app.world_mut()
        .resource_mut::<Shell>()
        .frontend
        .settings
        .wide = true;
    apply(app.world_mut(), Act::Upscaled);
    app.world_mut()
        .resource_mut::<crate::Bindings>()
        .0
        .rebind(
            rr_input::Context::Race,
            rr_input::Action::Accelerate,
            vec![rr_input::Key::F10],
        )
        .unwrap();
    storage::preferences(app.world_mut());
    let (p, _) =
        rr_session::persistence::preferences::Preferences::decode(&std::fs::read(&path).unwrap())
            .unwrap();
    assert_eq!(p.settings.music_volume, 37);
    assert!(p.settings.wide);
    assert!(p.settings.upscaled);
    assert_eq!(
        p.bindings.contexts[&rr_input::Context::Race][&rr_input::Action::Accelerate],
        vec![rr_input::Key::F10]
    );
    apply(app.world_mut(), Act::ResetGraphics);
    storage::preferences(app.world_mut());
    assert_eq!(app.world().resource::<Shell>().frontend.career, career);
    let (p, _) =
        rr_session::persistence::preferences::Preferences::decode(&std::fs::read(&path).unwrap())
            .unwrap();
    assert!(!p.settings.wide);
    assert!(!p.settings.upscaled);
    assert_eq!(p.settings.music_volume, 37);
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(rr_session::persistence::disk::sidecar(&path, ".bak")).unwrap();
}

#[test]
fn upscaled_menu_switch_updates_live_presentation_without_changing_the_race() {
    let mut app = app();
    start(&mut app, Mode::Thrash, 1);
    apply(app.world_mut(), Act::Open(Screen::Paused));
    apply(app.world_mut(), Act::Open(Screen::Settings));
    let race = app.world().resource::<Race>().0.state_hash();
    assert!(
        menu(app.world().resource::<Shell>())
            .1
            .iter()
            .any(|(label, _)| label == "UPSCALED: OFF")
    );
    app.world_mut().resource_mut::<VisualAssets>().switch_tick = Some(1);
    apply(app.world_mut(), Act::Upscaled);
    let visual = app.world().resource::<VisualAssets>();
    assert!(visual.initially_enhanced);
    assert!(visual.switch_tick.is_none());
    assert!(app.world().resource::<LaunchOptions>().enhanced);
    assert!(
        menu(app.world().resource::<Shell>())
            .1
            .iter()
            .any(|(label, _)| label == "UPSCALED: ON")
    );
    assert_eq!(app.world().resource::<Race>().0.state_hash(), race);
    apply(app.world_mut(), Act::ResetGraphics);
    assert!(!app.world().resource::<VisualAssets>().initially_enhanced);
    assert!(!app.world().resource::<Shell>().frontend.settings.upscaled);
    assert_eq!(app.world().resource::<Race>().0.state_hash(), race);
    apply(app.world_mut(), Act::Back);
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Paused
    );
    apply(app.world_mut(), Act::Resume);
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Racing
    );
    assert!(!app.world().resource::<PauseState>().paused);
}

#[test]
fn keyboard_and_controller_navigate_the_same_menu_actions_without_a_mouse() {
    for controller in [false, true] {
        let mut app = app();
        app.init_resource::<ButtonInput<KeyCode>>();
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.add_systems(Update, navigate);
        // Main -> Options using down, down, confirm.
        for press in [0, 0, 1] {
            if controller {
                app.world_mut()
                    .get_mut::<Gamepad>(pad)
                    .unwrap()
                    .digital_mut()
                    .press(if press == 0 {
                        GamepadButton::DPadDown
                    } else {
                        GamepadButton::South
                    });
            } else {
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(if press == 0 {
                        KeyCode::ArrowDown
                    } else {
                        KeyCode::Enter
                    });
            }
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .reset_all();
            app.world_mut()
                .get_mut::<Gamepad>(pad)
                .unwrap()
                .digital_mut()
                .reset_all();
            app.update();
        }
        assert_eq!(
            app.world().resource::<Shell>().frontend.screen,
            Screen::Restroom
        );
        // Back returns through the actual navigation adapter.
        if controller {
            app.world_mut()
                .get_mut::<Gamepad>(pad)
                .unwrap()
                .digital_mut()
                .press(GamepadButton::East);
        } else {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Escape);
        }
        app.update();
        assert_eq!(
            app.world().resource::<Shell>().frontend.screen,
            Screen::Main
        );
    }
}

#[test]
fn pointer_menu_uses_camera_viewport_at_normal_hidpi_and_letterboxed_sizes() {
    use bevy::camera::{ComputedCameraValues, RenderTargetInfo, Viewport};
    for (physical, scale) in [
        (UVec2::new(640, 480), 1.),
        (UVec2::new(1280, 960), 2.),
        (UVec2::new(1280, 720), 1.),
    ] {
        let mut app = app();
        let factor = (physical.x as f32 / 640.).min(physical.y as f32 / 480.);
        let size = UVec2::new((640. * factor) as u32, (480. * factor) as u32);
        let origin = (physical - size) / 2;
        let mut projection = Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::Fixed {
                width: 640.,
                height: 480.,
            },
            ..OrthographicProjection::default_2d()
        });
        projection.update(size.x as f32, size.y as f32);
        let camera = Camera {
            viewport: Some(Viewport {
                physical_position: origin,
                physical_size: size,
                ..default()
            }),
            computed: ComputedCameraValues {
                clip_from_view: projection.get_clip_from_view(),
                target_info: Some(RenderTargetInfo {
                    physical_size: physical,
                    scale_factor: scale,
                }),
                ..default()
            },
            ..default()
        };
        app.world_mut()
            .spawn((Camera2d, camera, GlobalTransform::default()));
        let mut window = Window {
            resolution: bevy::window::WindowResolution::new(physical.x, physical.y)
                .with_scale_factor_override(scale),
            ..default()
        };
        let cursor = (origin.as_vec2() + Vec2::new(320., 140.) * factor) / scale;
        window.set_cursor_position(Some(cursor));
        app.world_mut().spawn(window);
        app.world_mut().spawn(view::MenuHitbox {
            index: 2,
            screen: Screen::Main,
            center: Vec2::new(0., 100.),
            size: Vec2::new(200., 30.),
        });
        app.init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, input::pointer);
        app.update();
        assert_eq!(app.world().resource::<Shell>().focus, 2);
        app.world_mut().resource_mut::<Shell>().focus = 0;
        app.update();
        assert_eq!(
            app.world().resource::<Shell>().focus,
            0,
            "stationary pointer must not steal keyboard focus"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(
            app.world().resource::<Shell>().frontend.screen,
            Screen::Restroom
        );
        // Old rendered hitboxes cannot activate another screen during a redraw.
        app.update();
        assert_eq!(
            app.world().resource::<Shell>().frontend.screen,
            Screen::Restroom
        );
    }
}

#[test]
fn original_main_menu_hitboxes_preserve_source_slots_and_skip_network_mode() {
    let mut app = app();
    app.insert_resource(original::OriginalUi::default());
    app.add_systems(
        Update,
        |mut commands: Commands, shell: Res<Shell>, ui: Res<original::OriginalUi>| {
            original::main_menu(&mut commands, &shell, &ui);
        },
    );
    app.update();
    let mut cells: Vec<_> = app
        .world_mut()
        .query::<&view::MenuHitbox>()
        .iter(app.world())
        .map(|b| (b.index, b.center.x, b.center.y, b.size))
        .collect();
    cells.sort_by_key(|c| c.0);
    assert_eq!(
        cells,
        vec![
            (0, -240., -210., Vec2::new(160., 60.)),
            (1, -80., -210., Vec2::new(160., 60.)),
            (2, 240., -210., Vec2::new(160., 60.))
        ]
    );
}

#[test]
fn original_restroom_disables_save_without_a_career_visually_and_for_pointer_input() {
    let mut app = app();
    app.world_mut().resource_mut::<Shell>().frontend.screen = Screen::Restroom;
    app.insert_resource(original::OriginalUi::default());
    app.add_systems(
        Update,
        |mut commands: Commands, shell: Res<Shell>, ui: Res<original::OriginalUi>| {
            assert!(original::source_menu(
                &mut commands,
                &shell,
                &ui,
                &menu(&shell).1
            ));
        },
    );
    app.update();
    let world = app.world_mut();
    let indices: Vec<_> = world
        .query::<&view::MenuHitbox>()
        .iter(world)
        .map(|b| b.index)
        .collect();
    assert!(!indices.contains(&1) && !indices.contains(&2));
    assert!(
        world
            .query::<(&Text2d, &TextColor)>()
            .iter(world)
            .any(|(text, color)| text.0 == "Save Game" && color.0 == Color::srgb(0.45, 0.45, 0.45))
    );
}

#[test]
#[ignore = "requires private source data/catalog and verifies all 25 course/level factories"]
fn source_scenery_factories_cover_all_twenty_five_course_levels() {
    let data = std::env::var("RR_SESSION_DATA").expect("RR_SESSION_DATA");
    let catalog = std::env::var("RR_SESSION_CATALOG").expect("RR_SESSION_CATALOG");
    let mut cases = Vec::new();
    for (stem, _) in COURSES {
        let mut options = LaunchOptions {
            data_root: Some(data.clone().into()),
            catalog: Some(catalog.clone().into()),
            course: Some(stem.into()),
            level: 1,
            bike: 1,
            opponents: 14,
            condition_percent: 100,
            effects: true,
            ..Default::default()
        };
        // Decode artwork once per course. Each level still constructs its own
        // scene, placements, actual source factory and authoritative world.
        let base = super::super::course_view(&options, &VisualAssets::default())
            .unwrap()
            .unwrap();
        for level in 1..=5 {
            options.level = level;
            let scene = rr_render::Scene::new(
                base.scene.course.clone(),
                base.scene.assets.clone(),
                rr_render::SceneOptions {
                    course: stem.into(),
                    level,
                    branch_mask: options.branch_mask,
                    seed: 1,
                },
            )
            .unwrap();
            let presentation = rr_presentation::CoursePresentation {
                scene: std::sync::Arc::new(scene),
                camera: base.camera.clone(),
                start_q8: base.start_q8,
            };
            let seed = u32::from(level) + 40;
            let mut simulation =
                super::super::build_race(&options, Some(&presentation), seed, Some(1)).0;
            assert_eq!(simulation.snapshot().actors.len(), 15, "{stem} L{level}");
            let mut recording = simulation.recording_setup(seed).unwrap();
            assert_eq!(recording.handling.as_ref().unwrap().motion_version, 3);
            let setup = serde_json::to_value(recording.encounter.as_ref().unwrap()).unwrap();
            assert_eq!(setup["rules"]["combat_version"], 4, "{stem} L{level}");
            assert_eq!(
                setup["rules"]["body"]["recovery_version"], 2,
                "{stem} L{level}"
            );
            assert!(recording.population.as_ref().unwrap().police.is_some());
            assert_eq!(setup["rules"]["scenery_version"], 1, "{stem} L{level}");
            assert_eq!(setup["rules"]["contact_walk_version"], 2, "{stem} L{level}");
            let profiles = setup["rules"]["source_roads"]["profiles"]
                .as_array()
                .unwrap();
            assert_eq!(
                profiles.len(),
                1 << presentation.scene.course.splits().len()
            );
            for (mask, profile) in profiles.iter().enumerate() {
                let choices = profile["choices"].as_array().unwrap();
                for (bit, choice) in choices.iter().enumerate() {
                    assert_eq!(choice.as_u64().unwrap(), ((mask >> bit) & 1) as u64);
                }
            }
            let obstacles = setup["rules"]["obstacles"].as_array().unwrap();
            let mut classes = std::collections::BTreeMap::<u64, usize>::new();
            for obstacle in obstacles {
                if let Some(source_box) = obstacle.get("source_box") {
                    let class = source_box["scenery"]["class"]
                        .as_u64()
                        .expect("new source boxes must retain their native query class");
                    *classes.entry(class).or_default() += 1;
                }
            }
            let initial = serde_json::to_value(simulation.encounters().unwrap()).unwrap();
            assert!(
                initial["scenery"].is_object(),
                "{stem} L{level} omitted loading state"
            );
            let initial_hash = simulation.state_hash();
            for _ in 0..3 {
                simulation.advance().unwrap();
            }
            recording.ticks = 3;
            let expected = simulation.state_hash();
            recording.expected_hash = Some(expected.clone());
            let bytes = serde_json::to_vec(&recording).unwrap();
            for rate in ["30", "60", "144", "irregular"] {
                let replay = rr_simulation::replay::Replay::read(&bytes).unwrap();
                let result = rr_simulation::replay::run(replay, rate).unwrap();
                assert_eq!(result.state_sha256, expected, "{stem} L{level} {rate}");
            }
            cases.push(serde_json::json!({
                "course": stem, "level": level, "actors": 15,
                "obstacles": obstacles.len(), "source_classes": classes,
                "initial_hash": initial_hash, "final_hash": expected,
                "ticks": 3, "schedules": ["30", "60", "144", "irregular"],
                "recording_bytes": bytes.len(),
                "route_profiles": profiles.len(),
            }));
        }
    }
    assert_eq!(cases.len(), 25);
    assert!(
        cases
            .iter()
            .any(|case| !case["source_classes"].as_object().unwrap().is_empty())
    );
    if let Ok(output) = std::env::var("RR_SCENERY_FACTORY_METRICS") {
        std::fs::write(
            output,
            serde_json::to_vec_pretty(&serde_json::json!({
                "outcome": "passed", "cases": cases,
                "scope": "Real content factories and three ordinary ticks at all course/level pairs; no full-race or career claim",
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
#[ignore = "requires private source data/catalog; inventories native scene cache identities"]
fn source_scenery_native_identity_inventory() {
    let data = std::env::var("RR_SESSION_DATA").expect("RR_SESSION_DATA");
    let catalog = std::env::var("RR_SESSION_CATALOG").expect("RR_SESSION_CATALOG");
    let mut cases = Vec::new();
    for (stem, _) in COURSES {
        let options = LaunchOptions {
            data_root: Some(data.clone().into()),
            catalog: Some(catalog.clone().into()),
            course: Some(stem.into()),
            level: 1,
            bike: 1,
            opponents: 14,
            condition_percent: 100,
            ..Default::default()
        };
        let base = super::super::course_view(&options, &VisualAssets::default())
            .unwrap()
            .unwrap();
        for mask in 0..(1 << base.scene.course.splits().len()) {
            let scene = rr_render::Scene::new(
                base.scene.course.clone(),
                base.scene.assets.clone(),
                rr_render::SceneOptions {
                    course: stem.into(),
                    level: 1,
                    branch_mask: mask,
                    seed: 1,
                },
            )
            .unwrap();
            let mut identities = std::collections::BTreeMap::<
                (u32, u32),
                std::collections::BTreeMap<(usize, i32), u8>,
            >::new();
            let mut duplicate_same_identity = 0usize;
            for (sample, point) in scene.route.points.iter().enumerate() {
                let mut insert = |node, local, offset, road_mask| {
                    let variants = identities.entry((node, local)).or_default();
                    if variants.contains_key(&(sample, offset)) {
                        duplicate_same_identity += 1;
                    }
                    *variants.entry((sample, offset)).or_default() |= road_mask;
                };
                insert(
                    point.node,
                    point.local,
                    point.offset,
                    point.dual.as_ref().map_or(3, |dual| 1 << dual.half),
                );
                if let Some(dual) = &point.dual {
                    insert(dual.node, dual.local, dual.offset, 1 << (1 - dual.half));
                }
            }
            let conflicts: Vec<_> = identities.iter().filter(|(_, variants)| variants.len() > 1)
                .map(|(&(node, local), variants)| serde_json::json!({
                    "node": node, "local": local,
                    "variants": variants.iter().map(|(&(sample, offset), &road_mask)|
                        serde_json::json!({"sample":sample,"offset":offset,"road_mask":road_mask})).collect::<Vec<_>>()
                })).collect();
            for level in 1..=5 {
                let placements = rr_course::scenery::place(&scene.course, level, 1).unwrap();
                let (mut mapped, mut unmapped, mut conflicting) = (0usize, 0usize, 0usize);
                let mut unmapped_paths = std::collections::BTreeMap::<u32, usize>::new();
                for instance in &placements.instances {
                    let category = match instance.kind.as_str() {
                        "roadside" => 0,
                        "billboard" if instance.raw[0] & 0x80000000 == 0 => 4,
                        _ => continue,
                    };
                    let (Some(family), Some(lateral)) = (instance.family, instance.lateral) else {
                        continue;
                    };
                    let Some(bank) = scene.assets.families.get(&family) else {
                        continue;
                    };
                    let Some(boxes) =
                        bank.collision_boxes(category, usize::from(instance.selector & 63))
                    else {
                        continue;
                    };
                    if boxes.is_empty() {
                        continue;
                    }
                    let path = &scene.course.sections[instance.section as usize].samples
                        [instance.sample as usize];
                    if category == 0 && !(lateral > -path.width[0] && lateral < path.width[1]) {
                        continue;
                    }
                    for (node_id, node) in &scene.course.nodes {
                        if node.kind != rr_course::NodeKind::Road
                            || node.link != instance.section
                            || instance.sample >= node.length
                        {
                            continue;
                        }
                        if let Some(variants) = identities.get(&(*node_id, instance.sample)) {
                            mapped += 1;
                            conflicting += usize::from(variants.len() > 1);
                        } else {
                            unmapped += 1;
                            *unmapped_paths.entry(*node_id).or_default() += 1;
                        }
                    }
                }
                cases.push(serde_json::json!({
                    "course":stem,"branch_mask":mask,"level":level,
                    "identity_keys":identities.len(),"duplicate_same_identity":duplicate_same_identity,
                    "conflicting_identity_keys":conflicts.len(),"first_conflicts":&conflicts[..conflicts.len().min(12)],
                    "mapped_physical_instances":mapped,"unmapped_physical_instances":unmapped,
                    "conflicting_physical_instances":conflicting,"unmapped_paths":unmapped_paths,
                }));
            }
        }
    }
    assert_eq!(cases.len(), 130);
    let path = std::env::var("RR_SCENERY_IDENTITY_METRICS").expect("RR_SCENERY_IDENTITY_METRICS");
    std::fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({
        "outcome":"observed", "cases":cases,
        "scope":"Independent selected/Dual cache identity and physical-instance inventory; conflicts and exclusions require resolution, not automatic acceptance",
    })).unwrap()).unwrap();
}

#[test]
#[ignore = "requires private source data/catalog; controlled physical branch render poses"]
fn source_physical_branch_presentation_inventory() {
    use rr_render::camera::{route_encounter_snapshot, route_position, route_snapshot_view};
    let data = std::env::var("RR_SESSION_DATA").unwrap();
    let catalog = std::env::var("RR_SESSION_CATALOG").unwrap();
    let output = std::path::PathBuf::from(std::env::var("RR_BRANCH_RENDER_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut cases = Vec::new();
    for (stem, _) in COURSES {
        let options = LaunchOptions {
            data_root: Some(data.clone().into()),
            catalog: Some(catalog.clone().into()),
            course: Some(stem.into()),
            level: 1,
            bike: 1,
            opponents: 0,
            condition_percent: 100,
            ..Default::default()
        };
        let base = super::super::course_view(&options, &VisualAssets::default())
            .unwrap()
            .unwrap();
        let original = super::super::build_race(&options, Some(&base), 1, Some(1))
            .0
            .snapshot();
        for mask in 0..(1 << base.scene.course.splits().len()) {
            let selected = base.scene.following_mask(mask).unwrap();
            let scene = selected.as_ref().unwrap_or(&base.scene);
            for (index, point) in scene.route.points.iter().enumerate().step_by(257) {
                assert_eq!(
                    route_position(&scene.route, point.node, i64::from(point.local) * 256 + 17),
                    Some(index as i64 * 256 + 17)
                );
            }
            let checkpoints: Vec<_> = scene
                .route
                .transitions
                .iter()
                .map(|transition| transition.index + 2)
                .collect();
            for (checkpoint, index) in checkpoints.into_iter().enumerate() {
                let point = &scene.route.points[index];
                let mut snapshot = original.clone();
                snapshot.actors.retain(|actor| actor.id.0 == 1);
                snapshot.road_users.clear();
                let actor = &mut snapshot.actors[0];
                let location = rr_simulation::topology::Location {
                    path: point.node,
                    progress: i64::from(point.local) * 256 + 128,
                    lateral: point.offset * 256,
                    ..Default::default()
                };
                let unit = actor.encounter.as_mut().unwrap();
                unit.recovery.bike_location = location;
                unit.source_route = Some(mask as u8);
                actor.position.section.0 = point.node;
                actor.rider.as_mut().unwrap().bike.lateral = location.lateral;
                let snapshot = route_encounter_snapshot(&scene.route, &snapshot);
                let camera =
                    route_snapshot_view(&scene.route, &snapshot, &snapshot, 1., base.start_q8);
                assert_eq!(camera.position_q8, (index as f64 - 1.) * 256. + 128.);
                assert_eq!(
                    snapshot.actors[0].position.progress.0,
                    (index as i64 * 256 + 128) * 8
                );
                let (frame, receipt) = scene.draw(&base.camera, &camera, &snapshot, false).unwrap();
                assert!(
                    receipt.rider_bounds.is_some(),
                    "{stem} mask {mask} checkpoint {checkpoint}"
                );
                let filename = format!("{stem}-{mask}-{checkpoint}.png");
                std::fs::write(output.join(&filename), frame.image.png().unwrap()).unwrap();
                cases.push(serde_json::json!({"course":stem,"mask":mask,"checkpoint":checkpoint,"sample":index,"node":point.node,"offset":point.offset,"image":filename,"rider_bounds":receipt.rider_bounds}));
            }
        }
    }
    std::fs::write(output.join("comparison.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "outcome":"passed", "cases":cases,
        "scope":"Controlled real-art physical-coordinate presentation poses at all26 route masks; no driven trajectory or native raster identity claim",
    })).unwrap()).unwrap();
}

#[test]
#[ignore = "requires private native per-sample terrain metadata and original art"]
fn source_native_terrain_inventory() {
    use rr_simulation::handling::Ground;
    let native: serde_json::Value =
        serde_json::from_slice(&std::fs::read(std::env::var("RR_NATIVE_GROUND").unwrap()).unwrap())
            .unwrap();
    let data = std::env::var("RR_SESSION_DATA").unwrap();
    let catalog = std::env::var("RR_SESSION_CATALOG").unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    let mut kinds = std::collections::BTreeMap::<u8, usize>::new();
    for (course_id, (stem, _)) in COURSES.into_iter().enumerate() {
        let options = LaunchOptions {
            data_root: Some(data.clone().into()),
            catalog: Some(catalog.clone().into()),
            course: Some(stem.into()),
            level: 1,
            bike: 1,
            opponents: 0,
            condition_percent: 100,
            ..Default::default()
        };
        let presentation = super::super::course_view(&options, &VisualAssets::default())
            .unwrap()
            .unwrap();
        for row in native["inventory"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["course"].as_u64() == Some(course_id as u64))
        {
            let path = row["node"].as_u64().unwrap() as u32;
            let local = row["local"].as_u64().unwrap() as usize;
            let boundary =
                super::super::source_roads::boundary(&presentation.scene, path, local, 0);
            let surface = boundary.surface.as_ref().unwrap();
            *kinds.entry(surface.kind).or_default() += 1;
            let mut actual = vec![i32::from(surface.kind), surface.center];
            actual.extend(boundary.road);
            actual.extend(boundary.outer);
            actual.extend(surface.base);
            actual.extend(surface.top);
            actual.extend(surface.height);
            actual.extend(boundary.selectors.map(i32::from));
            actual.push(i32::from(boundary.flags));
            actual.extend(surface.side_height);
            for point in surface.points.iter().flatten() {
                actual.extend(point);
            }
            actual.extend([
                i32::from(surface.event_type),
                i32::from(surface.rail_offset),
                i32::from(surface.rail_flags),
                surface.drawn_rail_height.unwrap_or(0),
            ]);
            let expected: Vec<i32> = row["boundary"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as i32)
                .collect();
            let section = presentation.scene.course.nodes[&path].link as usize;
            let sample = &presentation.scene.course.sections[section].samples[local];
            let ground = Ground {
                left: -sample.width[0],
                right: sample.width[1],
                left_shoulder: -sample.width[0] - sample.shoulder[0],
                right_shoulder: sample.width[1] + sample.shoulder[1],
                flags: sample.marking_flags,
                kind: super::super::source_roads::ground_kind(
                    &presentation.scene.terrain[section][local],
                    sample.marking_flags,
                ),
                ..Default::default()
            };
            let positions = [
                ground.left_shoulder - 1,
                ground.left_shoulder,
                ground.left - 49,
                ground.left,
                ground.left + 1,
                ground.right - 1,
                ground.right,
                ground.right + 48,
                ground.right + 49,
                ground.right_shoulder,
            ];
            let classes: Vec<_> = positions
                .into_iter()
                .map(|x| ground.surface(x * 256) as u64)
                .collect();
            let expected_classes: Vec<_> = row["classes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap())
                .collect();
            if actual != expected || classes != expected_classes {
                failures.push(serde_json::json!({"course":stem,"node":path,"local":local,"expected":expected,"actual":actual,"expected_classes":expected_classes,"actual_classes":classes}));
            }
            count += 1;
        }
    }
    let output = std::path::PathBuf::from(std::env::var("RR_GROUND_OUTPUT").unwrap());
    std::fs::write(output.join("terrain-inventory-comparison.json"),serde_json::to_vec_pretty(&serde_json::json!({"outcome":if failures.is_empty(){"passed"}else{"failed"},"observations":count,"mismatches":failures.len(),"kinds":kinds,"first_mismatches":failures.iter().take(12).collect::<Vec<_>>(),"scope":"Every physical source road sample: native terrain geometry, decoded rails and10off-road surface classes; isolated seed1 geometry"})).unwrap()).unwrap();
    assert!(count > 25000);
    assert!(
        failures.is_empty(),
        "{} source terrain inventory mismatches",
        failures.len()
    );
}

#[test]
#[ignore = "requires private source content; actual steered branch episodes with original artwork"]
fn source_steered_branch_episodes() {
    use rr_render::camera::{route_encounter_snapshot, route_snapshot_view};
    use rr_simulation::{ActorId, Command, QueuedCommand, Simulation};
    let data = std::env::var("RR_SESSION_DATA").unwrap();
    let catalog = std::env::var("RR_SESSION_CATALOG").unwrap();
    let output = std::path::PathBuf::from(std::env::var("RR_SOURCE_DRIVE_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut cases = Vec::new();
    for (stem, _) in COURSES {
        let options = LaunchOptions {
            data_root: Some(data.clone().into()),
            catalog: Some(catalog.clone().into()),
            course: Some(stem.into()),
            level: 5,
            bike: 1,
            opponents: 14,
            condition_percent: 100,
            effects: true,
            ..Default::default()
        };
        let base = super::super::course_view(&options, &VisualAssets::default())
            .unwrap()
            .unwrap();
        let original = super::super::build_race(&options, Some(&base), 1, Some(1))
            .0
            .recording_setup(1)
            .unwrap();
        let split = base
            .scene
            .route
            .transitions
            .iter()
            .find(|event| event.kind == rr_course::route::TransitionKind::Split)
            .unwrap();
        let before = &base.scene.route.points[split.index - 1];
        for half in [0usize, 1] {
            let mut record = original.clone();
            let config = record.handling.as_ref().unwrap();
            let setup = record.encounter.as_mut().unwrap();
            let roads = setup.rules.source_roads.as_ref().unwrap();
            let fork = &roads.forks[&before.node];
            let left = roads.profiles[0].identity(fork.halves[0], 0).unwrap();
            let edge = left[2] + roads.ground[&fork.halves[0]][0].right;
            let distance = (split.index as i64 - i64::from(config.start_sample) - 2) * 256;
            assert!(
                distance < setup.rules.finish_progress.unwrap(),
                "branch fixture must precede the selected race finish"
            );
            let lateral = (edge + if half == 0 { -160 } else { 160 }) * 256;
            setup.seeds[0].distance = distance;
            setup.seeds[0].lateral = lateral;
            setup.seeds[0].speed = 15000;
            setup.seeds[0].location = Some(setup.topology.locate(distance, lateral, 0).unwrap());
            assert!(!setup.rules.assists.fast_recovery && !setup.rules.assists.rescue_unreachable);
            let target = fork.halves[half];
            let mut sim = Simulation::with_population(
                record.seed,
                config.clone(),
                setup.clone(),
                record.population.clone().unwrap(),
            )
            .unwrap();
            let mut frames = Vec::new();
            let mut entered = false;
            let mut selected = None;
            for tick in 0..=45 {
                let canonical = sim.snapshot();
                let unit = canonical
                    .actors
                    .iter()
                    .find(|actor| actor.id == ActorId(1))
                    .unwrap()
                    .encounter
                    .as_ref()
                    .unwrap();
                entered |= unit.recovery.bike_location.path == target;
                let mask = unit
                    .recovery
                    .body
                    .as_ref()
                    .and_then(|body| body.source_route)
                    .or(unit.source_route)
                    .unwrap();
                if [0, 6, 12, 24, 45].contains(&tick) {
                    if selected.as_ref().is_none_or(|scene: &rr_render::Scene| {
                        scene.options.branch_mask != u32::from(mask)
                    }) {
                        selected = base.scene.following_mask(u32::from(mask)).unwrap();
                    }
                    let scene = selected.as_ref().unwrap_or(&base.scene);
                    let snapshot = route_encounter_snapshot(&scene.route, &canonical);
                    let camera =
                        route_snapshot_view(&scene.route, &snapshot, &snapshot, 1., base.start_q8);
                    let camera_spec = rr_render::camera::route_camera_spec(
                        &scene.route,
                        &base.camera,
                        &snapshot,
                        &snapshot,
                        1.,
                    );
                    let (frame, receipt) =
                        scene.draw(&camera_spec, &camera, &snapshot, false).unwrap();
                    let name = format!("{stem}-{half}-{tick}.png");
                    std::fs::write(output.join(&name), frame.image.png().unwrap()).unwrap();
                    frames.push(serde_json::json!({"tick":tick,"mask":mask,"path":unit.recovery.bike_location.path,"phase":unit.recovery.phase,"image":name,"rider_bounds":receipt.rider_bounds}));
                }
                if tick == 45 {
                    break;
                }
                let input = Command {
                    throttle: true,
                    steer: if half == 0 { -1 } else { 1 },
                    ..Default::default()
                };
                record.commands.push(QueuedCommand {
                    tick: rr_simulation::Tick(tick),
                    actor: ActorId(1),
                    sequence: 0,
                    input,
                });
                sim.step(input).unwrap();
            }
            record.ticks = 45;
            record.expected_hash = Some(sim.state_hash());
            let name = format!("{stem}-{half}.json");
            let bytes = serde_json::to_vec(&record).unwrap();
            std::fs::write(output.join(&name), &bytes).unwrap();
            let mut schedules = Vec::new();
            for rate in ["30", "60", "144", "irregular"] {
                let result = rr_simulation::replay::run(
                    rr_simulation::replay::Replay::read(&bytes).unwrap(),
                    rate,
                )
                .unwrap();
                assert_eq!(result.state_sha256, sim.state_hash());
                schedules.push(rate);
            }
            cases.push(serde_json::json!({"course":stem,"half":half,"target":target,"entered_target":entered,"frames":frames,"replay":name,"state_sha256":sim.state_hash(),"schedules":schedules}));
        }
    }
    let failures = cases
        .iter()
        .filter(|case| case["entered_target"] != true)
        .count();
    std::fs::write(output.join("comparison.json"),serde_json::to_vec_pretty(&serde_json::json!({"outcome":if failures==0{"passed"}else{"failed"},"cases":cases,"scope":"Actual ordinary controls fromcontrolledpre-forksourceposes, allrealstaticcontacts/fullpopulation, assistsdisabled; no whole-race claim"})).unwrap()).unwrap();
    assert_eq!(
        failures, 0,
        "source branch episodes failed to enter intended physical fork"
    );
}
