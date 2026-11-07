use super::*;
pub(super) fn menu_enabled(shell: &Shell, index: usize) -> bool {
    if shell.frontend.screen != Screen::Restroom {
        return true;
    }
    match index {
        1 | 2 => shell.frontend.career.is_some(),
        5 => shell.frontend.mode == Mode::Thrash,
        _ => true,
    }
}
pub(super) fn navigate(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    bindings: Res<crate::Bindings>,
    mut shell: ResMut<Shell>,
    mut actions: Local<Vec<Act>>,
    mut commands: Commands,
) {
    actions.clear();
    if !shell.enabled {
        return;
    }
    if let Some(mut name) = shell.awaiting_name.take() {
        if keys.just_pressed(KeyCode::Escape) {
            return;
        }
        if keys.just_pressed(KeyCode::Enter) {
            if let Some(c) = shell.frontend.career.as_mut() {
                c.alias = name;
            }
            shell.notice = "Player name updated. Save to keep it.".into();
            return;
        }
        if keys.just_pressed(KeyCode::Backspace) {
            name.pop();
        }
        for key in keys.get_just_pressed() {
            let label = format!("{key:?}");
            let ch = if *key == KeyCode::Space {
                Some(' ')
            } else {
                label
                    .strip_prefix("Key")
                    .or_else(|| label.strip_prefix("Digit"))
                    .and_then(|s| if s.len() == 1 { s.chars().next() } else { None })
            };
            if let Some(mut ch) = ch {
                if !keys.pressed(KeyCode::ShiftLeft) && !keys.pressed(KeyCode::ShiftRight) {
                    ch = ch.to_ascii_lowercase();
                }
                if name.len() < 19 {
                    name.push(ch);
                }
            }
        }
        shell.notice = format!("Name: {name}_   Enter saves | Escape cancels");
        shell.awaiting_name = Some(name);
        return;
    }
    if let Some(action) = shell.awaiting_binding {
        if keys.just_pressed(KeyCode::Escape) {
            shell.awaiting_binding = None;
            return;
        }
        if let Some(code) = keys.get_just_pressed().next()
            && let Ok(key) = serde_json::from_value::<rr_input::Key>(serde_json::Value::String(
                format!("{code:?}"),
            ))
        {
            commands.queue(move |world: &mut World| {
                if let Err(e) = world.resource_mut::<crate::Bindings>().0.rebind(
                    rr_input::Context::Race,
                    action,
                    vec![key],
                ) {
                    world.resource_mut::<Shell>().notice = e.to_string();
                }
            });
            shell.awaiting_binding = None;
            shell.notice = format!("{action:?} bound to {code:?}");
        }
        return;
    }
    if !shell.visible() {
        return;
    }
    let (_, items) = menu(&shell);
    if items.is_empty() {
        return;
    }
    shell.focus = shell.focus.min(items.len() - 1);
    let edge = |action| {
        bindings.0.contexts[&rr_input::Context::Menu]
            .get(&action)
            .is_some_and(|ks| {
                ks.iter()
                    .any(|k| keys.just_pressed(crate::input_adapter::key_code(*k)))
            })
    };
    let pad = |button| pads.iter().any(|p| p.just_pressed(button));
    let columns = match shell.frontend.screen {
        Screen::Main | Screen::Street => Some(1),
        Screen::Courses => Some(2),
        Screen::Characters => Some(3),
        _ => None,
    };
    if edge(rr_input::Action::NavigateDown) || pad(GamepadButton::DPadDown) {
        for _ in 0..items.len() {
            shell.focus = (shell.focus + 1) % items.len();
            if menu_enabled(&shell, shell.focus) {
                break;
            }
        }
    }
    if edge(rr_input::Action::NavigateUp) || pad(GamepadButton::DPadUp) {
        for _ in 0..items.len() {
            shell.focus = (shell.focus + items.len() - 1) % items.len();
            if menu_enabled(&shell, shell.focus) {
                break;
            }
        }
    }
    if let Some(stride) = columns {
        if keys.just_pressed(KeyCode::ArrowRight) || pad(GamepadButton::DPadRight) {
            shell.focus = (shell.focus + stride) % items.len();
        }
        if keys.just_pressed(KeyCode::ArrowLeft) || pad(GamepadButton::DPadLeft) {
            shell.focus = (shell.focus + items.len() - stride.min(items.len())) % items.len();
        }
    }
    if edge(rr_input::Action::Back) || pad(GamepadButton::East) {
        actions.push(Act::Back);
    }
    if (edge(rr_input::Action::Confirm) || pad(GamepadButton::South))
        && menu_enabled(&shell, shell.focus)
    {
        actions.push(items[shell.focus].1.clone());
    }
    if let Some(action) = actions.first().cloned() {
        commands.queue(move |world: &mut World| apply(world, action));
    }
}
pub(super) fn load_script(options: Res<LaunchOptions>, mut script: ResMut<Script>) {
    if let Some(p) = &options.ui_script {
        let bytes = std::fs::read(p).expect("UI script read");
        assert!(bytes.len() <= 65536, "UI script bound");
        let steps: Vec<ScriptStep> = serde_json::from_slice(&bytes).expect("typed UI script");
        assert!(
            steps.len() <= 256 && steps.windows(2).all(|w| w[0].frame <= w[1].frame),
            "UI script ordering/count"
        );
        script.steps = steps.into();
    }
}
pub(super) fn script(mut commands: Commands, mut script: ResMut<Script>) {
    script.frame += 1;
    while script
        .steps
        .front()
        .is_some_and(|s| s.frame <= script.frame)
    {
        let step = script.steps.pop_front().unwrap();
        commands.queue(move |world: &mut World| apply(world, step.action));
    }
}

/// Project pointer coordinates through the same camera viewport used for the logical UI.
/// Only pointer movement changes hover focus, so keyboard/controller focus remains stable.
pub(super) fn pointer(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    hitboxes: Query<&view::MenuHitbox>,
    mut previous: Local<Option<Vec2>>,
    mut shell: ResMut<Shell>,
    mut commands: Commands,
) {
    if !shell.visible() || shell.awaiting_binding.is_some() || shell.awaiting_name.is_some() {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        *previous = None;
        return;
    };
    let clicked = mouse.just_pressed(MouseButton::Left);
    if *previous == Some(cursor) && !clicked {
        return;
    }
    *previous = Some(cursor);
    let Ok((camera, transform)) = cameras.single() else {
        return;
    };
    let Ok(point) = camera.viewport_to_world_2d(transform, cursor) else {
        return;
    };
    if let Some(hit) = hitboxes.iter().find(|h| {
        h.screen == shell.frontend.screen
            && menu_enabled(&shell, h.index)
            && (point - h.center).abs().cmple(h.size / 2.).all()
    }) {
        let (_, items) = menu(&shell);
        if let Some((_, action)) = items.get(hit.index) {
            shell.focus = hit.index;
            if clicked {
                let action = action.clone();
                commands.queue(move |world: &mut World| apply(world, action));
            }
        }
    }
}
