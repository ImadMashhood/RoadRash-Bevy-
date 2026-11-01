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
}
#[test]
fn source_factory_repeated_courses_bikes_and_carried_damage_drop_all_scene_owners() {
    let (Ok(data), Ok(catalog)) = (
        std::env::var("RR_SESSION_DATA"),
        std::env::var("RR_SESSION_CATALOG"),
    ) else {
        return;
    };
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
    app.world_mut()
        .resource_mut::<crate::Bindings>()
        .0
        .rebind(
            rr_input::Context::Race,
            rr_input::Action::Accelerate,
            vec![rr_input::Key::KeyQ],
        )
        .unwrap();
    storage::preferences(app.world_mut());
    let (p, _) =
        rr_session::persistence::preferences::Preferences::decode(&std::fs::read(&path).unwrap())
            .unwrap();
    assert_eq!(p.settings.music_volume, 37);
    assert!(p.settings.wide);
    assert_eq!(
        p.bindings.contexts[&rr_input::Context::Race][&rr_input::Action::Accelerate],
        vec![rr_input::Key::KeyQ]
    );
    apply(app.world_mut(), Act::ResetGraphics);
    storage::preferences(app.world_mut());
    assert_eq!(app.world().resource::<Shell>().frontend.career, career);
    let (p, _) =
        rr_session::persistence::preferences::Preferences::decode(&std::fs::read(&path).unwrap())
            .unwrap();
    assert!(!p.settings.wide);
    assert_eq!(p.settings.music_volume, 37);
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(rr_session::persistence::disk::sidecar(&path, ".bak")).unwrap();
}
