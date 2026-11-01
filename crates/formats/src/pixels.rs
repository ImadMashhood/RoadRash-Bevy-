use crate::{DecodeError, Result, binary::*};
use image::{ImageDecoder, ImageEncoder};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}
impl Rgba {
    pub fn validate(&self) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || self.width > 4096
            || self.height > 4096
            || self.pixels.len() != self.width as usize * self.height as usize * 4
        {
            return Err(DecodeError::Invalid("RGBA dimensions/storage"));
        }
        Ok(())
    }
    pub fn png(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut out = vec![];
        image::codecs::png::PngEncoder::new(&mut out)
            .write_image(
                &self.pixels,
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|_| DecodeError::Invalid("PNG encode"))?;
        Ok(out)
    }
    pub fn from_png(b: &[u8]) -> Result<Self> {
        let reader = image::codecs::png::PngDecoder::new(Cursor::new(b))
            .map_err(|_| DecodeError::Invalid("PNG header"))?;
        let (w, h) = reader.dimensions();
        if w > 4096 || h > 4096 {
            return Err(DecodeError::Invalid("PNG bounds"));
        }
        let image = image::DynamicImage::from_decoder(reader)
            .map_err(|_| DecodeError::Invalid("PNG pixels"))?
            .to_rgba8();
        Ok(Self {
            width: w,
            height: h,
            pixels: image.into_raw(),
        })
    }
}
#[derive(Clone)]
pub struct Palette(pub [[u8; 4]; 256]);
impl Palette {
    pub fn read(b: &[u8]) -> Result<Self> {
        if b.len() != 1024 {
            return Err(DecodeError::Invalid("RGBX palette length"));
        }
        Ok(Self(std::array::from_fn(|i| {
            b[i * 4..i * 4 + 4].try_into().unwrap()
        })))
    }
    pub fn rgba(
        &self,
        width: u32,
        height: u32,
        indices: &[u8],
        transparent: bool,
        remap: Option<&[u8; 256]>,
    ) -> Result<Option<Rgba>> {
        if width == 0 || height == 0 {
            if indices.is_empty() {
                return Ok(None);
            }
            return Err(DecodeError::Invalid("empty frame storage"));
        }
        if width > 4096 || height > 4096 || width as usize * height as usize != indices.len() {
            return Err(DecodeError::Invalid("indexed image storage"));
        }
        let pixels = indices
            .iter()
            .flat_map(|&source| {
                let index = remap.map_or(source, |r| r[source as usize]);
                let c = self.0[index as usize];
                [
                    c[0],
                    c[1],
                    c[2],
                    if transparent && source == 0 { 0 } else { 255 },
                ]
            })
            .collect();
        Ok(Some(Rgba {
            width,
            height,
            pixels,
        }))
    }
}
/// RRI files contain JPEG. Decode modern RGB without emulating the original display's quantization.
pub fn rri(b: &[u8]) -> Result<Rgba> {
    let reader = image::codecs::jpeg::JpegDecoder::new(Cursor::new(b))
        .map_err(|_| DecodeError::Invalid("RRI JPEG header"))?;
    let (width, height) = reader.dimensions();
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(DecodeError::Invalid("RRI dimensions"));
    }
    let image = image::DynamicImage::from_decoder(reader)
        .map_err(|_| DecodeError::Invalid("RRI JPEG pixels"))?
        .to_rgba8();
    Ok(Rgba {
        width,
        height,
        pixels: image.into_raw(),
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mip {
    pub width: u32,
    pub height: u32,
    pub source_offset: u32,
    pub indices: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Texture {
    pub bounds: [i16; 4],
    pub flags: u32,
    pub name: String,
    pub flat_index: u8,
    pub header: Vec<u8>,
    pub mips: Vec<Mip>,
}
impl Texture {
    pub fn read(b: &[u8]) -> Result<Self> {
        range(b, 0, 56)?;
        if u32le(b, 0)? != 0xbc614e {
            return Err(DecodeError::Invalid("indexed texture magic"));
        }
        let w = u32le(b, 32)?;
        let h = u32le(b, 36)?;
        let levels = u32le(b, 40)?;
        if w == 0
            || w > 256
            || !w.is_power_of_two()
            || h == 0
            || h > 512
            || levels == 0
            || levels > 9
            || levels > w.trailing_zeros() + 1
        {
            return Err(DecodeError::Invalid("texture dimensions/mips"));
        }
        let header_size = 56 + 12 * (levels as usize - 1) + 4;
        range(b, 0, header_size)?;
        let mut mips = vec![];
        for level in 0..levels {
            let at = u32le(b, 56 + level as usize * 12)?
                .checked_add(28)
                .ok_or(DecodeError::Invalid("mip offset overflow"))?;
            let width = w >> level;
            let height = h.div_ceil(1 << level);
            if (at as usize) < header_size {
                return Err(DecodeError::Invalid("mip overlaps header"));
            }
            let indices = range(b, at as usize, (width * height) as usize)?.to_vec();
            mips.push(Mip {
                width,
                height,
                source_offset: at,
                indices,
            });
        }
        Ok(Self {
            bounds: [
                u16le(b, 4)? as i16,
                u16le(b, 6)? as i16,
                u16le(b, 8)? as i16,
                u16le(b, 10)? as i16,
            ],
            flags: u32le(b, 12)?,
            name: String::from_utf8_lossy(
                range(b, 16, 16)?
                    .split(|b| *b == 0)
                    .next()
                    .unwrap_or_default(),
            )
            .into_owned(),
            flat_index: u32le(b, 44)? as u8,
            header: b[..header_size].to_vec(),
            mips,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk {
    pub tag: u32,
    pub offset: u32,
    pub length: u32,
    pub header: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cel {
    pub width: u32,
    pub height: u32,
    pub flags: u32,
    pub plut: Vec<u16>,
    pub texture: Option<Texture>,
    pub legacy_pixel: Option<u32>,
    pub chunks: Vec<Chunk>,
}
fn dimension(n: u32) -> Result<u32> {
    if n & 0x80000000 != 0 {
        if n & 31 > 9 {
            return Err(DecodeError::Invalid("CCB exponent"));
        }
        Ok(1 << (n & 31))
    } else if n & 511 != 0 {
        Ok(n & 511)
    } else {
        Err(DecodeError::Invalid("zero CCB dimension"))
    }
}
impl Cel {
    pub fn read(b: &[u8]) -> Result<Self> {
        let mut out = Self {
            width: 0,
            height: 0,
            flags: 0,
            plut: vec![],
            texture: None,
            legacy_pixel: None,
            chunks: vec![],
        };
        let mut at = 0;
        let mut ccb = false;
        let mut pixels = false;
        let mut plut = false;
        while at < b.len() {
            let tag = u32le(b, at)?;
            let length = u32le(b, at + 4)? as usize;
            if length < 8 {
                return Err(DecodeError::Invalid("CEL chunk length"));
            }
            let chunk = range(b, at, length)?;
            out.chunks.push(Chunk {
                tag,
                offset: at as u32,
                length: length as u32,
                header: chunk[..chunk.len().min(80)].to_vec(),
            });
            match tag {
                0x43434220 => {
                    if ccb || length < 80 {
                        return Err(DecodeError::Invalid("duplicate/short CCB"));
                    }
                    ccb = true;
                    out.flags = u32le(chunk, 12)?;
                    out.width = dimension(u32le(chunk, 72)?)?;
                    out.height = dimension(u32le(chunk, 76)?)?;
                }
                0x504c5554 => {
                    if plut {
                        return Err(DecodeError::Invalid("duplicate PLUT"));
                    }
                    plut = true;
                    let count = u32le(chunk, 8)? as usize;
                    if count == 0 || count > 256 {
                        return Err(DecodeError::Invalid("PLUT count"));
                    }
                    for i in 0..count {
                        out.plut.push(u16le(chunk, 12 + i * 2)?);
                    }
                }
                0x50444154 | 0x52504454 | 0x52525044 => {
                    if pixels || !ccb || length < 12 {
                        return Err(DecodeError::Invalid("CEL pixels/CCB ordering"));
                    }
                    pixels = true;
                    let body = &chunk[8..];
                    if u32le(body, 0)? == 0xbc614e {
                        out.texture = Some(Texture::read(body)?);
                    } else if body.len() == 4 {
                        out.legacy_pixel = Some(u32le(body, 0)?);
                    } else {
                        return Err(DecodeError::Invalid("unsupported CEL pixels"));
                    }
                }
                _ => {}
            }
            at += length;
        }
        if !ccb || !pixels {
            return Err(DecodeError::Invalid("missing CCB/pixels"));
        }
        Ok(out)
    }
}
