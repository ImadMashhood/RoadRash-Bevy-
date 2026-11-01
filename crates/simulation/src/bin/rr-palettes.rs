mod support;
use sha2::{Digest, Sha256};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() != 2 {
        return Err("usage: rr-palettes DATA_ROOT OUTPUT_JSON".into());
    }
    let root = std::path::Path::new(&a[0]);
    let exe = std::fs::read(root.join("ROADRASH.EXE"))?;
    let remaps = rr_formats::pe::SupportedExe::read(&exe)?.all_rider_remaps()?;
    let bob = std::fs::read(root.join("DATA/BIKERS/HIBOB.DAT"))?;
    let frames = rr_formats::decode_bob(&bob, 326)?;
    let mut rows = vec![];
    for (color, map) in remaps.iter().enumerate() {
        for (frame, body) in frames.iter().enumerate() {
            if body.width == 0 || body.height == 0 {
                continue;
            }
            let pixels: Vec<_> = body.pixels.iter().map(|v| map[*v as usize]).collect();
            rows.push(serde_json::json!([
                color,
                frame,
                body.width,
                body.height,
                format!("{:x}", Sha256::digest(&pixels))
            ]));
        }
    }
    let shadows = std::fs::read(root.join("DATA/BIKERS/HISHADOW.DAT"))?;
    for (frame, body) in rr_formats::decode_bob(&shadows, 326)?.iter().enumerate() {
        if body.width == 0 || body.height == 0 {
            continue;
        }
        rows.push(serde_json::json!([
            224,
            frame,
            body.width,
            body.height,
            format!("{:x}", Sha256::digest(body.pixels))
        ]));
    }
    std::fs::write(
        support::output(root, std::path::Path::new(&a[1]))?,
        serde_json::to_vec(&rows)?,
    )?;
    Ok(())
}
