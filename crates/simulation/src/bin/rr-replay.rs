fn run() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if !(3..=4).contains(&a.len()) || a.get(3).is_some_and(|s| s != "--every-tick") {
        return Err(
            "usage: rr-replay <replay.json> <30|60|144|irregular> <report.json> [--every-tick]"
                .into(),
        );
    }
    let replay = rr_simulation::replay::Replay::read_file(std::path::Path::new(&a[0]))?;
    let report = rr_simulation::replay::run_with_tick_hashes(replay, &a[1], a.len() == 4)?;
    let bytes = serde_json::to_vec_pretty(&report)?;
    std::fs::write(&a[2], &bytes)?;
    println!("{}", report.state_sha256);
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
