fn run() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 3 {
        return Err("usage: rr-replay <replay.json> <30|60|144|irregular> <report.json>".into());
    }
    let replay = rr_simulation::replay::Replay::read_file(std::path::Path::new(&a[0]))?;
    let report = rr_simulation::replay::run(replay, &a[1])?;
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
