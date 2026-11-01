use crate::{DecodeError, Result, binary::*};
use sha2::{Digest, Sha256};
pub const EXE_SHA256: &str = "73d7f9744b5b1a9482b3909950cdf3fe34119fe13d023b031e9c6739a60f18a2";
pub struct Pe<'a> {
    bytes: &'a [u8],
    base: u32,
    sections: Vec<(u32, u32, u32)>,
}
impl<'a> Pe<'a> {
    /// Generic PE structural reader; never authorizes version-specific tables by itself.
    pub fn read(b: &'a [u8]) -> Result<Self> {
        if range(b, 0, 2)? != b"MZ" {
            return Err(DecodeError::Invalid("MZ signature"));
        }
        let p = u32le(b, 60)? as usize;
        if range(b, p, 4)? != b"PE\0\0" || u16le(b, p + 4)? != 0x14c {
            return Err(DecodeError::Invalid("PE32/i386 signature"));
        }
        let size = u16le(b, p + 20)? as usize;
        if size < 32 || u16le(b, p + 24)? != 0x10b {
            return Err(DecodeError::Invalid("PE optional header"));
        }
        range(b, p + 24, size)?;
        let base = u32le(b, p + 52)?;
        let count = u16le(b, p + 6)? as usize;
        if count > 96 {
            return Err(DecodeError::Invalid("PE section count"));
        }
        let mut sections = vec![];
        for i in 0..count {
            let at = p + 24 + size + i * 40;
            range(b, at, 40)?;
            let va = u32le(b, at + 12)?;
            let length = u32le(b, at + 16)?;
            let raw = u32le(b, at + 20)?;
            span(b, raw, length)?;
            sections.push((va, length, raw));
        }
        Ok(Self {
            bytes: b,
            base,
            sections,
        })
    }
    pub fn read_va(&self, address: u32, length: usize) -> Result<&'a [u8]> {
        let rva = address
            .checked_sub(self.base)
            .ok_or(DecodeError::Invalid("VA below image base"))?;
        for &(va, size, raw) in &self.sections {
            if let Some(displacement) = rva.checked_sub(va)
                && displacement < size
                && length <= (size - displacement) as usize
            {
                let at = u64::from(raw) + u64::from(displacement);
                return range(self.bytes, at as usize, length);
            }
        }
        Err(DecodeError::Invalid("unmapped/cross-section VA"))
    }
}
pub struct SupportedExe<'a>(Pe<'a>);
impl<'a> SupportedExe<'a> {
    pub fn read(b: &'a [u8]) -> Result<Self> {
        if format!("{:x}", Sha256::digest(b)) != EXE_SHA256 {
            return Err(DecodeError::Invalid("unsupported executable hash"));
        }
        Ok(Self(Pe::read(b)?))
    }
    pub fn read_va(&self, address: u32, length: usize) -> Result<&'a [u8]> {
        self.0.read_va(address, length)
    }
    /// Full runtime rider identity bank. The catalog keeps its original ten-preview contract.
    pub fn all_rider_remaps(&self) -> Result<Vec<[u8; 256]>> {
        Ok(self
            .read_va(0x46e158, 15 * 256)?
            .as_chunks::<256>()
            .0
            .to_vec())
    }
    pub fn rider_remaps(&self) -> Result<Vec<[u8; 256]>> {
        Ok(self
            .read_va(0x46e158, 10 * 256)?
            .as_chunks::<256>()
            .0
            .to_vec())
    }
}
