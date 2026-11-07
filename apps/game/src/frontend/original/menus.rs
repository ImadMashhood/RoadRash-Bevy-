//! Original screen geometry and source-localized captions.
use super::*;
pub(in crate::frontend) fn main_menu(commands: &mut Commands, shell: &Shell, ui: &OriginalUi) {
    // PE menu 9: four 160x60 cells at y=420. Network mode is visibly disabled.
    for (slot, index, id, fallback) in [
        (0, Some(0), 2, "Thrash Mode"),
        (1, Some(1), 4, "Big Game Mode"),
        (2, None, 13, "Mano-À-Mano"),
        (3, Some(2), 6, "Restroom"),
    ] {
        let caption = shell
            .texts
            .get(&shell.frontend.settings.language)
            .and_then(|t| t.strings.get(id))
            .filter(|s| !s.is_empty())
            .map_or(fallback, String::as_str);
        let text = caption.replace(" Mode", "\nMode");
        let center = Vec2::new(-240. + slot as f32 * 160., -210.);
        if let Some(index) = index {
            commands.spawn((
                MenuEntity,
                view::MenuHitbox {
                    index,
                    screen: Screen::Main,
                    center,
                    size: Vec2::new(160., 60.),
                },
                Sprite::from_color(Color::NONE, Vec2::new(160., 60.)),
                Transform::from_xyz(center.x, center.y, 41.),
            ));
        }
        for (dx, dy, color, z) in [
            (2., -2., Color::BLACK, 41.5),
            (
                0.,
                0.,
                if index.is_none() {
                    Color::srgb(0.45, 0.45, 0.45)
                } else if index == Some(shell.focus) {
                    Color::srgb(1., 1., 0.)
                } else {
                    Color::WHITE
                },
                42.,
            ),
        ] {
            commands.spawn((
                MenuEntity,
                Text2d::new(&text),
                TextFont {
                    font: ui.font.clone().into(),
                    font_size: bevy::text::FontSize::Px(29.),
                    ..default()
                },
                TextColor(color),
                Anchor::CENTER,
                Transform::from_xyz(center.x + dx, center.y + dy, z),
            ));
        }
    }
    let description_id = [3usize, 5, 7][shell.focus.min(2)];
    if let Some(text) = shell
        .texts
        .get(&shell.frontend.settings.language)
        .and_then(|t| t.strings.get(description_id))
    {
        let wrapped = rr_session::ui::wrap(text, 33).join("\n");
        commands.spawn((
            MenuEntity,
            Text2d::new(wrapped),
            TextFont {
                font: ui.body_font.clone().into(),
                font_size: bevy::text::FontSize::Px(20.),
                ..default()
            },
            TextColor(Color::WHITE),
            Anchor::TOP_LEFT,
            Transform::from_xyz(10., 210., 42.),
        ));
    }
}

pub(in crate::frontend) fn source_menu(
    commands: &mut Commands,
    shell: &Shell,
    ui: &OriginalUi,
    items: &[(String, Act)],
) -> bool {
    let screen = shell.frontend.screen;
    if !matches!(
        screen,
        Screen::Restroom | Screen::Courses | Screen::Characters | Screen::Street
    ) {
        return false;
    }
    if (screen == Screen::Restroom && items.len() != 13)
        || (screen == Screen::Courses && items.len() != 6)
        || (screen == Screen::Characters && items.len() != 9)
        || (screen == Screen::Street && items.len() != 3)
    {
        return false;
    }
    for (index, (label, _)) in items.iter().enumerate() {
        let (x, y, width, height, size) = match screen {
            Screen::Restroom => (0., 114. + index as f32 * 28., 310., 28., 26.),
            Screen::Street => (index as f32 * 213., 420., 213., 60., 27.),
            Screen::Courses => (
                (index / 2) as f32 * 213.,
                420. + (index % 2) as f32 * 30.,
                213.,
                30.,
                25.,
            ),
            _ => (
                (index / 3) as f32 * 213.,
                402. + (index % 3) as f32 * 24.,
                213.,
                24.,
                23.,
            ),
        };
        let center = Vec2::new(x + width / 2. - 320., 240. - y - height / 2.);
        let enabled = input::menu_enabled(shell, index);
        if enabled {
            commands.spawn((
                MenuEntity,
                view::MenuHitbox {
                    index,
                    screen,
                    center,
                    size: Vec2::new(width, height),
                },
                Sprite::from_color(Color::NONE, Vec2::new(width, height)),
                Transform::from_xyz(center.x, center.y, 41.),
            ));
        }
        let label = if screen == Screen::Restroom {
            let ids = [64, 66, 68, 70, 394, 74, 76, 78, 80, 82, 84, 86, 8];
            shell
                .texts
                .get(&shell.frontend.settings.language)
                .and_then(|t| t.strings.get(ids[index]))
                .filter(|s| !s.is_empty())
                .cloned()
                .map(|s| {
                    if index == 5 {
                        format!("{s}: {}", shell.frontend.level)
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| label.clone())
        } else if screen == Screen::Street {
            shell
                .texts
                .get(&shell.frontend.settings.language)
                .and_then(|t| t.strings.get([48, 50, 8][index]))
                .filter(|s| !s.is_empty())
                .cloned()
                .unwrap_or_else(|| label.clone())
        } else {
            label.clone()
        };
        for (dx, dy, color, z) in [
            (2., -2., Color::BLACK, 41.8),
            (
                0.,
                0.,
                if !enabled {
                    Color::srgb(0.45, 0.45, 0.45)
                } else if index == shell.focus {
                    Color::srgb(1., 1., 0.)
                } else {
                    Color::WHITE
                },
                42.,
            ),
        ] {
            commands.spawn((
                MenuEntity,
                Text2d::new(&label),
                TextFont {
                    font: ui.font.clone().into(),
                    font_size: bevy::text::FontSize::Px(size),
                    ..default()
                },
                TextColor(color),
                if screen == Screen::Restroom {
                    Anchor::TOP_LEFT
                } else {
                    Anchor::TOP_CENTER
                },
                Transform::from_xyz(
                    if screen == Screen::Restroom {
                        x - 320. + dx
                    } else {
                        center.x + dx
                    },
                    240. - y + dy,
                    z,
                ),
            ));
        }
    }
    if screen == Screen::Restroom {
        let ids = [65, 67, 69, 71, 395, 75, 77, 79, 81, 83, 85, 87, 9];
        let description = shell
            .texts
            .get(&shell.frontend.settings.language)
            .and_then(|t| t.strings.get(ids[shell.focus.min(12)]))
            .map_or("", String::as_str);
        let text = if shell.notice.is_empty() {
            description
        } else {
            &shell.notice
        };
        commands.spawn((
            MenuEntity,
            Text2d::new(rr_session::ui::wrap(text, 30).join("\n")),
            TextFont {
                font: ui.body_font.clone().into(),
                font_size: bevy::text::FontSize::Px(20.),
                ..default()
            },
            TextColor(Color::WHITE),
            Anchor::TOP_LEFT,
            Transform::from_xyz(10., 210., 42.),
        ));
    }
    true
}
