//! Bevy shell: commands create domain transitions; only the factory touches imported assets.
mod input;
mod storage;
#[cfg(test)]
mod tests;
mod view;
use super::{InputCommand, LaunchOptions, PauseState, Previous, Race};
use bevy::{prelude::*, sprite::Anchor};
use input::{load_script, navigate, script};
use rr_presentation::{CoursePresentation, PresentationStage, PresentedSnapshot, VisualAssets};
use rr_session::{
    career::Career,
    catalog::{COURSES, Catalog},
    flow::{Frontend, Screen},
    race::{Entry, Mode, Phase, RaceIdentity, RaceSession},
};
use rr_simulation::{ActorId, Simulation};
use storage::{flush_recording, save};
use view::{banner, menu, render};
#[derive(Resource)]
pub struct Shell {
    pub frontend: Frontend,
    pub catalog: Catalog,
    pub session: Option<RaceSession>,
    enabled: bool,
    focus: usize,
    notice: String,
    next_quick: u64,
    quit_app: bool,
    pub awaiting_binding: Option<rr_input::Action>,
    awaiting_name: Option<String>,
}
pub fn pause_controls(shell: Option<Res<Shell>>) -> bool {
    shell.is_none_or(|s| !s.enabled || matches!(s.frontend.screen, Screen::Racing | Screen::Paused))
}
impl Shell {
    pub fn new(options: &LaunchOptions, simulation: &Simulation, replay: bool) -> Self {
        let mut notice = String::new();
        let catalog = options
            .data_root
            .as_ref()
            .map(|root| {
                std::fs::read(root.join("ROADRASH.EXE"))
                    .map_err(|e| e.to_string())
                    .and_then(|b| Catalog::from_exe(&b))
            })
            .transpose()
            .unwrap_or_else(|e| {
                notice = e;
                None
            })
            .unwrap_or_else(Catalog::authored);
        let mut frontend = Frontend::default();
        frontend.settings.effects = options.effects;
        frontend.settings.enhanced = options.enhanced;
        frontend.settings.wide = options.framing == rr_render::Framing::Wide;
        if options.career_file.exists() {
            match std::fs::read(&options.career_file)
                .map_err(|e| e.to_string())
                .and_then(|b| Career::decode(&b, &catalog))
            {
                Ok(c) => frontend.career = Some(c),
                Err(e) => notice = format!("Career could not be loaded: {e}"),
            }
        }
        let menu = options
            .frontend
            .unwrap_or(options.course.is_none() && options.smoke_frames.is_none());
        let enabled = !replay && (options.smoke_frames.is_none() || options.frontend == Some(true));
        let session = if enabled && !menu {
            frontend.screen = Screen::Racing;
            frontend.level = options.level;
            frontend.bike = options.bike.saturating_sub(1) as u8;
            frontend.course = COURSES
                .iter()
                .position(|c| Some(c.0) == options.course.as_deref())
                .unwrap_or(0) as u8
                + 1;
            Some(Self::make_session(&frontend, &catalog, simulation, 1))
        } else {
            None
        };
        Self {
            frontend,
            catalog,
            session,
            enabled,
            focus: 0,
            notice,
            next_quick: 2,
            quit_app: false,
            awaiting_binding: None,
            awaiting_name: None,
        }
    }
    fn make_session(front: &Frontend, catalog: &Catalog, sim: &Simulation, id: u64) -> RaceSession {
        let identity = RaceIdentity {
            id,
            mode: front.mode,
            course: front.course,
            level: front.level,
            bike: front.bike,
            seed: 1,
            roster: sim
                .snapshot()
                .actors
                .iter()
                .map(|a| Entry {
                    actor: a.id,
                    name: if a.id == ActorId(1) {
                        front
                            .career
                            .as_ref()
                            .filter(|_| front.mode == Mode::BigGame)
                            .filter(|c| !c.alias.is_empty())
                            .map_or_else(
                                || {
                                    catalog
                                        .character(front.character)
                                        .map_or("Player".into(), |c| c.name.clone())
                                },
                                |c| c.alias.clone(),
                            )
                    } else {
                        catalog
                            .characters
                            .iter()
                            .find(|c| u64::from(c.personality) == a.id.0 - 1)
                            .map_or_else(|| format!("Rival {}", a.id.0 - 1), |c| c.name.clone())
                    },
                })
                .collect(),
        };
        RaceSession::new(
            identity,
            sim.encounters().map(|w| w.setup.topology.clone()),
            &sim.snapshot(),
            180,
            None,
        )
        .expect("validated race session")
        .with_condition_capacity(
            sim.recording_setup(1)
                .ok()
                .and_then(|r| r.handling)
                .and_then(|c| {
                    c.specs
                        .into_iter()
                        .find(|s| s.id == u32::from(front.bike) + 1)
                })
                .map_or(100, |s| s.condition),
        )
        .expect("source condition capacity")
    }
    pub fn race_active(&self) -> bool {
        !self.enabled || self.frontend.screen == Screen::Racing
    }
    pub fn visible(&self) -> bool {
        self.enabled && !matches!(self.frontend.screen, Screen::Racing)
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Act {
    Begin(Mode),
    Open(Screen),
    Back,
    Main,
    Character(u8),
    Course(u8),
    Level(u8),
    Bike(u8),
    Buy,
    Repair,
    Read,
    Track(u8),
    StopMusic,
    Effects,
    Music,
    Enhanced,
    Wide,
    MusicVolume,
    EffectVolume,
    Bind(rr_input::Action),
    DefaultBindings,
    Save,
    Name,
    Load,
    Continue,
    Resume,
    Restart,
    QuitRace,
    Exit,
}
#[derive(Component)]
struct MenuEntity;
#[derive(Component)]
struct RaceBanner;
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptStep {
    frame: u32,
    action: Act,
}
#[derive(Resource, Default)]
struct Script {
    frame: u32,
    steps: std::collections::VecDeque<ScriptStep>,
}
pub struct FrontendPlugin;
impl Plugin for FrontendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Script>()
            .add_systems(Startup, (banner, load_script, clear_initial_menu))
            .add_systems(Update, script.before(navigate))
            .add_systems(
                Update,
                (navigate, coordinate, render)
                    .chain()
                    .before(PresentationStage::Prepare),
            );
    }
}
fn apply(world: &mut World, action: Act) {
    world.resource_scope(|world, mut shell: Mut<Shell>| {
        let before = shell.frontend.screen;
        let result = (|| -> Result<(), String> {
            match action {
                Act::Begin(mode) => shell.frontend.begin(mode),
                Act::Open(screen) => {
                    shell.frontend.open(screen)?;
                    if screen == Screen::Paused {
                        world.resource_mut::<PauseState>().paused = true;
                    }
                }
                Act::Back => {
                    if shell.frontend.screen == Screen::Paused {
                        return resume(world, &mut shell);
                    }
                    shell.frontend.back();
                }
                Act::Main => {
                    shell.frontend.return_main();
                    release_race(world, &mut shell);
                }
                Act::Character(id) => {
                    let catalog = shell.catalog.clone();
                    shell.frontend.choose_character(&catalog, id)?;
                }
                Act::Course(course) => shell.frontend.select_course(course)?,
                Act::Level(level) => {
                    let catalog = shell.catalog.clone();
                    shell.frontend.select_level(&catalog, level)?;
                }
                Act::Bike(bike) => {
                    shell.catalog.bike(bike)?;
                    shell.frontend.selected_bike = bike;
                    shell.frontend.open(Screen::Bike)?;
                }
                Act::Buy => {
                    let catalog = shell.catalog.clone();
                    let result = shell.frontend.confirm_purchase(&catalog)?;
                    shell.notice = format!("{result:?}");
                    save(world, &shell)?;
                }
                Act::Repair => {
                    let catalog = shell.catalog.clone();
                    let cost = shell.frontend.confirm_repair(&catalog)?;
                    shell.notice = format!("Repaired for ${cost}");
                    save(world, &shell)?;
                }
                Act::Read => {
                    shell.frontend.read_dialogue()?;
                    shell.notice =
                        "Keep your bike in shape, watch the bends, and leave room to pass.".into();
                }
                Act::Track(track) => {
                    shell.frontend.choose_track(track)?;
                    shell.notice = "Track selected | audio preview unavailable.".into();
                }
                Act::StopMusic => shell.frontend.settings.track = None,
                Act::Effects => shell.frontend.settings.effects = !shell.frontend.settings.effects,
                Act::Music => shell.frontend.settings.music = !shell.frontend.settings.music,
                Act::Enhanced => {
                    shell.frontend.settings.enhanced = !shell.frontend.settings.enhanced
                }
                Act::Wide => shell.frontend.settings.wide = !shell.frontend.settings.wide,
                Act::MusicVolume => {
                    shell.frontend.settings.music_volume =
                        (shell.frontend.settings.music_volume + 10) % 110
                }
                Act::EffectVolume => {
                    shell.frontend.settings.effect_volume =
                        (shell.frontend.settings.effect_volume + 10) % 110
                }
                Act::Bind(action) => {
                    shell.awaiting_binding = Some(action);
                    shell.notice = "Press a keyboard key; Escape cancels.".into();
                }
                Act::DefaultBindings => {
                    world.resource_mut::<super::Bindings>().0 = rr_input::ActionMap::default()
                }
                Act::Name => {
                    shell.awaiting_name = Some(
                        shell
                            .frontend
                            .career
                            .as_ref()
                            .ok_or("Create or load Big Game first.")?
                            .alias
                            .clone(),
                    );
                    shell.notice =
                        "Type your name (19 characters); Enter saves, Escape cancels.".into();
                }
                Act::Save => {
                    save(world, &shell)?;
                    shell.notice = "Big Game saved.".into();
                }
                Act::Load => {
                    let p = world.resource::<LaunchOptions>().career_file.clone();
                    let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
                    shell.frontend.career = Some(Career::decode(&bytes, &shell.catalog)?);
                    shell.frontend.mode = Mode::BigGame;
                    shell.frontend.sync_career();
                    shell.notice = "Big Game loaded.".into();
                }
                Act::Continue => {
                    shell.frontend.return_from_results()?;
                    release_race(world, &mut shell);
                }
                Act::Resume => return resume(world, &mut shell),
                Act::Restart => {
                    shell.frontend.screen = Screen::Loading;
                    shell.session = None;
                }
                Act::QuitRace => {
                    let snapshot = world.resource::<Race>().0.snapshot();
                    if let Some(s) = shell.session.as_mut() {
                        s.quit(&snapshot);
                    }
                }
                Act::Exit => shell.quit_app = true,
            }
            Ok(())
        })();
        if let Err(e) = result {
            shell.notice = e;
        }
        if before != shell.frontend.screen {
            shell.focus = 0;
        }
    });
}
fn resume(world: &mut World, shell: &mut Shell) -> Result<(), String> {
    if !world.resource::<super::LiveInput>().0.drive.neutral() {
        return Err("Release driving controls to resume.".into());
    }
    shell.frontend.back();
    shell.frontend.screen = Screen::Racing;
    world.resource_mut::<PauseState>().paused = false;
    world.resource_mut::<PauseState>().release_required = false;
    Ok(())
}
fn release_race(world: &mut World, shell: &mut Shell) {
    flush_recording(world);
    shell.session = None;
    let mut empty = Simulation::default();
    let _ = empty.despawn(ActorId(1));
    world.resource_mut::<Race>().0 = empty;
    world.resource_mut::<Previous>().0 = Default::default();
    *world.resource_mut::<PresentedSnapshot>() = Default::default();
    world.remove_resource::<CoursePresentation>();
    world.resource_mut::<PauseState>().paused = false;
    world.resource_mut::<InputCommand>().0 = Default::default();
}
fn coordinate(world: &mut World) {
    world.resource_scope(|world, mut shell: Mut<Shell>| {
        if !shell.enabled {
            return;
        }
        if shell.quit_app {
            world.write_message(bevy::app::AppExit::Success);
            return;
        }
        if shell.session.as_ref().and_then(|s| s.summary()).is_some()
            && matches!(
                shell.frontend.screen,
                Screen::Racing | Screen::Paused | Screen::QuitConfirm
            )
        {
            let summary = shell.session.as_ref().unwrap().summary().unwrap().clone();
            let catalog = shell.catalog.clone();
            match shell.frontend.completed(&catalog, summary) {
                Ok(()) => {
                    if shell.frontend.mode == Mode::BigGame
                        && let Err(e) = save(world, &shell)
                    {
                        shell.notice = e;
                    }
                    world.resource_mut::<PauseState>().paused = true;
                    shell.focus = 0;
                }
                Err(e) => shell.notice = e,
            }
        }
        if shell.frontend.screen == Screen::Racing && world.resource::<PauseState>().paused {
            let _ = shell.frontend.open(Screen::Paused);
            shell.focus = 0;
        }
        if shell.frontend.screen == Screen::Paused && !world.resource::<PauseState>().paused {
            shell.frontend.back();
            shell.frontend.screen = Screen::Racing;
        }
        if shell.frontend.screen != Screen::Loading {
            return;
        }
        let mut options = world.resource::<LaunchOptions>().clone();
        options.course = options
            .data_root
            .as_ref()
            .map(|_| COURSES[usize::from(shell.frontend.course - 1)].0.into());
        options.level = shell.frontend.level;
        options.bike = u32::from(shell.frontend.bike) + 1;
        options.rider_color = shell
            .catalog
            .character(shell.frontend.character)
            .map_or(0, |c| c.personality);
        options.enhanced = shell.frontend.settings.enhanced;
        options.effects = shell.frontend.settings.effects;
        options.condition_percent = if shell.frontend.mode == Mode::BigGame {
            shell.frontend.career.as_ref().map_or(100, |c| c.condition)
        } else {
            100
        };
        options.framing = if shell.frontend.settings.wide {
            rr_render::Framing::Wide
        } else {
            rr_render::Framing::Classic
        };
        let result = super::course_view(&options, world.resource::<VisualAssets>())
            .map_err(|e| e.to_string());
        match result {
            Ok(course) => {
                flush_recording(world);
                let race =
                    super::build_race(&options, course.as_ref(), 1, Some(shell.frontend.character));
                let id = if shell.frontend.mode == Mode::BigGame {
                    shell
                        .frontend
                        .career
                        .as_mut()
                        .unwrap()
                        .allocate_race()
                        .unwrap()
                } else {
                    let id = shell.next_quick;
                    shell.next_quick += 1;
                    id
                };
                shell.session = Some(Shell::make_session(
                    &shell.frontend,
                    &shell.catalog,
                    &race.0,
                    id,
                ));
                *world.resource_mut::<PresentedSnapshot>() =
                    PresentedSnapshot(race.0.snapshot(), race.0.snapshot(), 0.);
                let data = options.data_root.as_deref();
                if let Err(e) = world
                    .resource_mut::<super::race_recording::Recorder>()
                    .rebase(&race.0, data, id)
                {
                    shell.notice = e.to_string();
                }
                world.insert_resource(race);
                world.resource_mut::<Previous>().0 = Default::default();
                world.resource_mut::<PauseState>().paused = false;
                world.resource_mut::<InputCommand>().0 = Default::default();
                world.resource_mut::<super::LiveInput>().0 = Default::default();
                if let Some(course) = course {
                    world.insert_resource(course);
                }
                world.resource_mut::<VisualAssets>().initially_enhanced = options.enhanced;
                world.insert_resource(options);
                let _ = shell.frontend.loaded();
                shell.notice.clear();
            }
            Err(e) => {
                shell.notice = e;
                shell.frontend.screen = Screen::Courses;
            }
        }
    });
}
fn clear_initial_menu(world: &mut World) {
    world.resource_scope(|world, mut shell: Mut<Shell>| {
        if shell.visible() {
            release_race(world, &mut shell);
        }
    });
}
