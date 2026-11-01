use crate::{DecodeError, Result};
pub fn range(b: &[u8], at: usize, n: usize) -> Result<&[u8]> {
    b.get(
        at..at
            .checked_add(n)
            .ok_or(DecodeError::Invalid("offset overflow"))?,
    )
    .ok_or(DecodeError::Truncated {
        offset: at,
        length: n,
    })
}
pub fn u16le(b: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(range(b, at, 2)?.try_into().unwrap()))
}
pub fn u32le(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(range(b, at, 4)?.try_into().unwrap()))
}
pub fn span(b: &[u8], at: u32, n: u32) -> Result<&[u8]> {
    range(b, at as usize, n as usize)
}
