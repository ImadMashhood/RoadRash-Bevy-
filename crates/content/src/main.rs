use rr_content::{Catalog, Recipe, Result};
use std::path::PathBuf;
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.first().and_then(|s|s.to_str()) {
        Some("build") if args.len()==3 => {let install=rr_import::active_install(&PathBuf::from(&args[1]))?;let (_,report)=rr_content::build::build(&install.data_root,&rr_import::supported_manifest()?,&PathBuf::from(&args[2]),Recipe::default())?;println!("{}",serde_json::to_string_pretty(&report)?);},
        Some("inspect") if args.len()==2 =>{let root=PathBuf::from(&args[1]);let catalog=Catalog::read(&root.join("catalog.json"))?;println!("{} sources, {} semantic assets; open {}",catalog.sources.len(),catalog.entries.len(),root.join("inspector.html").display());},
        Some("find") if args.len()==3=>{let catalog=Catalog::read(&PathBuf::from(&args[1]).join("catalog.json"))?;let needle=args[2].to_string_lossy().to_lowercase();for e in catalog.entries.iter().filter(|e|e.id.0.contains(&needle)){println!("{} {:?} {:?}",e.id.0,e.kind,e.native_size);}},
        _=>return Err("usage: rr-content build <install-root> <output> | inspect <output> | find <output> <substring>".into()),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
