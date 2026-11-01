//! Reproducible bounded mutation fuzz target. All seeds are authored or codec-generated.
use rr_formats::{
    decode_bob,
    family::Family,
    pe::{Pe, SupportedExe},
    pixels::{Cel, Rgba, Texture},
    resource::{Directory, unpack},
};
use std::panic::{AssertUnwindSafe, catch_unwind};
fn probe(bytes: &[u8]) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        let _ = Directory::read(bytes);
        let _ = Texture::read(bytes);
        let _ = Cel::read(bytes);
        let _ = Family::read_unpacked(bytes);
        let _ = unpack(bytes);
        let _ = decode_bob(bytes, 2);
        if let Ok(pe) = Pe::read(bytes) {
            let _ = pe.read_va(u32::MAX, usize::MAX);
        }
        let _ = SupportedExe::read(bytes);
        let _ = rr_formats::pixels::rri(bytes);
        let _ = Rgba::from_png(bytes);
    }))
    .is_ok()
}
fn word(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn seeds() -> Vec<Vec<u8>> {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../../docs/fixtures/authored.json")).unwrap();
    let hex = fixture["resource_container"]["bytes_hex"].as_str().unwrap();
    let resource = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let mut texture = vec![0; 66];
    word(&mut texture, 0, 0xbc614e);
    word(&mut texture, 32, 2);
    word(&mut texture, 36, 3);
    word(&mut texture, 40, 1);
    word(&mut texture, 56, 32);
    texture[60..66].copy_from_slice(&[0, 1, 2, 3, 4, 5]);
    let mut cel = vec![0; 80];
    word(&mut cel, 0, 0x43434220);
    word(&mut cel, 4, 80);
    word(&mut cel, 72, 2);
    word(&mut cel, 76, 3);
    cel.extend(0x50444154u32.to_le_bytes());
    cel.extend((texture.len() as u32 + 8).to_le_bytes());
    cel.extend(&texture);
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .encode(&[255, 0, 0], 1, 1, image::ExtendedColorType::Rgb8)
        .unwrap();
    let png = Rgba {
        width: 1,
        height: 1,
        pixels: vec![1, 2, 3, 255],
    }
    .png()
    .unwrap();
    vec![
        resource,
        texture,
        cel,
        jpeg,
        png,
        vec![0, 0, 1, 1, 42],
        vec![0; 16],
    ]
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: rr-format-fuzz <iterations 1..1000000> <report.json>".into());
    }
    let iterations: u64 = args[0].parse()?;
    if iterations == 0 || iterations > 1_000_000 {
        return Err("iteration bound".into());
    }
    let seeds = seeds();
    let mut random = 0x524f414452415348u64;
    let mut max_input = 0;
    for case in 0..iterations {
        let mut bytes = seeds[case as usize % seeds.len()].clone();
        for _ in 0..1 + case % 8 {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let at = random as usize % (bytes.len() + 1);
            match random % 4 {
                0 if at < bytes.len() => bytes[at] ^= (random >> 32) as u8,
                1 if bytes.len() < 4096 => bytes.insert(at, (random >> 24) as u8),
                2 if at < bytes.len() => {
                    bytes.remove(at);
                }
                _ => bytes.truncate(at),
            }
        }
        max_input = max_input.max(bytes.len());
        if !probe(&bytes) {
            let path = std::path::Path::new(&args[1]).with_extension("failure.bin");
            fs_write(&path, &bytes)?;
            return Err(format!(
                "panic case {case}, seed 0x524f414452415348; failure saved to {}",
                path.display()
            )
            .into());
        }
    }
    let report = serde_json::json!({"iterations":iterations,"seed":"0x524f414452415348","targets":10,"max_input_bytes":max_input,"input_limit":4096,"decoded_family_limit_bytes":67108864,"panics":0,"outcome":"passed","method":"deterministic mutation fuzzing; authored/codec-generated seeds"});
    std::fs::write(&args[1], serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}
fn fs_write(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
