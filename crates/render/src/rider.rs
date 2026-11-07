//! Startup-loaded rider bank. Every source frame retains its own RRAN registration.
use crate::{
    Result,
    raster::Texture,
    source::{SpriteAsset, rider_frame_registration},
};
use rr_formats::{
    binary::range, decode_bob, pe::SupportedExe, pixels::Palette, resource::Directory,
};
use std::{fs, path::Path};
pub struct RiderAssets {
    pub bodies: Vec<Option<SpriteAsset>>,
    pub shadows: Vec<Option<SpriteAsset>>,
    pub color: u8,
}
impl RiderAssets {
    /// Police use the native cop sheets and their separate body/shadow RRAN banks.
    pub fn load_police(data: &Path) -> Result<Self> {
        let palette = Palette::read(&fs::read(data.join("DATA/PALETTE.RAW"))?)?;
        let global = fs::read(data.join("DATA/GLOBAL.RSC"))?;
        let directory = Directory::read(&global)?;
        let mut banks = vec![];
        for (name, id) in [("LOCOP.DAT", 2), ("HISHADOC.DAT", 4)] {
            let bytes = fs::read(data.join("DATA/BIKERS").join(name))?;
            let entry = directory
                .entries
                .iter()
                .find(|entry| entry.tag == "ANIM" && entry.id == id)
                .ok_or("missing police body/shadow RRAN")?;
            let metadata = entry.payload(&global)?;
            let mut sprites = vec![];
            for (index, frame) in decode_bob(&bytes, 45)?.iter().enumerate() {
                let Some(image) = palette.rgba(
                    u32::from(frame.width),
                    u32::from(frame.height),
                    frame.pixels,
                    true,
                    None,
                )?
                else {
                    sprites.push(None);
                    continue;
                };
                let (logical, pivot) = rider_frame_registration(metadata, index as u32)?;
                sprites.push(Some(SpriteAsset {
                    texture: Texture::new(image, false)?,
                    logical,
                    pivot,
                }));
            }
            banks.push(sprites);
        }
        Ok(Self {
            bodies: banks.remove(0),
            shadows: banks.remove(0),
            color: 0xde,
        })
    }
    pub fn load(data: &Path, color: u8) -> Result<Self> {
        if color >= 15 {
            return Err("rider palette ID must be 0..14".into());
        }
        let exe = fs::read(data.join("ROADRASH.EXE"))?;
        let remaps = SupportedExe::read(&exe)?.all_rider_remaps()?;
        let palette = Palette::read(&fs::read(data.join("DATA/PALETTE.RAW"))?)?;
        let global = fs::read(data.join("DATA/GLOBAL.RSC"))?;
        let directory = Directory::read(&global)?;
        let mut banks = vec![];
        for (name, id) in [("HIBOB.DAT", 1), ("HISHADOW.DAT", 3)] {
            let b = fs::read(data.join("DATA/BIKERS").join(name))?;
            let frames = decode_bob(&b, 326)?;
            let r = directory
                .entries
                .iter()
                .find(|r| r.tag == "ANIM" && r.id == id)
                .ok_or("missing rider/shadow RRAN")?;
            let metadata = range(&global, r.offset as usize, r.length as usize)?;
            let mut sprites = vec![];
            for (i, f) in frames.iter().enumerate() {
                if f.width == 0 || f.height == 0 {
                    sprites.push(None);
                    continue;
                }
                let Some(image) = palette.rgba(
                    u32::from(f.width),
                    u32::from(f.height),
                    f.pixels,
                    true,
                    if id == 1 {
                        Some(&remaps[color as usize])
                    } else {
                        None
                    },
                )?
                else {
                    sprites.push(None);
                    continue;
                };
                let (logical, pivot) = rider_frame_registration(metadata, i as u32)?;
                sprites.push(Some(SpriteAsset {
                    texture: Texture::new(image, false)?,
                    logical,
                    pivot,
                }));
            }
            banks.push(sprites);
        }
        Ok(Self {
            bodies: banks.remove(0),
            shadows: banks.remove(0),
            color,
        })
    }
    pub fn body(&self, frame: usize) -> Option<&SpriteAsset> {
        self.bodies.get(frame)?.as_ref()
    }
    pub fn shadow(&self, frame: usize) -> Option<&SpriteAsset> {
        self.shadows.get(frame)?.as_ref()
    }
}
