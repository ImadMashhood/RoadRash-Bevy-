use rr_formats::strings::*;
use std::collections::BTreeMap;
fn fixture() -> Vec<u8> {
    let mut b = vec![0u8; 0x1200];
    let word = |b: &mut [u8], at, v: u16| b[at..at + 2].copy_from_slice(&v.to_le_bytes());
    let dword = |b: &mut [u8], at, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    b[..2].copy_from_slice(b"MZ");
    dword(&mut b, 60, 128);
    b[128..132].copy_from_slice(b"PE\0\0");
    word(&mut b, 132, 0x14c);
    word(&mut b, 134, 1);
    word(&mut b, 148, 224);
    word(&mut b, 152, 0x10b);
    dword(&mut b, 180, 0x400000);
    dword(&mut b, 244, 16);
    dword(&mut b, 264, 0x1000);
    dword(&mut b, 268, 0x1000);
    dword(&mut b, 388, 0x1000);
    dword(&mut b, 392, 0x1000);
    dword(&mut b, 396, 0x200);
    let base = 0x200;
    for offset in [0, 24, 48] {
        word(&mut b, base + offset + 14, 1);
    }
    for (at, id, next) in [(16, 6, 0x80000018), (40, 1, 0x80000030), (64, 0x409, 72)] {
        dword(&mut b, base + at, id);
        dword(&mut b, base + at + 4, next);
    }
    let mut raw = vec![];
    for i in 0..16 {
        let text = match i {
            0 => "100 2",
            1 => "Français — Grüße",
            2 => "Hello",
            _ => "",
        };
        let u: Vec<_> = text.encode_utf16().collect();
        raw.extend((u.len() as u16).to_le_bytes());
        for v in u {
            raw.extend(v.to_le_bytes());
        }
    }
    dword(&mut b, base + 72, 0x1100);
    dword(&mut b, base + 76, raw.len() as u32);
    b[0x300..0x300 + raw.len()].copy_from_slice(&raw);
    b
}
#[test]
fn unicode_strings_decode_without_executing_a_dll_and_all_truncated_prefixes_reject() {
    let b = fixture();
    let map = resources(&b).unwrap();
    assert_eq!(map[&1], "Français — Grüße");
    let table = game_table(&map, 0).unwrap();
    assert_eq!(table.len(), 3);
    assert_eq!(table[2], "Hello");
    for n in 0..b.len() {
        assert!(resources(&b[..n]).is_err());
    }
}
#[test]
fn malformed_resource_offsets_and_counts_never_panic() {
    let b = fixture();
    for at in [
        60, 128, 132, 134, 148, 152, 180, 244, 264, 268, 388, 392, 396, 526, 532, 550, 556, 574,
        580, 584, 588, 768,
    ] {
        let mut bad = b.clone();
        bad[at..at + 4].fill(255);
        assert!(std::panic::catch_unwind(|| resources(&bad)).is_ok());
    }
}
#[test]
fn continuation_resolution_rejects_cycles_missing_records_and_excessive_headers() {
    let mut map = BTreeMap::from([
        (0, "100 2".into()),
        (1, "Long \u{1}ÿ".into()),
        (2, "Second".into()),
        (3001, "continued".into()),
    ]);
    assert_eq!(game_table(&map, 0).unwrap()[1], "Long continued");
    map.insert(3001, "\u{1}ÿ".into());
    assert!(game_table(&map, 0).is_err());
    map.remove(&3001);
    assert!(game_table(&map, 0).is_err());
    map.insert(0, "999999999 999999999".into());
    assert!(game_table(&map, 0).is_err());
}
