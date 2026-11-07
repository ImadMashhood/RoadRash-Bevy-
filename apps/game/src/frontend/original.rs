//! Native dashboard artwork and EXE layout; no gameplay state is changed here.
mod menus;
mod raster;
use super::*;
pub(super) use menus::{main_menu, source_menu};
use raster::{blit, gap_value, meter_level, needle, source_sine};
use rr_formats::{pe::SupportedExe, pixels::Palette};

#[derive(Resource, Default)]
pub(super) struct OriginalUi {
    pub font: Handle<Font>,
    pub body_font: Handle<Font>,
    pub ready: bool,
    dashboards: Vec<Vec<u8>>,
    specs: Vec<rr_simulation::handling::BikeSpec>,
    meter: Vec<u8>,
    palette: Vec<[u8; 4]>,
    layout: Vec<i32>,
    sine: Vec<i32>,
    name_slots: Vec<u32>,
    course_name_bases: Vec<u32>,
    image: Handle<Image>,
    rpm: rr_media::audio::Rpm,
    tick: Option<u64>,
}
#[derive(Component)]
pub(super) struct Dashboard;
#[derive(Component)]
pub(super) struct DashboardText(usize);

fn dashboard_surface(images: &mut Assets<Image>) -> Handle<Image> {
    // Upload releases the previous owned texture. Never release Bevy's shared
    // default white image: solid-color sprites still depend on that asset.
    let mut image = images.add(Image::default());
    rr_presentation::upload_rgba(images, &mut image, 640, 480, vec![0; 640 * 480 * 4]);
    image
}

pub(super) fn setup(
    mut commands: Commands,
    shell: Res<Shell>,
    mut fonts: ResMut<Assets<Font>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut ui = OriginalUi::default();
    if let Some(root) = &shell.source_ui_root {
        let load = || -> Result<OriginalUi, Box<dyn std::error::Error>> {
            let exe = std::fs::read(root.join("ROADRASH.EXE"))?;
            let pe = SupportedExe::read(&exe)?;
            let layout = pe
                .read_va(0x474f50 + 2 * 0xdc, 0xdc)?
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| i32::from_le_bytes(*b))
                .collect();
            let words = |va: u32, count: usize| -> Result<Vec<u32>, Box<dyn std::error::Error>> {
                Ok(pe
                    .read_va(va, count * 4)?
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| u32::from_le_bytes(*b))
                    .collect())
            };
            let sine = rr_simulation::handling::Tables::read(&exe)?.sine;
            let name_slots = words(0x464188, 16)?;
            let course_name_bases = words(0x4635b8, 5)?;
            let palette = Palette::read(&std::fs::read(root.join("DATA/PALETTE.RAW"))?)?
                .0
                .to_vec();
            let dashboards = ["RAT", "SUPER", "SPORT"]
                .iter()
                .map(|family| {
                    std::fs::read(root.join(format!("IMAGES/BIKES/{family}/MHIDASH.BOB")))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if dashboards
                .iter()
                .any(|d| d.is_empty() || d.len() % 640 != 0 || d.len() > 640 * 122)
            {
                return Err("Invalid native dashboard dimensions".into());
            }
            let meter = std::fs::read(root.join("IMAGES/BIGMETER.BOB"))?;
            if meter.len() != 190 * 64 {
                return Err("Invalid native meters".into());
            }
            let specs = rr_simulation::handling::BikeSpec::read(&std::fs::read(
                root.join("DATA/BIKESPEC.RSC"),
            )?)?;
            Ok(OriginalUi {
                specs,
                sine,
                name_slots,
                course_name_bases,
                dashboards,
                meter,
                palette,
                layout,
                ready: true,
                ..Default::default()
            })
        };
        match load() {
            Ok(source) => ui = source,
            Err(error) => warn!("Original dashboard unavailable: {error}"),
        }
        for (path, handle) in [
            ("TEXT/BADLOC.DLL", &mut ui.font),
            ("TEXT/FUTB.DLL", &mut ui.body_font),
        ] {
            if let Ok(bytes) = std::fs::read(root.join(path)) {
                *handle = fonts.add(Font::from_bytes(bytes));
            }
        }
    }
    if ui.ready {
        ui.image = dashboard_surface(&mut images);
        commands.spawn((
            Dashboard,
            Sprite::from_image(ui.image.clone()),
            Transform::from_xyz(0., 0., 44.),
            Visibility::Hidden,
        ));
        for (index, offset) in [0usize, 5, 10, 25, 30].into_iter().enumerate() {
            let x = ui.layout[offset + 1] as f32;
            let y = ui.layout[offset + 2] as f32;
            let width = (ui.layout[offset + 3] - ui.layout[offset + 1]) as f32;
            let right = ui.layout[offset] & 2 != 0;
            commands.spawn((
                DashboardText(index),
                Text2d::new(""),
                TextFont {
                    font: ui.font.clone().into(),
                    font_size: bevy::text::FontSize::Px(27.),
                    ..default()
                },
                TextColor(if index == 0 {
                    Color::srgb(1., 1., 0.)
                } else {
                    Color::WHITE
                }),
                if right {
                    Anchor::TOP_RIGHT
                } else {
                    Anchor::TOP_LEFT
                },
                Transform::from_xyz(x - 320. + if right { width } else { 0. }, 240. - y, 46.),
                Visibility::Hidden,
            ));
        }
    }
    commands.insert_resource(ui);
}
#[allow(clippy::type_complexity)]
pub(super) fn update(
    mut ui: ResMut<OriginalUi>,
    shell: Res<Shell>,
    snapshot: Res<PresentedSnapshot>,
    race: Res<Race>,
    mut images: ResMut<Assets<Image>>,
    mut panels: Query<(&mut Sprite, &mut Visibility), With<Dashboard>>,
    mut labels: Query<
        (&DashboardText, &mut Text2d, &mut TextColor, &mut Visibility),
        Without<Dashboard>,
    >,
) {
    if !ui.ready {
        return;
    }
    let actor = snapshot.0.actors.iter().find(|a| a.id == ActorId(1));
    let show = shell.race_active()
        && actor.is_some()
        && !shell.frontend.settings.high_contrast
        && shell.frontend.settings.text_scale == 100;
    for (_, mut visibility) in &mut panels {
        *visibility = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (_, _, _, mut visibility) in &mut labels {
        *visibility = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !show {
        return;
    }
    let actor = actor.unwrap();
    let (Some(rider), Some(unit)) = (&actor.rider, &actor.encounter) else {
        return;
    };
    let bike = &rider.bike;
    let capacity = shell
        .session
        .as_ref()
        .map_or(shell.hud_capacity, RaceSession::condition_capacity);
    let others = || {
        snapshot
            .0
            .actors
            .iter()
            .filter(|a| a.id != ActorId(1) && a.rider.is_some())
    };
    let ahead = others()
        .filter(|a| a.rider.as_ref().unwrap().bike.distance_q8 >= bike.distance_q8)
        .min_by_key(|a| a.rider.as_ref().unwrap().bike.distance_q8 - bike.distance_q8);
    let behind = others()
        .filter(|a| a.rider.as_ref().unwrap().bike.distance_q8 < bike.distance_q8)
        .min_by_key(|a| bike.distance_q8 - a.rider.as_ref().unwrap().bike.distance_q8);
    let nearest = match (ahead, behind) {
        (Some(a), Some(b))
            if (bike.distance_q8 - b.rider.as_ref().unwrap().bike.distance_q8) * 2
                < a.rider.as_ref().unwrap().bike.distance_q8 - bike.distance_q8 =>
        {
            Some(b)
        }
        (Some(a), _) => Some(a),
        (_, b) => b,
    };
    let target_health = nearest.and_then(|a| a.encounter.as_ref()).map_or(0, |u| {
        meter_level(u.fighter.stats.health, u.fighter.stats.base)
    });
    let mut pixels = vec![0u8; 640 * 480 * 4];
    for (value, offset) in [
        (
            meter_level(unit.fighter.stats.health, unit.fighter.stats.base),
            40,
        ),
        (meter_level(bike.condition, capacity), 45),
        (target_health, 50),
    ] {
        let (x, y) = (ui.layout[offset + 1], ui.layout[offset + 2]);
        if ui.layout[offset] == 0 {
            blit(
                &mut pixels,
                &ui.meter,
                190,
                &ui.palette,
                [62, 16, 126, 34],
                [x, y],
                false,
            );
            blit(
                &mut pixels,
                &ui.meter,
                190,
                &ui.palette,
                [62, 0, 62 + value * 2, 16],
                [x + 1, y + 1],
                false,
            );
        } else {
            blit(
                &mut pixels,
                &ui.meter,
                190,
                &ui.palette,
                [11, 0, 62, 31],
                [x, y],
                false,
            );
            blit(
                &mut pixels,
                &ui.meter,
                190,
                &ui.palette,
                [11, 32, 62, 63 - value],
                [x, y],
                false,
            );
        }
    }
    let art = &ui.dashboards[usize::from(shell.frontend.bike / 5).min(2)];
    blit(
        &mut pixels,
        art,
        640,
        &ui.palette,
        [0, 0, 640, (art.len() / 640) as i32],
        [0, 480 - (art.len() / 640) as i32],
        true,
    );
    if ui.tick != Some(snapshot.0.tick) {
        ui.tick = Some(snapshot.0.tick);
        // The engine's presentation RPM has independent state; the HUD never feeds it back.
        let ratio = ui
            .specs
            .iter()
            .find(|s| s.id == u32::from(shell.frontend.bike) + 1)
            .and_then(|s| s.gears.get(bike.gear))
            .map_or(256, |g| g.ratio);
        ui.rpm.update(ratio, bike.speed, bike.height, bike.crashed);
    }
    let velocity = bike.speed as u32;
    let sum = ((velocity & 0xfffff001) as i32 >> 1) as u32;
    let speed_angle = (sum
        .wrapping_add((velocity & 0xffffff80).wrapping_mul(16))
        .wrapping_add((velocity & 0xfffffe00).wrapping_mul(4)) as i32
        / 200
        - 1024)
        .rem_euclid(4096);
    for (angle, offset) in [(speed_angle, 21), (ui.rpm.current / 44 - 1024, 16)] {
        let x = ui.layout[offset];
        let y = ui.layout[offset + 1];
        let end = [
            x + ((i64::from(source_sine(&ui.sine, angle)) * i64::from(ui.layout[offset + 2])) >> 32)
                as i32,
            y - ((i64::from(source_sine(&ui.sine, angle + 1024))
                * i64::from(ui.layout[offset + 3]))
                >> 32) as i32,
        ];
        needle(&mut pixels, [x, y], end);
    }
    let target_ahead =
        nearest.is_some_and(|a| a.rider.as_ref().unwrap().bike.distance_q8 >= bike.distance_q8);
    if nearest.is_some() {
        let x = ui.layout[11] - 12;
        let y = ui.layout[12] + if target_ahead { 9 } else { 11 };
        for (dx, dy, rgba) in [(0, 0, [0, 0, 0, 255]), (-2, -2, [255, 255, 255, 255])] {
            for row in 0..12 {
                let r = if target_ahead { row } else { 11 - row };
                let half = if r < 6 { r } else { 2 };
                for column in 5 - half..=5 + half {
                    let px = x + dx + 3 + column;
                    let py = y + dy + 3 + row;
                    if (0..640).contains(&px) && (0..480).contains(&py) {
                        pixels[((py * 640 + px) * 4) as usize..][..4].copy_from_slice(&rgba);
                    }
                }
            }
        }
    }
    let mut handle = ui.image.clone();
    rr_presentation::upload_rgba(&mut images, &mut handle, 640, 480, pixels);
    ui.image = handle.clone();
    for (mut sprite, _) in &mut panels {
        sprite.image = handle.clone();
    }
    let place = 1 + snapshot
        .0
        .actors
        .iter()
        .filter(|a| {
            a.id != ActorId(1)
                && a.rider
                    .as_ref()
                    .is_some_and(|r| r.bike.distance_q8 > bike.distance_q8)
        })
        .count();
    let player = shell
        .frontend
        .career
        .as_ref()
        .filter(|_| shell.frontend.mode == Mode::BigGame)
        .filter(|c| !c.alias.is_empty())
        .map(|c| c.alias.as_str())
        .unwrap_or_else(|| {
            shell
                .catalog
                .character(shell.frontend.character)
                .map_or("Player", |c| c.name.as_str())
        });
    let target = nearest.map_or(String::new(), |a| {
        let index = race
            .0
            .population()
            .and_then(|p| p.brains.get(&a.id))
            .and_then(|b| b.traits.overtake_index)
            .map(usize::from);
        // The native constructor gives the six spare grid slots name indices
        // 10..15 even though their driving personality is copied from row zero.
        let index = if (10..=15).contains(&a.id.0) {
            Some(a.id.0 as usize)
        } else {
            index
        };
        let slot = index.and_then(|index| {
            if index < 10 {
                ui.name_slots.get(index).copied()
            } else {
                ui.course_name_bases
                    .get(usize::from(shell.frontend.course.saturating_sub(1)))
                    .map(|base| base + (index as u32 - 10))
            }
        });
        slot.and_then(|slot| {
            shell
                .texts
                .get(&shell.frontend.settings.language)
                .and_then(|t| t.strings.get(slot as usize))
        })
        .cloned()
        .unwrap_or_else(|| format!("Rider {}", a.id.0 - 1))
    });
    let gap = nearest
        .map(|a| (a.rider.as_ref().unwrap().bike.distance_q8 - bike.distance_q8).abs())
        .unwrap_or(0);
    let distance = ((bike.distance_q8 >> 8).max(0) / 33) as u8;
    for (label, mut text, mut color, _) in &mut labels {
        if label.0 == 2 {
            color.0 = if target_ahead {
                Color::srgb(231. / 255., 16. / 255., 0.)
            } else {
                Color::srgb(40. / 255., 163. / 255., 28. / 255.)
            };
        }
        text.0 = match label.0 {
            0 => player.into(),
            1 => target.clone(),
            2 => {
                if nearest.is_some() {
                    format!("{}.{:03}", gap_value(gap) / 1000, gap_value(gap) % 1000)
                } else {
                    String::new()
                }
            }
            3 => place.to_string(),
            _ => format!("{:02}.{}", distance / 10, distance % 10),
        };
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;
    #[test]
    fn dashboard_upload_preserves_shared_white_and_owns_one_surface() {
        let mut images = Assets::<Image>::default();
        let shared = Handle::<Image>::default();
        images.insert(shared.id(), Image::default()).unwrap();
        let surface = dashboard_surface(&mut images);
        assert_ne!(surface.id(), shared.id());
        assert!(images.get(shared.id()).is_some());
        assert!(images.get(surface.id()).is_some());
        assert_eq!(images.len(), 2);
    }
}
