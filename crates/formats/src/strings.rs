//! Bounded PE RT_STRING reader. Loads text as data; never loads a DLL into the process.
use crate::{DecodeError, Result, binary::*, pe::Pe};
use std::collections::BTreeMap;
fn invalid() -> DecodeError {
    DecodeError::Invalid("PE string resource structure")
}
pub fn resources(bytes: &[u8]) -> Result<BTreeMap<u32, String>> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(invalid());
    }
    let pe = Pe::read(bytes)?;
    let at = u32le(bytes, 60)? as usize;
    if u16le(bytes, at + 20)? < 120 || u32le(bytes, at + 116)? < 3 {
        return Err(invalid());
    }
    let image_base = u32le(bytes, at + 52)?;
    let resource_rva = u32le(bytes, at + 136)?;
    let resource_size = u32le(bytes, at + 140)? as usize;
    if resource_size > 16 * 1024 * 1024 {
        return Err(invalid());
    }
    let directory = pe.read_va(
        image_base.checked_add(resource_rva).ok_or_else(invalid)?,
        resource_size,
    )?;
    let entries = |offset: u32| -> Result<Vec<(u32, u32)>> {
        let p = offset as usize;
        let count = usize::from(u16le(directory, p + 12)?) + usize::from(u16le(directory, p + 14)?);
        if count > 4096 {
            return Err(invalid());
        }
        range(directory, p + 16, count * 8)?;
        (0..count)
            .map(|i| {
                Ok((
                    u32le(directory, p + 16 + i * 8)?,
                    u32le(directory, p + 20 + i * 8)?,
                ))
            })
            .collect()
    };
    let sub = |offset: u32| {
        if offset & 0x80000000 == 0 {
            Err(invalid())
        } else {
            Ok(offset & 0x7fffffff)
        }
    };
    let mut result = BTreeMap::new();
    let mut total_decoded = 0usize;
    for (kind, table) in entries(0)? {
        if kind != 6 {
            continue;
        }
        for (block, names) in entries(sub(table)?)? {
            if !(1..=4096).contains(&block) {
                return Err(invalid());
            }
            for (_language, data) in entries(sub(names)?)? {
                if data & 0x80000000 != 0 {
                    return Err(invalid());
                }
                let data = data as usize;
                let rva = u32le(directory, data)?;
                let size = u32le(directory, data + 4)? as usize;
                total_decoded = total_decoded.checked_add(size).ok_or_else(invalid)?;
                if size > 131104 || total_decoded > 32 * 1024 * 1024 {
                    return Err(invalid());
                }
                let block_bytes =
                    pe.read_va(image_base.checked_add(rva).ok_or_else(invalid)?, size)?;
                let mut p = 0;
                for i in 0..16 {
                    let n = usize::from(u16le(block_bytes, p)?);
                    p += 2;
                    let raw = range(block_bytes, p, n * 2)?;
                    if n > 0 {
                        let units: Vec<_> = raw
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|b| u16::from_le_bytes([b[0], b[1]]))
                            .collect();
                        let text = String::from_utf16(&units).map_err(|_| invalid())?;
                        result.insert((block - 1) * 16 + i, text);
                    }
                    p += n * 2;
                }
            }
        }
    }
    Ok(result)
}
/// Resolve the original game table's bounded continuation records. Unicode is
/// retained for rendering instead of round-tripping translated text through ANSI.
pub fn game_table(resources: &BTreeMap<u32, String>, base: u32) -> Result<Vec<String>> {
    let header = resources.get(&base).ok_or_else(invalid)?;
    let words: Vec<_> = header.split_whitespace().collect();
    if words.len() != 2 {
        return Err(invalid());
    }
    let budget: usize = words[0].parse().map_err(|_| invalid())?;
    let count: usize = words[1].parse().map_err(|_| invalid())?;
    if count == 0 || count > 65536 || budget > 16 * 1024 * 1024 {
        return Err(invalid());
    }
    let mut output = vec![String::new(); count + 1];
    let mut total = 0;
    for (i, value) in output.iter_mut().enumerate().skip(1) {
        *value = resources
            .get(&(base + i as u32))
            .cloned()
            .unwrap_or_default();
        for depth in 0..=64 {
            if !value.ends_with('ÿ') {
                break;
            }
            if depth == 64 {
                return Err(invalid());
            }
            value.pop();
            let index = value.pop().ok_or_else(invalid)? as u32;
            if index > 255 {
                return Err(invalid());
            }
            let id = (3000i32 + index as u8 as i8 as i32) as u32;
            let continuation = resources
                .get(&id)
                .filter(|s| !s.is_empty())
                .ok_or_else(invalid)?;
            value.push_str(continuation);
            if value.len() > 65536 {
                return Err(invalid());
            }
        }
        total += value.len() + 1;
        // UTF-8 may take up to three bytes for each legacy ANSI glyph.
        if total > budget.saturating_mul(3) {
            return Err(invalid());
        }
    }
    Ok(output)
}
