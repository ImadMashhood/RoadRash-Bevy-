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
/// Source RRAN geometry is independent of the HI/LO BOB pixel resolution.
pub fn rider_registration(global: &[u8], frame: u32) -> Result<([f64; 2], [f64; 2])> {
    let directory = Directory::read(global)?;
    rider_frame_registration(resource(&directory, global, "ANIM", 1)?, frame)
}
pub fn rider_frame_registration(bytes: &[u8], frame: u32) -> Result<([f64; 2], [f64; 2])> {
    if u32le(bytes, 0)? != 0x5252414e || u32le(bytes, 20)? != 0x52524644 {
        return Err("rider RRAN/RRFD header".into());
    }
    let extent = u32le(bytes, 24)? as usize;
    if extent < 8 || !(extent - 8).is_multiple_of(28) {
        return Err("rider frame extent".into());
    }
    range(bytes, 20, extent)?;
    if frame as usize >= (extent - 8) / 28 {
        return Err("rider registration frame out of range".into());
    }
    let at = 28 + frame as usize * 28;
    let logical = [
        u32le(bytes, at + 20)? & 65535,
        u32le(bytes, at + 24)? & 65535,
    ];
    if logical.contains(&0) || logical.iter().any(|n| *n > 1024) {
        return Err("rider logical dimensions".into());
    }
    let hotspot = u32le(bytes, at + 8)? as usize;
    let pivot = [
        u32le(bytes, hotspot)? as i32,
        u32le(bytes, hotspot + 4)? as i32,
    ];
    if pivot.iter().any(|n| n.unsigned_abs() > 4096) {
        return Err("rider hotspot bounds".into());
    }
    let logical = logical.map(f64::from);
    Ok((
        logical,
        [
            f64::from(pivot[0]) / logical[0],
            f64::from(pivot[1]) / logical[1],
        ],
    ))
}
#[derive(Clone)]
pub struct Material {
    pub texture: Texture,
    pub end_texture: Texture,
    pub height: f64,
}
#[derive(Clone)]
pub struct FamilyAssets {
    pub definition: Arc<Family>,
    pub groups: BTreeMap<u32, Material>,
    pub sprites: BTreeMap<u32, SpriteAsset>,
    pub pedestrian_frames: BTreeMap<u8, Vec<Option<SpriteAsset>>>,
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
pub type PedestrianVoices = BTreeMap<(u16, u8), [Option<u8>; 2]>;
#[derive(Clone)]
pub struct SourceAssets {
    pub police_animation: Option<Arc<rr_simulation::animation::Library>>,
    pub pedestrian_appearances: Vec<rr_simulation::population::PedestrianAppearance>,
    pub pedestrian_voices: Arc<PedestrianVoices>,
    pub animations: Option<Arc<crate::rider::RiderAssets>>,
    pub palettes: BTreeMap<u8, Arc<crate::rider::RiderAssets>>,
    pub vehicles: Vec<Vec<Option<SpriteAsset>>>,
    pub vehicle_masks: Vec<u32>,
    pub police: Vec<Option<SpriteAsset>>,
    pub depths: DepthTable,
    pub road: Vec<Texture>,
    pub ground: Vec<Texture>,
    pub horizon: Texture,
    pub water: Texture,
    pub roof: Texture,
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
    animation_sprite(bytes, base, palette, 2)
}
fn animation_sprite(
    bytes: &[u8],
    base: usize,
    palette: &Palette,
    index: usize,
) -> Result<Option<SpriteAsset>> {
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
    let index = index.min(count - 1);
    let frame = base + 28 + index * 28;
    let mut pixel_frame = frame;
    if u32le(bytes, base + 32)? != 0 {
        let first_body = pointer(base + 32, 12)? + 8;
        if u32le(bytes, first_body)? == 0xbc614e && u32le(bytes, first_body + 12)? & 1 != 0 {
            pixel_frame = base + 28;
        }
    }
    // Three-view resources may store pixels only in the third view. Keep each
    // requested hotspot while sharing that texture, matching native RRAN aliases.
    if count == 3 && u32le(bytes, base + 32)? == 0 {
        pixel_frame = base + 28 + 2 * 28;
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
        let exe_bytes = fs::read(data.join("ROADRASH.EXE"))?;
        let exe = rr_formats::pe::SupportedExe::read(&exe_bytes)?;
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
        let (logical, pivot) =
            rider_registration(&global, entry.frame.ok_or("rider frame index")?)?;
        let rider = SpriteAsset {
            texture,
            logical,
            pivot,
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
        let mut pedestrian_appearances = vec![];
        let mut pedestrian_voices = BTreeMap::new();
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
                groups
                    .entry(t.group_offset)
                    .and_modify(|m: &mut Material| m.texture = texture.clone())
                    .or_insert_with(|| Material {
                        end_texture: texture.clone(),
                        texture,
                        height,
                    });
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
            let mut family = FamilyAssets {
                pedestrian_frames: BTreeMap::new(),
                definition: Arc::new(definition),
                groups,
                sprites,
            };
            for selector in 0..64 {
                let Some(graphics) = family.resolve(2, selector, Some(0)) else {
                    continue;
                };
                let Some(cans) = family.resolve(2, selector, Some(1)) else {
                    continue;
                };
                if u32le(&bytes, graphics as usize)? != 0x5252414e {
                    continue;
                }
                let mut end = cans as usize;
                let mut valid = true;
                for _ in 0..5 {
                    let extent = u32le(&bytes, end + 4)? as usize;
                    if extent < 8 || end.checked_add(extent).is_none_or(|e| e > bytes.len()) {
                        valid = false;
                        break;
                    }
                    end += extent;
                }
                if !valid {
                    notes.insert(format!(
                        "family {id} selector {selector}: unavailable CANS extent"
                    ));
                    continue;
                }
                let animation = match rr_simulation::animation::Library::pedestrian(
                    &exe_bytes,
                    &bytes[cans as usize..end],
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        notes.insert(format!("family {id} selector {selector}: unavailable pedestrian animation: {error}"));
                        continue;
                    }
                };
                let count =
                    (u32le(&bytes, graphics as usize + 24)? as usize).saturating_sub(8) / 28;
                if count > 4096 {
                    return Err("Pedestrian frame budget".into());
                }
                let mut frames = vec![];
                for frame in 0..count {
                    let sprite = animation_sprite(&bytes, graphics as usize, &palette, frame)?;
                    if let Some(sprite) = &sprite {
                        pixel_budget += sprite.texture.image.pixels.len();
                    }
                    if pixel_budget > 256 * 1024 * 1024 {
                        return Err("renderer pedestrian pixel budget".into());
                    }
                    frames.push(sprite);
                }
                if animation.views[0]
                    .iter()
                    .flatten()
                    .any(|cell| frames.get(cell.frame).is_none_or(Option::is_none))
                {
                    notes.insert(format!(
                        "family {id} selector {selector}: missing required pedestrian pixels"
                    ));
                    continue;
                }
                family.pedestrian_frames.insert(selector as u8, frames);
                let mut voices = [None; 2];
                for (k, voice) in voices.iter_mut().enumerate() {
                    if let Some(offset) = family.resolve(2, selector, Some(2 + k)) {
                        let raw = rr_formats::binary::range(&bytes, offset as usize + 8, 4)?;
                        let slot = i32::from_be_bytes(raw.try_into().expect("four voice bytes"))
                            .checked_sub(0x44)
                            .unwrap_or(-1);
                        if (1..69).contains(&slot) {
                            *voice = Some(slot as u8);
                        }
                    }
                }
                pedestrian_voices.insert((id, selector as u8), voices);
                pedestrian_appearances.push(rr_simulation::population::PedestrianAppearance {
                    family: id,
                    selector: selector as u8,
                    animation,
                });
            }
            families.insert(id, family);
        }
        let car_bytes = fs::read(data.join(format!("DATA/CARS/{stem}.CAR")))?;
        let car_directory = Directory::read(&car_bytes)?;
        let vehicle_masks = rr_simulation::population::vehicle_masks(&car_bytes)?;
        let mut vehicles = vec![];
        let mut animations: Vec<_> = car_directory
            .entries
            .iter()
            .filter(|e| e.tag == "ANIM")
            .collect();
        animations.sort_by_key(|e| e.id);
        for entry in animations.into_iter().take(vehicle_masks.len()) {
            let bytes = range(&car_bytes, entry.offset as usize, entry.length as usize)?;
            let mut views = vec![];
            for index in 0..3 {
                views.push(animation_sprite(bytes, 0, &palette, index)?);
            }
            vehicles.push(views);
        }
        let cop_bytes = fs::read(data.join("DATA/BIKERS/LOCOP.DAT"))?;
        let cop_frames = rr_formats::decode_bob(&cop_bytes, 45)?;
        let cop_metadata = resource(&Directory::read(&global)?, &global, "ANIM", 2)?;
        let mut police = vec![];
        for (index, frame) in cop_frames.iter().enumerate() {
            if frame.width == 0 || frame.height == 0 {
                police.push(None);
                continue;
            }
            let image = palette
                .rgba(
                    u32::from(frame.width),
                    u32::from(frame.height),
                    frame.pixels,
                    true,
                    None,
                )?
                .ok_or("cop pixels")?;
            let (logical, pivot) = rider_frame_registration(cop_metadata, index as u32)?;
            police.push(Some(SpriteAsset {
                texture: Texture::new(image, false)?,
                logical,
                pivot,
            }));
        }
        Ok(Self {
            police_animation: Some(Arc::new(rr_simulation::animation::Library::read(
                &exe_bytes, &global,
            )?)),
            pedestrian_appearances,
            pedestrian_voices: Arc::new(pedestrian_voices),
            palettes: BTreeMap::new(),
            vehicles,
            vehicle_masks,
            police,
            animations: Some(Arc::new(crate::rider::RiderAssets::load(data, 0)?)),
            depths,
            road: road.into_iter().map(Option::unwrap).collect(),
            ground: ground.into_iter().map(Option::unwrap).collect(),
            horizon,
            water: cel(0x10)?,
            roof: Texture::solid({
                let mut color = palette.0[253];
                color[3] = 255;
                color
            }),
            rider,
            enhanced: None,
            families,
            notes,
        })
    }
    pub fn authored() -> Self {
        let image = |c| Texture::solid(c);
        Self {
            police_animation: None,
            pedestrian_appearances: vec![],
            pedestrian_voices: Default::default(),
            palettes: BTreeMap::new(),
            vehicles: vec![],
            vehicle_masks: vec![],
            police: vec![],
            animations: None,
            depths: DepthTable::authored(),
            road: (0..8)
                .map(|i| image([50 + i * 10, 50 + i * 10, 55 + i * 10, 255]))
                .collect(),
            ground: vec![image([40, 120, 35, 255]); 3],
            horizon: image([90, 150, 220, 255]),
            water: image([30, 90, 140, 255]),
            roof: image([95, 95, 90, 255]),
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

impl SourceAssets {
    /// Actual decoded RGBA allocation bytes, deduplicated by shared image identity.
    /// Excludes structs, maps, renderer buffers and driver allocations.
    pub fn pixel_residency(&self) -> BTreeMap<&'static str, usize> {
        let mut seen = std::collections::BTreeSet::new();
        let mut bytes = BTreeMap::new();
        let mut add = |role, texture: &Texture| {
            for image in std::iter::once(&texture.image).chain(&texture.mipmaps) {
                if seen.insert(Arc::as_ptr(image) as usize) {
                    *bytes.entry(role).or_insert(0) += image.pixels.len();
                }
            }
        };
        for bank in self.palettes.values().chain(self.animations.iter()) {
            for sprite in bank.bodies.iter().chain(&bank.shadows).flatten() {
                add("riders_and_shadows", &sprite.texture);
            }
        }
        for t in &self.road {
            add("road", t);
        }
        for t in &self.ground {
            add("ground", t);
        }
        add("horizon", &self.horizon);
        add("water", &self.water);
        add("roof", &self.roof);
        add("fallback_rider", &self.rider.texture);
        for sprite in self
            .vehicles
            .iter()
            .flatten()
            .chain(self.police.iter())
            .flatten()
        {
            add("traffic_and_police", &sprite.texture);
        }
        for family in self.families.values() {
            for material in family.groups.values() {
                add("building_and_terrain", &material.texture);
                add("building_and_terrain", &material.end_texture);
            }
            for sprite in family.sprites.values() {
                add("scenery", &sprite.texture);
            }
            for sprite in family.pedestrian_frames.values().flatten().flatten() {
                add("pedestrians", &sprite.texture);
            }
        }
        if let Some(enhanced) = &self.enhanced {
            add("enhanced", enhanced);
        }
        bytes
    }
}

#[cfg(test)]
mod vehicle_alias_tests {
    #[test]
    fn third_view_texture_alias_keeps_each_view_hotspot() {
        let mut bytes = vec![0u8; 300];
        let word =
            |b: &mut [u8], at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        for (at, value) in [
            (0, 0x5252414e),
            (4, 20),
            (8, 112),
            (20, 0x52524644),
            (24, 92),
            (112, 0x414e494d),
            (116, 20),
            (128, 3),
            (36, 284),
            (88, 212),
            (92, 292),
            (220, 0xbc614e),
            (252, 2),
            (256, 2),
            (260, 1),
            (276, 32),
            (284, 0),
            (288, 1),
            (292, 1),
            (296, 2),
        ] {
            word(&mut bytes, at, value);
        }
        bytes[228..230].copy_from_slice(&2i16.to_le_bytes());
        bytes[230..232].copy_from_slice(&2i16.to_le_bytes());
        bytes[280..284].copy_from_slice(&[0, 1, 1, 0]);
        let mut palette = rr_formats::pixels::Palette([[0; 4]; 256]);
        palette.0[1] = [255, 10, 20, 0];
        let first = super::animation_sprite(&bytes, 0, &palette, 0)
            .unwrap()
            .unwrap();
        let third = super::animation_sprite(&bytes, 0, &palette, 2)
            .unwrap()
            .unwrap();
        assert_eq!(first.texture.image.pixels, third.texture.image.pixels);
        assert_eq!(first.pivot, [0., 0.5]);
        assert_eq!(third.pivot, [0.5, 1.]);
        assert!(
            first
                .texture
                .image
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] != 0)
        );
    }
}
