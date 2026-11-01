//! Stable interface text keys and parameter substitution, separate from layouts.
use crate::{Result, text::Language};
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Deserialize)]
pub struct Catalog {
    pub entries: Vec<Entry>,
}
#[derive(Deserialize)]
pub struct Entry {
    pub key: String,
    pub labels: [String; 5],
}
impl Catalog {
    pub fn built_in() -> Self {
        serde_json::from_str(include_str!("../../../docs/fixtures/ui-text.json"))
            .expect("validated authored UI dictionary")
    }
    pub fn validate(&self) -> Result<()> {
        let mut keys = std::collections::BTreeSet::new();
        fn parameters(text: &str) -> Result<std::collections::BTreeSet<&str>> {
            let mut result = std::collections::BTreeSet::new();
            for tail in text.split('{').skip(1) {
                let (name, _) = tail.split_once('}').ok_or("Unclosed interface parameter")?;
                if name.is_empty() {
                    return Err("Empty interface parameter".into());
                }
                result.insert(name);
            }
            Ok(result)
        }
        for entry in &self.entries {
            let expected = parameters(&entry.labels[0])?;
            for label in &entry.labels {
                if parameters(label)? != expected {
                    return Err("Translated parameter mismatch".into());
                }
            }
            if entry.key.is_empty()
                || !keys.insert(&entry.key)
                || entry.labels.iter().any(String::is_empty)
            {
                return Err("Invalid or duplicate interface text key".into());
            }
        }
        Ok(())
    }
    pub fn message(
        &self,
        key: &str,
        language: Language,
        values: &BTreeMap<&str, String>,
    ) -> Result<String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.key == key)
            .ok_or("Unknown interface text key")?;
        let index = Language::ALL.iter().position(|l| *l == language).unwrap();
        let mut text = entry.labels[index].clone();
        for (key, value) in values {
            text = text.replace(&format!("{{{key}}}"), value);
        }
        if text.contains('{') || text.contains('}') {
            return Err("Missing interface text parameter".into());
        }
        Ok(text)
    }
    pub fn label(&self, english: &str, language: Language) -> String {
        let (base, suffix) = english
            .split_once(':')
            .map_or((english, None), |(a, b)| (a, Some(b)));
        let index = Language::ALL.iter().position(|l| *l == language).unwrap();
        let label = self
            .entries
            .iter()
            .find(|e| e.labels[0] == base)
            .map_or(base, |e| e.labels[index].as_str());
        match suffix {
            Some(value) => {
                let value = match value.trim() {
                    "true" => self.label("ON", language),
                    "false" => self.label("OFF", language),
                    _ => value.trim().into(),
                };
                format!("{label}: {value}")
            }
            None => label.into(),
        }
    }
}
/// Layout budget is independent of physical DPI: the application renders inside
/// an aspect-preserving logical canvas. Long text is wrapped before pagination.
pub fn wrap(text: &str, columns: usize) -> Vec<String> {
    let columns = columns.max(1);
    let mut lines = vec![];
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if line.chars().count() + usize::from(!line.is_empty()) + word.chars().count() > columns
                && !line.is_empty()
            {
                lines.push(std::mem::take(&mut line));
            }
            for character in word.chars() {
                if line.chars().count() == columns {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(character);
            }
            line.push(' ');
        }
        lines.push(line.trim_end().into());
    }
    lines
}
pub fn visible_rows(scale: u16, extra_lines: usize) -> usize {
    let font = 14 * usize::from(scale.clamp(75, 150)) / 100;
    (400 / font.max(1)).saturating_sub(extra_lines).clamp(3, 12)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub font: f32,
    pub columns: usize,
    pub menu_rows: usize,
    pub notice_rows: usize,
    pub credit_rows: usize,
    pub table_rows: usize,
    pub compact: bool,
}
impl Default for Layout {
    fn default() -> Self {
        Self::new(640., 480., 100)
    }
}
impl Layout {
    pub fn new(width: f32, height: f32, scale: u16) -> Self {
        let view = (width.max(320.) / 640.)
            .min(height.max(240.) / 480.)
            .min(1.);
        let font = 14. * f32::from(scale.clamp(75, 150)) / 100. / view;
        let rows = (420. / (font * 1.25)).floor() as usize;
        Self {
            font,
            columns: (580. / (font * 0.62)).floor().max(16.) as usize,
            menu_rows: rows.saturating_sub(10).div_ceil(2).clamp(1, 8),
            notice_rows: rows.saturating_sub(14).clamp(1, 4),
            credit_rows: rows.saturating_sub(8).clamp(1, 8),
            table_rows: rows.saturating_sub(14).clamp(1, 6),
            compact: view < 1.,
        }
    }
}

/// Read-only HUD conversion; speed follows the native display divisor (not an invented physical unit).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudValues {
    pub speed: i32,
    pub gear: u8,
    pub health: i32,
    pub maximum: i32,
    pub condition_percent: i32,
    pub nitro: i32,
}
impl HudValues {
    pub fn from_snapshot(snapshot: &rr_simulation::Snapshot, capacity: i32) -> Option<Self> {
        let actor = snapshot.actors.iter().find(|a| a.id.0 == 1)?;
        let rider = actor.rider.as_ref()?;
        let unit = actor.encounter.as_ref()?;
        Some(Self {
            speed: rider.bike.speed.max(0) / 100,
            gear: (rider.bike.gear + 1) as u8,
            health: unit.fighter.stats.health,
            maximum: unit.fighter.stats.maximum,
            condition_percent: (rider.bike.condition * 100 / capacity.max(1)).clamp(0, 100),
            nitro: rider.actions.map_or(0, |a| a.charges),
        })
    }
}
/// USD game values use locale grouping; no currency conversion or float rounding.
pub fn number(value: u64, language: Language) -> String {
    let separator = match language {
        Language::English => ',',
        Language::French => ' ',
        _ => '.',
    };
    let digits = value.to_string();
    let mut result = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            result.push(separator);
        }
        result.push(c);
    }
    result
}
impl Catalog {
    pub fn counted(&self, key: &str, count: u64, language: Language) -> Result<String> {
        let singular = if language == Language::French {
            count <= 1
        } else {
            count == 1
        };
        self.message(
            &format!("{key}_{}", if singular { "one" } else { "other" }),
            language,
            &BTreeMap::from([("count", number(count, language))]),
        )
    }
}
