//! Native economy adapter and private catalog census; no extracted payload is published.
use rr_session::{
    career::{Career, Purchase},
    catalog::Catalog,
    race::{Entry, Mode, Outcome, RaceIdentity, Standing, Summary},
};
use rr_simulation::ActorId;
use serde_json::json;
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: rr-session-check DATA_ROOT OUTPUT_DIR".into());
    }
    let data = Path::new(&args[0]).canonicalize()?;
    let output = std::path::absolute(&args[1])?;
    let mut ancestor = output.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("output ancestor")?;
    }
    let resolved = ancestor
        .canonicalize()?
        .join(output.strip_prefix(ancestor)?);
    if resolved.starts_with(&data) || data.starts_with(&resolved) {
        return Err("output overlaps immutable source".into());
    }
    std::fs::create_dir_all(&resolved)?;
    let catalog = Catalog::from_exe(&std::fs::read(data.join("ROADRASH.EXE"))?)?;
    let mut characters = vec![];
    for c in &catalog.characters {
        characters.push(json!([c.id, c.personality, c.starting_cash, c.bikes]));
    }
    let prices: Vec<_> = catalog.bikes.iter().map(|b| b.price).collect();
    let mut purchases = vec![];
    for current in 0..15 {
        for target in 0..15 {
            let needed = u64::from(catalog.bike(target)?.price)
                .saturating_sub(u64::from(catalog.bike(current)?.price / 2));
            for cash in [0, needed.saturating_sub(1), needed, needed + 1] {
                let mut c = Career::new(&catalog, 1)?;
                c.bike = current;
                c.cash = cash;
                let status = match c.purchase(&catalog, target, c.revision()) {
                    Ok(Purchase::Bought) => 0,
                    Ok(Purchase::AlreadyOwned) => 1,
                    Err(_) => 2,
                };
                purchases.push(json!([status, c.cash, c.bike]));
            }
        }
    }
    let mut outcomes = vec![];
    for level in 1..=5 {
        for kind in 0..3 {
            for place in 1..=15 {
                for cash in [0, 400, 1_000_000] {
                    let mut career = Career::new(&catalog, 1)?;
                    career.level = level;
                    career.bike = 0;
                    career.cash = cash;
                    let id = career.allocate_race()?;
                    let outcome = match kind {
                        0 => {
                            if place <= 3 {
                                Outcome::Won
                            } else {
                                Outcome::Lost
                            }
                        }
                        1 => Outcome::Arrested,
                        _ => Outcome::Wrecked,
                    };
                    let roster: Vec<_> = (1..=15)
                        .map(|i| Entry {
                            actor: ActorId(i),
                            name: format!("Rider {i}"),
                        })
                        .collect();
                    let identity = RaceIdentity {
                        id,
                        mode: Mode::BigGame,
                        level,
                        course: 1,
                        bike: 0,
                        seed: 1,
                        roster: roster.clone(),
                    };
                    let mut ordered = roster;
                    let player = ordered.remove(0);
                    ordered.insert((place - 1) as usize, player);
                    let standings: Vec<_> = ordered
                        .into_iter()
                        .enumerate()
                        .map(|(i, r)| Standing {
                            actor: r.actor,
                            name: r.name,
                            place: i as u8 + 1,
                            progress: Some(100),
                            terminal: None,
                            completed_at: None,
                        })
                        .collect();
                    // Adapter constructs declared result data; trajectory/terminal creation is separately tested.
                    let summary: Summary = serde_json::from_value(
                        json!({"identity":identity,"outcome":outcome,"elapsed_ticks":60,"standings":standings,"player_place":place,"bike_condition":100}),
                    )?;
                    let report = career
                        .settle(&catalog, &summary)?
                        .ok_or("missing settlement")?;
                    outcomes.push(json!([
                        report.prize,
                        report.penalty,
                        report.game_over,
                        career.cash,
                        career.qualified,
                        career.bike,
                        career.level
                    ]));
                }
            }
        }
    }
    std::fs::write(
        resolved.join("economy-rust.json"),
        serde_json::to_vec(
            &json!({"characters":characters,"prices":prices,"prizes":catalog.prizes,"purchases":purchases,"outcomes":outcomes}),
        )?,
    )?;
    println!("8 characters, 15 bikes; 900 purchase and 675 result transaction cases");
    Ok(())
}
