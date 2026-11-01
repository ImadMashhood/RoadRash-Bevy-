#![forbid(unsafe_code)]
//! Versioned semantic catalog and content-addressed, reproducible asset conversion.
pub mod build;
use rr_formats::pixels::Rgba;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const SCHEMA: u32 = 1;
pub const CONVERTER_VERSION: &str = "rr-content-3";
fn legacy_converter() -> String {
    "legacy-unrecorded".into()
}
pub const DECODER_VERSION: &str = rr_formats::DECODER_VERSION;
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AssetId(pub String);
impl AssetId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if !value.starts_with("rr:")
            || value.len() <= 3
            || value.len() > 512
            || value[3..].split('/').any(|s| s.is_empty() || s == ".")
            || value.contains("..")
            || !value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b":/._-".contains(&b))
        {
            return Err("invalid namespaced asset ID".into());
        }
        Ok(Self(value))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Rider,
    Police,
    Shadow,
    SceneryTexture,
    CourseTexture,
    Ui,
    Background,
    Audio,
    Video,
    BikeSpec,
    Course,
    Palette,
    Executable,
    Text,
    Resource,
    Other,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subcategory {
    Road,
    Terrain,
    Building,
    Prop,
    Actor,
    Shadow,
    CharacterArt,
    MenuArt,
    BikeArt,
    Hud,
    Horizon,
    Cloud,
    AudioEffect,
    Music,
    Video,
    SourceMetadata,
    #[default]
    Unclassified,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum ClassificationConfidence {
    SourceDerived,
    Inferred,
    #[default]
    Unreviewed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: AssetId,
    pub kind: Kind,
    #[serde(default)]
    pub subcategory: Subcategory,
    #[serde(default)]
    pub classification_confidence: ClassificationConfidence,
    pub source: String,
    pub source_sha256: String,
    pub resource: Option<(String, u32)>,
    pub frame: Option<u32>,
    pub locator: Option<u32>,
    pub native_size: [u32; 2],
    pub trim: Rect,
    pub logical_size: [u32; 2],
    pub pivot_twice: [i32; 2],
    pub pivot_policy: String,
    pub indexed_blob: Option<String>,
    pub preview_variants: Vec<String>,
    pub dependencies: Vec<String>,
    pub metadata: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub path: String,
    pub logical_id: String,
    pub sha256: String,
    pub size: u64,
    pub kind: Kind,
    pub status: String,
    pub cache_key: String,
    pub master_blob: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    #[serde(default = "legacy_converter")]
    pub converter_version: String,
    pub schema_version: u32,
    pub decoder: String,
    pub rgba_policy: String,
    pub sprite_alpha_zero: bool,
}
impl Default for Recipe {
    fn default() -> Self {
        Self {
            schema_version: 1,
            converter_version: CONVERTER_VERSION.into(),
            decoder: DECODER_VERSION.into(),
            rgba_policy: "standard-jpeg-native-rgbx".into(),
            sprite_alpha_zero: true,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub schema_version: u32,
    pub recipe: Recipe,
    pub sources: Vec<Source>,
    pub entries: Vec<Entry>,
    pub palettes: Vec<String>,
    pub metadata_blob: String,
}
impl Catalog {
    pub fn read(path: &Path) -> Result<Self> {
        use std::io::Read;
        let file = fs::File::open(path)?;
        let mut b = vec![];
        file.take(64 * 1024 * 1024 + 1).read_to_end(&mut b)?;
        if b.len() > 64 * 1024 * 1024 {
            return Err("catalog size limit".into());
        }
        let value: serde_json::Value = serde_json::from_slice(&b)?;
        if value["schema_version"].as_u64() != Some(u64::from(SCHEMA)) {
            return Err("unsupported catalog schema; rebuild into a new output directory".into());
        }
        let catalog: Self = serde_json::from_value(value)?;
        catalog.validate()?;
        Ok(catalog)
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA {
            return Err("unsupported catalog schema".into());
        }
        let mut ids = BTreeMap::new();
        for e in &self.entries {
            AssetId::new(e.id.0.clone())?;
            if ids.insert(&e.id, &e.kind).is_some() {
                return Err(format!("duplicate semantic asset: {}", e.id.0).into());
            }
            for blob in e.preview_variants.iter().chain(e.indexed_blob.iter()) {
                validate_blob(blob)?;
            }
            if e.native_size
                .iter()
                .chain(e.logical_size.iter())
                .any(|n| *n > 4096)
                || u64::from(e.trim.x) + u64::from(e.trim.width) > u64::from(e.native_size[0])
                || u64::from(e.trim.y) + u64::from(e.trim.height) > u64::from(e.native_size[1])
            {
                return Err(format!("invalid trim: {}", e.id.0).into());
            }
        }
        Ok(())
    }
    pub fn resolve(&self, packs: &[Pack]) -> Result<BTreeMap<AssetId, Entry>> {
        self.validate()?;
        let mut resolved: BTreeMap<_, _> = self
            .entries
            .iter()
            .map(|e| (e.id.clone(), e.clone()))
            .collect();
        for pack in packs {
            for (id, blob) in &pack.overrides {
                validate_blob(blob)?;
                let entry = resolved
                    .get_mut(id)
                    .ok_or_else(|| format!("pack {} replaces missing ID {}", pack.name, id.0))?;
                entry.preview_variants = vec![blob.clone()];
            }
            for reference in &pack.references {
                if !resolved.contains_key(&reference.from) {
                    return Err(format!(
                        "pack {}: referring asset {} missing",
                        pack.name, reference.from.0
                    )
                    .into());
                }
                let target = resolved.get(&reference.to).ok_or_else(|| {
                    format!(
                        "pack {}: {} refers to missing {}",
                        pack.name, reference.from.0, reference.to.0
                    )
                })?;
                if target.kind != reference.expected_kind {
                    return Err(format!(
                        "pack {}: {} expects {:?} at {}, found {:?}",
                        pack.name,
                        reference.from.0,
                        reference.expected_kind,
                        reference.to.0,
                        target.kind
                    )
                    .into());
                }
            }
        }
        Ok(resolved)
    }
}
#[derive(Default)]
pub struct Pack {
    pub name: String,
    pub overrides: BTreeMap<AssetId, String>,
    pub references: Vec<Reference>,
}
pub struct Reference {
    pub from: AssetId,
    pub to: AssetId,
    pub expected_kind: Kind,
}
pub fn validate_blob(blob: &str) -> Result<()> {
    let Some((digest, extension)) = blob.split_once('.') else {
        return Err("invalid blob name".into());
    };
    if digest.len() != 64
        || !digest.bytes().all(|b| b.is_ascii_hexdigit())
        || !["png", "bin", "json"].contains(&extension)
    {
        return Err("invalid blob name".into());
    }
    Ok(())
}
pub fn put_blob(root: &Path, bytes: &[u8], extension: &str) -> Result<String> {
    let name = format!("{}.{}", hash(bytes), extension);
    validate_blob(&name)?;
    let path = root.join("objects").join(&name);
    if !path.exists() || fs::read(&path)? != bytes {
        fs::write(path, bytes)?;
    }
    Ok(name)
}
pub fn blob_valid(root: &Path, name: &str) -> bool {
    validate_blob(name).is_ok()
        && fs::read(root.join("objects").join(name)).is_ok_and(|b| hash(&b) == name[..64])
}
pub fn trim(image: &Rgba) -> Rect {
    let mut left = image.width;
    let mut top = image.height;
    let mut right = 0;
    let mut bottom = 0;
    for y in 0..image.height {
        for x in 0..image.width {
            if image.pixels[((y * image.width + x) * 4 + 3) as usize] != 0 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
            }
        }
    }
    if right == 0 {
        Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    } else {
        Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }
}
/// Offset in half-pixel units when a trimmed image is drawn at a logical contact anchor.
pub fn trim_placement(entry: &Entry) -> [i32; 2] {
    [
        entry.trim.x as i32 * 2 - entry.pivot_twice[0],
        entry.trim.y as i32 * 2 - entry.pivot_twice[1],
    ]
}
