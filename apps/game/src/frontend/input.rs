use super::*;
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
                world
                    .resource_mut::<crate::Bindings>()
                    .0
                    .contexts
                    .get_mut(&rr_input::Context::Race)
                    .unwrap()
                    .insert(action, vec![key]);
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
    if edge(rr_input::Action::NavigateDown) || pad(GamepadButton::DPadDown) {
        shell.focus = (shell.focus + 1) % items.len();
    }
    if edge(rr_input::Action::NavigateUp) || pad(GamepadButton::DPadUp) {
        shell.focus = (shell.focus + items.len() - 1) % items.len();
    }
    if edge(rr_input::Action::Back) || pad(GamepadButton::East) {
        actions.push(Act::Back);
    }
    if edge(rr_input::Action::Confirm) || pad(GamepadButton::South) {
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
