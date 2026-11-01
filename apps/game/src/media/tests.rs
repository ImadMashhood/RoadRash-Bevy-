use super::*;
use rr_presentation::PresentedSnapshot;
use rr_session::flow::Screen;
use std::time::Duration;
fn app() -> App {
    let mut app = App::new();
    let options = LaunchOptions {
        frontend: Some(true),
        ..Default::default()
    };
    let race = Race::default();
    let shell = Shell::new(&options, &race.0, false);
    app.insert_resource(options)
        .insert_resource(race)
        .insert_resource(shell)
        .init_resource::<Driver>()
        .init_resource::<PauseState>()
        .init_resource::<PresentedSnapshot>()
        .init_resource::<Assets<Image>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(MediaPlugin);
    app
}
#[test]
fn disabled_backend_and_media_failure_leave_gameplay_and_frontend_unchanged() {
    let mut app = app();
    let before = app.world().resource::<Race>().0.state_hash();
    app.update();
    assert_eq!(app.world().non_send::<Device>().status, "disabled");
    app.world_mut().resource_mut::<Media>().video_request = Some("../invalid".into());
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(app.world().resource::<Race>().0.state_hash(), before);
    assert_eq!(
        app.world().resource::<Shell>().frontend.screen,
        Screen::Main
    );
    assert!(app.world().resource::<Media>().movie.is_none());
    assert_eq!(app.world().non_send::<Device>().voices(), 0);
}
fn source() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "rr-bevy-media-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("VIDEO")).unwrap();
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-nostdin",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=16x16:r=10:d=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=8000:duration=1",
            "-c:v",
            "rawvideo",
            "-pix_fmt",
            "bgr24",
            "-c:a",
            "pcm_s16le",
            "-shortest",
        ])
        .arg(root.join("VIDEO/TEST.AVI"))
        .status()
        .unwrap();
    assert!(status.success());
    root
}
fn wait(app: &mut App, predicate: impl Fn(&Media) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        app.update();
        if predicate(app.world().resource::<Media>()) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "{:?}",
            app.world().resource::<Media>().diagnostics
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn actual_bevy_video_pause_skip_and_repeated_playback_reuse_the_surface() {
    let root = source();
    let mut app = app();
    app.world_mut().resource_mut::<LaunchOptions>().data_root = Some(root.clone());
    let hash = app.world().resource::<Race>().0.state_hash();
    app.update();
    let image_count = app.world().resource::<Assets<Image>>().len();
    for attempt in 0..3 {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.world_mut().resource_mut::<Media>().video_request = Some("TEST".into());
        app.update();
        assert!(app.world().resource::<Media>().movie_target.is_some());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Enter);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        wait(&mut app, |m| m.movie.is_some() && m.video_frames > 0);
        // A late, paused skip uses the same ownership cleanup as an early skip.
        if attempt == 2 {
            app.world_mut()
                .resource_mut::<Media>()
                .movie
                .as_mut()
                .unwrap()
                .clock
                .advance(850_000);
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Space);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        assert!(
            app.world()
                .resource::<Media>()
                .movie
                .as_ref()
                .unwrap()
                .clock
                .paused()
        );
        let position = app
            .world()
            .resource::<Media>()
            .movie
            .as_ref()
            .unwrap()
            .clock
            .elapsed_us();
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<Media>()
                .movie
                .as_ref()
                .unwrap()
                .clock
                .elapsed_us(),
            position
        );
        app.world_mut().resource_mut::<Media>().skip_video = true;
        app.update();
        assert!(app.world().resource::<Media>().movie.is_none());
        assert_eq!(app.world().resource::<Assets<Image>>().len(), image_count);
        assert_eq!(app.world().resource::<Race>().0.state_hash(), hash);
        assert_eq!(
            app.world().resource::<Shell>().frontend.screen,
            Screen::Main
        );
    }
    let completed = app.world().resource::<Media>().movies_completed;
    app.world_mut().resource_mut::<Media>().video_request = Some("TEST".into());
    wait(&mut app, |m| m.movies_completed > completed);
    assert!(app.world().resource::<Media>().movie_target.is_none());
    assert_eq!(app.world().resource::<Assets<Image>>().len(), image_count);
    assert_eq!(app.world().resource::<Race>().0.state_hash(), hash);
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn immediate_skip_during_loading_returns_to_owner_and_drains_one_bounded_job() {
    let root = source();
    let mut app = app();
    app.world_mut().resource_mut::<LaunchOptions>().data_root = Some(root.clone());
    app.update();
    app.world_mut().resource_mut::<Media>().video_request = Some("TEST".into());
    app.update();
    app.world_mut().resource_mut::<Media>().skip_video = true;
    app.update();
    assert!(app.world().resource::<Media>().movie_target.is_none());
    wait(&mut app, |m| m.movie_loading.is_none());
    assert!(app.world().resource::<Media>().movie.is_none());
    assert_eq!(app.world().non_send::<Device>().voices(), 0);
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}
