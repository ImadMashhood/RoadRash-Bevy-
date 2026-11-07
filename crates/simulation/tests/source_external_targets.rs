use rr_simulation::{
    Command, Simulation,
    combat::{AttackKind, Request, Side},
    encounter, fixtures,
    population::{Kind, Setup, Spawn},
    topology::Location,
};

fn world(progress: i64) -> Simulation {
    let config = fixtures::riding();
    let mut encounter = encounter::Setup::authored(&config).unwrap();
    encounter.rules.combat_version = 4;
    let mut simulation = Simulation::with_population(
        1,
        config,
        encounter,
        Setup {
            traffic: false,
            periodic_police: false,
            ..Default::default()
        },
    )
    .unwrap();
    simulation
        .spawn_road_user(&Spawn {
            id: 1,
            location: Location {
                progress,
                lateral: -50000,
                ..Default::default()
            },
            kind: Kind::Vehicle,
            family: None,
            selector: 0,
            subtype: 0,
            speed: 0,
            direction: 1,
        })
        .unwrap()
        .unwrap();
    simulation
}
fn attack() -> Command {
    Command {
        attack: Some(Request {
            kind: AttackKind::Punch,
            side: Side::Auto,
        }),
        ..Default::default()
    }
}

#[test]
fn player_faces_a_new_external_target_on_the_first_think() {
    let mut simulation = world(200);
    simulation.step(attack()).unwrap();
    assert_eq!(
        simulation.encounters().unwrap().units[&rr_simulation::ActorId(1)]
            .fighter
            .swing
            .as_ref()
            .unwrap()
            .request
            .side,
        Side::Left
    );
}

#[test]
fn removed_external_target_does_not_control_the_next_thinks_swing_side() {
    let mut simulation = world(0);
    simulation.step(Command::default()).unwrap();
    assert!(simulation.population().unwrap().agents.is_empty());
    simulation.step(attack()).unwrap();
    assert_eq!(
        simulation.encounters().unwrap().units[&rr_simulation::ActorId(1)]
            .fighter
            .swing
            .as_ref()
            .unwrap()
            .request
            .side,
        Side::Right
    );
}
