mod support;
use rr_simulation::animation::{Library, Player};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() != 2 {
        return Err("usage: rr-animation DATA_ROOT OUTPUT_JSON".into());
    }
    let root = std::path::Path::new(&a[0]);
    let lib = Library::read(
        &std::fs::read(root.join("ROADRASH.EXE"))?,
        &std::fs::read(root.join("DATA/GLOBAL.RSC"))?,
    )?;
    let mut rows = vec![];
    for bank in 0..2 {
        for code in 0..52 {
            if lib.banks[bank][code].is_none() {
                continue;
            }
            let mut p = Player::default();
            p.set(&lib, bank, code as i8);
            for tick in 0..180 {
                let Some(f) = p.advance(&lib, bank, tick)? else {
                    break;
                };
                let (c, _) = lib.select(bank, &f, 0, 0)?;
                let mut angled = vec![];
                for yaw in [-0x100000, 0, 0x100000] {
                    for roll in [-0x200000, 0, 0x200000] {
                        let (cell, result_roll) = lib.select(bank, &f, yaw, roll)?;
                        angled.push(vec![
                            cell.frame as i64,
                            cell.flags as i64,
                            result_roll as i64,
                        ]);
                    }
                }
                rows.push(serde_json::json!([
                    bank, code, tick, p.code, p.frame, p.loops, p.deadline, f.render, f.duration,
                    f.control, f.flags, c.frame, c.flags, angled
                ]));
            }
        }
    }
    std::fs::write(
        support::output(root, std::path::Path::new(&a[1]))?,
        serde_json::to_vec(&rows)?,
    )?;
    Ok(())
}
