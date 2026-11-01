//! Bounded hierarchy walk; unknown leaves remain opaque metadata instead of being assigned invented semantics.
use crate::{DecodeError, Result, binary::*, pixels::Texture, resource};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FamilyNode {
    pub offset: u32,
    pub tag: u32,
    pub children: Vec<u32>,
    pub raw_header: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FamilyTexture {
    pub group_offset: u32,
    pub texture_offset: u32,
    pub column: u16,
    pub row: u16,
    pub logical_width: u16,
    pub logical_height: u16,
    pub texture: Texture,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Family {
    pub nodes: Vec<FamilyNode>,
    pub textures: Vec<FamilyTexture>,
    pub unpacked_length: usize,
}
#[derive(Default)]
struct Budget {
    pixels: usize,
    edges: usize,
}

fn relative(b: &[u8], base: u32, offset: u32, n: usize) -> Result<u32> {
    let at = base
        .checked_add(offset)
        .ok_or(DecodeError::Invalid("family pointer overflow"))?;
    range(b, at as usize, n)?;
    Ok(at)
}
fn power(n: u16) -> Result<u16> {
    if n == 255 {
        Ok(0)
    } else if n <= 9 {
        Ok(1 << n)
    } else {
        Err(DecodeError::Invalid("group dimension exponent"))
    }
}
impl Family {
    pub fn read(payload: &[u8]) -> Result<Self> {
        let b = resource::unpack(payload)?;
        Self::read_unpacked(&b)
    }
    pub fn read_unpacked(b: &[u8]) -> Result<Self> {
        let mut nodes = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        let mut textures = vec![];
        let mut budget = Budget::default();
        fn visit(
            b: &[u8],
            at: u32,
            depth: usize,
            nodes: &mut BTreeMap<u32, FamilyNode>,
            visiting: &mut BTreeSet<u32>,
            textures: &mut Vec<FamilyTexture>,
            budget: &mut Budget,
        ) -> Result<()> {
            if depth > 32 || nodes.len() > 65536 || textures.len() > 65536 {
                return Err(DecodeError::Invalid("family hierarchy limit"));
            }
            if nodes.contains_key(&at) {
                return Ok(());
            }
            if !visiting.insert(at) {
                return Err(DecodeError::Invalid("family cycle"));
            }
            let tag = u32le(b, at as usize)?;
            let mut children = vec![];
            if tag < 0x1000 {
                for i in 0..tag {
                    let delta = u32le(b, at as usize + 4 + i as usize * 4)?;
                    let target = relative(b, at, delta, 4)?;
                    budget.edges += 1;
                    if budget.edges > 65536 {
                        return Err(DecodeError::Invalid("family edge budget"));
                    }
                    children.push(target);
                    visit(b, target, depth + 1, nodes, visiting, textures, budget)?;
                }
            } else if tag == 0x4f465354 {
                let target = relative(b, at, u32le(b, at as usize + 4)?, 4)?;
                budget.edges += 1;
                if budget.edges > 65536 {
                    return Err(DecodeError::Invalid("family edge budget"));
                }
                children.push(target);
                visit(b, target, depth + 1, nodes, visiting, textures, budget)?;
            } else if tag == 0x434c4750 {
                let h = range(b, at as usize, 28)?;
                let mode = h[20] & 3;
                let columns = usize::from(h[24]) + 1;
                let rows = usize::from(h[25]) + 1;
                let table = relative(b, at, u32::from(h[21]) * 4, 4)?;
                if table < at + 28 || mode > 1 {
                    return Err(DecodeError::Invalid("family group table/mode"));
                }
                // Validate the same CCB/hotspot references that the source loader consumes.
                relative(b, at + 12, u32le(h, 8)?, 68)?;
                relative(b, at + 12, u32le(h, 12)?, 4)?;
                relative(b, at + 8, u32le(h, 16)?, 4)?;
                let mut cell =
                    |delta: u32, x: usize, y: usize, w: u16, height: u16| -> Result<()> {
                        let offset = relative(b, at + 8, delta, 56)?;
                        let texture = Texture::read(&b[offset as usize..])?;
                        budget.pixels = budget
                            .pixels
                            .checked_add(
                                texture.mips.iter().map(|m| m.indices.len()).sum::<usize>(),
                            )
                            .ok_or(DecodeError::Invalid("family pixel budget overflow"))?;
                        if budget.pixels > 64 * 1024 * 1024 {
                            return Err(DecodeError::Invalid("family decoded pixel budget"));
                        }
                        textures.push(FamilyTexture {
                            group_offset: at,
                            texture_offset: offset,
                            column: x as u16,
                            row: y as u16,
                            logical_width: w,
                            logical_height: height,
                            texture,
                        });
                        Ok(())
                    };
                if mode == 0 {
                    for y in 0..rows {
                        let row = relative(b, at, u32le(b, table as usize + y * 4)?, columns * 4)?;
                        for x in 0..columns {
                            cell(
                                u32le(b, row as usize + x * 4)?,
                                x,
                                y,
                                power(u16::from(h[26]) + x as u16)?,
                                power(u16::from(h[27]) + y as u16)?,
                            )?;
                        }
                    }
                } else {
                    let thresholds = relative(b, at, u32::from(h[22]) * 4, columns * 2)?;
                    if thresholds < at + 28 {
                        return Err(DecodeError::Invalid("family threshold table"));
                    }
                    for x in 0..columns {
                        let dims = range(b, thresholds as usize + x * 2, 2)?;
                        let (w, hgt) = if h[20] & 0x40 != 0 {
                            (power(u16::from(dims[0]))?, power(u16::from(dims[1]))?)
                        } else {
                            (u16::from(dims[0]), u16::from(dims[1]))
                        };
                        if h[20] & 0x40 == 0 && (w == 0 || hgt == 0) {
                            return Err(DecodeError::Invalid("family zero dimensions"));
                        }
                        cell(u32le(b, table as usize + x * 4)?, x, 0, w, hgt)?;
                    }
                }
            }
            let start = at as usize;
            let end = b.len().min(start + 32);
            nodes.insert(
                at,
                FamilyNode {
                    offset: at,
                    tag,
                    children,
                    raw_header: b[start..end].to_vec(),
                },
            );
            visiting.remove(&at);
            Ok(())
        }
        visit(
            b,
            0,
            0,
            &mut nodes,
            &mut visiting,
            &mut textures,
            &mut budget,
        )?;
        Ok(Self {
            nodes: nodes.into_values().collect(),
            textures,
            unpacked_length: b.len(),
        })
    }
}
