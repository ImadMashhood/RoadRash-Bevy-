use rr_render::{
    CameraSpec,
    camera::{View, encounter_snapshot},
    fixture,
};
use rr_simulation::{Command, Simulation, encounter::Setup, fixtures, recovery::Impact};

#[test]
fn new_source_crash_draws_each_node_only_after_native_descriptor_selection() {
    let config = fixtures::riding();
    let mut setup = Setup::authored(&config).unwrap();
    setup.rules.combat_version = 1;
    let mut sim = Simulation::with_encounters(1, config, setup).unwrap();
    sim.step(Command {
        impact: Some(Impact::default()),
        ..Default::default()
    })
    .unwrap();
    let canonical = sim.snapshot();
    let hash = sim.state_hash();
    let mut source = canonical.clone();
    let unit = source.actors[0].encounter.as_mut().unwrap();
    unit.source_route = Some(0);
    unit.recovery.body.as_mut().unwrap().animation.selected = None;
    unit.recovery.bike_animation.selected = None;
    let scene = fixture::scene("straight").unwrap();
    let view = View {
        position_q8: 50. * 256.,
        ..Default::default()
    };
    let render = |snapshot: &rr_simulation::Snapshot| {
        scene
            .draw(
                &CameraSpec::default(),
                &view,
                &encounter_snapshot(snapshot),
                false,
            )
            .unwrap()
            .0
            .image
            .pixels
    };
    let hidden = render(&source);
    let mut empty = source.clone();
    empty.actors.clear();
    assert_eq!(
        hidden,
        render(&empty),
        "unselected initial crash states must not draw the riding-frame default"
    );
    source.actors[0]
        .encounter
        .as_mut()
        .unwrap()
        .recovery
        .body
        .as_mut()
        .unwrap()
        .animation
        .selected = Some((0, 0));
    let body = render(&source);
    assert_ne!(body, hidden);
    source.actors[0]
        .encounter
        .as_mut()
        .unwrap()
        .recovery
        .body
        .as_mut()
        .unwrap()
        .animation
        .selected = None;
    source.actors[0]
        .encounter
        .as_mut()
        .unwrap()
        .recovery
        .bike_animation
        .selected = Some((0, 0));
    let bike = render(&source);
    assert_ne!(bike, hidden);
    assert_ne!(
        body, bike,
        "body and quiet bike retain independent positions"
    );
    let projected = encounter_snapshot(&source);
    assert!(
        projected.actors.iter().any(|actor| actor.id.0 == 1),
        "camera owner must remain in presentation state while its pixels are hidden"
    );
    assert_eq!(sim.snapshot(), canonical);
    assert_eq!(sim.state_hash(), hash);
}
