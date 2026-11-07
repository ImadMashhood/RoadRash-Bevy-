//! Bounded RRAN frame collision lists, independent of pixel decoding or scaling.
use crate::{
    DecodeError, Result,
    binary::{range, u32le},
};

pub fn read(bytes: &[u8], base: usize, index: usize) -> Result<Vec<[i32; 4]>> {
    range(bytes, base, 28)?;
    if u32le(bytes, base)? != 0x5252414e
        || u32le(bytes, base + 4)? != 20
        || u32le(bytes, base + 20)? != 0x52524644
    {
        return Err(DecodeError::Invalid("collision animation header"));
    }
    let info = base
        .checked_add(u32le(bytes, base + 8)? as usize)
        .ok_or(DecodeError::Invalid("collision info overflow"))?;
    range(bytes, info, 20)?;
    if u32le(bytes, info)? != 0x414e494d {
        return Err(DecodeError::Invalid("collision ANIM info"));
    }
    let count = u32le(bytes, info + 16)? as usize;
    if count == 0 || count > 4096 || u32le(bytes, base + 24)? as usize != count * 28 + 8 {
        return Err(DecodeError::Invalid("collision frame count"));
    }
    range(bytes, base + 28, count * 28)?;
    let frame = base + 28 + index.min(count - 1) * 28;
    let count = u32le(bytes, frame + 16)? as usize;
    let pointer = u32le(bytes, frame + 12)? as usize;
    // Some inactive frames retain stale absolute addresses; never follow them.
    if count == 0 || pointer == 0 {
        return Ok(vec![]);
    }
    if count > 4096 {
        return Err(DecodeError::Invalid("collision box count"));
    }
    let start = base
        .checked_add(pointer)
        .ok_or(DecodeError::Invalid("collision box pointer overflow"))?;
    range(bytes, start, count * 16)?;
    (0..count)
        .map(|i| {
            let at = start + i * 16;
            Ok([
                u32le(bytes, at)? as i32,
                u32le(bytes, at + 4)? as i32,
                u32le(bytes, at + 8)? as i32,
                u32le(bytes, at + 12)? as i32,
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 180];
        for (at, v) in [
            (0, 0x5252414e),
            (4, 20),
            (8, 112),
            (20, 0x52524644),
            (24, 92),
            (112, 0x414e494d),
            (128, 3),
            (96, 144),
            (100, 2),
        ] {
            put(&mut b, at, v);
        }
        for (i, n) in [-256i32, 0, 256, 4096, -512, 128, 512, 8192]
            .iter()
            .enumerate()
        {
            put(&mut b, 144 + i * 4, *n as u32);
        }
        b
    }
    #[test]
    fn reads_the_requested_views_boxes_with_signed_coordinates() {
        let b = fixture();
        assert!(read(&b, 0, 0).unwrap().is_empty());
        assert_eq!(
            read(&b, 0, 2).unwrap(),
            vec![[-256, 0, 256, 4096], [-512, 128, 512, 8192]]
        );
    }
    #[test]
    fn ignores_stale_pointer_on_an_inactive_frame() {
        let mut b = fixture();
        put(&mut b, 40, u32::MAX);
        assert!(read(&b, 0, 0).unwrap().is_empty());
    }
    #[test]
    fn rejects_truncated_or_unbounded_collision_lists() {
        let mut b = fixture();
        b.truncate(170);
        assert!(read(&b, 0, 2).is_err());
        let mut b = fixture();
        put(&mut b, 100, u32::MAX);
        assert!(read(&b, 0, 2).is_err());
    }
}

/// Physical frame texture flags, including native three-view texture aliases.
/// Pixel decoding, hotspot registration and palette changes do not affect this word.
pub fn texture_flags(bytes: &[u8], base: usize, index: usize) -> Result<Option<u32>> {
    let _ = read(bytes, base, index)?;
    let info = base
        .checked_add(u32le(bytes, base + 8)? as usize)
        .ok_or(DecodeError::Invalid("collision info overflow"))?;
    let count = u32le(bytes, info + 16)? as usize;
    let raw = |i: usize| -> Result<Option<u32>> {
        let pointer = u32le(bytes, base + 28 + i * 28 + 4)? as usize;
        if pointer == 0 {
            return Ok(None);
        }
        let body = base
            .checked_add(pointer)
            .and_then(|v| v.checked_add(8))
            .ok_or(DecodeError::Invalid("physical texture pointer"))?;
        range(bytes, body, 4)?;
        (u32le(bytes, body)? == 0xbc614e)
            .then(|| u32le(bytes, body + 12))
            .transpose()
    };
    let index = index.min(count - 1);
    let first = raw(0)?;
    let selected = if count == 3 && first.is_none() {
        2
    } else if first.is_some_and(|f| f & 1 != 0) && index / 3 * 3 + 2 < count {
        index / 3 * 3
    } else {
        index
    };
    raw(selected)
}

#[cfg(test)]
mod physical_flags_tests {
    use super::*;
    fn put(b: &mut [u8], at: usize, n: u32) {
        b[at..at + 4].copy_from_slice(&n.to_le_bytes());
    }
    fn frames() -> Vec<u8> {
        let mut b = vec![0; 256];
        for (at, n) in [
            (0, 0x5252414e),
            (4, 20),
            (8, 112),
            (20, 0x52524644),
            (24, 92),
            (112, 0x414e494d),
            (128, 3),
        ] {
            put(&mut b, at, n);
        }
        b
    }
    #[test]
    fn physical_flags_follow_three_view_texture_aliases() {
        let mut b = frames();
        for (at, n) in [
            (32, 176),
            (184, 0xbc614e),
            (196, 0x4001),
            (88, 208),
            (216, 0xbc614e),
            (228, 0x20),
        ] {
            put(&mut b, at, n);
        }
        assert_eq!(texture_flags(&b, 0, 2).unwrap(), Some(0x4001));
        put(&mut b, 32, 0);
        for index in 0..3 {
            assert_eq!(texture_flags(&b, 0, index).unwrap(), Some(0x20));
        }
    }
    #[test]
    fn physical_flags_ignore_a_four_byte_legacy_word_at_the_resource_tail() {
        let mut b = frames();
        put(&mut b, 32, 244);
        put(&mut b, 252, 0x123456);
        assert_eq!(texture_flags(&b, 0, 0).unwrap(), None);
        put(&mut b, 252, 0xbc614e);
        assert!(texture_flags(&b, 0, 0).is_err());
    }
}
