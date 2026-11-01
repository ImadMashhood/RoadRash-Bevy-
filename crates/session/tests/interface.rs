use rr_session::{flow::Settings, persistence::preferences::Preferences, text::*, ui::*};
use std::collections::BTreeMap;
#[test]
fn stable_ui_keys_translate_parameterized_values_for_all_five_languages() {
    let catalog = Catalog::built_in();
    catalog.validate().unwrap();
    let values = BTreeMap::from([
        ("level", "5".into()),
        ("cash", "12500".into()),
        ("condition", "90".into()),
        ("maximum", "100".into()),
    ]);
    for language in Language::ALL {
        let text = catalog
            .message("career_summary", language, &values)
            .unwrap();
        assert!(text.contains("12500") && text.contains("90/100"));
        assert!(!catalog.label("BACK", language).is_empty());
        assert!(
            catalog
                .message("career_summary", language, &BTreeMap::new())
                .is_err()
        );
    }
    assert_eq!(
        catalog.label("LANGUAGE: English", Language::German),
        "SPRACHE: English"
    );
}
#[test]
fn layout_wraps_long_unbroken_glyphs_and_keeps_focus_budget_at_all_scales() {
    for scale in [75, 100, 125, 150] {
        let width = 68 * 100 / usize::from(scale);
        for line in wrap(
            "Übergrößenträger très_longue_étiquette_avec_des_accents_éééééééééééééééééééééééééé",
            width,
        ) {
            assert!(line.chars().count() <= width);
        }
        assert!((3..=12).contains(&visible_rows(scale, 10)));
    }
}
#[test]
fn accessibility_preferences_roundtrip_and_graphics_reset_preserves_them() {
    let mut p = Preferences {
        settings: Settings {
            language: Language::French,
            text_scale: 150,
            high_contrast: true,
            reduced_motion: true,
            reduced_flashes: true,
            subtitles: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let (mut value, warnings) = Preferences::decode(&p.encode().unwrap()).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(value.settings, p.settings);
    value.settings.reset_graphics();
    assert_eq!(value.settings, p.settings);
    p.settings.text_scale = 151;
    assert!(p.encode().is_err());
}
#[test]
fn old_custom_binding_keeps_priority_over_new_optional_defaults() {
    let input = br#"{"bindings":{"schema_version":1,"contexts":{"race":{"accelerate":["KeyQ"]}}}}"#;
    let (p, warnings) = Preferences::decode(input).unwrap();
    assert_eq!(
        p.bindings.contexts[&rr_input::Context::Race][&rr_input::Action::Accelerate],
        vec![rr_input::Key::KeyQ]
    );
    assert!(!warnings.is_empty());
    p.bindings.validate().unwrap();
}

#[test]
fn hud_checkpoints_use_native_speed_and_exact_snapshot_health_damage_and_gear() {
    let c = rr_simulation::fixtures::riding();
    let e = rr_simulation::encounter::Setup::authored(&c).unwrap();
    let simulation = rr_simulation::Simulation::with_encounters(7, c, e).unwrap();
    let before = simulation.state_hash();
    let mut snapshot = simulation.snapshot();
    let actor = &mut snapshot.actors[0];
    let rider = actor.rider.as_mut().unwrap();
    rider.bike.speed = 12345;
    rider.bike.gear = 2;
    rider.bike.condition = 75;
    actor.encounter.as_mut().unwrap().fighter.stats.health = 2048;
    actor.encounter.as_mut().unwrap().fighter.stats.maximum = 4096;
    let actual = HudValues::from_snapshot(&snapshot, 200).unwrap();
    assert_eq!(
        actual,
        HudValues {
            speed: 123,
            gear: 3,
            health: 2048,
            maximum: 4096,
            condition_percent: 37,
            nitro: 0
        }
    );
    assert_eq!(simulation.state_hash(), before);
    let catalog = Catalog::built_in();
    let values = BTreeMap::from([
        ("speed", "123".into()),
        ("gear", "3".into()),
        ("health", "2048".into()),
        ("maximum", "4096".into()),
        ("condition", "37".into()),
        ("nitro", "0".into()),
    ]);
    for language in Language::ALL {
        for key in ["hud", "hud_small"] {
            let text = catalog.message(key, language, &values).unwrap();
            assert!(text.contains("123") && text.contains("2048/4096") && text.contains("37%"));
        }
    }
}
#[test]
fn plural_selection_and_large_integer_grouping_do_not_use_float_or_layout_concatenation() {
    let c = Catalog::built_in();
    assert_eq!(number(1234567890, Language::German), "1.234.567.890");
    assert_eq!(number(1234567890, Language::French), "1 234 567 890");
    assert_eq!(
        c.counted("racer", 0, Language::English).unwrap(),
        "0 riders"
    );
    assert_eq!(c.counted("racer", 1, Language::English).unwrap(), "1 rider");
    assert_eq!(
        c.counted("racer", 2, Language::Italian).unwrap(),
        "2 piloti"
    );
    assert_eq!(c.counted("racer", 0, Language::French).unwrap(), "0 pilote");
    let mut invalid = Catalog::built_in();
    invalid.entries[0].labels[1] = "{missing}".into();
    assert!(invalid.validate().is_err());
}
