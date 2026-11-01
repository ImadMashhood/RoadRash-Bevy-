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
        Transform::from_xyz(0., 120., 45.),
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
                rr_input::Action::Pause,
            ] {
                item!(format!("REBIND {a:?}"), Act::Bind(a));
            }
            item!("KEYBOARD DEFAULTS", Act::DefaultBindings);
            item!("DONE", Act::Back);
            "CONFIGURE INPUT".into()
        }
        Credits => {
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
    (title, items)
}
pub(super) fn render(
    mut commands: Commands,
    shell: Res<Shell>,
    assets: Res<AssetServer>,
    options: Res<LaunchOptions>,
    existing: Query<Entity, With<MenuEntity>>,
    mut banner: Query<&mut Text2d, With<RaceBanner>>,
    mut last: Local<String>,
) {
    let countdown = shell
        .session
        .as_ref()
        .map_or(String::new(), |s| match s.phase() {
            Phase::Countdown { remaining } => format!("{}", remaining.div_ceil(60)),
            _ => format!(
                "Place {} / {}",
                s.standings()
                    .iter()
                    .find(|r| r.actor == ActorId(1))
                    .map_or(0, |r| r.place),
                s.standings().len()
            ),
        });
    for mut text in &mut banner {
        text.0 = if shell.enabled && shell.frontend.screen == Screen::Racing {
            countdown.clone()
        } else {
            String::new()
        };
    }
    let key = format!(
        "{:?}|{}|{}|{}|{:?}|{:?}",
        shell.frontend.screen,
        shell.focus,
        shell.notice,
        shell.frontend.level,
        shell.frontend.settings,
        shell.frontend.career
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
    let mut body = format!("{title}\n\n");
    if let Some(c) = &shell.frontend.career {
        body.push_str(&format!(
            "BIG GAME   Level {}   ${}   Bike {}/{}\n\n",
            c.level, c.cash, c.condition, c.maximum_condition
        ));
    }
    let first = if items.len() > 16 {
        shell
            .focus
            .saturating_sub(6)
            .min(items.len().saturating_sub(12))
    } else {
        0
    };
    for (i, (label, _)) in items
        .iter()
        .enumerate()
        .skip(first)
        .take(if items.len() > 16 { 12 } else { 16 })
    {
        body.push_str(&format!(
            "{} {}\n",
            if i == shell.focus { ">" } else { " " },
            label
        ));
    }
    if matches!(shell.frontend.screen, Screen::Results | Screen::Standings)
        && let Some(s) = &shell.frontend.summary
    {
        body.push_str(&format!(
            "\nPlace {}   Time {:.2}s\n",
            s.place(),
            s.elapsed_ticks() as f64 / 60.
        ));
        for row in s.standings().iter().take(15) {
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
            body.push_str(&format!(
                "Prize ${}   Penalty ${}{}\n",
                t.prize,
                t.penalty,
                if t.game_over { "   Career reset" } else { "" }
            ));
        }
    }
    if shell.frontend.screen == Screen::Credits {
        body.push_str("\nRust recreation by Imad Mashhood\nEngine: Bevy | Original game: Electronic Arts\nResearch: arkits/road-rash (pinned reference)\nImported original assets belong to their owners.\n");
    }
    if shell.frontend.screen == Screen::Characters && shell.focus < shell.catalog.characters.len() {
        let c = &shell.catalog.characters[shell.focus];
        body.push_str(&format!(
            "\nStarting cash: ${}\nPersonality {} | Bike {}\n",
            c.starting_cash,
            c.personality,
            u16::from(c.bikes[usize::from(shell.frontend.level - 1)]) + 1
        ));
    }
    if shell.frontend.screen == Screen::Bike
        && let Some(data) = &options.data_root
        && let Ok(bytes) = std::fs::read(data.join("DATA/BIKESPEC.RSC"))
        && let Ok(specs) = rr_simulation::handling::BikeSpec::read(&bytes)
        && let Some(spec) = specs
            .iter()
            .find(|s| s.id == u32::from(shell.frontend.selected_bike) + 1)
    {
        body.push_str(&format!(
            "\nMass: {}  Gears: {}\nThrottle: {}  Brake: {}\nGrip: {}  Condition: {}\n",
            spec.mass,
            spec.gears.len(),
            spec.throttle,
            spec.brake,
            spec.grip_scale,
            spec.condition
        ));
    }
    if !shell.notice.is_empty() {
        body.push_str(&format!("\n{}\n", shell.notice));
    }
    body.push_str("\nUp/Down / D-pad | Enter / A | Esc / B");
    let font = if matches!(
        shell.frontend.screen,
        Screen::Standings | Screen::Results | Screen::Showroom
    ) {
        12.
    } else {
        14.
    };
    commands.spawn((
        MenuEntity,
        Text2d::new(body),
        TextFont {
            font_size: bevy::text::FontSize::Px(font),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.92, 0.95)),
        Anchor::TOP_LEFT,
        Transform::from_xyz(-300., 220., 42.),
    ));
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
