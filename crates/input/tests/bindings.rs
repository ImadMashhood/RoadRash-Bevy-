use rr_input::{Action::*, ActionMap, Context::*, Key::*};
#[test]
fn defaults_resolve_actions_and_allow_context_specific_keys() {
    let map = ActionMap::default();
    map.validate().unwrap();
    assert!(map.active(Race, [KeyW]).contains(&Accelerate));
    assert!(map.active(Menu, [KeyW]).contains(&NavigateUp));
    assert!(map.active(Paused, [Enter]).contains(&Resume));
    assert_eq!(
        map.active(Race, [ArrowLeft, ArrowRight]),
        [SteerLeft, SteerRight].into_iter().collect()
    );
}
#[test]
fn conflicts_name_both_actions_and_do_not_modify_a_valid_map() {
    let mut map = ActionMap::default();
    let before = map.clone();
    let error = map.rebind(Race, Brake, vec![KeyW]).unwrap_err().to_string();
    assert!(error.contains("Accelerate") && error.contains("Brake") && error.contains("KeyW"));
    assert_eq!(before, map);
    assert!(map.rebind(Race, Accelerate, vec![Space, Space]).is_err());
    assert!(map.rebind(Race, Accelerate, vec![]).is_err());
    assert!(map.rebind(Race, Confirm, vec![Space]).is_err());
}
#[test]
fn restoring_one_context_preserves_other_customizations() {
    let mut map = ActionMap::default();
    map.rebind(Race, Accelerate, vec![Space]).unwrap();
    map.rebind(Menu, Confirm, vec![KeyQ]).unwrap();
    map.reset(Some(Race));
    assert_eq!(map.contexts[&Race], ActionMap::default().contexts[&Race]);
    assert_eq!(map.contexts[&Menu][&Confirm], vec![KeyQ]);
    map.reset(None);
    assert_eq!(map, ActionMap::default());
}
#[test]
fn schemas_missing_actions_and_unknown_keys_are_rejected() {
    let mut map = ActionMap {
        schema_version: 2,
        ..ActionMap::default()
    };
    assert!(map.validate().is_err());
    map = ActionMap::default();
    map.contexts.get_mut(&Race).unwrap().remove(&Accelerate);
    assert!(map.validate().is_err());
    let valid = serde_json::to_string(&ActionMap::default()).unwrap();
    assert!(serde_json::from_str::<ActionMap>(&valid.replace("KeyW", "UnknownKey")).is_err());
    assert!(
        serde_json::from_str::<ActionMap>(&valid.replace(
            "\"schema_version\":1",
            "\"schema_version\":1,\"unexpected\":0"
        ))
        .is_err()
    );
}
#[test]
fn validated_config_round_trips_and_rejected_writes_preserve_previous_file() {
    let root = std::env::temp_dir().join(format!("rr-input-bindings-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("bindings.json");
    let mut map = ActionMap::default();
    map.save(&path).unwrap();
    let old = std::fs::read(&path).unwrap();
    assert_eq!(ActionMap::read(&path).unwrap(), map);
    map.rebind(Race, Accelerate, vec![Space]).unwrap();
    map.save(&path).unwrap();
    assert_eq!(ActionMap::read(&path).unwrap(), map);
    let good = std::fs::read(&path).unwrap();
    map.schema_version = 999;
    assert!(map.save(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), good);
    assert_ne!(good, old);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    std::fs::write(&path, vec![b' '; 65537]).unwrap();
    assert!(ActionMap::read(&path).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
