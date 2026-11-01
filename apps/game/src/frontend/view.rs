use super::*;
pub(super) fn banner(mut commands: Commands) {
    commands.spawn((
        RaceBanner,
        Text2d::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(20.),
            ..default()
        },
        TextColor(Color::WHITE),
        Anchor::TOP_LEFT,
        Transform::from_xyz(-300., 220., 45.),
    ));
    commands.spawn((
        HudBackdrop,
        Sprite::from_color(Color::BLACK, Vec2::new(600., 60.)),
        Transform::from_xyz(0., 190., 44.),
        Visibility::Hidden,
    ));
}
pub(super) fn menu(shell: &Shell) -> (String, Vec<(String, Act)>) {
    use Screen::*;
    let f = &shell.frontend;
    let mut items = vec![];
    macro_rules! item {
        ($label:expr,$act:expr) => {
            items.push(($label.to_string(), $act))
        };
    }
    let title = match f.screen {
        Main => {
            item!("THRASH / QUICK RACE", Act::Begin(Mode::Thrash));
            item!(
                if f.career.is_some() {
                    "RESUME BIG GAME"
                } else {
                    "NEW BIG GAME"
                },
                Act::Begin(Mode::BigGame)
            );
            item!("RESTROOM / OPTIONS", Act::Open(Restroom));
            item!("CREDITS", Act::Open(Credits));
            item!("EXIT", Act::Open(QuitConfirm));
            "ROAD RASH | RUST".into()
        }
        Characters => {
            for c in &shell.catalog.characters {
                item!(&c.name, Act::Character(c.id));
            }
            item!("BACK", Act::Back);
            "CHOOSE YOUR RIDER".into()
        }
        Courses => {
            for (i, (_, name)) in COURSES.iter().enumerate() {
                let done = f.mode == Mode::BigGame
                    && f.career
                        .as_ref()
                        .is_some_and(|c| c.qualified & (1 << i) != 0);
                item!(
                    format!("{name}{}", if done { "  [qualified]" } else { "" }),
                    Act::Course(i as u8 + 1)
                );
            }
            if f.mode == Mode::Thrash {
                item!(
                    format!("LEVEL {}  (cycle 1-5)", f.level),
                    Act::Level(f.level % 5 + 1)
                );
            }
            item!("BACK", Act::Back);
            format!("SELECT COURSE | LEVEL {}", f.level)
        }
        Street => {
            item!("RACE", Act::Open(Courses));
            item!("DER PANZER KLUB", Act::Open(Club));
            item!("OLLEY'S SHOWROOM", Act::Open(Showroom));
            item!("REPAIR BIKE", Act::Open(Repair));
            item!("RESTROOM", Act::Open(Restroom));
            item!("GO HOME", Act::Main);
            "THE STREET".into()
        }
        Club => {
            item!("SCHMOOZE", Act::Open(Schmooze));
            item!("BULLETIN BOARD / COURSES", Act::Open(Courses));
            item!("RESTROOM", Act::Open(Restroom));
            item!("THE STREET", Act::Back);
            "DER PANZER KLUB".into()
        }
        Showroom => {
            for b in &shell.catalog.bikes {
                item!(format!("{}  ${}", b.name, b.price), Act::Bike(b.id));
            }
            item!("THE STREET", Act::Back);
            "SHOWROOM | TRADE-IN AT HALF PRICE".into()
        }
        Bike => {
            item!("BIKE SPECIFICATIONS", Act::BikeDetails);
            if let Ok(bike) = shell.catalog.bike(f.selected_bike)
                && let Some(name) = bike
                    .source_image
                    .strip_prefix("IMAGES/BIKES/")
                    .and_then(|s| s.strip_suffix(".RRI"))
                && shell.videos.iter().any(|v| v == name)
            {
                item!("VIEW BIKE VIDEO", Act::PlayVideo(name.into()));
            }
            item!("BUY / REVIEW QUOTE", Act::Open(Purchase));
            item!("SHOWROOM", Act::Back);
            shell
                .catalog
                .bike(f.selected_bike)
                .map_or("BIKE".into(), |b| format!("{} | ${}", b.name, b.price))
        }
        Purchase => {
            item!("CONFIRM PURCHASE", Act::Buy);
            item!("CANCEL", Act::Back);
            "BUY BIKE?".into()
        }
        Repair => {
            let quote = f
                .career
                .as_ref()
                .and_then(|c| c.repair_quote(&shell.catalog).ok())
                .unwrap_or(0);
            item!(format!("CONFIRM REPAIR | ${quote}"), Act::Repair);
            item!("CANCEL", Act::Back);
            "REPAIR".into()
        }
        Schmooze => {
            if !f.dialogue_read {
                item!("TALK ABOUT RACING", Act::Read);
            }
            item!("RETURN TO CLUB", Act::Back);
            format!(
                "SCHMOOZE | {}",
                shell.catalog.characters[usize::from(f.dialogue_partner.unwrap_or(0))].name
            )
        }
        Restroom => {
            item!("SAVE BIG GAME", Act::Save);
            item!("LOAD BIG GAME", Act::BrowseLoads);
            item!("SAVE AS / SLOTS", Act::SaveAs);
            item!("RESTORE CAREER BACKUP", Act::RestoreBackup);
            item!("SELECT CHARACTER / NEW RIDER", Act::Open(Characters));
            item!("PLAYER NAME", Act::Name);
            item!("DISPLAY & SOUND", Act::Open(Settings));
            item!("CONFIGURE INPUT", Act::Open(Controls));
            item!("JUKEBOX", Act::Open(Jukebox));
            item!("CREDITS", Act::Open(Credits));
            item!("DONE", Act::Back);
            item!("GO HOME", Act::Main);
            "RESTROOM".into()
        }
        Saves => {
            for slot in 0..5 {
                let path = rr_session::persistence::paths::slot_path(&shell.slot_base, slot)
                    .expect("bounded slot");
                let status = if path.exists() { "occupied" } else { "empty" };
                item!(format!("SLOT {slot}: {status}"), Act::ChooseSlot(slot));
            }
            item!("CANCEL", Act::Back);
            if shell.save_mode {
                "SAVE CAREER TO SLOT".into()
            } else {
                "LOAD CAREER FROM SLOT".into()
            }
        }
        Settings => {
            item!(format!("VERTICAL SYNC: {}", f.settings.vsync), Act::Vsync);
            item!(
                format!("LANGUAGE: {}", f.settings.language.name()),
                Act::Language
            );
            item!(
                format!("TEXT SIZE: {}%", f.settings.text_scale),
                Act::TextScale
            );
            item!(
                format!("HIGH CONTRAST: {}", f.settings.high_contrast),
                Act::HighContrast
            );
            item!(
                format!("REDUCED CAMERA MOTION: {}", f.settings.reduced_motion),
                Act::ReducedMotion
            );
            item!(
                format!("REDUCED FLASHES: {}", f.settings.reduced_flashes),
                Act::ReducedFlashes
            );
            item!(
                format!("SOUND CAPTIONS: {}", f.settings.subtitles),
                Act::Subtitles
            );
            item!(
                format!("ENHANCED TEXTURES: {}", f.settings.enhanced),
                Act::Enhanced
            );
            item!(format!("WIDE FRAMING: {}", f.settings.wide), Act::Wide);
            item!(format!("EFFECTS: {}", f.settings.effects), Act::Effects);
            item!(format!("MUSIC: {}", f.settings.music), Act::Music);
            item!(
                format!("MUSIC VOLUME: {}", f.settings.music_volume),
                Act::MusicVolume
            );
            item!(
                format!("EFFECT VOLUME: {}", f.settings.effect_volume),
                Act::EffectVolume
            );
            item!(
                format!("MASTER VOLUME: {}", f.settings.master_volume),
                Act::MasterVolume
            );
            item!(
                format!("ENGINE VOLUME: {}", f.settings.engine_volume),
                Act::EngineVolume
            );
            item!(
                format!("AUTOPLAY VIDEOS: {}", f.settings.videos),
                Act::Videos
            );
            item!("RESET GRAPHICS", Act::ResetGraphics);
            item!("RESTORE PREFERENCES BACKUP", Act::RestorePreferences);
            item!("DONE", Act::Back);
            "SETTINGS".into()
        }
        Controls => {
            for a in [
                rr_input::Action::Accelerate,
                rr_input::Action::Brake,
                rr_input::Action::SteerLeft,
                rr_input::Action::SteerRight,
                rr_input::Action::Punch,
                rr_input::Action::Kick,
                rr_input::Action::Backhand,
                rr_input::Action::Nitro,
                rr_input::Action::Dismount,
                rr_input::Action::LeanLeft,
                rr_input::Action::LeanRight,
                rr_input::Action::Pause,
            ] {
                item!(format!("REBIND {a:?}"), Act::Bind(a));
            }
            item!("KEYBOARD DEFAULTS", Act::DefaultBindings);
            item!("DONE", Act::Back);
            "CONFIGURE INPUT".into()
        }
        Credits => {
            item!("NEXT PAGE", Act::CreditsNext);
            item!("PREVIOUS PAGE", Act::CreditsPrevious);
            item!("BACK", Act::Back);
            "ROAD RASH RUST | CREDITS".into()
        }
        Jukebox => {
            for (i, name) in [
                "Hammerbox",
                "Monster Magnet",
                "Paw",
                "Soundgarden",
                "Swervedriver",
                "Therapy?",
            ]
            .iter()
            .enumerate()
            {
                item!(*name, Act::Artist(i as u8));
            }
            item!("VIDEOS", Act::Open(Videos));
            item!("STOP", Act::StopMusic);
            item!("DONE", Act::Back);
            "JUKEBOX".into()
        }
        Tracks => {
            let (artist, start, count) =
                rr_media::cues::ARTISTS[usize::from(shell.selected_artist)];
            for index in start..start + count {
                item!(rr_media::cues::TRACKS[index], Act::Track(index as u8));
            }
            item!("BACK", Act::Back);
            format!("{artist} | SELECT TRACK")
        }
        Videos => {
            for name in &shell.videos {
                item!(name, Act::PlayVideo(name.clone()));
            }
            item!("BACK", Act::Back);
            "VIDEOS".into()
        }
        Results | Standings => {
            if f.screen == Results {
                item!("STANDINGS", Act::Open(Standings));
            }
            item!("CONTINUE", Act::Continue);
            format!(
                "RACE RESULT | {}",
                f.summary
                    .as_ref()
                    .map_or("", |s| outcome_label(s.outcome()))
            )
        }
        LevelUp | Finale => {
            item!("CONTINUE", Act::Continue);
            if f.screen == Finale {
                "BIG GAME COMPLETE".into()
            } else {
                format!("LEVEL {} UNLOCKED", f.level)
            }
        }
        Paused => {
            item!("RESUME", Act::Resume);
            item!("RESTART RACE", Act::Restart);
            item!("QUIT RACE", Act::Open(QuitConfirm));
            "PAUSED".into()
        }
        QuitConfirm => {
            item!("CANCEL", Act::Back);
            if f.summary.is_none() && shell.session.is_some() {
                item!("QUIT TO RESULTS", Act::QuitRace);
            } else {
                item!("EXIT APPLICATION", Act::Exit);
            }
            "CONFIRM QUIT".into()
        }
        Loading => "LOADING".into(),
        Racing => "".into(),
    };
    let columns = shell.layout.columns;
    if rr_session::ui::wrap(&shell.notice, columns).len() > shell.layout.notice_rows {
        item!("MORE TEXT", Act::NextText);
    }
    if matches!(f.screen, Results | Standings)
        && f.summary
            .as_ref()
            .is_some_and(|s| s.standings().len() > shell.layout.table_rows)
    {
        item!("NEXT RACERS", Act::RacersNext);
        item!("PREVIOUS RACERS", Act::RacersPrevious);
    }
    let language = f.settings.language;
    (
        shell.ui_text.label(&title, language),
        items
            .into_iter()
            .map(|(label, action)| (shell.ui_text.label(&label, language), action))
            .collect(),
    )
}
type HudQueries<'w, 's> = (
    Query<'w, 's, (&'static mut Text2d, &'static mut TextFont), With<RaceBanner>>,
    Query<
        'w,
        's,
        (
            &'static mut Sprite,
            &'static mut Transform,
            &'static mut Visibility,
        ),
        With<HudBackdrop>,
    >,
);
pub(super) fn render(
    mut commands: Commands,
    shell: Res<Shell>,
    media: Option<Res<crate::media::Media>>,
    (assets, options, snapshot): (Res<AssetServer>, Res<LaunchOptions>, Res<PresentedSnapshot>),
    existing: Query<Entity, With<MenuEntity>>,
    (mut banner, mut backdrops): HudQueries,
    mut last: Local<String>,
) {
    let countdown = shell
        .session
        .as_ref()
        .map_or(String::new(), |s| match s.phase() {
            Phase::Countdown { remaining } => format!("{}", remaining.div_ceil(60)),
            _ => format!(
                "#{} | {}",
                s.standings()
                    .iter()
                    .find(|r| r.actor == ActorId(1))
                    .map_or(0, |r| r.place),
                shell
                    .ui_text
                    .counted(
                        "racer",
                        s.standings().len() as u64,
                        shell.frontend.settings.language
                    )
                    .expect("racer count")
            ),
        });
    let capacity = shell
        .session
        .as_ref()
        .map_or(shell.hud_capacity, RaceSession::condition_capacity);
    let hud = rr_session::ui::HudValues::from_snapshot(&snapshot.0, capacity);
    let shown = shell.race_active() && hud.is_some();
    let mut status = String::new();
    if shown && let Some(hud) = hud {
        let values = std::collections::BTreeMap::from([
            ("speed", hud.speed.to_string()),
            ("gear", hud.gear.to_string()),
            ("health", hud.health.to_string()),
            ("maximum", hud.maximum.to_string()),
            ("condition", hud.condition_percent.to_string()),
            ("nitro", hud.nitro.to_string()),
        ]);
        status = shell
            .ui_text
            .message(
                if shell.layout.columns < 32 {
                    "hud_small"
                } else {
                    "hud"
                },
                shell.frontend.settings.language,
                &values,
            )
            .expect("HUD parameters");
        if !countdown.is_empty() {
            status.push('\n');
            status.push_str(&countdown);
        }
        if snapshot.0.road_users.iter().any(|a| a.siren) {
            status.push_str("\nPOLICE");
        }
        let caption = media.as_ref().map_or("", |m| m.caption.as_str());
        if !caption.is_empty() {
            status.push('\n');
            status.push_str(
                &shell
                    .ui_text
                    .label(caption, shell.frontend.settings.language),
            );
        }
        status = rr_session::ui::wrap(&status, shell.layout.columns).join("\n");
    }
    for (mut text, mut font) in &mut banner {
        text.0.clone_from(&status);
        font.font_size = bevy::text::FontSize::Px(shell.layout.font);
    }
    for (mut sprite, mut transform, mut visibility) in &mut backdrops {
        let height = (status.lines().count() as f32 * shell.layout.font * 1.25 + 8.).min(430.);
        sprite.custom_size = Some(Vec2::new(600., height));
        sprite.color = Color::srgba(
            0.,
            0.,
            0.,
            if shell.frontend.settings.high_contrast {
                1.
            } else {
                0.85
            },
        );
        transform.translation.y = 224. - height / 2.;
        *visibility = if shown {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let key = format!(
        "{:?}|{}|{}|{}|{:?}|{:?}|{}|{}|{:?}",
        shell.frontend.screen,
        shell.focus,
        shell.notice,
        shell.frontend.level,
        shell.frontend.settings,
        shell.frontend.career,
        shell.credits_page,
        shell.standings_page,
        shell.layout
    );
    if *last == key {
        return;
    }
    *last = key;
    for e in &existing {
        commands.entity(e).despawn();
    }
    if !shell.visible() {
        return;
    }
    let (title, items) = menu(&shell);
    commands.spawn((
        MenuEntity,
        Sprite::from_color(Color::srgb(0.025, 0.035, 0.055), Vec2::new(2000., 1000.)),
        Transform::from_xyz(0., 0., 40.),
    ));
    let title = if shell.layout.compact && shell.frontend.screen == Screen::Credits {
        shell
            .ui_text
            .label("CREDITS", shell.frontend.settings.language)
    } else {
        title
    };
    let mut body = if shell.layout.compact {
        format!("{title}\n")
    } else {
        format!("{title}\n\n")
    };
    if let Some(c) = &shell.frontend.career
        && shell.frontend.screen != Screen::Credits
    {
        if shell.layout.compact {
            if shell.frontend.screen != Screen::Credits {
                body.push_str(&format!("L{} ${} B{}%\n", c.level, c.cash, c.condition));
            }
        } else {
            let values = std::collections::BTreeMap::from([
                ("level", c.level.to_string()),
                (
                    "cash",
                    rr_session::ui::number(c.cash, shell.frontend.settings.language),
                ),
                ("condition", c.condition.to_string()),
                ("maximum", c.maximum_condition.to_string()),
            ]);
            body.push_str(
                &shell
                    .ui_text
                    .message("career_summary", shell.frontend.settings.language, &values)
                    .expect("complete interface parameters"),
            );
            body.push_str("\n\n");
        }
    }
    let visible = shell.layout.menu_rows;
    let first = if items.len() > visible {
        shell
            .focus
            .saturating_sub(visible / 2)
            .min(items.len().saturating_sub(visible))
    } else {
        0
    };
    for (i, (label, _)) in items.iter().enumerate().skip(first).take(visible) {
        body.push_str(&format!(
            "{} {}\n",
            if i == shell.focus { ">" } else { " " },
            label
        ));
    }
    if matches!(shell.frontend.screen, Screen::Results | Screen::Standings)
        && let Some(s) = &shell.frontend.summary
    {
        if shell.layout.compact {
            body.push_str(&format!(
                "#{} {:.1}s\n",
                s.place(),
                s.elapsed_ticks() as f64 / 60.
            ));
        } else {
            body.push_str(&format!(
                "\nPlace {}   Time {:.2}s\n",
                s.place(),
                s.elapsed_ticks() as f64 / 60.
            ));
        }
        let pages = s.standings().len().div_ceil(shell.layout.table_rows).max(1);
        let page = shell.standings_page.min(pages - 1);
        for row in s
            .standings()
            .iter()
            .skip(page * shell.layout.table_rows)
            .take(shell.layout.table_rows)
        {
            if shell.layout.compact {
                let name: String = row.name.chars().take(14).collect();
                body.push_str(&format!("{}. {}\n", row.place, name));
                continue;
            }
            body.push_str(&format!(
                "{:2}. {} | {}\n",
                row.place,
                row.name,
                match row.terminal {
                    Some(rr_simulation::recovery::Outcome::Finished) => "Finished",
                    Some(rr_simulation::recovery::Outcome::Arrested) => "Arrested",
                    Some(rr_simulation::recovery::Outcome::Wrecked) => "Wrecked",
                    None => "Unfinished",
                }
            ));
        }
        if let Some(t) = &shell.frontend.settlement {
            if shell.layout.compact {
                body.push_str(&format!("+${} -${}\n", t.prize, t.penalty));
            } else {
                body.push_str(&format!(
                    "Prize ${}   Penalty ${}{}\n",
                    t.prize,
                    t.penalty,
                    if t.game_over { "   Career reset" } else { "" }
                ));
            }
        }
    }
    if shell.frontend.screen == Screen::Credits {
        if shell.texts.contains_key(&shell.frontend.settings.language) {
            let lines = shell.credit_lines();
            let pages = lines.len().div_ceil(shell.layout.credit_rows).max(1);
            let page = shell.credits_page.min(pages - 1);
            if shell.layout.compact {
                body.push_str(&format!("{}/{}\n", page + 1, pages));
            } else {
                body.push_str(&format!("Original game credits — {}/{}\n", page + 1, pages));
            }
            for line in lines
                .iter()
                .skip(page * shell.layout.credit_rows)
                .take(shell.layout.credit_rows)
            {
                body.push_str(&rr_session::text::readable(line));
                body.push('\n');
            }
        } else {
            body.push_str("Import game data to view the original credits.\n");
        }
    }
    if shell.frontend.screen == Screen::Characters && shell.focus < shell.catalog.characters.len() {
        let c = &shell.catalog.characters[shell.focus];
        if shell.layout.compact {
            body.push_str(&format!(
                "${} / B{}\n",
                c.starting_cash,
                u16::from(c.bikes[usize::from(shell.frontend.level - 1)]) + 1
            ));
        } else {
            body.push_str(&format!(
                "\nStarting cash: ${}\nPersonality {} | Bike {}\n",
                c.starting_cash,
                c.personality,
                u16::from(c.bikes[usize::from(shell.frontend.level - 1)]) + 1
            ));
        }
    }
    if !shell.notice.is_empty() {
        let columns = shell.layout.columns;
        let lines = rr_session::ui::wrap(&shell.notice, columns);
        let pages = lines.len().div_ceil(shell.layout.notice_rows).max(1);
        let page = shell.notice_page.min(pages - 1);
        if !shell.layout.compact {
            body.push('\n');
        }
        for line in lines
            .iter()
            .skip(page * shell.layout.notice_rows)
            .take(shell.layout.notice_rows)
        {
            body.push_str(line);
            body.push('\n');
        }
        if pages > 1 {
            body.push_str(&format!("({}/{})\n", page + 1, pages));
        }
    }
    body.push_str(if shell.layout.compact {
        "U/D Enter/A Esc/B"
    } else {
        "\nUp/Down / D-pad | Enter / A | Esc / B"
    });
    let font = shell.layout.font;
    commands.spawn((
        MenuEntity,
        Text2d::new(rr_session::ui::wrap(&body, shell.layout.columns).join("\n")),
        TextFont {
            font_size: bevy::text::FontSize::Px(font),
            ..default()
        },
        TextColor(if shell.frontend.settings.high_contrast {
            Color::WHITE
        } else {
            Color::srgb(0.9, 0.92, 0.95)
        }),
        Anchor::TOP_LEFT,
        Transform::from_xyz(-300., 220., 42.),
    ));
    if shell.layout.compact {
        return;
    }
    // A preview is presentation-only; family identity and gameplay stats remain typed domain records.
    let path = if shell.frontend.screen == Screen::Characters {
        shell
            .catalog
            .characters
            .get(shell.focus)
            .map(|c| &c.portrait)
    } else if shell.frontend.screen == Screen::Bike {
        shell
            .catalog
            .bike(shell.frontend.selected_bike)
            .ok()
            .map(|b| &b.source_image)
    } else {
        None
    };
    if let (Some(path), Some(root)) = (path, &options.catalog)
        && let Ok(c) = rr_content::Catalog::read(&root.join("catalog.json"))
        && let Some(entry) = c
            .entries
            .iter()
            .find(|e| e.source.eq_ignore_ascii_case(path))
        && let Some(blob) = entry.preview_variants.first()
    {
        commands.spawn((
            MenuEntity,
            Sprite {
                image: assets.load(blob.clone()),
                custom_size: Some({
                    let size = Vec2::new(entry.native_size[0] as f32, entry.native_size[1] as f32);
                    size * (160. / size.x).min(180. / size.y)
                }),
                ..default()
            },
            Transform::from_xyz(210., 70., 43.),
        ));
    }
}

pub(super) fn outcome_label(outcome: rr_session::race::Outcome) -> &'static str {
    use rr_session::race::Outcome::*;
    match outcome {
        Won => "WINNER",
        Lost => "FINISHED",
        Arrested => "BUSTED",
        Wrecked => "WRECKED",
        Quit => "QUIT",
        TimedOut => "TIME EXPIRED",
    }
}
