//! Export locally owned source frames and exact tick timelines for the review tool.
use rr_simulation::animation::{Library, Player};
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: rr-riders DATA_ROOT OUTPUT_DIR COLOR_0_TO_14".into());
    }
    let data = std::path::Path::new(&args[0]);
    let out = rr_render::suite::output_path(data, std::path::Path::new(&args[1]))?;
    std::fs::create_dir_all(&out)?;
    let color = args[2].to_string_lossy().parse()?;
    let assets = rr_render::rider::RiderAssets::load(data, color)?;
    let library = Library::read(
        &std::fs::read(data.join("ROADRASH.EXE"))?,
        &std::fs::read(data.join("DATA/GLOBAL.RSC"))?,
    )?;
    let mut sprites = vec![];
    for (i, body) in assets.bodies.iter().enumerate() {
        let Some(body) = body else {
            sprites.push(json!(null));
            continue;
        };
        let body_path = format!("body-{i:03}.png");
        std::fs::write(out.join(&body_path), body.texture.image.png()?)?;
        let shadow = if let Some(s) = assets.shadow(i) {
            let name = format!("shadow-{i:03}.png");
            std::fs::write(out.join(&name), s.texture.image.png()?)?;
            Some(json!({"image":name,"logical":s.logical,"pivot":s.pivot}))
        } else {
            None
        };
        sprites.push(
            json!({"image":body_path,"logical":body.logical,"pivot":body.pivot,"shadow":shadow}),
        );
    }
    let mut sequences = vec![];
    for code in 0..52 {
        let Some(frames) = &library.banks[0][code] else {
            continue;
        };
        let mut p = Player::default();
        p.set(&library, 0, code as i8);
        let mut ticks = vec![];
        for tick in 0..240 {
            if p.code != code as i8 {
                break;
            }
            let Some(f) = p.advance(&library, 0, tick)? else {
                break;
            };
            let (c, _) = library.select(0, &f, 0, 0)?;
            ticks.push(json!({"tick":tick,"frame":c.frame,"mirror":c.flags&1!=0,"view_flags":c.flags,"contact":f.flags&0x600000!=0}));
        }
        sequences.push(json!({"code":code,"frames":frames,"ticks":ticks}));
    }
    let metadata = json!({"schema_version":1,"tick_hz":60,"color":color,"sprites":sprites,"sequences":sequences,"views":library.views[0],"fallback":library.fallback,"attack_codes":library.attacks,"transition_policy":"hard cut; ground anchor preserved; source flags alone control mirroring"});
    std::fs::write(out.join("riders.json"), serde_json::to_vec(&metadata)?)?;
    Ok(())
}
