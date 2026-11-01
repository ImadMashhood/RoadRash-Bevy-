//! Startup-only source/catalog resolution. Runtime drawing never reads files.
use crate::{Result, camera::DepthTable, raster::Texture};
use rr_formats::{
    binary::{range, u32le},
    family::Family,
    pixels::{Cel, Palette, Rgba},
    resource::{Directory, unpack},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::Arc,
};
#[derive(Clone)]
pub struct SpriteAsset {
    pub texture: Texture,
    pub logical: [f64; 2],
    pub pivot: [f64; 2],
}
#[derive(Clone)]
pub struct Material {
    pub texture: Texture,
    pub height: f64,
}
#[derive(Clone)]
pub struct FamilyAssets {
    pub definition: Arc<Family>,
    pub groups: BTreeMap<u32, Material>,
    pub sprites: BTreeMap<u32, SpriteAsset>,
}
impl FamilyAssets {
    fn unalias(&self, mut at: u32) -> Option<&rr_formats::family::FamilyNode> {
        for _ in 0..33 {
            let n = self
                .definition
                .nodes
                .binary_search_by_key(&at, |n| n.offset)
                .ok()
                .map(|i| &self.definition.nodes[i])?;
            if n.tag == 0x4f465354 {
                at = *n.children.first()?
            } else {
                return Some(n);
            }
        }
        None
    }
    pub fn resolve(&self, category: usize, selector: usize, frame: Option<usize>) -> Option<u32> {
        let cat = *self.unalias(0)?.children.get(category)?;
        let selected = *self.unalias(cat)?.children.get(selector)?;
        let selected = if let Some(frame) = frame {
            *self.unalias(selected)?.children.get(frame)?
        } else {
            selected
        };
        Some(self.unalias(selected)?.offset)
    }
    pub fn material(
        &self,
        category: usize,
        selector: usize,
        frame: Option<usize>,
    ) -> Option<&Material> {
        let at = self
            .resolve(category, selector, frame)
            .or_else(|| self.resolve(category, selector, None))?;
        if let Some(g) = self.groups.get(&at) {
            return Some(g);
        }
        let n = self.unalias(at)?;
        for child in &n.children {
            if let Some(g) = self.groups.get(&self.unalias(*child)?.offset) {
                return Some(g);
            }
        }
        None
    }
    pub fn sprite(&self, category: usize, selector: usize) -> Option<&SpriteAsset> {
        let frame = if category == 4 { Some(0) } else { None };
        let at = self.resolve(category, selector, frame)?;
        self.sprites.get(&at)
    }
}
#[derive(Clone)]
pub struct SourceAssets {
    pub depths: DepthTable,
    pub road: Vec<Texture>,
    pub ground: Vec<Texture>,
    pub horizon: Texture,
    pub water: Texture,
    pub rider: SpriteAsset,
    pub enhanced: Option<Texture>,
    pub families: BTreeMap<u16, FamilyAssets>,
    pub notes: BTreeSet<String>,
}
fn rgba_texture(
    t: &rr_formats::pixels::Texture,
    p: &Palette,
    transparent: bool,
) -> Result<Texture> {
    let mut mipmaps = vec![];
    for m in &t.mips {
        mipmaps.push(Arc::new(
            p.rgba(m.width, m.height, &m.indices, transparent, None)?
                .ok_or("empty RGBA source")?,
        ));
    }
    let image = mipmaps.first().ok_or("empty source mips")?.clone();
    Ok(Texture {
        image,
        mipmaps,
        repeat: true,
    })
}
fn resource<'a>(dir: &Directory, bytes: &'a [u8], tag: &str, id: u32) -> Result<&'a [u8]> {
    dir.entries
        .iter()
        .find(|e| e.tag == tag && e.id == id)
        .ok_or_else(|| format!("missing source {tag} {id}"))?
        .payload(bytes)
        .map_err(Into::into)
}
pub fn scenery_sprite(bytes: &[u8], base: usize, palette: &Palette) -> Result<Option<SpriteAsset>> {
    if u32le(bytes, base)? != 0x5252414e
        || u32le(bytes, base + 4)? != 20
        || u32le(bytes, base + 20)? != 0x52524644
    {
        return Err("invalid source RRAN header".into());
    }
    let pointer = |p: usize, n: usize| -> Result<usize> {
        let at = base
            .checked_add(u32le(bytes, p)? as usize)
            .ok_or("animation offset overflow")?;
        range(bytes, at, n)?;
        Ok(at)
    };
    let info = pointer(base + 8, 20)?;
    if u32le(bytes, info)? != 0x414e494d {
        return Err("invalid ANIM info".into());
    }
    let count = u32le(bytes, info + 16)? as usize;
    if count == 0 || count > 4096 || u32le(bytes, base + 24)? as usize != count * 28 + 8 {
        return Err("animation count bound".into());
    }
    range(bytes, base + 28, count * 28)?;
    // Static scenery requests the third view. Three-frame source aliases place
    // actual pixels there. A flagged first texture aliases its three view rows;
    // hotspots remain view-specific. Animation playback is a later phase.
    let index = 2.min(count - 1);
    let frame = base + 28 + index * 28;
    let mut pixel_frame = frame;
    if u32le(bytes, base + 32)? != 0 {
        let first_body = pointer(base + 32, 12)? + 8;
        if u32le(bytes, first_body)? == 0xbc614e && u32le(bytes, first_body + 12)? & 1 != 0 {
            pixel_frame = base + 28;
        }
    }
    if u32le(bytes, pixel_frame + 4)? == 0 {
        return Ok(None);
    }
    let body = pointer(pixel_frame + 4, 12)? + 8;
    if u32le(bytes, body)? != 0xbc614e {
        return Ok(None);
    }
    let texture = rr_formats::pixels::Texture::read(&bytes[body..])?;
    let mip = &texture.mips[0];
    let mut pivot = [f64::from(mip.width) / 2., f64::from(mip.height)];
    if u32le(bytes, frame + 8)? != 0 {
        let p = pointer(frame + 8, 8)?;
        pivot = [
            f64::from(u32le(bytes, p)? as i32) - f64::from(texture.bounds[0]),
            f64::from(u32le(bytes, p + 4)? as i32) - f64::from(texture.bounds[1]),
        ];
    }
    let logical = [f64::from(mip.width), f64::from(mip.height)];
    Ok(Some(SpriteAsset {
        texture: rgba_texture(&texture, palette, true)?,
        logical,
        pivot: [pivot[0] / logical[0], pivot[1] / logical[1]],
    }))
}
impl SourceAssets {
    pub fn load(
        data: &Path,
        catalog_root: &Path,
        stem: &str,
        course: &rr_course::Course,
    ) -> Result<Self> {
        let names = ["CITY", "HIWAY", "MEDLY", "NAPA", "CANYN"];
        let ci = names
            .iter()
            .position(|s| *s == stem)
            .ok_or("unknown course source name")?;
        let exe = fs::read(data.join("ROADRASH.EXE"))?;
        let exe = rr_formats::pe::SupportedExe::read(&exe)?;
        let depths = DepthTable::source(&exe)?;
        let palette = Palette::read(&fs::read(data.join("DATA/PALETTE.RAW"))?)?;
        let global = fs::read(data.join("DATA/GLOBAL.RSC"))?;
        let global_dir = Directory::read(&global)?;
        let cel = |id| -> Result<Texture> {
            let c = Cel::read(resource(&global_dir, &global, "CEL ", id)?)?;
            rgba_texture(
                c.texture
                    .as_ref()
                    .ok_or("road material lacks indexed pixels")?,
                &palette,
                false,
            )
        };
        let descriptor = exe.read_va(0x46dfb0, 300)?;
        let destinations = [
            0x4c3cc0, 0x4c3cb0, 0x4c3cd0, 0x4c3cd4, 0x4c3cd8, 0x4c3ce0, 0x4c3ce4, 0x4c3ce8,
        ];
        let terrain_dest = [0x4c3cc4, 0x4c3cb8, 0x4c3cbc];
        let mut road: Vec<Option<Texture>> = vec![None; 8];
        let mut ground = vec![None; 3];
        let mut ended = false;
        for row in descriptor.as_chunks::<20>().0.iter() {
            let dest = u32le(row, 0)?;
            if dest == 0 {
                ended = true;
                break;
            }
            if let Some(i) = destinations.iter().position(|v| *v == dest) {
                if road[i].is_some() || u32le(row, 4)? != 0x43454c20 {
                    return Err("invalid road binding table".into());
                }
                road[i] = Some(cel(u32le(row, 8)?
                    .checked_add(1)
                    .ok_or("material ID overflow")?)?)
            }
            if let Some(i) = terrain_dest.iter().position(|v| *v == dest) {
                ground[i] = Some(cel(u32le(row, 8)?
                    .checked_add(1)
                    .ok_or("ground ID overflow")?)?)
            }
        }
        road[3] = road[2].clone();
        if !ended || road.iter().any(Option::is_none) || ground.iter().any(Option::is_none) {
            return Err("incomplete road/ground material table".into());
        }
        let ptr = u32le(exe.read_va(0x467aa0 + ci as u32 * 4, 4)?, 0)?;
        let raw = exe.read_va(ptr, 16)?;
        let name = std::str::from_utf8(raw.split(|c| *c == 0).next().unwrap_or_default())?
            .to_ascii_uppercase();
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric()) {
            return Err("invalid horizon source stem".into());
        }
        let horizon_bytes = fs::read(data.join(format!("IMAGES/HORIZONS/{name}.BOB")))?;
        if horizon_bytes.len() != 720 * 320 {
            return Err("horizon source dimensions".into());
        }
        let horizon = Texture::new(
            palette
                .rgba(720, 320, &horizon_bytes, false, None)?
                .unwrap(),
            true,
        )?;
        let catalog = rr_content::Catalog::read(&catalog_root.join("catalog.json"))?;
        let entry = catalog
            .entries
            .iter()
            .find(|e| {
                e.kind == rr_content::Kind::Rider
                    && e.frame == Some(2)
                    && !e.preview_variants.is_empty()
            })
            .ok_or("catalog requires rider frame 2")?;
        let blob = &entry.preview_variants[0];
        if !rr_content::blob_valid(catalog_root, blob) {
            return Err("corrupt catalog rider pixels".into());
        }
        let texture = Texture::new(
            Rgba::from_png(&fs::read(catalog_root.join("objects").join(blob))?)?,
            false,
        )?;
        let logical = [
            f64::from(entry.logical_size[0]),
            f64::from(entry.logical_size[1]),
        ];
        let rider = SpriteAsset {
            texture,
            logical,
            pivot: [
                f64::from(entry.pivot_twice[0]) / 2. / logical[0],
                f64::from(entry.pivot_twice[1]) / 2. / logical[1],
            ],
        };
        let family_file = fs::read(data.join("DATA/FAMILIES.RSC"))?;
        let family_dir = Directory::read(&family_file)?;
        let needed: BTreeSet<_> = course
            .sections
            .iter()
            .flat_map(|s| s.family_events.iter().map(|f| f.family))
            .filter(|f| *f != 0)
            .collect();
        let mut families = BTreeMap::new();
        let mut notes = BTreeSet::new();
        let mut pixel_budget = 0usize;
        for id in needed {
            let payload = resource(&family_dir, &family_file, "FAM ", u32::from(id))?;
            let bytes = unpack(payload)?;
            let definition = Family::read_unpacked(&bytes)?;
            let mut groups = BTreeMap::new();
            for t in &definition.textures {
                let texture = rgba_texture(&t.texture, &palette, false)?;
                pixel_budget += texture
                    .mipmaps
                    .iter()
                    .map(|m| m.pixels.len())
                    .sum::<usize>();
                if pixel_budget > 256 * 1024 * 1024 {
                    return Err("renderer decoded pixel budget".into());
                }
                let node = definition
                    .nodes
                    .iter()
                    .find(|n| n.offset == t.group_offset)
                    .ok_or("missing CLGP metadata")?;
                let height = f64::from(*node.raw_header.get(22).ok_or("short CLGP")?) * 16.;
                groups.insert(t.group_offset, Material { texture, height });
            }
            let mut sprites = BTreeMap::new();
            for n in &definition.nodes {
                if n.tag == 0x5252414e {
                    match scenery_sprite(&bytes, n.offset as usize, &palette)? {
                        Some(sprite) => {
                            pixel_budget += sprite
                                .texture
                                .mipmaps
                                .iter()
                                .map(|m| m.pixels.len())
                                .sum::<usize>();
                            if pixel_budget > 256 * 1024 * 1024 {
                                return Err("renderer sprite pixel budget".into());
                            }
                            sprites.insert(n.offset, sprite);
                        }
                        None => {
                            notes.insert(format!(
                                "family {id} RRAN {} has opaque/empty static-view pixels",
                                n.offset
                            ));
                        }
                    }
                }
            }
            families.insert(
                id,
                FamilyAssets {
                    definition: Arc::new(definition),
                    groups,
                    sprites,
                },
            );
        }
        Ok(Self {
            depths,
            road: road.into_iter().map(Option::unwrap).collect(),
            ground: ground.into_iter().map(Option::unwrap).collect(),
            horizon,
            water: cel(0x10)?,
            rider,
            enhanced: None,
            families,
            notes,
        })
    }
    pub fn authored() -> Self {
        let image = |c| Texture::solid(c);
        Self {
            depths: DepthTable::authored(),
            road: (0..8)
                .map(|i| image([50 + i * 10, 50 + i * 10, 55 + i * 10, 255]))
                .collect(),
            ground: vec![image([40, 120, 35, 255]); 3],
            horizon: image([90, 150, 220, 255]),
            water: image([30, 90, 140, 255]),
            rider: SpriteAsset {
                texture: image([230, 45, 60, 255]),
                logical: [60., 100.],
                pivot: [0.5, 1.],
            },
            enhanced: None,
            families: BTreeMap::new(),
            notes: BTreeSet::new(),
        }
    }
    pub fn set_override(&mut self, path: &Path) -> Result<()> {
        let metadata = fs::metadata(path)?;
        if metadata.len() > 64 * 1024 * 1024 {
            return Err("override file bound".into());
        }
        self.enhanced = Some(Texture::new(Rgba::from_png(&fs::read(path)?)?, false)?);
        Ok(())
    }
}
