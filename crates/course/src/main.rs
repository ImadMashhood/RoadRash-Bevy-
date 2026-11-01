use std::path::PathBuf;
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 || args[0] != "inspect" {
        return Err("usage: rr-course inspect <private-install-root> <private-output>".into());
    }
    let install = rr_import::active_install(&PathBuf::from(&args[1]))?;
    rr_course::export::inspect(&install.data_root, &PathBuf::from(&args[2]))?;
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
