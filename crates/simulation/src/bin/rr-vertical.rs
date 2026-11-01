mod support;
use rr_simulation::{
    Command,
    handling::{Bike, BikeSpec, Ground, Tables},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() != 2 {
        return Err("usage: rr-vertical DATA_ROOT OUTPUT_JSON".into());
    }
    let data = std::path::Path::new(&a[0]);
    let specs = BikeSpec::read(&std::fs::read(data.join("DATA/BIKESPEC.RSC"))?)?;
    let t = Tables::read(&std::fs::read(data.join("ROADRASH.EXE"))?)?;
    let mut rows = vec![];
    for s in &specs {
        for scenario in 0..3 {
            let mut b = Bike::new(s, &t)?;
            b.speed = 8000;
            b.vertical_speed = if scenario == 0 {
                2000
            } else if scenario == 1 {
                0
            } else {
                -2000
            };
            b.height = if scenario == 2 { 20000 } else { 0 };
            for tick in 0..180 {
                b.step(
                    s,
                    &t,
                    Command {
                        throttle: true,
                        ..Default::default()
                    },
                    Ground::flat(),
                );
                rows.push(vec![
                    s.id as i64,
                    scenario,
                    tick + 1,
                    b.speed as i64,
                    b.contact as i64,
                    b.height as i64,
                    b.vertical_speed as i64,
                    b.gravity as i64,
                    b.pitch as i64,
                ]);
            }
        }
    }
    std::fs::write(
        support::output(data, std::path::Path::new(&a[1]))?,
        serde_json::to_vec(&rows)?,
    )?;
    Ok(())
}
