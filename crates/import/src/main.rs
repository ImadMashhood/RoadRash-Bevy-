use rr_import::{ImportError, Limits, active_install, import, supported_manifest};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
fn run() -> Result<(), ImportError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 2 && args[0] == "--active" {
        let active = active_install(&PathBuf::from(&args[1]))?;
        println!("{}", active.data_root.display());
        return Ok(());
    }
    if args.len() != 2 {
        return Err(ImportError::Invalid("usage: rr-import <source.iso | extracted ROADRASH folder> <private install root>\n       rr-import --active <private install root>".into()));
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = cancelled.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
        .map_err(|e| ImportError::Invalid(e.to_string()))?;
    match import(
        &PathBuf::from(&args[0]),
        &PathBuf::from(&args[1]),
        &supported_manifest()?,
        &Limits::default(),
        || cancelled.load(Ordering::Relaxed),
    ) {
        Ok(result) => {
            println!(
                "Imported {} verified files to {}",
                result.files,
                result.data_root.display()
            );
            Ok(())
        }
        Err(ImportError::Validation(problems)) => {
            for p in &problems {
                eprintln!("{}: {}: {}", p.path, p.kind, p.detail);
            }
            Err(ImportError::Validation(problems))
        }
        Err(e) => Err(e),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
