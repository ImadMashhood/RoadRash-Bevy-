//! Reconciles presentation-owned siren voices; Phase 16 supplies audio playback.
use bevy::prelude::*;
use rr_simulation::population::{Cue, SirenBus};
#[derive(Resource, Default)]
pub struct Warnings {
    bus: SirenBus,
    pub events: Vec<Cue>,
}
pub fn update(
    view: Res<rr_presentation::PresentedSnapshot>,
    pause: Res<super::PauseState>,
    options: Res<super::LaunchOptions>,
    mut warnings: ResMut<Warnings>,
) {
    let desired = view
        .0
        .road_users
        .iter()
        .filter(|a| !pause.paused && options.effects && a.siren)
        .map(|a| a.id);
    warnings.events = warnings.bus.reconcile(desired);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bevy_pause_and_effect_toggle_stop_the_owned_loop() {
        use rr_simulation::{population::*, topology::*};
        let mut population = World::new(Setup::default());
        let id = population
            .spawn(
                &Spawn {
                    id: 1,
                    location: Location {
                        progress: 1000,
                        ..Default::default()
                    },
                    kind: Kind::Police,
                    family: None,
                    selector: 0,
                    subtype: 0,
                    speed: 0,
                    direction: 1,
                },
                0,
                &Topology::straight(10000),
            )
            .unwrap()
            .unwrap();
        population.agents.get_mut(&id).unwrap().siren = true;
        let snapshot = rr_simulation::Snapshot {
            road_users: population.agents.values().cloned().collect(),
            ..Default::default()
        };
        let mut app = App::new();
        app.insert_resource(rr_presentation::PresentedSnapshot(
            snapshot.clone(),
            snapshot,
            0.,
        ))
        .init_resource::<super::super::PauseState>()
        .insert_resource(super::super::LaunchOptions {
            effects: true,
            ..Default::default()
        })
        .init_resource::<Warnings>()
        .add_systems(Update, update);
        app.update();
        assert_eq!(
            app.world().resource::<Warnings>().events,
            vec![Cue::SirenStart(id)]
        );
        app.world_mut()
            .resource_mut::<super::super::PauseState>()
            .paused = true;
        app.update();
        assert_eq!(
            app.world().resource::<Warnings>().events,
            vec![Cue::SirenStop(id)]
        );
        app.world_mut()
            .resource_mut::<super::super::PauseState>()
            .paused = false;
        app.update();
        assert_eq!(
            app.world().resource::<Warnings>().events,
            vec![Cue::SirenStart(id)]
        );
        app.world_mut()
            .resource_mut::<super::super::LaunchOptions>()
            .effects = false;
        app.update();
        assert_eq!(
            app.world().resource::<Warnings>().events,
            vec![Cue::SirenStop(id)]
        );
        app.update();
        assert!(app.world().resource::<Warnings>().events.is_empty());
    }
}
