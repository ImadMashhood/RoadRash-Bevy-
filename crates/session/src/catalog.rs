use crate::Result;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Character {
    pub id: u8,
    pub name: String,
    pub personality: u8,
    pub starting_cash: u32,
    pub bikes: [u8; 5],
    pub portrait: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bike {
    pub id: u8,
    pub name: String,
    pub price: u32,
    pub source_image: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub characters: Vec<Character>,
    pub bikes: Vec<Bike>,
    pub prizes: [u32; 15],
    pub source_verified: bool,
}
pub const COURSES: [(&str, &str); 5] = [
    ("CITY", "The City"),
    ("HIWAY", "Sierra Nevada"),
    ("MEDLY", "The Peninsula"),
    ("NAPA", "Napa Valley"),
    ("CANYN", "Pacific Highway"),
];
impl Catalog {
    /// Synthetic data for redistributable tests and the no-import demonstration.
    pub fn authored() -> Self {
        Self {
            characters: (1..=8)
                .map(|id| Character {
                    id,
                    name: format!("Rider {id}"),
                    personality: id,
                    starting_cash: 1000,
                    bikes: [0; 5],
                    portrait: String::new(),
                })
                .collect(),
            bikes: (0..15)
                .map(|id| Bike {
                    id,
                    name: format!("Bike {}", id + 1),
                    price: 1000 + u32::from(id) * 500,
                    source_image: String::new(),
                })
                .collect(),
            prizes: std::array::from_fn(|i| (15 - i) as u32 * 100),
            source_verified: false,
        }
    }
    pub fn character(&self, id: u8) -> Result<&Character> {
        self.characters
            .iter()
            .find(|c| c.id == id)
            .ok_or("invalid character ID".into())
    }
    pub fn bike(&self, id: u8) -> Result<&Bike> {
        self.bikes
            .iter()
            .find(|b| b.id == id)
            .ok_or("invalid bike ID".into())
    }
    /// Offsets are accepted only after the existing supported-executable SHA-256 gate.
    pub fn from_exe(bytes: &[u8]) -> Result<Self> {
        let pe = rr_formats::pe::SupportedExe::read(bytes).map_err(|e| e.to_string())?;
        let word = |at| -> Result<u32> {
            rr_formats::binary::u32le(pe.read_va(at, 4).map_err(|e| e.to_string())?, 0)
                .map_err(|e| e.to_string())
        };
        // Names/paths are the already-reviewed public metadata, never extracted pixels/dialogue.
        let metadata: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/baseline/executable/selectable-actors.json"
        ))
        .map_err(|e| e.to_string())?;
        let mut out = Self {
            characters: vec![],
            bikes: vec![],
            prizes: [0; 15],
            source_verified: true,
        };
        for c in metadata["characters"]
            .as_array()
            .ok_or("character metadata")?
        {
            let id = c["character_id"].as_u64().ok_or("character ID")? as u8;
            let personality = c["personality_id"].as_u64().ok_or("personality ID")? as u8;
            let raw = pe
                .read_va(0x469000 + u32::from(id) * 2, 2)
                .map_err(|e| e.to_string())?;
            let mut bikes = [0; 5];
            for (level, b) in bikes.iter_mut().enumerate() {
                *b = pe
                    .read_va(0x4641c8 + u32::from(personality) * 5 + level as u32, 1)
                    .map_err(|e| e.to_string())?[0];
                if *b >= 15 {
                    return Err("default bike range".into());
                }
            }
            out.characters.push(Character {
                id,
                name: c["display_name"].as_str().ok_or("character name")?.into(),
                personality,
                starting_cash: u32::from(u16::from_le_bytes([raw[0], raw[1]])),
                bikes,
                portrait: c["source_images"][0]
                    .as_str()
                    .ok_or("portrait path")?
                    .into(),
            });
        }
        for b in metadata["bikes"].as_array().ok_or("bike metadata")? {
            let id = b["bike_id"].as_u64().ok_or("bike ID")? as u8;
            out.bikes.push(Bike {
                id,
                name: b["showroom_label"].as_str().ok_or("bike name")?.into(),
                price: word(0x468fc0 + u32::from(id) * 4)?,
                source_image: b["source_image"].as_str().ok_or("bike image")?.into(),
            });
        }
        for (i, p) in out.prizes.iter_mut().enumerate() {
            *p = word(0x46ab48 + i as u32 * 0x4c)?;
        }
        if out.characters.len() != 8 || out.bikes.len() != 15 {
            return Err("catalog cardinality".into());
        }
        Ok(out)
    }
}
