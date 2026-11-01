fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: rr-race-check REPLAY RATE NEW_REPORT.json".into());
    }
    let replay = rr_simulation::replay::Replay::read_file(std::path::Path::new(&args[0]))?;
    let report = rr_session::playback::run(replay, args[1].to_str().ok_or("rate encoding")?, true)?;
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?
        .write_all(&serde_json::to_vec_pretty(&report)?)?;
    println!(
        "{} session ticks; {} simulation ticks; result {}",
        report.session_ticks, report.simulation_ticks, report.result_sha256
    );
    Ok(())
}
