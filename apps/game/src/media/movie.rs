use super::*;
use bevy::{
    asset::RenderAssetUsages,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use rr_media::video::{self, Clock, Poll, Stream};
use rr_session::{flow::Screen, race::Outcome};
#[derive(Component)]
struct MoviePicture;
#[derive(Component)]
struct MovieBackdrop;
#[derive(Component)]
struct MovieLabel;
#[derive(Resource)]
struct Surface(Handle<Image>);
pub(super) struct Prepared {
    pub stream: Stream,
    pub audio: Option<Arc<Pcm>>,
    pub name: String,
}
pub(crate) struct Playing {
    pub clock: Clock,
    stream: Stream,
    audio: Option<Arc<Pcm>>,
    owner: Screen,
    pub(super) last_frame: Option<u64>,
    pub(crate) name: String,
    last_wall: Instant,
}
pub(super) fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
    commands.insert_resource(Surface(handle.clone()));
    commands.spawn((
        MovieBackdrop,
        Sprite::from_color(Color::BLACK, Vec2::new(2000., 1000.)),
        Transform::from_xyz(0., 0., 59.),
        Visibility::Hidden,
    ));
    commands.spawn((
        MoviePicture,
        Sprite {
            image: handle,
            custom_size: Some(Vec2::new(640., 480.)),
            ..default()
        },
        Transform::from_xyz(0., 0., 60.),
        Visibility::Hidden,
    ));
    commands.spawn((
        MovieLabel,
        Text2d::new("Loading media..."),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0., -220., 61.),
        Visibility::Hidden,
    ));
}
pub(super) fn update(world: &mut World, media: &mut Media, device: &mut Device) {
    let options = world.resource::<LaunchOptions>().clone();
    let front = world.resource::<Shell>().frontend.clone();
    if let Some(summary) = &front.summary {
        let key = (summary.identity().id, summary.outcome());
        if media.seen_result != Some(key) {
            media.seen_result = Some(key);
            if front.settings.videos {
                let prefix = match summary.outcome() {
                    Outcome::Won => Some("WIN"),
                    Outcome::Lost => Some("LOSE"),
                    Outcome::Arrested => Some("BUSTED"),
                    Outcome::Wrecked => Some("WRECK"),
                    _ => None,
                };
                if let Some(prefix) = prefix {
                    media.video_request = Some(format!("{prefix}1"));
                }
            }
        }
    }
    let began = media.video_request.is_some();
    if let Some(name) = media.video_request.take() {
        clear(media, device);
        if options.data_root.is_some()
            && !name.is_empty()
            && name.len() <= 40
            && name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        {
            media.pending_video = Some(name);
            media.movie_target = Some(front.screen);
        } else {
            media
                .diagnostics
                .push("Invalid/unavailable movie identity".into());
        }
    }
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let escape = keys.just_pressed(KeyCode::Escape);
    let confirm = keys.just_pressed(KeyCode::Enter);
    let toggle = keys.just_pressed(KeyCode::Space);
    let mut pads = world.query::<&Gamepad>();
    let back_pad = pads
        .iter(world)
        .any(|p| p.just_pressed(GamepadButton::East));
    let confirm_pad = pads
        .iter(world)
        .any(|p| p.just_pressed(GamepadButton::South));
    let skip = media.skip_video || escape || back_pad || (!began && (confirm || confirm_pad));
    media.skip_video = false;
    if (media.movie.is_some() || media.movie_target.is_some())
        && (skip || media.movie_target.is_some_and(|s| s != front.screen))
    {
        if skip {
            media.movies_skipped += 1;
        }
        clear(media, device);
    }
    if let Some(rx) = &media.movie_loading {
        match rx.try_recv() {
            Ok(Ok(p)) if media.movie_target.is_some() && media.pending_video.is_none() => {
                media.movie = Some(Playing {
                    clock: Clock::default(),
                    stream: p.stream,
                    audio: p.audio,
                    owner: front.screen,
                    last_frame: None,
                    name: p.name,
                    last_wall: Instant::now(),
                });
                media.movie_loading = None;
            }
            Ok(Ok(_)) => {
                media.movie_loading = None;
            }
            Ok(Err(e)) => {
                media.movie_loading = None;
                if media.pending_video.is_none() {
                    media.movie_target = None;
                    world.resource_mut::<Shell>().notice = format!("Video unavailable: {e}");
                }
                media.diagnostics.push(e);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                media.movie_loading = None;
                media.movie_target = None;
            }
        }
    }
    if media.movie_loading.is_none()
        && let Some(name) = media.pending_video.take()
        && let Some(root) = options.data_root.as_ref()
    {
        let root = root.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        media.movie_loading = Some(Inbox::new(rx));
        std::thread::spawn(move || {
            let result = (|| {
                let path =
                    video::resolve(&root, std::path::Path::new(&format!("VIDEO/{name}.AVI")))?;
                let info = video::probe(&path)?;
                let audio = video::audio(&path, &info)?.map(Arc::new);
                let stream = Stream::open(&path, info)?;
                Ok(Prepared {
                    stream,
                    audio,
                    name,
                })
            })();
            let _ = tx.send(result);
        });
    }
    let mut finished = false;
    if let Some(movie) = media.movie.as_mut() {
        if toggle {
            movie.clock.set_paused(!movie.clock.paused());
        }
        let now = Instant::now();
        let delta = now.duration_since(movie.last_wall).as_nanos() as u64;
        movie.last_wall = now;
        if let Some(pcm) = &movie.audio
            && device.connected()
            && device.movie.is_none()
        {
            device.start_movie(pcm.clone(), movie.clock.elapsed_us());
        }
        if let Some(position) = device.movie_position() {
            movie.clock.synchronize_audio(position);
        } else {
            movie.clock.advance_ns(delta);
        }
        let target = movie.stream.info.target_frame(movie.clock.elapsed_us());
        match movie.stream.poll(target) {
            Poll::Frame(frame) => {
                if let Some(previous) = movie.last_frame {
                    media.video_drops += frame.index.saturating_sub(previous + 1);
                } else {
                    media.video_drops += frame.index;
                }
                media.last_rgb_mean = frame
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|p| u64::from(p[0]) + u64::from(p[1]) + u64::from(p[2]))
                    .sum::<u64>() as f64
                    / (frame.rgba.len() / 4 * 3) as f64;
                movie.last_frame = Some(frame.index);
                media.video_frames += 1;
                upload(
                    world,
                    movie.stream.info.width,
                    movie.stream.info.height,
                    frame.rgba,
                );
            }
            Poll::Failed(e) => {
                media.diagnostics.push(e.clone());
                world.resource_mut::<Shell>().notice = format!("Video stopped: {e}");
                finished = true;
            }
            Poll::Waiting | Poll::Ended => {}
        }
        if movie.clock.elapsed_us() >= movie.stream.info.duration_us || movie.owner != front.screen
        {
            finished = true;
        }
    }
    if finished {
        media.movies_completed += 1;
        clear(media, device);
    }
    let visible = media.movie.is_some() || media.movie_target.is_some();
    let mut pictures = world.query_filtered::<(&mut Sprite, &mut Visibility), With<MoviePicture>>();
    for (mut sprite, mut visibility) in pictures.iter_mut(world) {
        *visibility = if media.movie.as_ref().is_some_and(|m| m.last_frame.is_some()) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(movie) = &media.movie {
            let size = Vec2::new(
                movie.stream.info.width as f32,
                movie.stream.info.height as f32,
            );
            sprite.custom_size = Some(size * (620. / size.x).min(420. / size.y));
        }
    }
    let mut backgrounds = world.query_filtered::<&mut Visibility, With<MovieBackdrop>>();
    for mut v in backgrounds.iter_mut(world) {
        *v = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let mut labels = world.query_filtered::<(&mut Text2d, &mut Visibility), With<MovieLabel>>();
    for (mut text, mut visibility) in labels.iter_mut(world) {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        text.0 = media
            .movie
            .as_ref()
            .map_or("Loading video | Enter/Esc to skip".into(), |m| {
                format!(
                    "{} | Space: pause/resume | Enter/Esc: skip{}",
                    m.name,
                    if m.clock.paused() { " | PAUSED" } else { "" }
                )
            });
    }
}
fn clear(media: &mut Media, device: &mut Device) {
    media.movie = None;
    media.pending_video = None;
    media.movie_target = None;
    device.stop_movie();
}

fn upload(world: &mut World, width: u32, height: u32, pixels: Vec<u8>) {
    let mut handle = world.resource::<Surface>().0.clone();
    let changed = rr_presentation::upload_rgba(
        &mut world.resource_mut::<Assets<Image>>(),
        &mut handle,
        width,
        height,
        pixels,
    );
    if changed {
        world.resource_mut::<Surface>().0 = handle.clone();
        let mut pictures = world.query_filtered::<&mut Sprite, With<MoviePicture>>();
        for mut sprite in pictures.iter_mut(world) {
            sprite.image = handle.clone();
        }
    }
}
