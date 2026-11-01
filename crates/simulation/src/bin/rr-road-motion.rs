mod support;
use rr_simulation::{
    Command,
    handling::{Bike, Ground},
    riding::Configuration,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() != 2 {
        return Err("usage: rr-road-motion DATA_ROOT OUTPUT_JSON".into());
    }
    let root = std::path::Path::new(&a[0]);
    let course = rr_course::Course::read(&std::fs::read(root.join("DATA/COURSES/CITY.CRS"))?)?;
    let route = course.route(&vec![0; course.splits().len()])?;
    let mut c = Configuration::load(root, &route, 1, 50)?;
    let mut rows = vec![];
    for s in c.specs.clone() {
        for scenario in 0..3 {
            c.ground = (0..20000)
                .map(|i| Ground {
                    left: -50000,
                    right: 50000,
                    left_shoulder: -50250,
                    right_shoulder: 50250,
                    bend: if scenario == 0 {
                        0
                    } else if (i / 30) % 2 == 1 {
                        4
                    } else {
                        -4
                    },
                    rise: if scenario == 2 {
                        if (i / 30) % 2 == 1 { 10 } else { -10 }
                    } else {
                        0
                    },
                    ..Ground::default()
                })
                .collect();
            let mut b = Bike::new(&s, &c.tables)?;
            b.speed = 8000;
            b.previous_rise = c.ground[50].rise;
            for tick in 0..600 {
                let g = c.ground[50 + (b.distance_q8 >> 8) as usize];
                b.step_on_route(
                    &s,
                    &c.tables,
                    Command {
                        throttle: true,
                        ..Default::default()
                    },
                    g,
                    |old, new| c.swept_ground(old, new),
                );
                rows.push(vec![
                    s.id as i64,
                    scenario,
                    tick + 1,
                    b.speed as i64,
                    b.distance_q8,
                    b.lateral as i64,
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
        support::output(root, std::path::Path::new(&a[1]))?,
        serde_json::to_vec(&rows)?,
    )?;
    Ok(())
}
