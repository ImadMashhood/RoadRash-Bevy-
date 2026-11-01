//! Tick-driven source animation descriptors, independent of pixel resolution.
use rr_formats::{
    binary::{range, u32le},
    pe::SupportedExe,
    resource::Directory,
};
use serde::{Deserialize, Serialize};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Frame {
    pub render: [u8; 12],
    pub duration: i8,
    pub control: u8,
    pub flags: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cell {
    pub frame: usize,
    pub flags: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Library {
    pub banks: [Vec<Option<Vec<Frame>>>; 2],
    pub views: [Vec<Vec<Cell>>; 2],
    pub fallback: Vec<i8>,
    pub attacks: [[[i8; 4]; 2]; 2],
}
struct Layout {
    views: Vec<Vec<Cell>>,
    codes: Vec<u32>,
    info: Vec<[u32; 2]>,
    flags: Vec<u32>,
}
impl Layout {
    fn read(b: &[u8]) -> Result<Self> {
        let w = |p| -> Result<u32> { Ok(u32le(b, p)?) };
        let mut chunk = 0;
        while chunk < b.len() {
            let n = w(chunk + 4)? as usize;
            if n < 8 {
                return Err("short CANS chunk".into());
            }
            range(b, chunk, n)?;
            chunk += n;
        }
        if w(0)? != 0x43414e53 {
            return Err("CANS magic".into());
        }
        let kind = w(8)?;
        if !(1..=4).contains(&kind) {
            return Err("CANS type".into());
        }
        let count = w(if kind == 1 { 44 } else { 12 })? as usize;
        if count == 0 || count > 4096 {
            return Err("CANS count".into());
        }
        let header = w(4)? as usize;
        let mut group = if kind == 1 { 48 } else { 16 };
        let mut views = vec![vec![]; count];
        let mut info = vec![[0; 2]; count];
        while group < header {
            let id = w(group)? as usize;
            let frames = w(group + 4)? as usize;
            let delay = w(group + 8)?;
            if id == 0 || id > count || frames == 0 || frames > 32767 || !views[id - 1].is_empty() {
                return Err("CANS group".into());
            }
            range(b, group, 16 + frames * 28)?;
            for f in 0..frames {
                let frame = (w(group + 16 + f * 28)? >> 16) as usize;
                if frame == 0 {
                    return Err("CANS frame".into());
                }
                views[id - 1].push(Cell {
                    frame: frame - 1,
                    flags: 0,
                });
            }
            info[id - 1] = [frames as u32, delay];
            group += 16 + frames * 28;
        }
        if group != header || views.iter().any(Vec::is_empty) {
            return Err("CANS incomplete groups".into());
        }
        let mut cel = if kind == 1 { 0 } else { header };
        cel += w(cel + 4)? as usize + 12;
        let mut flags = vec![];
        for cells in &mut views {
            let words = w(cel)? as usize;
            if words < cells.len() || words > 65536 {
                return Err("CANS cel count".into());
            }
            range(b, cel, 4 + words * 4)?;
            flags.push(w(cel + 4)?);
            for (f, c) in cells.iter_mut().enumerate() {
                c.flags = w(cel + 4 + f * 4)?;
            }
            cel += 4 + words * 4;
        }
        let mut table = header;
        for _ in 0..3 {
            table += w(table + 4)? as usize;
        }
        table += 12;
        range(b, table, count * 16)?;
        let codes = (0..count)
            .map(|i| w(table + i * 16))
            .collect::<Result<_>>()?;
        Ok(Self {
            views,
            codes,
            flags,
            info,
        })
    }
    fn sequence(&self, source: &[u8]) -> Result<Option<Vec<Frame>>> {
        let mut out = vec![];
        for raw in source.as_chunks::<8>().0 {
            let code = u32le(raw, 0)? as i32;
            let wanted = code.unsigned_abs();
            let mut starts = [-1i32; 3];
            let mut ends = [0i32; 3];
            let mut cur = 0;
            for g in 0..3 {
                while cur < self.codes.len() && self.codes[cur] != wanted {
                    cur += 1;
                }
                if cur == self.codes.len() {
                    break;
                }
                starts[g] = cur as i32;
                let flag = self.flags[cur];
                cur += 1;
                while cur < self.codes.len()
                    && self.codes[cur] == wanted
                    && (self.flags[cur] & 6) == (flag & 6)
                {
                    cur += 1;
                }
                ends[g] = cur as i32;
            }
            if starts[0] < 0 {
                return Ok(None);
            }
            if starts[1] < 0 {
                starts[1] = starts[0];
                ends[1] = ends[0];
                starts[0] = -1;
                starts[2] = -1;
            } else if starts[2] < 0 {
                if self.flags[starts[1] as usize] & 6 != 0 {
                    if self.flags[starts[1] as usize] & 2 == 0 {
                        starts[2] = starts[1];
                        starts[1] = starts[0];
                        starts[0] = -1;
                        ends[1] = ends[0];
                    } else {
                        starts.swap(0, 1);
                        let e = ends[0] ^ ends[1];
                        ends[1] = ends[0];
                        ends[0] = e;
                    }
                }
            } else {
                if self.flags[starts[0] as usize] & 2 == 0 {
                    let other = if self.flags[starts[1] as usize] & 2 == 0 {
                        2
                    } else {
                        1
                    };
                    starts.swap(0, other);
                    let e = ends[0] ^ ends[other];
                    ends[other] = ends[0];
                    ends[0] = e;
                }
                if self.flags[starts[1] as usize] & 4 != 0 {
                    starts.swap(1, 2);
                    ends.swap(1, 2);
                }
            }
            let middle = starts[1] as usize;
            let flags = self.flags[middle];
            let info = self.info[middle];
            let mut render = [0u8; 12];
            render[..4].copy_from_slice(&(middle as u32).to_le_bytes());
            render[4] = raw[4];
            if flags & 0xf800 == 0 {
                render[6] = info[0] as u8;
            } else {
                render[9] = 1;
                render[5] = info[0] as u8;
                render[6] = (ends[1] - starts[1]) as u8;
            }
            let bit = |mask, v| if flags & mask != 0 { v } else { 0 };
            render[7] = (((bit(0x2000, 0xaaaaa)
                + bit(0x8000, 0x200000)
                + bit(0x1000, 0x55555)
                + bit(0x4000, 0x155555)
                + bit(0x800, 0x2aaaa))
                >> 16)
                * 2) as u8;
            render[8] = (((bit(0x400, 0x400000)
                + bit(0x40, 0x55555)
                + bit(0x200, 0x2aaaaa)
                + bit(0x80, 0xaaaaa)
                + bit(0x20, 0x2aaaa)
                + bit(0x100, 0x155555))
                >> 16)
                * 2) as u8;
            render[10] = if starts[0] < 0 {
                0
            } else {
                (starts[0] - starts[1]) as u8
            };
            render[11] = if starts[2] < 0 {
                0
            } else {
                (starts[2] - starts[1]) as u8
            };
            out.push(Frame {
                render,
                duration: (info[1] / 20) as u8 as i8,
                control: raw[5],
                flags,
            });
        }
        Ok(Some(out))
    }
}
impl Library {
    /// Native pedestrian CANS layout plus the seven executable sequence tables.
    pub fn pedestrian(exe: &[u8], cans: &[u8]) -> Result<Self> {
        let pe = SupportedExe::read(exe)?;
        let layout = Layout::read(cans)?;
        let mut bank = vec![None; 52];
        for (code, (va, count)) in [
            (0x46de48, 6),
            (0x46de78, 1),
            (0x46de80, 2),
            (0x46de90, 6),
            (0x46dec0, 2),
            (0x46ded0, 6),
            (0x46df00, 6),
        ]
        .into_iter()
        .enumerate()
        {
            bank[code] = layout.sequence(pe.read_va(va, count * 8)?)?;
        }
        if bank[1].is_none() {
            return Err("Pedestrian idle sequence is absent".into());
        }
        let mut fallback = vec![-1; 52];
        for code in 0..52 {
            if bank[code].is_none() {
                fallback[code] = 1;
            }
        }
        let value = Self {
            banks: [bank, vec![None; 52]],
            views: [layout.views, vec![vec![Cell { frame: 0, flags: 0 }]]],
            fallback,
            attacks: [[[0; 4]; 2]; 2],
        };
        value.validate()?;
        Ok(value)
    }
    pub fn read(exe: &[u8], global: &[u8]) -> Result<Self> {
        let pe = SupportedExe::read(exe)?;
        let dir = Directory::read(global)?;
        let mut layouts = vec![];
        for id in 1..=2 {
            let r = dir
                .entries
                .iter()
                .find(|r| r.tag == "CANS" && r.id == id)
                .ok_or("missing CANS")?;
            layouts.push(Layout::read(range(
                global,
                r.offset as usize,
                r.length as usize,
            )?)?);
        }
        let mut relocated = std::collections::BTreeMap::new();
        for &(bank, source, dest, count) in BINDINGS {
            relocated.insert(
                dest,
                layouts[bank].sequence(pe.read_va(source, count * 8)?)?,
            );
        }
        let mut fallback = vec![];
        for i in 0..52 {
            fallback.push(u32le(pe.read_va(0x464430 + i * 4, 4)?, 0)? as u8 as i8);
        }
        let mut banks: [Vec<Option<Vec<Frame>>>; 2] = [vec![], vec![]];
        for (bank, output) in banks.iter_mut().enumerate() {
            let va = u32le(pe.read_va(0x474d78 + bank as u32 * 4, 4)?, 0)?;
            for code in 0..52 {
                let ptr = u32le(pe.read_va(va + code * 4, 4)?, 0)?;
                let value = if ptr == 0 || ptr == 0x4b9df0 || ptr == 0x4b9280 {
                    None
                } else {
                    relocated
                        .get(&ptr)
                        .ok_or("unresolved animation slot")?
                        .clone()
                };
                output.push(value);
            }
        }
        let views = [layouts.remove(0).views, layouts.remove(0).views];
        let mut attacks = [[[0; 4]; 2]; 2];
        for (side, kinds) in attacks.iter_mut().enumerate() {
            for (kind, weapons) in kinds.iter_mut().enumerate() {
                for (w, code) in weapons.iter_mut().enumerate() {
                    *code = u32le(
                        pe.read_va(0x4644f8 + side as u32 * 32 + (kind * 4 + w) as u32 * 4, 4)?,
                        0,
                    )? as u8 as i8;
                }
            }
        }
        let result = Self {
            banks,
            views,
            fallback,
            attacks,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<()> {
        if self.fallback.len() != 52
            || self.views.iter().any(|bank| {
                bank.is_empty()
                    || bank.len() > 4096
                    || bank
                        .iter()
                        .any(|group| group.is_empty() || group.len() > 32767)
            })
            || self.views[0].iter().flatten().any(|c| c.frame >= 326)
            || self.views[1].iter().flatten().any(|c| c.frame >= 45)
        {
            return Err("animation views/fallback bounds".into());
        }
        for (bank, sequences) in self.banks.iter().enumerate() {
            if sequences.len() != 52 || self.views[bank].is_empty() {
                return Err("animation bank count".into());
            }
            for sequence in sequences.iter().flatten() {
                if sequence.is_empty() || sequence.len() > 127 {
                    return Err("animation length".into());
                }
                for (i, f) in sequence.iter().enumerate() {
                    if f.duration <= 0 {
                        return Err("nonpositive animation duration".into());
                    }
                    match f.control & 15 {
                        0 if i + 1 >= sequence.len() => {
                            return Err("animation falls off sequence".into());
                        }
                        1 if (f.control >> 4) as usize >= sequence.len() => {
                            return Err("animation loop target".into());
                        }
                        _ => {}
                    }
                    self.select(bank, f, 0, 0)?;
                }
            }
        }
        for code in [2, 8, 10, 11, 12, 13, 28, 29, 36, 37, 38, 39, 40, 44, 48] {
            self.resolve(0, code)
                .ok_or("missing required animation state")?;
        }
        Ok(())
    }
    pub fn pose_code(&self, pose: Pose, weapon: Weapon) -> i8 {
        let (side, kind, w) = match pose {
            Pose::PunchLeft => (0, 0, weapon as usize),
            Pose::PunchRight => (1, 0, weapon as usize),
            Pose::KickLeft => (0, 0, 3),
            Pose::KickRight => (1, 0, 3),
            Pose::BackhandLeft => (0, 1, weapon as usize),
            Pose::BackhandRight => (1, 1, weapon as usize),
            _ => return pose.code(),
        };
        self.attacks[side][kind][w]
    }
    pub fn resolve(&self, bank: usize, code: i8) -> Option<usize> {
        let available = |i: usize| {
            self.banks
                .get(bank)?
                .get(i)?
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|_| i)
        };
        if code < 0 {
            return available(0);
        }
        available(code as usize).or_else(|| {
            let next = *self.fallback.get(code as usize)?;
            if next < 0 {
                available(0)
            } else {
                available(next as usize)
            }
        })
    }
    pub fn select(
        &self,
        bank: usize,
        f: &Frame,
        mut yaw: i32,
        mut roll: i32,
    ) -> Result<(Cell, i32)> {
        let d = f.render;
        let base = u32::from_le_bytes(d[..4].try_into()?) as usize;
        let views = self.views.get(bank).ok_or("bank")?;
        let first = views.get(base).and_then(|v| v.first()).ok_or("base view")?;
        if first.flags & 8 != 0 {
            yaw = yaw.wrapping_add(0x7f0000);
        }
        if yaw > 0x7effff {
            yaw = yaw.wrapping_sub(0xff0000);
        }
        if roll > 0x7effff {
            roll = roll.wrapping_sub(0xff0000);
        }
        let byte = |v: u8| i32::from(v as i8);
        let mut view = 0;
        if d[8] != 0 {
            view = if d[8] < 250 {
                let factor = byte(d[6] & 0xfe) * 4096 / i32::from(d[8]);
                factor.wrapping_mul(i32::from(d[8]) * 128 + yaw / 256) / 16 + 0x8000
            } else {
                let n = byte(d[6]);
                match n {
                    2 => (yaw + 0x800000) / 128,
                    4 => (yaw + 0x800000) / 64,
                    8 => (yaw + 0x800000) / 32,
                    _ => (yaw + 0x800000).wrapping_mul(n) / 256,
                }
            };
            view = (view >> 16).min(byte(d[6]) - 1).max(0);
        }
        let mut elev = 0;
        if d[9] != 0 && byte(d[5]) >= 2 {
            let step = i32::from(d[7]) * 65536 / byte(d[5]);
            if step == 0 {
                return Err("zero view step".into());
            }
            elev = ((i32::from(d[7]) * 32768 + roll) / step) as u8 as i8 as i32;
            elev = elev.clamp(0, byte(d[5]) - 1);
        }
        let group = base + if d[9] != 0 { view as usize } else { 0 };
        let index = byte(d[4]) + if d[9] != 0 { elev } else { view };
        if index < 0 {
            return Err("negative frame".into());
        }
        let c = views
            .get(group)
            .and_then(|v| v.get(index as usize))
            .ok_or("view frame out of range")?
            .clone();
        if d[9] != 0 && byte(d[5]) >= 2 {
            let flags = views
                .get(base + view as usize)
                .and_then(|v| v.get(elev as usize))
                .ok_or("roll correction view")?
                .flags;
            let mut correction = 0i32;
            for (mask, value) in [
                (0x2000, 0xaaaaa),
                (0x8000, 0x200000),
                (0x800, 0x2aaaa),
                (0x1000, 0x55555),
                (0x4000, 0x155555),
            ] {
                if flags & mask != 0 {
                    correction += value;
                }
            }
            correction = correction.clamp(-7, 7);
            if flags & 0x10 != 0 {
                correction = -correction;
            }
            roll = roll.wrapping_sub(correction);
        }
        Ok((c, roll))
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Player {
    pub code: i8,
    pub frame: usize,
    pub queued: Option<i8>,
    pub loops: u8,
    pub deadline: Option<u32>,
    pub selected: Option<(usize, usize)>,
}
impl Player {
    fn banks_empty(&self, lib: &Library, bank: usize) -> bool {
        lib.banks
            .get(bank)
            .and_then(|b| b.first())
            .is_none_or(Option::is_none)
    }
    pub fn set(&mut self, lib: &Library, bank: usize, code: i8) -> bool {
        if let Some(code) = lib.resolve(bank, code) {
            self.code = code as i8;
            self.frame = 0;
            self.loops = 0;
            self.deadline = None;
            true
        } else {
            false
        }
    }
    pub fn advance(&mut self, lib: &Library, bank: usize, tick: u32) -> Result<Option<Frame>> {
        let get = |code: i8, index: usize| -> Result<&Frame> {
            lib.banks
                .get(bank)
                .and_then(|b| b.get(code.max(0) as usize))
                .and_then(Option::as_ref)
                .and_then(|s| s.get(index))
                .ok_or_else(|| "invalid playback frame".into())
        };
        if self.code < 0 && self.banks_empty(lib, bank) {
            self.selected = None;
            return Ok(None);
        }
        let f = get(self.code, self.frame)?.clone();
        self.selected = Some((self.code.max(0) as usize, self.frame));
        let deadline = *self
            .deadline
            .get_or_insert(tick.wrapping_add(f.duration as u32));
        if deadline <= tick {
            match f.control & 15 {
                0 => self.frame += 1,
                1 if self.queued.is_none() => {
                    self.frame = (f.control >> 4) as usize;
                    self.loops = self.loops.wrapping_add(1);
                }
                _ => {
                    self.frame = 0;
                    self.loops = 0;
                    self.code = self.queued.take().unwrap_or(-1);
                }
            }
            if !(self.code < 0 && self.banks_empty(lib, bank)) {
                self.deadline =
                    Some(deadline.wrapping_add(get(self.code, self.frame)?.duration as u32));
            }
        }
        Ok(Some(f))
    }
}
/// Semantic poses are authoritative inputs. Combat hit resolution remains phase 11.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Pose {
    HitReact,
    #[default]
    Ride,
    Idle,
    Accelerate,
    Tuck,
    PunchLeft,
    PunchRight,
    KickLeft,
    KickRight,
    BackhandLeft,
    BackhandRight,
    Crash,
    Slide,
    GetUp,
    Stand,
    Walk,
    Remount,
}
impl Pose {
    pub fn code(self) -> i8 {
        match self {
            Self::HitReact => 9,
            Self::Ride => 2,
            Self::Idle => 8,
            Self::Accelerate => 4,
            Self::Tuck => 3,
            Self::PunchLeft => 10,
            Self::PunchRight => 11,
            Self::KickLeft => 12,
            Self::KickRight => 13,
            Self::BackhandLeft => 14,
            Self::BackhandRight => 15,
            Self::Crash => 33,
            Self::Slide => 29,
            Self::GetUp => 36,
            Self::Stand => 37,
            Self::Walk => 38,
            Self::Remount => 41,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Weapon {
    #[default]
    Fists,
    Club,
    Chain,
}

// Relocation schema only; all descriptor values are imported from the verified EXE.
const BINDINGS: &[(usize, u32, u32, usize)] = &[
    (0, 0x004746c0, 0x004b92e0, 1),
    (0, 0x004746c8, 0x004b93f0, 1),
    (0, 0x004746d0, 0x004b9200, 1),
    (0, 0x004746d8, 0x004b9470, 1),
    (0, 0x004746e0, 0x004b9a90, 1),
    (0, 0x004746e8, 0x004b93c0, 2),
    (0, 0x004746f8, 0x004b9bb0, 2),
    (0, 0x00474708, 0x004b9450, 1),
    (0, 0x00474710, 0x004b9ef0, 1),
    (0, 0x00474718, 0x004b9b40, 2),
    (0, 0x00474728, 0x004b99b0, 5),
    (0, 0x00474750, 0x004b96c0, 5),
    (0, 0x00474778, 0x004b9a20, 5),
    (0, 0x004747a0, 0x004b9650, 5),
    (0, 0x004747c8, 0x004b9100, 2),
    (0, 0x004747d8, 0x004b98f0, 2),
    (0, 0x004747e8, 0x004b9130, 3),
    (0, 0x00474800, 0x004b9190, 3),
    (0, 0x00474818, 0x004b9020, 3),
    (0, 0x00474830, 0x004b9380, 3),
    (0, 0x00474868, 0x004b9410, 3),
    (0, 0x00474880, 0x004b9300, 3),
    (0, 0x00474898, 0x004b9eb0, 3),
    (0, 0x004748b0, 0x004b9b70, 3),
    (0, 0x004748c8, 0x004b91d0, 2),
    (0, 0x004748d8, 0x004b92b0, 2),
    (0, 0x004748e8, 0x004b9860, 7),
    (0, 0x00474920, 0x004b9540, 8),
    (0, 0x00474960, 0x004b97a0, 4),
    (0, 0x00474980, 0x004b9920, 4),
    (0, 0x004749a0, 0x004ba060, 4),
    (0, 0x004749c0, 0x004b9d00, 5),
    (0, 0x004749e8, 0x004b94f0, 4),
    (0, 0x00474a08, 0x004b9b10, 2),
    (0, 0x00474a18, 0x004b9060, 5),
    (0, 0x00474a40, 0x004b95e0, 4),
    (0, 0x00474a60, 0x004b9d70, 6),
    (0, 0x00474a90, 0x004b9fa0, 5),
    (0, 0x00474ab8, 0x004b9c20, 5),
    (0, 0x00474ae0, 0x004b9f10, 5),
    (0, 0x00474b08, 0x004b97f0, 5),
    (0, 0x00474b30, 0x004b9e20, 5),
    (0, 0x00474b58, 0x004ba010, 4),
    (0, 0x00474b78, 0x004b9230, 4),
    (0, 0x00474b98, 0x004b9970, 3),
    (0, 0x00474bb0, 0x004ba0b0, 3),
    (0, 0x00474bc8, 0x004b9f80, 1),
    (0, 0x00474bd0, 0x004b9170, 1),
    (1, 0x004746d0, 0x004b9af0, 1),
    (1, 0x00474710, 0x004b9e90, 1),
    (1, 0x00474718, 0x004b90d0, 2),
    (1, 0x004747e8, 0x004b9ab0, 3),
    (1, 0x00474800, 0x004b9340, 3),
    (1, 0x004748e8, 0x004ba0f0, 7),
    (1, 0x004749c0, 0x004b9c90, 5),
    (1, 0x00474b30, 0x004b9730, 5),
    (1, 0x00474bc8, 0x004b9630, 1),
    (1, 0x00474b98, 0x004b9be0, 3),
    (1, 0x00474a40, 0x004b9490, 4),
];
