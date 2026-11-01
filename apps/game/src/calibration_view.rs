//! Native calibration tool; owns drafts and draws the production input curve.
use crate::gamepad_calibration::{pad_key, pad_state};
use bevy::{
    camera::Viewport,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    sprite::Anchor,
};
use rr_input::{
    calibration::{Calibration, CalibrationFile, Capture, Response},
    devices::PadState,
};
use std::{collections::BTreeMap, path::PathBuf};
const WIDTH: f32 = 1120.;
const HEIGHT: f32 = 760.;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Next,
    Field(usize),
    Adjust(i32),
    Center,
    Sweep,
    Save,
    Reset,
}
#[derive(Clone)]
struct Connected {
    id: u64,
    key: String,
    name: String,
    pad: PadState,
}
#[derive(Clone, Copy)]
enum CaptureKind {
    Center,
    Range,
}
struct Measuring {
    kind: CaptureKind,
    samples: Capture,
    elapsed: f64,
}
#[derive(Resource)]
pub struct Editor {
    pub profiles: CalibrationFile,
    pub path: PathBuf,
    pub selected: Option<String>,
    session: Option<u64>,
    drafts: BTreeMap<String, Calibration>,
    field: usize,
    measuring: Option<Measuring>,
    status: String,
    connected: Vec<Connected>,
    pub frames: u32,
    pub smoke_frames: Option<u32>,
    pub capture: Option<PathBuf>,
    pub report: Option<PathBuf>,
}
impl Editor {
    pub fn new(profiles: CalibrationFile, path: PathBuf, selected: Option<String>) -> Self {
        Self {
            profiles,
            path,
            selected,
            session: None,
            drafts: BTreeMap::new(),
            field: 0,
            measuring: None,
            status: "Choose your controller, then release the stick.".into(),
            connected: vec![],
            frames: 0,
            smoke_frames: None,
            capture: None,
            report: None,
        }
    }
    fn profile(&self) -> Calibration {
        self.selected
            .as_ref()
            .map(|key| {
                self.drafts
                    .get(key)
                    .copied()
                    .unwrap_or_else(|| self.profiles.profile(key))
            })
            .unwrap_or(self.profiles.default)
    }
    fn device(&self) -> Option<&Connected> {
        self.connected
            .iter()
            .find(|d| Some(d.id) == self.session && Some(&d.key) == self.selected.as_ref())
    }
    fn dirty(&self) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|key| self.profile() != self.profiles.profile(key))
    }
    fn change(&mut self, profile: Calibration) {
        if let Err(e) = profile.validate() {
            self.status = e.to_string();
            return;
        }
        if let Some(key) = &self.selected {
            self.drafts.insert(key.clone(), profile);
            self.status = "Unsaved changes. Press Save to keep this profile.".into();
        }
    }
    fn action(&mut self, action: Action, coarse: bool) {
        if action == Action::Next {
            if self.connected.is_empty() {
                self.status = "No controller connected.".into();
                return;
            }
            let index = self
                .connected
                .iter()
                .position(|d| Some(d.id) == self.session)
                .map_or(0, |i| (i + 1) % self.connected.len());
            self.selected = Some(self.connected[index].key.clone());
            self.session = Some(self.connected[index].id);
            self.measuring = None;
            self.status = "Profile switched. Other unsaved drafts are retained.".into();
            return;
        }
        if let Action::Field(field) = action {
            self.field = field.min(7);
            return;
        }
        if self.device().is_none() {
            self.status = "Connect the selected controller first.".into();
            return;
        }
        match action {
            Action::Adjust(direction) => {
                self.measuring = None;
                let mut profile = self.profile();
                let step = if coarse { 0.05 } else { 0.01 } * f64::from(direction);
                match self.field {
                    0 => profile.center += step,
                    1 => profile.minimum += step,
                    2 => profile.maximum += step,
                    3 => profile.dead_zone += step,
                    4 => profile.saturation += step,
                    5 => {
                        let at = match profile.response {
                            Response::Linear => 0,
                            Response::Square => 1,
                            Response::Cubic => 2,
                        };
                        profile.response = [Response::Linear, Response::Square, Response::Cubic]
                            [(at + direction).rem_euclid(3) as usize];
                    }
                    6 => profile.invert = !profile.invert,
                    7 => profile.trigger_threshold += step,
                    _ => unreachable!(),
                }
                self.change(profile);
            }
            Action::Center => {
                self.measuring = Some(Measuring {
                    kind: CaptureKind::Center,
                    samples: Capture::default(),
                    elapsed: 0.,
                });
                self.status = "Release the stick. Measuring center for 1.5 seconds...".into();
            }
            Action::Sweep => {
                if matches!(
                    self.measuring.as_ref().map(|m| m.kind),
                    Some(CaptureKind::Range)
                ) {
                    let measurement = self.measuring.take().unwrap();
                    match measurement.samples.range(self.profile()) {
                        Ok(profile) => self.change(profile),
                        Err(e) => self.status = e.to_string(),
                    }
                } else {
                    self.measuring = Some(Measuring {
                        kind: CaptureKind::Range,
                        samples: Capture::default(),
                        elapsed: 0.,
                    });
                    self.status = "Move fully left and right, then press Finish sweep.".into();
                }
            }
            Action::Save => {
                if self.measuring.is_some() {
                    self.status = "Finish or cancel the capture before saving.".into();
                    return;
                }
                let key = self.selected.clone().unwrap();
                let mut next = self.profiles.clone();
                let result = next
                    .set(key, self.profile())
                    .and_then(|()| next.save(&self.path));
                match result {
                    Ok(()) => {
                        self.profiles = next;
                        self.status =
                            "Profile saved. Load this file when starting the game.".into();
                    }
                    Err(e) => {
                        eprintln!("Save failed: {e}");
                        self.status =
                            "Save failed. Check profile path and folder permissions.".into();
                    }
                }
            }
            Action::Reset => {
                self.measuring = None;
                self.change(self.profiles.default);
            }
            Action::Next | Action::Field(_) => unreachable!(),
        }
    }
    fn observe(&mut self, connected: Vec<Connected>, seconds: f64) {
        self.connected = connected;
        if self.selected.is_none() {
            self.selected = self.connected.first().map(|d| d.key.clone());
        }
        if self.session.is_none() {
            self.session = self
                .connected
                .iter()
                .find(|d| Some(&d.key) == self.selected.as_ref())
                .map(|d| d.id);
        }
        let Some(raw) = self.device().map(|d| d.pad.stick_x) else {
            if self.measuring.take().is_some() {
                self.status = "Controller disconnected. Capture cancelled; draft retained.".into();
            }
            return;
        };
        if let Some(measurement) = &mut self.measuring {
            measurement.samples.push(raw);
            measurement.elapsed += seconds;
            if matches!(measurement.kind, CaptureKind::Center) && measurement.elapsed >= 1.5 {
                let measurement = self.measuring.take().unwrap();
                match measurement.samples.center(self.profile()) {
                    Ok(profile) => self.change(profile),
                    Err(e) => self.status = e.to_string(),
                }
            }
        }
    }
}
#[derive(Component)]
enum Label {
    Device,
    Field(usize),
    Raw,
    Output,
    Drive,
    Status,
    Sweep,
    Dirty,
    Path,
}
#[derive(Component)]
enum Shape {
    Field(usize),
    Curve(usize),
    Raw,
    Output,
    Point,
    Center,
}
#[derive(Clone)]
struct Hit {
    center: Vec2,
    size: Vec2,
    action: Action,
}
fn hits() -> Vec<Hit> {
    let mut hits = vec![Hit {
        center: Vec2::new(476., 234.),
        size: Vec2::new(70., 36.),
        action: Action::Next,
    }];
    for field in 0..8 {
        let y = 132. - field as f32 * 39.;
        hits.push(Hit {
            center: Vec2::new(-308., y),
            size: Vec2::new(424., 34.),
            action: Action::Field(field),
        });
        hits.push(Hit {
            center: Vec2::new(-142., y),
            size: Vec2::new(32., 30.),
            action: Action::Adjust(-1),
        });
        hits.push(Hit {
            center: Vec2::new(-100., y),
            size: Vec2::new(32., 30.),
            action: Action::Adjust(1),
        });
    }
    for (x, y, action) in [
        (-418., -212., Action::Center),
        (-200., -212., Action::Sweep),
        (-418., -274., Action::Save),
        (-200., -274., Action::Reset),
    ] {
        hits.push(Hit {
            center: Vec2::new(x, y),
            size: Vec2::new(194., 42.),
            action,
        });
    }
    hits
}
fn field_name(index: usize) -> &'static str {
    [
        "Center",
        "Left limit",
        "Right limit",
        "Dead zone",
        "Full lock",
        "Response",
        "Invert stick",
        "Trigger",
    ][index]
}
fn field_value(profile: Calibration, index: usize) -> String {
    match index {
        0 => format!("{:+.4}", profile.center),
        1 => format!("{:+.3}", profile.minimum),
        2 => format!("{:+.3}", profile.maximum),
        3 => format!("{:.0}%", profile.dead_zone * 100.),
        4 => format!("{:.0}%", profile.saturation * 100.),
        5 => format!("{:?}", profile.response),
        6 => if profile.invert { "On" } else { "Off" }.into(),
        _ => format!("{:.0}%", profile.trigger_threshold * 100.),
    }
}
fn text(
    commands: &mut Commands,
    value: impl Into<String>,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    label: Option<Label>,
) {
    let mut entity = commands.spawn((
        Text2d::new(value),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        Anchor::TOP_LEFT,
        Transform::from_xyz(x, y, 3.),
    ));
    if let Some(label) = label {
        entity.insert(label);
    }
}
fn rect(commands: &mut Commands, center: Vec2, size: Vec2, color: Color, shape: Option<Shape>) {
    let mut entity = commands.spawn((
        Sprite::from_color(color, size),
        Transform::from_xyz(center.x, center.y, 0.),
    ));
    if let Some(shape) = shape {
        entity.insert(shape);
    }
}
fn line(commands: &mut Commands, a: Vec2, b: Vec2, color: Color, shape: Option<Shape>) {
    let delta = b - a;
    let center = (a + b) / 2.;
    let mut entity = commands.spawn((
        Sprite::from_color(color, Vec2::new(delta.length().max(0.01), 1.5)),
        Transform {
            translation: center.extend(1.),
            rotation: Quat::from_rotation_z(delta.y.atan2(delta.x)),
            ..default()
        },
    ));
    if let Some(shape) = shape {
        entity.insert(shape);
    }
}
fn setup(mut commands: Commands) {
    let white = Color::srgb(0.9, 0.94, 1.);
    let muted = Color::srgb(0.57, 0.66, 0.77);
    let cyan = Color::srgb(0.2, 0.87, 0.83);
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::Fixed {
                width: WIDTH,
                height: HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    text(
        &mut commands,
        "Controller calibration",
        -516.,
        342.,
        30.,
        white,
        None,
    );
    text(
        &mut commands,
        "Match steering to your controller. Preview changes before saving.",
        -516.,
        300.,
        17.,
        muted,
        None,
    );
    rect(
        &mut commands,
        Vec2::new(0., 234.),
        Vec2::new(1032., 56.),
        Color::srgb(0.09, 0.14, 0.21),
        None,
    );
    text(
        &mut commands,
        "Waiting for controller...",
        -496.,
        246.,
        19.,
        white,
        Some(Label::Device),
    );
    text(&mut commands, "SETTINGS", -516., 182., 17., muted, None);
    text(
        &mut commands,
        "STEERING RESPONSE",
        12.,
        216.,
        17.,
        muted,
        None,
    );
    for field in 0..8 {
        let y = 132. - field as f32 * 39.;
        rect(
            &mut commands,
            Vec2::new(-308., y),
            Vec2::new(424., 34.),
            Color::srgb(0.06, 0.1, 0.16),
            Some(Shape::Field(field)),
        );
        text(
            &mut commands,
            field_name(field),
            -504.,
            y + 11.,
            17.,
            white,
            None,
        );
        text(
            &mut commands,
            "",
            -304.,
            y + 11.,
            17.,
            cyan,
            Some(Label::Field(field)),
        );
    }
    rect(
        &mut commands,
        Vec2::new(252., 68.),
        Vec2::new(480., 246.),
        Color::srgb(0.055, 0.09, 0.14),
        None,
    );
    for i in 0..=10 {
        let x = 12. + i as f32 * 48.;
        let y = -42. + i as f32 * 22.;
        line(
            &mut commands,
            Vec2::new(x, -42.),
            Vec2::new(x, 178.),
            Color::srgb(0.13, 0.19, 0.25),
            None,
        );
        line(
            &mut commands,
            Vec2::new(12., y),
            Vec2::new(492., y),
            Color::srgb(0.13, 0.19, 0.25),
            None,
        );
    }
    line(
        &mut commands,
        Vec2::new(252., -42.),
        Vec2::new(252., 178.),
        Color::srgb(0.9, 0.6, 0.22),
        Some(Shape::Center),
    );
    for i in 0..200 {
        line(
            &mut commands,
            Vec2::ZERO,
            Vec2::X,
            cyan,
            Some(Shape::Curve(i)),
        );
    }
    rect(
        &mut commands,
        Vec2::ZERO,
        Vec2::splat(10.),
        white,
        Some(Shape::Point),
    );
    text(&mut commands, "-1", 12., -48., 13., muted, None);
    text(&mut commands, "0", 248., -48., 13., muted, None);
    text(&mut commands, "+1", 472., -48., 13., muted, None);
    text(
        &mut commands,
        "Raw stick: 0.000",
        12.,
        -91.,
        19.,
        white,
        Some(Label::Raw),
    );
    text(
        &mut commands,
        "Steering: 0.000",
        262.,
        -91.,
        19.,
        cyan,
        Some(Label::Output),
    );
    for y in [-132., -168.] {
        rect(
            &mut commands,
            Vec2::new(252., y),
            Vec2::new(480., 12.),
            Color::srgb(0.14, 0.2, 0.27),
            None,
        );
    }
    rect(
        &mut commands,
        Vec2::new(252., -132.),
        Vec2::new(4., 20.),
        Color::srgb(1., 0.7, 0.3),
        Some(Shape::Raw),
    );
    rect(
        &mut commands,
        Vec2::new(252., -168.),
        Vec2::new(1., 16.),
        cyan,
        Some(Shape::Output),
    );
    text(
        &mut commands,
        "",
        12.,
        -199.,
        16.,
        muted,
        Some(Label::Drive),
    );
    text(
        &mut commands,
        "",
        12.,
        -242.,
        16.,
        white,
        Some(Label::Status),
    );
    text(
        &mut commands,
        "",
        12.,
        -285.,
        16.,
        Color::srgb(1., 0.7, 0.3),
        Some(Label::Dirty),
    );
    for hit in hits()
        .into_iter()
        .filter(|hit| !matches!(hit.action, Action::Field(_)))
    {
        let color = if hit.action == Action::Save {
            Color::srgb(0.06, 0.35, 0.34)
        } else {
            Color::srgb(0.14, 0.2, 0.29)
        };
        rect(&mut commands, hit.center, hit.size, color, None);
        let (label, tag, size) = match hit.action {
            Action::Next => ("Next", None, 16.),
            Action::Adjust(-1) => ("-", None, 20.),
            Action::Adjust(_) => ("+", None, 20.),
            Action::Center => ("Capture center", None, 16.),
            Action::Sweep => ("Start sweep", Some(Label::Sweep), 16.),
            Action::Save => ("Save profile", None, 16.),
            Action::Reset => ("Reset draft", None, 16.),
            _ => unreachable!(),
        };
        let x = hit.center.x - label.len() as f32 * size * 0.3;
        text(
            &mut commands,
            label,
            x,
            hit.center.y + size * 0.55,
            size,
            white,
            tag,
        );
    }
    text(
        &mut commands,
        "Save before closing. Game loads profiles at launch.",
        -516.,
        -310.,
        14.,
        muted,
        None,
    );
    text(
        &mut commands,
        "Up/Down: select  Left/Right: adjust  Tab: device  Enter: center  M: sweep  S: save  R: reset  Esc: exit",
        -516.,
        -345.,
        14.,
        muted,
        None,
    );
    text(&mut commands, "", 12., -310., 12., muted, Some(Label::Path));
}
fn fit(
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<&mut Camera, With<Camera2d>>,
) {
    let size = window.physical_size();
    if size.x == 0 || size.y == 0 {
        return;
    }
    let scale = (size.x as f32 / WIDTH).min(size.y as f32 / HEIGHT);
    let extent = UVec2::new(
        (WIDTH * scale).floor().max(1.) as u32,
        (HEIGHT * scale).floor().max(1.) as u32,
    );
    for mut camera in &mut cameras {
        camera.viewport = Some(Viewport {
            physical_position: (size - extent) / 2,
            physical_size: extent,
            ..default()
        });
    }
}
fn world_cursor(window: &Window) -> Option<Vec2> {
    let pos = window.cursor_position()?;
    let scale = (window.width() / WIDTH).min(window.height() / HEIGHT);
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    let origin = Vec2::new(
        (window.width() - WIDTH * scale) / 2.,
        (window.height() - HEIGHT * scale) / 2.,
    );
    let pos = (pos - origin) / scale;
    Some(Vec2::new(pos.x - WIDTH / 2., HEIGHT / 2. - pos.y))
}
fn update(
    mut editor: ResMut<Editor>,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    pads: Query<(Entity, &Gamepad, Option<&Name>)>,
    mut exit: MessageWriter<AppExit>,
) {
    let mut connected = pads
        .iter()
        .map(|(entity, pad, name)| Connected {
            id: entity.to_bits(),
            key: pad_key(pad, name),
            name: name.map_or("Unnamed controller", Name::as_str).to_string(),
            pad: pad_state(pad),
        })
        .collect::<Vec<_>>();
    connected.sort_by(|a, b| a.key.cmp(&b.key).then(a.id.cmp(&b.id)));
    editor.observe(connected, time.delta_secs_f64());
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        editor.field = (editor.field + 7) % 8;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        editor.field = (editor.field + 1) % 8;
    }
    let mut actions = vec![];
    for (key, action) in [
        (KeyCode::Tab, Action::Next),
        (KeyCode::ArrowLeft, Action::Adjust(-1)),
        (KeyCode::ArrowRight, Action::Adjust(1)),
        (KeyCode::Enter, Action::Center),
        (KeyCode::KeyM, Action::Sweep),
        (KeyCode::KeyS, Action::Save),
        (KeyCode::KeyR, Action::Reset),
    ] {
        if keys.just_pressed(key) {
            actions.push(action);
        }
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Some(point) = world_cursor(&window)
    {
        let all = hits();
        if let Some(hit) = all
            .iter()
            .rev()
            .find(|hit| (point - hit.center).abs().cmple(hit.size / 2.).all())
        {
            // Plus/minus clicks also select their row, independent of keyboard focus.
            if matches!(hit.action, Action::Adjust(_)) {
                editor.field = ((132. - hit.center.y) / 39.).round() as usize;
            }
            actions.push(hit.action);
        }
    }
    for action in actions {
        editor.action(
            action,
            keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
        );
    }
}
fn draw(
    editor: Res<Editor>,
    mut labels: Query<(&Label, &mut Text2d)>,
    mut shapes: Query<(&Shape, &mut Transform, &mut Sprite)>,
) {
    let profile = editor.profile();
    let pad = editor.device().map(|d| d.pad).unwrap_or_default();
    let raw = if pad.stick_x.is_finite() {
        pad.stick_x.clamp(-1., 1.)
    } else {
        0.
    };
    let connected = editor.device().is_some();
    let output = if connected {
        f32::from(profile.steering_q15(pad.stick_x)) / 32767.
    } else {
        0.
    };
    let curve = profile.curve();
    for (label, mut text) in &mut labels {
        text.0 = match label {
            Label::Device => editor
                .device()
                .map(|d| {
                    format!(
                        "{}{}{}",
                        d.name
                            .chars()
                            .filter(|c| !c.is_control())
                            .take(54)
                            .collect::<String>(),
                        {
                            let units = editor
                                .connected
                                .iter()
                                .filter(|other| other.key == d.key)
                                .collect::<Vec<_>>();
                            if units.len() > 1 {
                                format!(
                                    " [unit {}/{}]",
                                    units.iter().position(|unit| unit.id == d.id).unwrap() + 1,
                                    units.len()
                                )
                            } else {
                                String::new()
                            }
                        },
                        if editor.dirty() { " *" } else { "" }
                    )
                })
                .unwrap_or_else(|| {
                    if editor.connected.is_empty() {
                        "No controller connected. Connect one to preview input.".into()
                    } else {
                        "Waiting for selected controller. Next chooses another.".into()
                    }
                }),
            Label::Field(field) => field_value(profile, *field),
            Label::Raw => {
                if !connected {
                    "Raw stick: --".into()
                } else if !pad.stick_x.is_finite() {
                    "Raw stick: invalid".into()
                } else {
                    format!("Raw stick: {raw:+.3}")
                }
            }
            Label::Output => {
                if connected {
                    format!("Steering: {output:+.3}")
                } else {
                    "Steering: --".into()
                }
            }
            Label::Drive => {
                let drive = pad.sample_with(profile).drive;
                format!(
                    "Accelerate: {}   Brake: {}   Race steer: {:+.2}",
                    if drive.throttle { "on" } else { "off" },
                    if drive.brake { "on" } else { "off" },
                    f32::from(drive.steer_q15) / 32767.
                )
            }
            Label::Status => {
                let status = editor.status.chars().collect::<Vec<_>>();
                status
                    .chunks(48)
                    .map(|chunk| chunk.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Label::Sweep => if matches!(
                editor.measuring.as_ref().map(|m| m.kind),
                Some(CaptureKind::Range)
            ) {
                "Finish sweep"
            } else {
                "Start sweep"
            }
            .into(),
            Label::Dirty => if editor.dirty() {
                "Unsaved draft"
            } else {
                "Matches saved / default profile"
            }
            .into(),
            Label::Path => {
                let path = editor.path.display().to_string();
                let chars = path.chars().collect::<Vec<_>>();
                if chars.len() > 60 {
                    format!(
                        "File: ...{}",
                        chars[chars.len() - 56..].iter().collect::<String>()
                    )
                } else {
                    format!("File: {path}")
                }
            }
        };
    }
    for (shape, mut transform, mut sprite) in &mut shapes {
        match shape {
            Shape::Field(field) => {
                sprite.color = if *field == editor.field {
                    Color::srgb(0.12, 0.22, 0.3)
                } else {
                    Color::srgb(0.06, 0.1, 0.16)
                }
            }
            Shape::Curve(i) => {
                let a = Vec2::new(
                    252. + curve[*i][0] as f32 * 240.,
                    68. + curve[*i][1] as f32 * 110.,
                );
                let b = Vec2::new(
                    252. + curve[*i + 1][0] as f32 * 240.,
                    68. + curve[*i + 1][1] as f32 * 110.,
                );
                let delta = b - a;
                transform.translation = ((a + b) / 2.).extend(1.);
                transform.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
                sprite.custom_size = Some(Vec2::new(delta.length().max(0.01) + 1., 2.));
            }
            Shape::Raw => transform.translation.x = 252. + raw * 240.,
            Shape::Output => {
                transform.translation.x = 252. + output * 120.;
                sprite.custom_size = Some(Vec2::new((output * 240.).abs().max(1.), 16.));
            }
            Shape::Point => {
                transform.translation = Vec3::new(252. + raw * 240., 68. + output * 110., 2.)
            }
            Shape::Center => {
                transform.translation.x = 252. + profile.center as f32 * 240.;
                transform.translation.z = 1.5;
                sprite.custom_size = Some(Vec2::new(220., 2.));
            }
        }
    }
}
fn smoke(mut editor: ResMut<Editor>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    editor.frames += 1;
    if editor.frames == 60
        && let Some(path) = &editor.capture
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    }
    if editor
        .smoke_frames
        .is_some_and(|limit| editor.frames >= limit)
    {
        if let Some(path) = &editor.report {
            let pad = editor.device().map(|d| d.pad).unwrap_or_default();
            let profile = editor.profile();
            let report = serde_json::json!({"frames":editor.frames,"selected":editor.selected,"connected":editor.device().is_some(),"profile":profile,"raw":pad.stick_x,"steer_q15":profile.steering_q15(pad.stick_x),"curve":profile.curve(),"dirty":editor.dirty(),"status":editor.status});
            if let Err(error) = std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()) {
                eprintln!("{error}");
                exit.write(AppExit::error());
                return;
            }
        }
        exit.write(AppExit::Success);
    }
}
pub fn install(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.035, 0.055, 0.09)))
        .add_systems(Startup, setup)
        .add_systems(Update, (fit, update, draw, smoke).chain());
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connected(key: &str, raw: f32) -> Connected {
        Connected {
            id: if key.ends_with("one") { 1 } else { 2 },
            key: key.into(),
            name: "Authored controller".into(),
            pad: PadState {
                stick_x: raw,
                ..Default::default()
            },
        }
    }
    #[test]
    fn editor_retains_drafts_switches_models_and_saves_only_valid_selected_profile() {
        let root = std::env::temp_dir().join(format!("rr-editor-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("profiles.json");
        let one = rr_input::calibration::device_key(None, None, "one");
        let two = rr_input::calibration::device_key(None, None, "two");
        let mut editor = Editor::new(CalibrationFile::default(), path.clone(), None);
        editor.observe(vec![connected(&one, 0.), connected(&two, 0.)], 0.);
        editor.action(Action::Field(3), false);
        editor.action(Action::Adjust(1), false);
        assert!(editor.dirty());
        let draft = editor.profile();
        editor.action(Action::Next, false);
        assert!(!editor.dirty());
        editor.action(Action::Next, false);
        assert_eq!(editor.profile(), draft);
        editor.action(Action::Save, false);
        assert!(!editor.dirty());
        let disk = CalibrationFile::read(&path).unwrap();
        assert_eq!(disk.devices.len(), 1);
        assert_eq!(disk.profile(&one), draft);
        let before = std::fs::read(&path).unwrap();
        editor.action(Action::Field(1), false);
        editor.action(Action::Adjust(-1), false);
        assert_eq!(
            editor.profile(),
            draft,
            "invalid -1.01 minimum must not replace valid draft"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        editor.action(Action::Reset, false);
        assert!(editor.dirty());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn center_capture_uses_live_samples_and_disconnect_cancels_learning() {
        let key = rr_input::calibration::device_key(None, None, "center");
        let mut editor = Editor::new(
            CalibrationFile::default(),
            PathBuf::from("unused"),
            Some(key.clone()),
        );
        editor.observe(vec![connected(&key, 0.07)], 0.);
        editor.action(Action::Center, false);
        for _ in 0..100 {
            editor.observe(vec![connected(&key, 0.07)], 0.02);
        }
        assert!((editor.profile().center - 0.07).abs() < 0.00001);
        assert!(editor.dirty());
        let before = editor.profile();
        editor.action(Action::Sweep, false);
        editor.observe(vec![], 0.1);
        assert!(editor.measuring.is_none());
        assert_eq!(editor.profile(), before);
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    #[test]
    fn actual_mouse_hit_testing_edits_the_clicked_row_and_saves_its_profile() {
        let root =
            std::env::temp_dir().join(format!("rr-calibration-click-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("profile.json");
        let mut app = App::new();
        app.init_resource::<Time<Real>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_message::<AppExit>()
            .insert_resource(Editor::new(CalibrationFile::default(), path.clone(), None))
            .add_systems(Update, update);
        app.world_mut()
            .spawn((Gamepad::default(), Name::new("Authored click controller")));
        let mut window = Window {
            resolution: (1120, 760).into(),
            ..Default::default()
        };
        window.set_cursor_position(Some(Vec2::new(460., 365.)));
        let window_id = app
            .world_mut()
            .spawn((window, bevy::window::PrimaryWindow))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(app.world().resource::<Editor>().field, 3);
        assert!((app.world().resource::<Editor>().profile().dead_zone - 0.16).abs() < 0.00001);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.world_mut()
            .get_mut::<Window>(window_id)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(142., 654.)));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        let saved = CalibrationFile::read(&path).unwrap();
        assert_eq!(saved.devices.len(), 1);
        assert!((saved.devices.values().next().unwrap().dead_zone - 0.16).abs() < 0.00001);
        assert!(!app.world().resource::<Editor>().dirty());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod identical_model_tests {
    use super::*;
    #[test]
    fn identical_models_keep_separate_live_sessions_and_never_mix_capture_samples() {
        let key = rr_input::calibration::device_key(Some(1), Some(2), "same model");
        let device = |id, raw| Connected {
            id,
            key: key.clone(),
            name: "Same model".into(),
            pad: PadState {
                stick_x: raw,
                ..Default::default()
            },
        };
        let mut editor = Editor::new(
            CalibrationFile::default(),
            PathBuf::from("unused"),
            Some(key.clone()),
        );
        editor.observe(vec![device(1, 0.), device(2, 0.8)], 0.);
        assert_eq!(editor.device().unwrap().id, 1);
        editor.action(Action::Center, false);
        for _ in 0..100 {
            editor.observe(vec![device(1, 0.), device(2, 0.8)], 0.02);
        }
        assert_eq!(editor.profile().center, 0.);
        editor.action(Action::Next, false);
        assert_eq!(editor.device().unwrap().id, 2);
        editor.action(Action::Center, false);
        editor.observe(vec![device(1, 0.)], 0.1);
        assert!(editor.device().is_none());
        assert!(editor.measuring.is_none());
    }
}
