//! Authored test-only pixels for verifying renderer overrides; no original media used.
fn main() {
    let path = std::env::args_os().nth(1).expect("output PNG path");
    let mut pixels = vec![0u8; 80 * 108 * 4];
    for y in 0..108 {
        for x in 0..80 {
            let i = (y * 80 + x) * 4;
            let color = if (x / 8 + y / 8) % 2 == 0 {
                [60, 180, 240, 255]
            } else {
                [240, 100, 160, 255]
            };
            pixels[i..i + 4].copy_from_slice(&color);
        }
    }
    let image = rr_formats::pixels::Rgba {
        width: 80,
        height: 108,
        pixels,
    };
    std::fs::write(path, image.png().unwrap()).unwrap();
}
