use rr_session::{
    career::{Career, Purchase},
    catalog::Catalog,
    flow::{Frontend, Screen},
    race::*,
};
use rr_simulation::{
    ActorId, Command, Simulation, Snapshot,
    recovery::Outcome as Terminal,
    topology::{Location, Path, Portal, Topology},
};
use std::collections::BTreeMap;
fn identity(id: u64, mode: Mode, level: u8, course: u8, bike: u8, count: u64) -> RaceIdentity {
    RaceIdentity {
        id,
        mode,
        level,
        course,
        bike,
        seed: 1,
        roster: (1..=count)
            .map(|n| Entry {
                actor: ActorId(n),
                name: format!("Rider {n}"),
            })
            .collect(),
    }
}
fn snap(count: u64) -> Snapshot {
    let mut sim = Simulation::default();
    for _ in 1..count {
        sim.spawn().unwrap();
    }
    sim.snapshot()
}
fn result(
    id: u64,
    mode: Mode,
    level: u8,
    course: u8,
    bike: u8,
    place: u8,
    outcome: Outcome,
) -> Summary {
    let mut snapshot = snap(15);
    let mut session = RaceSession::new(
        identity(id, mode, level, course, bike, 15),
        None,
        &snapshot,
        0,
        if outcome == Outcome::TimedOut {
            Some(1)
        } else {
            None
        },
    )
    .unwrap();
    for a in &mut snapshot.actors {
        a.position.progress.0 = if a.id == ActorId(1) {
            100
        } else if a.id.0 <= u64::from(place) {
            200
        } else {
            0
        };
        if a.id != ActorId(1) && a.id.0 <= u64::from(place) {
            a.finished = true;
        }
    }
    snapshot.tick = 1;
    if outcome == Outcome::Quit {
        session.quit(&snapshot);
    } else {
        session.observe(&snapshot);
        snapshot.tick = 2;
        snapshot
            .actors
            .iter_mut()
            .find(|a| a.id == ActorId(1))
            .unwrap()
            .finished = true;
        session.observe(&snapshot);
    }
    session.summary().unwrap().clone()
}
fn terminal_result(id: u64, level: u8, bike: u8, terminal: Terminal) -> Summary {
    let replay = rr_simulation::replay::Replay::read(include_bytes!(
        "../../../docs/fixtures/encounter-replay.json"
    ))
    .unwrap();
    let mut sim = rr_simulation::replay::Playback::new(replay)
        .unwrap()
        .simulation;
    let mut session = RaceSession::new(
        identity(
            id,
            Mode::BigGame,
            level,
            1,
            bike,
            sim.snapshot().actors.len() as u64,
        ),
        sim.encounters().map(|w| w.setup.topology.clone()),
        &sim.snapshot(),
        0,
        None,
    )
    .unwrap();
    sim.step(Command {
        terminal: Some(terminal),
        ..Default::default()
    })
    .unwrap();
    session.observe(&sim.snapshot());
    session.summary().unwrap().clone()
}
#[test]
fn countdown_freezes_every_actor_rng_and_held_controls_until_tick_181() {
    let mut sim = Simulation::default();
    let hash = sim.state_hash();
    let mut session = RaceSession::new(
        identity(1, Mode::Thrash, 1, 1, 0, 1),
        None,
        &sim.snapshot(),
        180,
        None,
    )
    .unwrap();
    for _ in 0..180 {
        assert!(!session.permit_tick());
        assert_eq!(sim.state_hash(), hash);
    }
    assert!(session.permit_tick());
    sim.step(Command {
        throttle: true,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(sim.tick().0, 1);
    assert_ne!(sim.state_hash(), hash);
}
#[test]
fn invalid_setup_is_rejected() {
    let snapshot = snap(1);
    for bad in [
        identity(0, Mode::Thrash, 1, 1, 0, 1),
        identity(1, Mode::Thrash, 0, 1, 0, 1),
        identity(1, Mode::Thrash, 1, 6, 0, 1),
        identity(1, Mode::Thrash, 1, 1, 15, 1),
    ] {
        assert!(RaceSession::new(bad, None, &snapshot, 0, None).is_err());
    }
    assert!(
        RaceSession::new(
            identity(1, Mode::Thrash, 1, 1, 0, 1),
            None,
            &snapshot,
            601,
            None
        )
        .is_err()
    );
}
#[test]
fn stable_standings_follow_overtakes_and_equal_progress() {
    let mut snapshot = snap(3);
    let mut s = RaceSession::new(
        identity(1, Mode::Thrash, 1, 1, 0, 3),
        None,
        &snapshot,
        0,
        None,
    )
    .unwrap();
    assert_eq!(
        s.standings().iter().map(|r| r.actor.0).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    snapshot.actors[2].position.progress.0 = 300;
    s.observe(&snapshot);
    assert_eq!(s.standings()[0].actor, ActorId(3));
    snapshot.actors[1].position.progress.0 = 400;
    s.observe(&snapshot);
    assert_eq!(s.standings()[0].actor, ActorId(2));
}
#[test]
fn finish_ties_use_tick_then_stable_actor_id_and_freeze_results() {
    let mut snapshot = snap(3);
    let mut s = RaceSession::new(
        identity(1, Mode::Thrash, 1, 1, 0, 3),
        None,
        &snapshot,
        0,
        None,
    )
    .unwrap();
    snapshot.tick = 77;
    for a in &mut snapshot.actors {
        a.finished = true;
    }
    snapshot.actors.reverse();
    s.observe(&snapshot);
    let frozen = s.summary().unwrap().clone();
    assert_eq!(frozen.place(), 1);
    assert_eq!(
        frozen
            .standings()
            .iter()
            .map(|r| r.actor.0)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    snapshot.actors.clear();
    s.observe(&snapshot);
    s.quit(&snapshot);
    assert_eq!(s.summary().unwrap(), &frozen);
    assert!(!s.permit_tick());
}
#[test]
fn native_top_three_qualify_fourth_loses() {
    for place in 1..=15 {
        let r = result(1, Mode::BigGame, 1, 1, 0, place, Outcome::Won);
        assert_eq!(r.place(), place);
        assert_eq!(
            r.outcome(),
            if place <= 3 {
                Outcome::Won
            } else {
                Outcome::Lost
            }
        );
    }
}
#[test]
fn timeout_and_quit_are_explicit_non_finisher_outcomes() {
    let mut snapshot = snap(1);
    snapshot.actors[0].finished = false;
    let mut s = RaceSession::new(
        identity(1, Mode::Thrash, 1, 1, 0, 1),
        None,
        &snapshot,
        0,
        Some(2),
    )
    .unwrap();
    snapshot.tick = 1;
    s.observe(&snapshot);
    assert!(s.summary().is_none());
    snapshot.tick = 2;
    s.observe(&snapshot);
    assert_eq!(s.summary().unwrap().outcome(), Outcome::TimedOut);
    let q = result(1, Mode::Thrash, 1, 1, 0, 1, Outcome::Quit);
    assert_eq!(q.outcome(), Outcome::Quit);
}
fn branch_topology() -> Topology {
    let edge = |end, target, target_end| Portal {
        end,
        target,
        target_end,
    };
    Topology {
        paths: BTreeMap::from([
            (
                0,
                Path {
                    length: 100,
                    selected_start: Some(0),
                    branch: 0,
                    rises: vec![],
                    portals: vec![edge(true, 1, false), edge(true, 2, false)],
                },
            ),
            (
                1,
                Path {
                    length: 100,
                    selected_start: Some(100),
                    branch: 0,
                    rises: vec![],
                    portals: vec![edge(false, 0, true), edge(true, 3, false)],
                },
            ),
            (
                2,
                Path {
                    length: 400,
                    selected_start: None,
                    branch: 1,
                    rises: vec![],
                    portals: vec![edge(false, 0, true), edge(true, 3, false)],
                },
            ),
            (
                3,
                Path {
                    length: 100,
                    selected_start: Some(200),
                    branch: 0,
                    rises: vec![],
                    portals: vec![edge(false, 1, true), edge(false, 2, true)],
                },
            ),
            (
                4,
                Path {
                    length: 100,
                    selected_start: None,
                    branch: 2,
                    rises: vec![],
                    portals: vec![],
                },
            ),
        ]),
    }
}
#[test]
fn unequal_branches_rank_by_fraction_between_rejoin_anchors() {
    let map = ProgressMap::new(branch_topology()).unwrap();
    let at = |path, progress| Location {
        path,
        progress,
        ..Default::default()
    };
    assert_eq!(map.at(at(1, 50)), Some(150));
    assert_eq!(map.at(at(2, 200)), Some(150));
    assert_eq!(map.at(at(2, 300)), Some(175));
    assert_eq!(map.at(at(2, 400)), Some(200));
    assert_eq!(map.at(at(3, 0)), Some(200));
    assert_eq!(map.at(at(4, 99)), None);
    assert_eq!(map.at(at(2, 401)), None);
}
#[test]
fn finish_boundary_and_overshoot_are_caught_by_real_simulation() {
    for travel in [0, 1, 1000] {
        let replay = rr_simulation::replay::Replay::read(include_bytes!(
            "../../../docs/fixtures/encounter-replay.json"
        ))
        .unwrap();
        let config = replay.handling.unwrap();
        let mut setup = replay.encounter.unwrap();
        setup.rules.finish_progress = Some(10);
        setup.seeds.truncate(1);
        setup
            .inventory
            .items
            .retain(|_, i| i.owner == Some(ActorId(1)));
        setup.seeds[0].distance = if travel == 0 { 10 } else { 9 };
        setup.seeds[0].speed = travel * 256;
        let mut sim = Simulation::with_encounters(1, config, setup).unwrap();
        sim.step(Command {
            throttle: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            sim.encounters().unwrap().units[&ActorId(1)]
                .recovery
                .outcome,
            Some(Terminal::Finished)
        );
    }
}
#[test]
fn terminal_priority_arrest_and_wreck_propagates_to_results() {
    for (t, o) in [
        (Terminal::Arrested, Outcome::Arrested),
        (Terminal::Wrecked, Outcome::Wrecked),
    ] {
        let r = terminal_result(1, 1, 0, t);
        assert_eq!(r.outcome(), o);
    }
}
#[test]
fn rewards_are_exactly_once_even_after_serialized_resume() {
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 1).unwrap();
    let id = c.allocate_race().unwrap();
    let r = result(id, Mode::BigGame, 1, 1, c.bike, 2, Outcome::Won);
    let report = c.settle(&cat, &r).unwrap().unwrap();
    assert_eq!(report.prize, u64::from(cat.prizes[1]));
    let frozen = c.clone();
    assert!(c.settle(&cat, &r).unwrap().is_none());
    assert_eq!(c, frozen);
    let mut loaded = Career::decode(&c.encode().unwrap(), &cat).unwrap();
    assert!(loaded.settle(&cat, &r).unwrap().is_none());
    assert_eq!(loaded, c);
}
#[test]
fn thrash_cannot_modify_career_cash_or_unlocks() {
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 1).unwrap();
    let before = c.clone();
    for place in 1..=15 {
        assert!(
            c.settle(&cat, &result(1, Mode::Thrash, 1, 1, 0, place, Outcome::Won))
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(c, before);
}
#[test]
fn penalties_affordability_and_reset_match_native_policy() {
    let cat = Catalog::authored();
    for terminal in [Terminal::Arrested, Terminal::Wrecked] {
        let charge = if terminal == Terminal::Arrested {
            400
        } else {
            100
        };
        for cash in [charge - 1, charge, charge + 1] {
            let mut c = Career::new(&cat, 1).unwrap();
            c.cash = cash;
            let id = c.allocate_race().unwrap();
            let r = terminal_result(id, 1, c.bike, terminal);
            let receipt = c.settle(&cat, &r).unwrap().unwrap();
            assert_eq!(receipt.penalty, charge);
            assert_eq!(receipt.game_over, cash < charge);
            assert_eq!(c.cash, if cash < charge { 1000 } else { cash - charge });
        }
    }
}
#[test]
fn purchase_trade_in_exact_cash_owned_and_stale_confirmations() {
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 1).unwrap();
    c.cash = 999;
    let before = c.clone();
    assert!(c.purchase(&cat, 1, c.revision()).is_err());
    assert_eq!(before, c);
    c.cash = 1000;
    let revision = c.revision();
    assert_eq!(c.purchase(&cat, 1, revision).unwrap(), Purchase::Bought);
    assert_eq!(c.cash, 0);
    assert!(c.purchase(&cat, 2, revision).is_err());
    let before = c.clone();
    assert_eq!(
        c.purchase(&cat, 1, c.revision()).unwrap(),
        Purchase::AlreadyOwned
    );
    assert_eq!(c, before);
}
#[test]
fn repair_rounding_exact_cash_and_double_confirmation() {
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 1).unwrap();
    c.condition = 49;
    c.cash = 50;
    assert_eq!(c.repair_quote(&cat).unwrap(), 51);
    let before = c.clone();
    assert!(c.repair(&cat, c.revision()).is_err());
    assert_eq!(before, c);
    c.cash = 51;
    let rev = c.revision();
    assert_eq!(c.repair(&cat, rev).unwrap(), 51);
    assert_eq!(c.cash, 0);
    assert_eq!(c.condition, 100);
    assert!(c.repair(&cat, rev).is_err());
    assert_eq!(c.repair(&cat, c.revision()).unwrap(), 0);
}
#[test]
fn all_characters_and_invalid_ids_validate_after_resume() {
    let cat = Catalog::authored();
    for id in 1..=8 {
        let c = Career::new(&cat, id).unwrap();
        assert_eq!(Career::decode(&c.encode().unwrap(), &cat).unwrap(), c);
    }
    for id in [0, 9, 255] {
        assert!(Career::new(&cat, id).is_err());
    }
    let c = Career::new(&cat, 1).unwrap();
    for field in ["character", "bike", "level", "qualified", "condition"] {
        let mut v: serde_json::Value = serde_json::from_slice(&c.encode().unwrap()).unwrap();
        v["career"][field] = serde_json::json!(255);
        assert!(
            Career::decode(&serde_json::to_vec(&v).unwrap(), &cat).is_err(),
            "{field}"
        );
    }
}
#[test]
fn snapshot_rejects_unknown_versions_fields_and_oversize() {
    let cat = Catalog::authored();
    assert!(Career::decode(&[b' '; 8193], &cat).is_err());
    let c = Career::new(&cat, 1).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&c.encode().unwrap()).unwrap();
    v["schema_version"] = 2.into();
    assert!(Career::decode(&serde_json::to_vec(&v).unwrap(), &cat).is_err());
    v["schema_version"] = 1.into();
    v["extra"] = true.into();
    assert!(Career::decode(&serde_json::to_vec(&v).unwrap(), &cat).is_err());
}
#[test]
fn five_course_qualification_advances_exactly_once_at_all_five_levels() {
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 1).unwrap();
    for level in 1..=5 {
        for course in 1..=5 {
            let id = c.allocate_race().unwrap();
            let r = result(id, Mode::BigGame, level, course, c.bike, 3, Outcome::Won);
            let receipt = c.settle(&cat, &r).unwrap().unwrap();
            assert_eq!(receipt.level_advanced, course == 5 && level < 5);
            assert_eq!(receipt.finale, course == 5 && level == 5);
            if course < 5 {
                assert_eq!(c.level, level);
                assert!(!c.completed);
            }
        }
    }
    assert_eq!(c.level, 5);
    assert!(c.completed);
}
#[test]
fn duplicate_course_and_fourth_place_do_not_unlock_next_level() {
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 1).unwrap();
    for (place, course) in [(1, 1), (1, 1), (4, 2), (3, 3), (2, 4), (1, 5)] {
        let id = c.allocate_race().unwrap();
        c.settle(
            &cat,
            &result(id, Mode::BigGame, 1, course, c.bike, place, Outcome::Won),
        )
        .unwrap();
    }
    assert_eq!(c.level, 1);
    assert_eq!(c.qualified, 29);
}
#[test]
fn wrong_or_unissued_race_results_cannot_spend_or_reward() {
    let cat = Catalog::authored();
    let mut c = Career::new(&cat, 1).unwrap();
    let before = c.clone();
    assert!(
        c.settle(&cat, &result(1, Mode::BigGame, 1, 1, 0, 1, Outcome::Won))
            .is_err()
    );
    assert_eq!(c, before);
    c.allocate_race().unwrap();
    assert!(
        c.settle(&cat, &result(1, Mode::BigGame, 2, 1, 0, 1, Outcome::Won))
            .is_err()
    );
    assert!(
        c.settle(&cat, &result(1, Mode::BigGame, 1, 1, 1, 1, Outcome::Won))
            .is_err()
    );
}
#[test]
fn frontend_typed_back_graph_covers_every_menu_destination() {
    let cat = Catalog::authored();
    let mut f = Frontend::default();
    for s in [Screen::Restroom, Screen::Credits, Screen::QuitConfirm] {
        f.open(s).unwrap();
        f.back();
        assert_eq!(f.screen, Screen::Main);
    }
    f.begin(Mode::BigGame);
    f.choose_character(&cat, 1).unwrap();
    for s in [
        Screen::Courses,
        Screen::Club,
        Screen::Showroom,
        Screen::Restroom,
        Screen::Repair,
    ] {
        f.open(s).unwrap();
        f.back();
        assert_eq!(f.screen, Screen::Street);
    }
    f.open(Screen::Restroom).unwrap();
    for s in [
        Screen::Settings,
        Screen::Controls,
        Screen::Credits,
        Screen::Jukebox,
        Screen::Characters,
        Screen::Courses,
    ] {
        f.open(s).unwrap();
        f.back();
        assert_eq!(f.screen, Screen::Restroom);
    }
    f.back();
    f.open(Screen::Showroom).unwrap();
    f.open(Screen::Bike).unwrap();
    f.open(Screen::Purchase).unwrap();
    f.back();
    assert_eq!(f.screen, Screen::Bike);
    f.back();
    f.back();
    f.open(Screen::Club).unwrap();
    f.open(Screen::Schmooze).unwrap();
    f.back();
    assert_eq!(f.screen, Screen::Club);
    assert!(f.open(Screen::Purchase).is_err());
}
#[test]
fn career_resume_and_quick_mode_keep_progress_separate() {
    let cat = Catalog::authored();
    let mut f = Frontend::default();
    f.begin(Mode::BigGame);
    f.choose_character(&cat, 2).unwrap();
    f.career.as_mut().unwrap().cash = 7777;
    let frozen = f.career.clone();
    f.return_main();
    f.begin(Mode::Thrash);
    f.choose_character(&cat, 3).unwrap();
    f.select_level(&cat, 5).unwrap();
    assert_eq!(f.career, frozen);
    f.return_main();
    f.begin(Mode::BigGame);
    assert_eq!(f.screen, Screen::Street);
    assert_eq!(f.character, 2);
    assert_eq!(f.level, 1);
    assert_eq!(f.career.unwrap().cash, 7777);
}
#[test]
fn course_selection_rejects_locked_levels_and_marks_qualified_tracks() {
    let cat = Catalog::authored();
    let mut f = Frontend::default();
    f.begin(Mode::BigGame);
    f.choose_character(&cat, 1).unwrap();
    f.open(Screen::Courses).unwrap();
    assert!(f.select_level(&cat, 2).is_err());
    assert!(f.select_course(0).is_err());
    assert!(f.select_course(6).is_err());
    assert!(f.select_level(&cat, 1).is_ok());
    f.select_course(5).unwrap();
    f.loaded().unwrap();
    assert_eq!(f.screen, Screen::Racing);
}
#[test]
fn dialogue_cancel_repeat_and_unavailable_choice_are_safe() {
    let cat = Catalog::authored();
    let mut f = Frontend::default();
    f.begin(Mode::BigGame);
    f.choose_character(&cat, 1).unwrap();
    let career = f.career.clone();
    f.open(Screen::Club).unwrap();
    f.open(Screen::Schmooze).unwrap();
    f.back();
    assert_eq!(f.dialogue_seen, 1);
    for i in 1..8 {
        f.open(Screen::Schmooze).unwrap();
        assert_eq!(f.dialogue_partner, Some(i));
        f.read_dialogue().unwrap();
        assert!(f.read_dialogue().is_err());
        f.back();
    }
    assert_eq!(f.dialogue_seen, 255);
    f.open(Screen::Schmooze).unwrap();
    assert_eq!(f.dialogue_partner, Some(1));
    assert_eq!(f.career, career);
}
#[test]
fn jukebox_and_settings_return_to_owner_with_selection_preserved() {
    let mut f = Frontend::default();
    f.open(Screen::Restroom).unwrap();
    for screen in [Screen::Settings, Screen::Credits, Screen::Jukebox] {
        f.open(screen).unwrap();
        if screen == Screen::Jukebox {
            f.choose_track(5).unwrap();
            assert!(f.choose_track(14).is_err());
        }
        f.back();
        assert_eq!(f.screen, Screen::Restroom);
    }
    assert_eq!(f.settings.track, Some(5));
    f.return_main();
    assert_eq!(f.settings.track, None);
}
#[test]
fn results_return_does_not_reapply_settlement_or_hold_live_world() {
    let cat = Catalog::authored();
    let mut f = Frontend::default();
    f.begin(Mode::BigGame);
    f.choose_character(&cat, 1).unwrap();
    let id = f.career.as_mut().unwrap().allocate_race().unwrap();
    f.open(Screen::Courses).unwrap();
    f.select_course(1).unwrap();
    f.loaded().unwrap();
    f.completed(&cat, result(id, Mode::BigGame, 1, 1, 0, 1, Outcome::Won))
        .unwrap();
    let frozen = f.career.clone();
    f.open(Screen::Standings).unwrap();
    f.return_from_results().unwrap();
    assert_eq!(f.screen, Screen::Street);
    assert_eq!(f.career, frozen);
    assert!(
        f.completed(&cat, result(id, Mode::BigGame, 1, 1, 0, 1, Outcome::Won))
            .is_err()
    );
    assert_eq!(f.career, frozen);
}
#[test]
fn session_replay_preserves_finishes_results_checkpoints_and_effect_independence_at_four_rates() {
    let replay = rr_simulation::replay::Replay::read(include_bytes!(
        "../../../docs/fixtures/race-rules-replay.json"
    ))
    .unwrap();
    let baseline = rr_session::playback::run(replay.clone(), "60", true).unwrap();
    assert_eq!(baseline.summary.outcome(), Outcome::Won);
    assert!(!baseline.checkpoints.is_empty());
    assert_eq!(baseline.summary.place(), 1);
    assert!(baseline.simulation_ticks < 120);
    assert_eq!(baseline.session_ticks, 180 + baseline.simulation_ticks);
    for rate in ["30", "60", "144", "irregular"] {
        for effects in [false, true] {
            let r = rr_session::playback::run(replay.clone(), rate, effects).unwrap();
            assert_eq!(r.state_sha256, baseline.state_sha256);
            assert_eq!(r.result_sha256, baseline.result_sha256);
            assert_eq!(r.checkpoints, baseline.checkpoints);
            assert_eq!(r.summary, baseline.summary);
        }
    }
}
#[test]
fn quick_results_keep_quick_character_and_level_with_an_existing_career() {
    let cat = Catalog::authored();
    let mut f = Frontend::default();
    f.begin(Mode::BigGame);
    f.choose_character(&cat, 1).unwrap();
    let frozen = f.career.clone();
    f.return_main();
    f.begin(Mode::Thrash);
    f.choose_character(&cat, 3).unwrap();
    f.select_level(&cat, 5).unwrap();
    f.select_course(2).unwrap();
    f.loaded().unwrap();
    f.completed(
        &cat,
        result(1, Mode::Thrash, 5, 2, f.bike, 4, Outcome::Lost),
    )
    .unwrap();
    f.return_from_results().unwrap();
    assert_eq!(f.character, 3);
    assert_eq!(f.level, 5);
    assert_eq!(f.career, frozen);
}

#[test]
fn original_restroom_level_control_keeps_big_game_progress_locked() {
    let catalog = Catalog::authored();
    let mut front = Frontend::default();
    front.screen = Screen::Restroom;
    front.select_level(&catalog, 3).unwrap();
    assert_eq!(front.level, 3);
    front.return_main();
    front.begin(Mode::BigGame);
    front.choose_character(&catalog, 1).unwrap();
    front.open(Screen::Restroom).unwrap();
    assert!(front.select_level(&catalog, 2).is_err());
    assert_eq!(front.career.as_ref().unwrap().level, 1);
}

#[test]
fn original_menu_routes_keep_repair_credits_and_quit_accessible() {
    let catalog = Catalog::authored();
    let mut front = Frontend::default();
    front.begin(Mode::BigGame);
    front.choose_character(&catalog, 1).unwrap();
    front.open(Screen::Showroom).unwrap();
    front.open(Screen::Repair).unwrap();
    front.back();
    front.back();
    assert_eq!(front.screen, Screen::Street);
    front.open(Screen::Restroom).unwrap();
    front.open(Screen::Settings).unwrap();
    front.open(Screen::Credits).unwrap();
    front.back();
    front.open(Screen::QuitConfirm).unwrap();
    assert!(front.career.is_some());
}
