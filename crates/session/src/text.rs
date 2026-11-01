//! Local language data and source dialogue selection; no Bevy or dynamic DLL loading.
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use rr_formats::{
    binary::{u16le, u32le},
    pe::SupportedExe,
    strings,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    English,
    French,
    German,
    Spanish,
    Italian,
}
impl Language {
    pub const ALL: [Self; 5] = [
        Self::English,
        Self::French,
        Self::German,
        Self::Spanish,
        Self::Italian,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Spanish => "Español",
            Self::Italian => "Italiano",
        }
    }
    pub fn dll(self) -> &'static str {
        match self {
            Self::English => "ENU",
            Self::French => "FRA",
            Self::German => "DEU",
            Self::Spanish => "ESP",
            Self::Italian => "ITA",
        }
    }
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|l| *l == self).unwrap();
        Self::ALL[(i + 1) % 5]
    }
}
pub struct SourceText {
    pub language: Language,
    pub strings: Vec<String>,
    pub credits: [Vec<String>; 2],
    pub resource_count: usize,
    pub source_sha256: String,
    pub dialogue: Dialogue,
}
impl SourceText {
    pub fn load(data: &Path, language: Language) -> Result<Self> {
        let mut bytes = vec![];
        std::fs::File::open(data.join(format!("TEXT/{}.DLL", language.dll())))?
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        let resources = strings::resources(&bytes)?;
        let table = strings::game_table(&resources, 0)?;
        let credits = [
            strings::game_table(&resources, 1000)?,
            strings::game_table(&resources, 2000)?,
        ];
        Ok(Self {
            language,
            strings: table,
            credits,
            resource_count: resources.len(),
            source_sha256: format!("{:x}", Sha256::digest(&bytes)),
            dialogue: Dialogue::read(&std::fs::read(data.join("ROADRASH.EXE"))?)?,
        })
    }
    pub fn at(&self, id: usize) -> Option<&str> {
        self.strings
            .get(id)
            .map(String::as_str)
            .filter(|s| !s.is_empty())
    }
    pub fn spoken(
        &self,
        level: u8,
        partner: u8,
        character: u8,
        relations: Option<&[[i32; 3]; 10]>,
        draw: u32,
    ) -> Result<(u8, String)> {
        if partner >= 8 {
            return Err("Invalid dialogue partner".into());
        }
        let default = self.dialogue.relations(character)?;
        let relations = relations.unwrap_or(&default);
        let (attitude, id) = self.dialogue.select(
            level,
            partner,
            character,
            relations[usize::from(partner)],
            draw,
        )?;
        let text = self
            .at(id as usize)
            .ok_or("Dialogue text is absent in selected language")?;
        Ok((attitude, readable(text)))
    }
}
/// Source dialogue table and initial relationship triples. Runtime strings stay private.
pub struct Dialogue {
    pub rows: [[u16; 14]; 40],
    pub attitude: [u8; 4],
    initial: [[[i32; 3]; 10]; 10],
    reactions: [[[i32; 3]; 5]; 10],
}
impl Dialogue {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let pe = SupportedExe::read(bytes)?;
        let raw = pe.read_va(0x46a6e0, 40 * 28)?;
        let mut rows = [[0; 14]; 40];
        for (r, row) in rows.iter_mut().enumerate() {
            for (i, v) in row.iter_mut().enumerate() {
                *v = u16le(raw, r * 28 + i * 2)?;
            }
        }
        let attitude = pe.read_va(0x46ab44, 4)?.try_into().unwrap();
        let mut initial = [[[0; 3]; 10]; 10];
        for (p, row) in initial.iter_mut().enumerate() {
            let raw = pe.read_va(0x464558 + p as u32 * 0x204, 120)?;
            for (r, values) in row.iter_mut().enumerate() {
                for (k, v) in values.iter_mut().enumerate() {
                    *v = u32le(raw, r * 12 + k * 4)? as i32;
                }
            }
        }
        let mut reactions = [[[0; 3]; 5]; 10];
        for (partner, events) in reactions.iter_mut().enumerate() {
            let raw = pe.read_va(0x464558 + partner as u32 * 0x204 + 0xf0, 60)?;
            for (event, values) in events.iter_mut().enumerate() {
                for (axis, value) in values.iter_mut().enumerate() {
                    *value = u32le(raw, event * 12 + axis * 4)? as i32;
                }
            }
        }
        Ok(Self {
            reactions,
            rows,
            attitude,
            initial,
        })
    }
    pub fn react(&self, relations: &mut [[i32; 3]; 10], partner: u8, event: u8) -> Result<()> {
        let deltas = self
            .reactions
            .get(usize::from(partner))
            .and_then(|events| events.get(usize::from(event)))
            .ok_or("Invalid relationship event")?;
        for (value, delta) in relations[usize::from(partner)].iter_mut().zip(deltas) {
            *value = (i64::from(*value) + i64::from(*delta)).clamp(-0x100000, 0x100000) as i32;
        }
        Ok(())
    }
    pub fn relations(&self, character: u8) -> Result<[[i32; 3]; 10]> {
        if !(1..=8).contains(&character) {
            return Err("Invalid dialogue character".into());
        }
        Ok(std::array::from_fn(|partner| {
            self.initial[partner][usize::from(character)]
        }))
    }
    pub fn select(
        &self,
        level: u8,
        partner: u8,
        character: u8,
        relation: [i32; 3],
        draw: u32,
    ) -> Result<(u8, u16)> {
        if !(1..=5).contains(&level) || partner >= 8 || !(1..=8).contains(&character) {
            return Err("Invalid dialogue selection".into());
        }
        let level = i64::from(level - 1);
        let score = i64::from(relation[1]) - i64::from(relation[0]) + i64::from(relation[2]);
        let class = if score < 16000 - level * 1500 {
            1
        } else if score >= 32000 - level * 3000 {
            3
        } else {
            2
        };
        let att = self.attitude[class] % 3;
        let row = self.rows[level as usize * 8 + usize::from(partner)];
        let mut choices = vec![row[usize::from(att)], row[3 + usize::from(att)]];
        for (index, base) in [(12, 6), (13, 9)] {
            if row[index] == u16::from(character) {
                choices.push(row[base + usize::from(att)]);
            }
        }
        Ok((att, choices[draw as usize % choices.len()]))
    }
}
pub fn readable(text: &str) -> String {
    text.replace("\r\n", "\n")
        .chars()
        .map(|c| {
            if c == '\r' {
                '\n'
            } else if c.is_control() && c != '\n' && c != '\t' {
                ' '
            } else {
                c
            }
        })
        .collect()
}
/// Stable source string IDs, rather than positional assumptions in translated layouts.
pub fn language_inventory(data: &Path) -> Result<BTreeMap<String, serde_json::Value>> {
    let mut result = BTreeMap::new();
    for language in Language::ALL {
        let text = SourceText::load(data, language)?;
        result.insert(language.dll().into(),serde_json::json!({"resources":text.resource_count,"strings":text.strings.len(),
            "empty_ids":text.strings.iter().enumerate().skip(1).filter(|(_,s)|s.is_empty()).map(|(i,_)|i).collect::<Vec<_>>(),
            "credits":text.credits.each_ref().map(|lines|lines.len()),"sha256":text.source_sha256}));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dialogue() -> Dialogue {
        let mut rows = [[0; 14]; 40];
        for row in &mut rows {
            for (i, value) in row[..12].iter_mut().enumerate() {
                *value = 100 + i as u16;
            }
            row[12] = 1;
            row[13] = 2;
        }
        Dialogue {
            rows,
            attitude: [0, 0, 1, 2],
            initial: [[[0; 3]; 10]; 10],
            reactions: [[[1, -2, 3]; 5]; 10],
        }
    }
    #[test]
    fn dialogue_thresholds_personalized_choices_and_bounds() {
        let d = dialogue();
        assert_eq!(d.select(1, 0, 1, [0, 15999, 0], 0).unwrap(), (0, 100));
        assert_eq!(d.select(1, 0, 1, [0, 16000, 0], 0).unwrap(), (1, 101));
        assert_eq!(d.select(1, 0, 1, [0, 32000, 0], 0).unwrap(), (2, 102));
        assert_eq!(d.select(1, 0, 1, [0; 3], 2).unwrap(), (0, 106));
        assert_eq!(d.select(1, 0, 2, [0; 3], 2).unwrap(), (0, 109));
        assert_eq!(d.select(1, 0, 3, [0; 3], 2).unwrap(), (0, 100));
        assert!(d.select(0, 0, 1, [0; 3], 0).is_err());
        assert!(d.select(1, 8, 1, [0; 3], 0).is_err());
        assert!(d.select(1, 0, 9, [0; 3], 0).is_err());
    }
    #[test]
    fn reaction_saturation_preserves_other_people_and_rejects_invalid_events() {
        let d = dialogue();
        let mut r = [[0; 3]; 10];
        r[4] = [0x100000, -0x100000, 0];
        d.react(&mut r, 4, 2).unwrap();
        assert_eq!(r[4], [0x100000, -0x100000, 3]);
        assert_eq!(r[3], [0; 3]);
        let before = r;
        assert!(d.react(&mut r, 10, 0).is_err());
        assert!(d.react(&mut r, 4, 5).is_err());
        assert_eq!(r, before);
    }
}
