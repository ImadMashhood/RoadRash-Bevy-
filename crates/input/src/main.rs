use rr_input::{Action, ActionMap, Context, Key};
use std::path::Path;
fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_value(serde_json::Value::String(
        text.into(),
    ))?)
}
fn calibration(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    use rr_input::calibration::{Calibration, CalibrationFile, Response};
    match args {
        ["init",path]=>{
            if Path::new(path).exists(){return Err("calibration file already exists; use reset explicitly".into());}
            CalibrationFile::default().save(Path::new(path))?;
        }
        ["validate",path]=>{CalibrationFile::read(Path::new(path))?;println!("Calibration profiles valid");}
        ["show",path]=>println!("{}",serde_json::to_string_pretty(&CalibrationFile::read(Path::new(path))?)?),
        ["set",path,selector,field,value]=>{
            let mut file=CalibrationFile::read(Path::new(path))?;
            let mut profile=if *selector=="default"{file.default}else{file.profile(selector)};
            match *field {
                "minimum"=>profile.minimum=value.parse()?,"center"=>profile.center=value.parse()?,"maximum"=>profile.maximum=value.parse()?,
                "dead_zone"=>profile.dead_zone=value.parse()?,"saturation"=>profile.saturation=value.parse()?,
                "response"=>profile.response=parse::<Response>(value)?,"invert"=>profile.invert=value.parse()?,"trigger_threshold"=>profile.trigger_threshold=value.parse()?,
                _=>return Err("unknown calibration field".into())
            }
            profile.validate()?;
            if *selector=="default"{file.default=profile;}else{file.set((*selector).into(),profile)?;}
            file.save(Path::new(path))?;
        }
        ["reset",path,selector]=>{
            let mut file=CalibrationFile::read(Path::new(path))?;
            match *selector {"all"=>file=CalibrationFile::default(),"default"=>file.default=Calibration::default(),key=>{if file.devices.remove(key).is_none(){return Err("controller profile not found".into());}}}
            file.save(Path::new(path))?;
        }
        ["curve",path,selector]=>{
            let file=CalibrationFile::read(Path::new(path))?;let profile=if *selector=="default"{file.default}else{file.profile(selector)};
            println!("{}",serde_json::to_string_pretty(&serde_json::json!({"profile":profile,"points":profile.curve()}))?);
        }
        _=>return Err("usage: rr-input calibration init|validate|show FILE | set FILE default|MODEL_KEY FIELD VALUE | reset FILE default|MODEL_KEY|all | curve FILE default|MODEL_KEY".into())
    }
    Ok(())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
  ["calibration",rest @ ..]=>calibration(rest)?,
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
