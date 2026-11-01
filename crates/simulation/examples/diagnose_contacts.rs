//! Local replay diagnosis; output can contain private source-derived setup state.
use rr_simulation::replay::{Playback, Replay};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("replay path required")?;
    let r = Replay::read_file(std::path::Path::new(&path))?;
    let mut p = Playback::new(r)?;
    while !p.done() {
        p.simulation.advance()?;
        if p.simulation
            .effects()
            .iter()
            .any(|e| e.actor.0 == 1 && e.kind == rr_simulation::combat::EffectKind::Crash)
        {
            let s = p.simulation.snapshot();
            let u = s.actors[0].encounter.as_ref().unwrap();
            let at = u.recovery.bike_location;
            let near: Vec<_> = s
                .road_users
                .iter()
                .filter(|a| {
                    a.location.path == at.path && (a.location.progress - at.progress).abs() < 1024
                })
                .collect();
            println!(
                "{}",
                serde_json::json!({"tick":s.tick,"at":at,"phase":u.recovery.phase,"body":u.recovery.body,"effects":p.simulation.effects(),"population_contacts":p.simulation.population().map(|w|&w.contacts),"near":near,"contacts":p.simulation.encounters().unwrap().active_contacts})
            );
        }
    }
    Ok(())
}
