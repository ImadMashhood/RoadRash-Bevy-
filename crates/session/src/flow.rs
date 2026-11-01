use crate::{
    Result,
    career::{Career, Settlement},
    catalog::Catalog,
    race::{Mode, Summary},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Main,
    Characters,
    Courses,
    Street,
    Club,
    Showroom,
    Bike,
    Purchase,
    Repair,
    Schmooze,
    Restroom,
    Settings,
    Controls,
    Credits,
    Jukebox,
    Results,
    Standings,
    LevelUp,
    Finale,
    Loading,
    Racing,
    Paused,
    QuitConfirm,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub effects: bool,
    pub music: bool,
    pub music_volume: u8,
    pub effect_volume: u8,
    pub track: Option<u8>,
    pub enhanced: bool,
    pub wide: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            effects: true,
            music: true,
            music_volume: 80,
            effect_volume: 80,
            track: None,
            enhanced: false,
            wide: false,
        }
    }
}
#[derive(Clone)]
pub struct Frontend {
    pub screen: Screen,
    pub mode: Mode,
    pub career: Option<Career>,
    pub character: u8,
    pub level: u8,
    pub course: u8,
    pub bike: u8,
    pub selected_bike: u8,
    pub settings: Settings,
    pub summary: Option<Summary>,
    pub settlement: Option<Settlement>,
    stack: Vec<Screen>,
    quote_revision: Option<u64>,
    pub dialogue_seen: u8,
    pub dialogue_partner: Option<u8>,
    pub dialogue_read: bool,
}
impl Default for Frontend {
    fn default() -> Self {
        Self {
            screen: Screen::Main,
            mode: Mode::Thrash,
            career: None,
            character: 1,
            level: 1,
            course: 1,
            bike: 0,
            selected_bike: 0,
            settings: Settings::default(),
            summary: None,
            settlement: None,
            stack: vec![],
            quote_revision: None,
            dialogue_seen: 0,
            dialogue_partner: None,
            dialogue_read: false,
        }
    }
}
impl Frontend {
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }
    pub fn open(&mut self, next: Screen) -> Result<()> {
        use Screen::*;
        let valid = matches!(
            (self.screen, next),
            (
                Main,
                Characters | Courses | Restroom | Credits | QuitConfirm
            ) | (Street, Courses | Club | Showroom | Restroom | Repair)
                | (Club, Schmooze | Restroom | Courses)
                | (Showroom, Bike)
                | (Bike, Purchase)
                | (
                    Restroom,
                    Settings | Controls | Jukebox | Characters | Courses | Credits
                )
                | (Results, Standings)
                | (Racing, Paused)
                | (Paused, QuitConfirm)
        );
        if !valid || self.stack.len() >= 16 {
            return Err("unavailable frontend transition".into());
        }
        if matches!(
            next,
            Street | Club | Showroom | Purchase | Repair | Schmooze
        ) && self.mode != Mode::BigGame
        {
            return Err("career required".into());
        }
        if matches!(next, Purchase | Repair) && self.career.is_none() {
            return Err("career required".into());
        }
        self.stack.push(self.screen);
        self.screen = next;
        if matches!(next, Purchase | Repair) {
            self.quote_revision = Some(self.career.as_ref().ok_or("career required")?.revision());
        }
        if next == Schmooze {
            if self.dialogue_seen == 255 {
                self.dialogue_seen = 0;
            }
            let partner = (0..8).find(|i| self.dialogue_seen & (1 << i) == 0).unwrap();
            self.dialogue_partner = Some(partner);
            self.dialogue_read = false;
        }
        Ok(())
    }
    pub fn back(&mut self) {
        use Screen::*;
        if matches!(
            self.screen,
            Results | Standings | LevelUp | Finale | Loading | Racing
        ) {
            return;
        }
        if self.screen == Paused {
            self.stack.pop();
            self.screen = Racing;
            return;
        }
        if self.screen == Main {
            return;
        }
        self.screen = self.stack.pop().unwrap_or(Main);
        self.quote_revision = None;
    }
    pub fn begin(&mut self, mode: Mode) {
        self.mode = mode;
        self.summary = None;
        self.settlement = None;
        self.stack = vec![Screen::Main];
        self.screen = if mode == Mode::BigGame && self.career.is_some() {
            self.sync_career();
            Screen::Street
        } else {
            Screen::Characters
        };
    }
    pub fn choose_character(&mut self, catalog: &Catalog, id: u8) -> Result<()> {
        if self.screen != Screen::Characters {
            return Err("character screen required".into());
        }
        let c = catalog.character(id)?;
        self.character = id;
        if self.mode == Mode::BigGame {
            self.career = Some(Career::new(catalog, id)?);
            self.sync_career();
            self.screen = Screen::Street;
        } else {
            self.bike = c.bikes[usize::from(self.level - 1)];
            self.screen = Screen::Courses;
        }
        self.stack = vec![Screen::Main];
        Ok(())
    }
    pub fn sync_career(&mut self) {
        if self.mode != Mode::BigGame {
            return;
        }
        if let Some(c) = &self.career {
            self.character = c.character;
            self.level = c.level;
            self.bike = c.bike;
        }
    }
    pub fn select_level(&mut self, catalog: &Catalog, level: u8) -> Result<()> {
        if self.screen != Screen::Courses || !(1..=5).contains(&level) {
            return Err("invalid level selection".into());
        }
        if self.mode == Mode::BigGame {
            if level != self.career.as_ref().ok_or("career required")?.level {
                return Err("career level is locked".into());
            }
        } else {
            self.level = level;
            self.bike = catalog.character(self.character)?.bikes[usize::from(level - 1)];
        }
        Ok(())
    }
    pub fn select_course(&mut self, course: u8) -> Result<()> {
        if self.screen != Screen::Courses || !(1..=5).contains(&course) {
            return Err("invalid course selection".into());
        }
        self.course = course;
        self.screen = Screen::Loading;
        self.stack.clear();
        Ok(())
    }
    pub fn loaded(&mut self) -> Result<()> {
        if self.screen != Screen::Loading {
            return Err("loading screen required".into());
        }
        self.screen = Screen::Racing;
        self.stack.clear();
        Ok(())
    }
    pub fn completed(&mut self, catalog: &Catalog, summary: Summary) -> Result<()> {
        if !matches!(
            self.screen,
            Screen::Racing | Screen::Paused | Screen::QuitConfirm
        ) {
            return Err("active race required".into());
        }
        if summary.identity().mode != self.mode {
            return Err("result mode mismatch".into());
        }
        self.settlement = if self.mode == Mode::BigGame {
            self.career
                .as_mut()
                .ok_or("career required")?
                .settle(catalog, &summary)?
        } else {
            None
        };
        self.summary = Some(summary);
        self.screen = Screen::Results;
        self.stack.clear();
        self.settings.track = None;
        Ok(())
    }
    pub fn return_from_results(&mut self) -> Result<()> {
        if !matches!(
            self.screen,
            Screen::Results | Screen::Standings | Screen::LevelUp | Screen::Finale
        ) {
            return Err("results required".into());
        }
        let next = if self.screen == Screen::LevelUp || self.screen == Screen::Finale {
            if self.mode == Mode::BigGame {
                Screen::Street
            } else {
                Screen::Courses
            }
        } else if self.settlement.as_ref().is_some_and(|s| s.finale) {
            Screen::Finale
        } else if self.settlement.as_ref().is_some_and(|s| s.level_advanced) {
            Screen::LevelUp
        } else if self.mode == Mode::BigGame {
            Screen::Street
        } else {
            Screen::Courses
        };
        self.screen = next;
        self.stack = vec![Screen::Main];
        self.sync_career();
        Ok(())
    }
    pub fn confirm_purchase(&mut self, catalog: &Catalog) -> Result<crate::career::Purchase> {
        if self.screen != Screen::Purchase {
            return Err("purchase confirmation required".into());
        }
        let p = self.career.as_mut().ok_or("career required")?.purchase(
            catalog,
            self.selected_bike,
            self.quote_revision.ok_or("missing quote")?,
        )?;
        self.back();
        self.sync_career();
        Ok(p)
    }
    pub fn confirm_repair(&mut self, catalog: &Catalog) -> Result<u64> {
        if self.screen != Screen::Repair {
            return Err("repair confirmation required".into());
        }
        let cost = self
            .career
            .as_mut()
            .ok_or("career required")?
            .repair(catalog, self.quote_revision.ok_or("missing quote")?)?;
        self.back();
        Ok(cost)
    }
    /// Native Schmooze is dialogue selection, not a money/AI relationship mutation.
    pub fn read_dialogue(&mut self) -> Result<()> {
        if self.screen != Screen::Schmooze || self.dialogue_read {
            return Err("dialogue unavailable".into());
        }
        let partner = self.dialogue_partner.ok_or("no dialogue partner")?;
        self.dialogue_seen |= 1 << partner;
        self.dialogue_read = true;
        Ok(())
    }
    pub fn choose_track(&mut self, track: u8) -> Result<()> {
        if self.screen != Screen::Jukebox || track >= 6 {
            return Err("track unavailable".into());
        }
        self.settings.track = Some(track);
        Ok(())
    }
    pub fn return_main(&mut self) {
        self.screen = Screen::Main;
        self.stack.clear();
        self.summary = None;
        self.settlement = None;
        self.settings.track = None;
    }
}
