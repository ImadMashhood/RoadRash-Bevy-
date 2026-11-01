//! Bounded ISO9660 primary-volume reader. No Rock Ridge, Joliet, interleaving or multi-extent support.
use crate::{ImportError, Limits, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

const SECTOR: u64 = 2048;
#[derive(Debug, Clone)]
pub struct Entry {
    pub offset: u64,
    pub length: u64,
}
pub struct Iso {
    file: File,
    size: u64,
    pub entries: BTreeMap<String, Entry>,
}
impl Iso {
    pub fn open(path: &Path, limits: &Limits, cancelled: &impl Fn() -> bool) -> Result<Self> {
        let file = crate::open_regular(path)?;
        let size = file.metadata()?.len();
        if size > limits.max_source_bytes {
            return Err(ImportError::Invalid("ISO exceeds source-size limit".into()));
        }
        let mut iso = Self {
            file,
            size,
            entries: BTreeMap::new(),
        };
        let pvd = iso.read(16 * SECTOR, SECTOR as usize)?;
        if pvd.get(..7) != Some(b"\x01CD001\x01") {
            return Err(ImportError::Invalid(
                "unsupported ISO9660 primary volume descriptor".into(),
            ));
        }
        if u16::from_le_bytes([pvd[128], pvd[129]]) != 2048
            || u16::from_be_bytes([pvd[130], pvd[131]]) != 2048
        {
            return Err(ImportError::Invalid(
                "unsupported ISO logical block size".into(),
            ));
        }
        let n = usize::from(pvd[156]);
        let root = pvd
            .get(156..156 + n)
            .ok_or_else(|| ImportError::Invalid("invalid root record".into()))?
            .to_vec();
        let mut visited = BTreeSet::new();
        let mut count = 0;
        iso.walk(&root, "", 0, &mut visited, &mut count, limits, cancelled)?;
        Ok(iso)
    }
    fn read(&mut self, offset: u64, length: usize) -> Result<Vec<u8>> {
        if offset
            .checked_add(length as u64)
            .is_none_or(|end| end > self.size)
        {
            return Err(ImportError::Invalid("ISO extent outside image".into()));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; length];
        self.file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    fn extent(&self, record: &[u8]) -> Result<Entry> {
        if record.len() < 34 {
            return Err(ImportError::Invalid("short ISO directory record".into()));
        }
        let le = |at| u32::from_le_bytes(record[at..at + 4].try_into().unwrap());
        let be = |at| u32::from_be_bytes(record[at..at + 4].try_into().unwrap());
        if le(2) != be(6) || le(10) != be(14) {
            return Err(ImportError::Invalid(
                "inconsistent ISO both-endian extent".into(),
            ));
        }
        let offset = u64::from(le(2)) * SECTOR;
        let length = u64::from(le(10));
        if offset.checked_add(length).is_none_or(|end| end > self.size) {
            return Err(ImportError::Invalid("ISO extent outside image".into()));
        }
        if record[26] != 0 || record[27] != 0 || record[1] != 0 || record[25] & 0x80 != 0 {
            return Err(ImportError::Invalid(
                "unsupported ISO extended/interleaved/multi-extent record".into(),
            ));
        }
        Ok(Entry { offset, length })
    }
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        record: &[u8],
        parent: &str,
        depth: usize,
        visited: &mut BTreeSet<u64>,
        count: &mut usize,
        limits: &Limits,
        cancelled: &impl Fn() -> bool,
    ) -> Result<()> {
        crate::check_cancel(cancelled)?;
        if depth > limits.max_depth {
            return Err(ImportError::Invalid(
                "ISO directory depth limit exceeded".into(),
            ));
        }
        let entry = self.extent(record)?;
        if entry.length > limits.max_directory_bytes || !visited.insert(entry.offset) {
            return Err(ImportError::Invalid(
                "oversized or cyclic ISO directory".into(),
            ));
        }
        let block = self.read(entry.offset, entry.length as usize)?;
        let mut names = BTreeSet::new();
        let mut at = 0;
        while at < block.len() {
            crate::check_cancel(cancelled)?;
            let n = usize::from(block[at]);
            if n == 0 {
                at = (at / 2048 + 1) * 2048;
                continue;
            }
            if n < 34 || at + n > block.len() || at % 2048 + n > 2048 {
                return Err(ImportError::Invalid("invalid ISO record size".into()));
            }
            let r = &block[at..at + n];
            at += n;
            let length = usize::from(r[32]);
            let name = r
                .get(33..33 + length)
                .ok_or_else(|| ImportError::Invalid("invalid ISO identifier length".into()))?;
            if name == [0] || name == [1] {
                continue;
            }
            *count += 1;
            if *count > limits.max_entries {
                return Err(ImportError::Invalid("ISO entry limit exceeded".into()));
            }
            let name = std::str::from_utf8(name)
                .map_err(|_| ImportError::Invalid("non-UTF8 primary ISO identifier".into()))?;
            let name = name.split(';').next().unwrap_or_default();
            crate::validate_component(name)?;
            if !names.insert(name.to_uppercase()) {
                return Err(ImportError::Invalid(format!(
                    "ambiguous ISO directory identifier: {name}"
                )));
            }
            let path = if parent.is_empty() {
                name.to_uppercase()
            } else {
                format!("{parent}/{}", name.to_uppercase())
            };
            if r[25] & 2 != 0 {
                self.walk(r, &path, depth + 1, visited, count, limits, cancelled)?;
            } else {
                let entry = self.extent(r)?;
                if entry.length > limits.max_file_bytes {
                    return Err(ImportError::Invalid(format!("oversized ISO file: {path}")));
                }
                if self.entries.insert(path.clone(), entry).is_some() {
                    return Err(ImportError::Invalid(format!("ambiguous ISO path: {path}")));
                }
            }
        }
        Ok(())
    }
    pub fn copy_file(
        &mut self,
        entry: &Entry,
        output: &mut File,
        expected: u64,
        cancelled: &impl Fn() -> bool,
    ) -> Result<String> {
        if entry.length != expected {
            return Err(ImportError::Invalid("source size changed".into()));
        }
        self.file.seek(SeekFrom::Start(entry.offset))?;
        crate::copy_hash(
            &mut (&mut self.file).take(entry.length),
            output,
            entry.length,
            cancelled,
        )
    }
}
