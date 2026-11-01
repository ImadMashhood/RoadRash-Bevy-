use crate::audio::{Bus, Rpm, pitch_q8};
use rr_simulation::{
    Snapshot,
    combat::{Effect, EffectKind},
    recovery::Phase,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
pub const TRACKS: [&str; 14] = [
    "HB_TRIP", "HB_PASS", "MM_DINO", "P_PANSY", "P_JESSIE", "P_BRIDGE", "SG_RUSTY", "SG_OUTSH",
    "SG_KICKS", "SG_SUPER", "SD_TRAIN", "SD_DUEL", "T_TEETH", "T_AUTO",
];
pub const ARTISTS: [(&str, usize, usize); 6] = [
    ("Hammerbox", 0, 2),
    ("Monster Magnet", 2, 1),
    ("Paw", 3, 3),
    ("Soundgarden", 6, 4),
    ("Swervedriver", 10, 2),
    ("Therapy?", 12, 2),
];
/// Original slot identities (intentional duplicate filenames retain distinct source slots).
pub const EFFECTS: [&str; 69] = [
    "ENGRAT", "ENGNITRO", "ENGTUNNE", "ENGOTHER", "ENGCAR11", "ENGOTHER", "ENGCAR11", "M_YEAH",
    "F_YEAH", "CHAINIMP", "CLUBIMPA", "BOUNCE", "CRASHHAR", "CRASHMED", "CRASHLIT", "GRUNTM1_",
    "GRUNTM2_", "GRUNTM3_", "GRUNTF1_", "GRUNTF2_", "GRUNTF3_", "KICK_MW4", "PUNCHHEA", "SWISHCHA",
    "SWISHGEN", "SKIDONNR", "SKIDOFFR", "FOOTSTEP", "RIDERSLI", "BMWLONG", "SIREN", "SIRENFST",
    "SK1_EATT", "BKM1_HEY", "BSW3_HEY", "FG2_EEK", "FG1_WATC", "BKM4_PTY", "SK1_DUDE", "TOUR_BAC",
    "DM_USOB", "BSM1_TAK", "BKW3_STP", "CM3_ARRG", "BKW1_DON", "HF_HELP", "DM_HOLYS", "OL_AAA",
    "AF1_EEK", "BKM1_HUH", "AM1_DOOO", "BSW1_NO", "CW_BACK", "BSW1_HUH", "BKM4_WAI", "JF_NO",
    "FCYC_WAT", "TOUR_AAA", "BKM2_HLD", "BKM4_RUM", "FM_SLOWD", "BSM1_WAT", "BSM3_WHA", "BSM2_WOA",
    "BSW2_AAA", "JM_AIIE", "BKW2_BIT", "CM1_HEYM", "OL_MAKE",
];
pub const ENGINE_VARIANTS: [&str; 3] = ["ENGRAT", "ENGSPORT", "ENGSUPER"];
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LoopId {
    Engine,
    Skid,
    Siren(u64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Voice {
    pub name: String,
    pub bus: Bus,
    pub pitch: u16,
    pub gain: f32,
    pub priority: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub version: u8,
    pub tick: u64,
    pub actor: u64,
    pub name: String,
    pub priority: u8,
}
#[derive(Default)]
pub struct Planner {
    pub rpm: Rpm,
    pub loops: BTreeMap<LoopId, Voice>,
    pub events: VecDeque<Event>,
    seen: BTreeSet<(u64, u64, u8)>,
    seen_order: VecDeque<(u64, u64, u8)>,
    last_tick: Option<u64>,
}
impl Planner {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn capture(
        &mut self,
        snapshot: &Snapshot,
        effects: &[Effect],
        ratio: i32,
        bike: u8,
        playing: bool,
        enabled: bool,
    ) {
        if self.last_tick.is_some_and(|t| snapshot.tick < t) {
            self.reset();
        }
        let fresh = self.last_tick != Some(snapshot.tick);
        self.last_tick = Some(snapshot.tick);
        self.loops.clear();
        if !playing {
            return;
        }
        if let Some(player) = snapshot.actors.iter().find(|a| a.id.0 == 1)
            && let Some(rider) = &player.rider
            && player
                .encounter
                .as_ref()
                .is_none_or(|u| u.recovery.outcome.is_none())
        {
            if fresh {
                self.rpm.update(
                    ratio,
                    rider.bike.speed,
                    rider.bike.height,
                    rider.bike.crashed,
                );
            }
            if player
                .encounter
                .as_ref()
                .is_none_or(|u| u.recovery.phase == Phase::Mounted)
            {
                self.loops.insert(
                    LoopId::Engine,
                    Voice {
                        name: if rider.actions.is_some_and(|s| s.active) {
                            "ENGNITRO"
                        } else {
                            ENGINE_VARIANTS[usize::from(bike.min(14)) / 5]
                        }
                        .into(),
                        bus: Bus::Engine,
                        pitch: pitch_q8(self.rpm.current, 22050),
                        gain: 0.25,
                        priority: 80,
                    },
                );
            }
            if enabled && rider.bike.skid != 0 {
                self.loops.insert(
                    LoopId::Skid,
                    Voice {
                        name: "SKIDONNR".into(),
                        bus: Bus::Effects,
                        pitch: 256,
                        gain: 0.15,
                        priority: 60,
                    },
                );
            }
        }
        if !enabled {
            return;
        }
        for cop in snapshot.road_users.iter().filter(|a| a.siren).take(1) {
            self.loops.insert(
                LoopId::Siren(cop.id),
                Voice {
                    name: "SIREN".into(),
                    bus: Bus::Effects,
                    pitch: 256,
                    gain: 0.2,
                    priority: 90,
                },
            );
        }
        for e in effects {
            let Some((name, priority)) = effect(e.kind) else {
                continue;
            };
            let key = (e.tick, e.actor.0, e.kind as u8);
            if !self.seen.insert(key) {
                continue;
            }
            self.seen_order.push_back(key);
            while self.seen_order.len() > 512 {
                if let Some(old) = self.seen_order.pop_front() {
                    self.seen.remove(&old);
                }
            }
            self.events.push_back(Event {
                version: 1,
                tick: e.tick,
                actor: e.actor.0,
                name: name.into(),
                priority,
            });
            if self.events.len() > 128 {
                let at = self
                    .events
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.priority)
                    .map(|(i, _)| i)
                    .unwrap();
                self.events.remove(at);
            }
        }
    }
}
pub fn effect(kind: EffectKind) -> Option<(&'static str, u8)> {
    use EffectKind::*;
    Some(match kind {
        Swing => ("SWISHGEN", 20),
        Hit => ("PUNCHHEA", 70),
        Kick => ("KICK_MW4", 70),
        Stolen => ("SWISHCHA", 45),
        Dropped => ("CLUBIMPA", 40),
        Crash => ("CRASHHAR", 100),
        Land => ("BOUNCE", 50),
        GetUp => ("FOOTSTEP", 25),
        Remount => ("BMWLONG", 40),
        Arrest => ("SIRENFST", 95),
        Finish => ("M_YEAH", 90),
        Wrecked => ("CRASHMED", 95),
        Rescue => return None,
        Contact => ("CRASHLIT", 60),
    })
}
/// Missing tracks are skipped with at most one complete catalog traversal.
pub fn next_track(start: usize, available: &[bool; 14]) -> Option<usize> {
    (0..14).map(|i| (start + i) % 14).find(|i| available[*i])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovieResult {
    Win,
    Lose,
    Busted,
    Wreck,
    Quiet,
}
/// Native filename families; variant selection is a caller-supplied presentation
/// draw, deliberately independent of the gameplay RNG.
pub fn result_movie(
    result: MovieResult,
    level_advanced: bool,
    finale: bool,
    draw: u32,
) -> Option<String> {
    if result == MovieResult::Win && finale {
        return Some("FINALWIN".into());
    }
    let (prefix, count) = match result {
        MovieResult::Win if level_advanced => ("LEVEL", 6),
        MovieResult::Win => ("WIN", 6),
        MovieResult::Lose => ("LOSE", 10),
        MovieResult::Busted => ("BUSTED", 6),
        MovieResult::Wreck => ("WRECK", 6),
        MovieResult::Quiet => return None,
    };
    Some(format!("{prefix}{}", draw % count + 1))
}
