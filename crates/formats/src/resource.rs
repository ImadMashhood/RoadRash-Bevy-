use crate::{DecodeError, Result, binary::*};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Resource {
    pub tag: String,
    pub id: u32,
    pub offset: u32,
    pub length: u32,
    /// All eight original words, including flags and uninterpreted runtime/cache slots.
    pub raw: [u32; 8],
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Directory {
    pub header: [u32; 6],
    pub table_header: [u32; 4],
    pub entries: Vec<Resource>,
}
impl Directory {
    pub fn read(b: &[u8]) -> Result<Self> {
        if range(b, 0, 4)? != b"CRSR" || range(b, 12, 4)? != b"LBTR" {
            return Err(DecodeError::Invalid("resource magic"));
        }
        let mut header = [0; 6];
        for (i, w) in header.iter_mut().enumerate() {
            *w = u32le(b, i * 4)?;
        }
        let at = header[4] as usize;
        if range(b, at, 4)? != b"LBTR" {
            return Err(DecodeError::Invalid("resource table magic"));
        }
        let mut table_header = [0; 4];
        for (i, w) in table_header.iter_mut().enumerate() {
            *w = u32le(b, at + i * 4)?;
        }
        let count = table_header[2] as usize;
        if count > 65536 {
            return Err(DecodeError::Invalid("resource count limit"));
        }
        let table_size = 16usize
            .checked_add(count * 32)
            .ok_or(DecodeError::Invalid("table size overflow"))?;
        range(b, at, table_size)?;
        let mut entries = Vec::with_capacity(count);
        let mut spans = Vec::new();
        for i in 0..count {
            let mut raw = [0; 8];
            for (j, w) in raw.iter_mut().enumerate() {
                *w = u32le(b, at + 16 + i * 32 + j * 4)?;
            }
            span(b, raw[2], raw[3])?;
            let start = u64::from(raw[2]);
            let end = start + u64::from(raw[3]);
            // Exact payload aliases are permitted. Partial overlaps and directory/header overlaps are rejected.
            if raw[3] > 0 && (start < 24 || (start < (at + table_size) as u64 && end > at as u64)) {
                return Err(DecodeError::Invalid("payload overlaps metadata"));
            }
            if raw[3] > 0 {
                spans.push((start, end));
            }
            let mut tag = raw[0].to_le_bytes();
            tag.reverse();
            if !tag.iter().all(|c| c.is_ascii_graphic() || *c == b' ') {
                return Err(DecodeError::Invalid("non-ASCII resource tag"));
            }
            entries.push(Resource {
                tag: String::from_utf8(tag.to_vec()).unwrap(),
                id: raw[1],
                offset: raw[2],
                length: raw[3],
                raw,
            });
        }
        spans.sort_unstable();
        for pair in spans.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b.0 < a.1 && a != b {
                return Err(DecodeError::Invalid("partially overlapping payloads"));
            }
        }
        Ok(Self {
            header,
            table_header,
            entries,
        })
    }
}
impl Resource {
    pub fn payload<'a>(&self, b: &'a [u8]) -> Result<&'a [u8]> {
        span(b, self.offset, self.length)
    }
}
/// Resource envelope used inside FAM payloads. Output is bounded even for malicious DCL data.
pub fn unpack(b: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read;
    let expected = u32le(b, 0)? as usize;
    let stored = u32le(b, 4)? as usize;
    let crc = u32le(b, 8)?;
    let compressed = u32le(b, 12)?;
    if expected > 16 * 1024 * 1024 {
        return Err(DecodeError::Invalid("unpacked resource limit"));
    }
    let payload = range(b, 16, stored)?;
    if compressed == 0 {
        if expected != stored {
            return Err(DecodeError::Invalid("stored resource size"));
        }
        return Ok(payload.to_vec());
    }
    if compressed != 1 {
        return Err(DecodeError::Invalid("compression flag"));
    }
    let mut out = Vec::new();
    explode::ExplodeReader::new(payload)
        .take(expected as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| DecodeError::Invalid("DCL stream"))?;
    if out.len() != expected || crc32fast::hash(&out) != crc {
        return Err(DecodeError::Invalid("unpacked length/CRC"));
    }
    Ok(out)
}
