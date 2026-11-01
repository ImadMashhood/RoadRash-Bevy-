mod support;
use rr_simulation::{
    Command,
    handling::{Bike, BikeSpec, Ground, Tables},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: rr-handling DATA_ROOT OUTPUT_JSON".into());
    }
    let root = std::path::Path::new(&args[0]);
    let specs = BikeSpec::read(&std::fs::read(root.join("DATA/BIKESPEC.RSC"))?)?;
    let t = Tables::read(&std::fs::read(root.join("ROADRASH.EXE"))?)?;
    let mut rows = vec![];
    for s in &specs {
        for scenario in 0..7 {
            let mut b = Bike::new(s, &t)?;
            for tick in 0..2400 {
                let steer = if scenario == 1 {
                    -1
                } else if scenario == 2 {
                    1
                } else {
                    0
                };
                let surface = if scenario > 2 { scenario - 3 } else { 0 };
                let ground = match surface {
                    0 => Ground::flat(),
                    1 => Ground {
                        left: 1,
                        right: 2,
                        left_shoulder: -1000000,
                        ..Ground::flat()
                    },
                    2 => Ground {
                        left: 100,
                        right: 200,
                        left_shoulder: -1000000,
                        kind: 1,
                        ..Ground::flat()
                    },
                    _ => Ground {
                        left: 1,
                        right: 2,
                        left_shoulder: 1,
                        ..Ground::flat()
                    },
                };
                // Isolate fixed surface class from lateral integration; compare identical low-level APIs.
                b.lateral = 0;
                b.step(
                    s,
                    &t,
                    Command {
                        throttle: tick < 1800,
                        brake: tick >= 1800 && scenario == 0,
                        steer,
                        ..Default::default()
                    },
                    ground,
                );
                b.contact = 74;
                b.height = 0;
                b.vertical_speed = 0;
                rows.push(vec![
                    s.id as i64,
                    scenario as i64,
                    tick + 1,
                    b.speed as i64,
                    b.gear as i64,
                    b.force as i64,
                    b.lean as i64,
                    b.lateral_speed as i64,
                    b.lateral_force as i64,
                    b.acceleration as i64,
                    i64::from(b.crashed),
                ]);
            }
        }
    }
    std::fs::write(
        support::output(root, std::path::Path::new(&args[1]))?,
        serde_json::to_vec(&rows)?,
    )?;
    eprintln!("{} specifications, {} trace rows", specs.len(), rows.len());
    Ok(())
}
