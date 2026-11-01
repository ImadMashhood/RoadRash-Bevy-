use rr_input::{Action, ActionMap, Context, Key};
use std::path::Path;
fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_value(serde_json::Value::String(
        text.into(),
    ))?)
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
  ["init",path]=>{if Path::new(path).exists(){return Err("bindings already exist; use reset explicitly".into());}ActionMap::default().save(Path::new(path))?;}
  ["validate",path]=>{ActionMap::read(Path::new(path))?;println!("Bindings valid: race/menu/paused contexts");}
  ["set",path,context,action,keys]=>{let mut map=ActionMap::read(Path::new(path))?;map.rebind(parse::<Context>(context)?,parse::<Action>(action)?,keys.split(',').map(parse::<Key>).collect::<Result<Vec<_>,_>>()?)?;map.save(Path::new(path))?;}
  ["reset",path,context]=>{let mut map=ActionMap::read(Path::new(path))?;map.reset(if *context=="all"{None}else{Some(parse::<Context>(context)?)});map.save(Path::new(path))?;}
  _=>return Err("usage: rr-input init FILE | validate FILE | set FILE race|menu|paused ACTION KeyW,ArrowUp | reset FILE race|menu|paused|all".into())
 }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
