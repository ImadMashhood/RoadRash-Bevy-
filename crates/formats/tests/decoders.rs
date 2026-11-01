use rr_formats::{
    binary, decode_bob,
    family::Family,
    pe::{Pe, SupportedExe},
    pixels::{Cel, Palette, Rgba, Texture},
    resource::{Directory, unpack},
};
fn word(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn resource() -> Vec<u8> {
    let mut b = vec![0; 112];
    b[..4].copy_from_slice(b"CRSR");
    word(&mut b, 4, 0x30);
    b[12..16].copy_from_slice(b"LBTR");
    word(&mut b, 16, 64);
    word(&mut b, 20, 48);
    b[48..52].copy_from_slice(&[1, 2, 3, 4]);
    b[64..68].copy_from_slice(b"LBTR");
    word(&mut b, 68, 48);
    word(&mut b, 72, 1);
    b[80..84].copy_from_slice(b"TSET");
    word(&mut b, 84, 7);
    word(&mut b, 88, 48);
    word(&mut b, 92, 4);
    word(&mut b, 96, 0x1234);
    word(&mut b, 108, 0xcafebabe);
    b
}
fn texture() -> Vec<u8> {
    let mut b = vec![0; 80];
    word(&mut b, 0, 0xbc614e);
    b[16..23].copy_from_slice(b"fixture");
    word(&mut b, 32, 2);
    word(&mut b, 36, 3);
    word(&mut b, 40, 2);
    word(&mut b, 56, 44);
    word(&mut b, 68, 50);
    b[72..80].copy_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7]);
    b
}
fn cel() -> Vec<u8> {
    let mut b = vec![0; 80];
    word(&mut b, 0, 0x43434220);
    word(&mut b, 4, 80);
    word(&mut b, 72, 0x80000001);
    word(&mut b, 76, 3);
    let t = texture();
    b.extend(0x50444154u32.to_le_bytes());
    b.extend((8 + t.len() as u32).to_le_bytes());
    b.extend(t);
    b
}
#[test]
fn resource_listing_unknown_words_and_metadata_round_trip() {
    let b = resource();
    let d = Directory::read(&b).unwrap();
    assert_eq!(d.entries[0].tag, "TEST");
    assert_eq!(d.entries[0].id, 7);
    assert_eq!(d.entries[0].payload(&b).unwrap(), [1, 2, 3, 4]);
    assert_eq!(d.entries[0].raw[4], 0x1234);
    assert_eq!(d.entries[0].raw[7], 0xcafebabe);
    let encoded = serde_json::to_vec(&d).unwrap();
    assert_eq!(serde_json::from_slice::<Directory>(&encoded).unwrap(), d);
}
#[test]
fn every_resource_truncation_and_bad_magic_fails() {
    let b = resource();
    for n in 0..b.len() {
        assert!(Directory::read(&b[..n]).is_err(), "{n}");
    }
    let mut b = b;
    b[..4].reverse();
    assert!(Directory::read(&b).is_err());
}
#[test]
fn payload_overflow_overlap_and_alias_policy() {
    let mut b = resource();
    word(&mut b, 88, u32::MAX - 1);
    word(&mut b, 92, 8);
    assert!(Directory::read(&b).is_err());
    let mut b = resource();
    b.resize(144, 0);
    word(&mut b, 72, 2);
    let first = b[80..112].to_vec();
    b[112..144].copy_from_slice(&first);
    word(&mut b, 116, 8);
    assert!(Directory::read(&b).is_ok());
    word(&mut b, 120, 50);
    assert!(Directory::read(&b).is_err());
}
#[test]
fn cel_and_ceil_height_mips_have_known_pixels() {
    let c = Cel::read(&cel()).unwrap();
    assert_eq!((c.width, c.height), (2, 3));
    let t = c.texture.unwrap();
    assert_eq!((t.mips[1].width, t.mips[1].height), (1, 2));
    assert_eq!(t.mips[0].indices, [0, 1, 2, 3, 4, 5]);
    assert_eq!(t.mips[1].indices, [6, 7]);
}
#[test]
fn malformed_texture_and_cel_dimensions_fail() {
    for field in [32, 36, 40, 56] {
        let mut t = texture();
        word(&mut t, field, if field == 56 { u32::MAX } else { 0 });
        assert!(Texture::read(&t).is_err());
    }
    let mut c = cel();
    word(&mut c, 72, 0x8000000f);
    assert!(Cel::read(&c).is_err());
    for n in 0..cel().len() {
        assert!(Cel::read(&cel()[..n]).is_err());
    }
}
#[test]
fn indexed_palette_remaps_preserve_original_transparency() {
    let bytes: Vec<_> = (0..256u32)
        .flat_map(|i| [i as u8, 0, 255 - i as u8, 99])
        .collect();
    let p = Palette::read(&bytes).unwrap();
    let mut remap = std::array::from_fn(|i| i as u8);
    remap[0] = 3;
    remap[1] = 2;
    let rgba = p.rgba(2, 1, &[0, 1], true, Some(&remap)).unwrap().unwrap();
    assert_eq!(rgba.pixels, [3, 0, 252, 0, 2, 0, 253, 255]);
    for background in [0, 255] {
        let composited = if rgba.pixels[3] == 0 {
            background
        } else {
            rgba.pixels[0]
        };
        assert_eq!(composited, background);
    }
    let png = rgba.png().unwrap();
    assert_eq!(Rgba::from_png(&png).unwrap(), rgba);
    assert_eq!(rgba.png().unwrap(), png);
}
#[test]
fn empty_frame_slots_and_order_survive() {
    let f = decode_bob(&[0, 0, 1, 1, 42, 0, 2], 3).unwrap();
    assert_eq!(f.len(), 3);
    assert!(f[0].pixels.is_empty());
    assert_eq!(f[1].pixels, [42]);
    assert!(f[2].pixels.is_empty());
}
#[test]
fn rri_is_bounded_jpeg_and_png_is_lossless() {
    let original = [240, 20, 30, 240, 20, 30, 240, 20, 30, 240, 20, 30];
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)
        .encode(&original, 2, 2, image::ExtendedColorType::Rgb8)
        .unwrap();
    let image = rr_formats::pixels::rri(&jpeg).unwrap();
    assert_eq!((image.width, image.height), (2, 2));
    assert!(
        image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[3] == 255 && p[0] > 220 && p[1] < 40)
    );
    assert!(rr_formats::pixels::rri(&jpeg[..10]).is_err());
    assert!(rr_formats::pixels::rri(&[0; 100]).is_err());
}
#[test]
fn pe_virtual_addresses_are_bounded_and_version_extraction_is_hash_gated() {
    let mut b = vec![0; 528];
    b[..2].copy_from_slice(b"MZ");
    word(&mut b, 60, 64);
    b[64..68].copy_from_slice(b"PE\0\0");
    b[68..70].copy_from_slice(&0x14cu16.to_le_bytes());
    b[70..72].copy_from_slice(&1u16.to_le_bytes());
    b[84..86].copy_from_slice(&32u16.to_le_bytes());
    b[88..90].copy_from_slice(&0x10bu16.to_le_bytes());
    word(&mut b, 116, 0x400000);
    word(&mut b, 132, 0x1000);
    word(&mut b, 136, 16);
    word(&mut b, 140, 512);
    b[516..520].copy_from_slice(&[9, 8, 7, 6]);
    let pe = Pe::read(&b).unwrap();
    assert_eq!(pe.read_va(0x401004, 4).unwrap(), [9, 8, 7, 6]);
    assert!(pe.read_va(0x40100f, 2).is_err());
    assert!(pe.read_va(0x3fffff, 1).is_err());
    assert!(SupportedExe::read(&b).is_err());
    assert!(binary::range(&b, usize::MAX, 2).is_err());
}
#[test]
fn family_cycles_and_unpacked_bombs_fail() {
    let mut b = vec![0; 8];
    word(&mut b, 0, 1);
    word(&mut b, 4, 0);
    assert!(Family::read_unpacked(&b).is_err());
    let mut b = vec![0; 16];
    word(&mut b, 0, u32::MAX);
    assert!(unpack(&b).is_err());
}
#[test]
fn dcl_stream_and_crc_have_known_answer() {
    let compressed = [0, 4, 0x82, 0x24, 0x25, 0x8f, 0x80, 0x7f];
    let expected = b"AIAIAIAIAIAIA";
    let mut b = vec![0; 16];
    word(&mut b, 0, expected.len() as u32);
    word(&mut b, 4, compressed.len() as u32);
    word(&mut b, 8, crc32fast::hash(expected));
    word(&mut b, 12, 1);
    b.extend(compressed);
    assert_eq!(unpack(&b).unwrap(), expected);
    b[8] ^= 1;
    assert!(unpack(&b).is_err());
}

#[test]
fn shared_leaf_fanout_cannot_exceed_hierarchy_edge_budget() {
    let arrays = 20usize;
    let width = 4095usize;
    let row_bytes = 4 + width * 4;
    let root_bytes = 4 + arrays * 4;
    let leaf = root_bytes + arrays * row_bytes;
    let mut b = vec![0u8; leaf + 4];
    word(&mut b, 0, arrays as u32);
    word(&mut b, leaf, 0x414e494d);
    for array in 0..arrays {
        let at = root_bytes + array * row_bytes;
        word(&mut b, 4 + array * 4, at as u32);
        word(&mut b, at, width as u32);
        for child in 0..width {
            word(&mut b, at + 4 + child * 4, (leaf - at) as u32);
        }
    }
    assert!(
        Family::read_unpacked(&b)
            .unwrap_err()
            .to_string()
            .contains("edge budget")
    );
}
