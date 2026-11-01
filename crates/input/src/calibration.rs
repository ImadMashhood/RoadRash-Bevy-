//! Controller-model calibration. Physical floats stop at this transport boundary.
use crate::{Error, Result, storage};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Response {
    #[default]
    Linear,
    Square,
    Cubic,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calibration {
    pub minimum: f64,
    pub center: f64,
    pub maximum: f64,
    pub dead_zone: f64,
    pub saturation: f64,
    pub response: Response,
    pub invert: bool,
    pub trigger_threshold: f64,
}
impl Default for Calibration {
    fn default() -> Self {
        Self {
            minimum: -1.,
            center: 0.,
            maximum: 1.,
            dead_zone: 0.15,
            saturation: 0.98,
            response: Response::Linear,
            invert: false,
            trigger_threshold: 0.5,
        }
    }
}
impl Calibration {
    pub fn validate(&self) -> Result<()> {
        for value in [
            self.minimum,
            self.center,
            self.maximum,
            self.dead_zone,
            self.saturation,
            self.trigger_threshold,
        ] {
            if !value.is_finite() {
                return Err(Error("Calibration values must be finite".into()));
            }
        }
        if self.minimum < -1.
            || self.maximum > 1.
            || self.center - self.minimum < 0.05
            || self.maximum - self.center < 0.05
        {
            return Err(Error(
                "Each side of center needs at least 0.05 travel in [-1,1]".into(),
            ));
        }
        if !(0.0..=0.5).contains(&self.dead_zone)
            || !(0.5..=1.).contains(&self.saturation)
            || self.saturation - self.dead_zone < 0.05
        {
            return Err(Error(
                "Dead zone must be 0..0.5 and full-lock threshold 0.5..1, with a 0.05 gap".into(),
            ));
        }
        if !(0.05..=0.95).contains(&self.trigger_threshold) {
            return Err(Error("Trigger threshold must be 0.05..0.95".into()));
        }
        Ok(())
    }
    pub fn steering_q15(&self, raw: f32) -> i16 {
        if !raw.is_finite() || self.validate().is_err() {
            return 0;
        }
        let raw = f64::from(raw.clamp(-1., 1.));
        let offset = raw - self.center;
        let range = if offset < 0. {
            self.center - self.minimum
        } else {
            self.maximum - self.center
        };
        let signed = (offset / range).clamp(-1., 1.);
        let magnitude =
            ((signed.abs() - self.dead_zone) / (self.saturation - self.dead_zone)).clamp(0., 1.);
        let response = match self.response {
            Response::Linear => magnitude,
            Response::Square => magnitude * magnitude,
            Response::Cubic => magnitude * magnitude * magnitude,
        };
        (response * signed.signum() * if self.invert { -32767. } else { 32767. }).round() as i16
    }
    pub fn trigger(&self, raw: f32) -> bool {
        raw.is_finite() && self.validate().is_ok() && f64::from(raw) >= self.trigger_threshold
    }
    /// Evaluate the exact same curve used by gameplay, not a UI approximation.
    pub fn curve(&self) -> Vec<[f64; 2]> {
        (0..=200)
            .map(|i| {
                let x = -1. + f64::from(i) / 100.;
                [x, f64::from(self.steering_q15(x as f32)) / 32767.]
            })
            .collect()
    }
}
/// Persist model identity, never transient ECS entities or USB port numbers.
pub fn device_key(vendor: Option<u16>, product: Option<u16>, name: &str) -> String {
    let part = |n: Option<u16>| n.map_or_else(|| "----".into(), |n| format!("{n:04x}"));
    let name = name
        .chars()
        .filter(|c| !c.is_control())
        .flat_map(char::to_lowercase)
        .take(128)
        .collect::<String>();
    let name = name.trim();
    format!(
        "{}:{}:{}",
        part(vendor),
        part(product),
        if name.is_empty() {
            "unnamed controller"
        } else {
            name
        }
    )
}
fn valid_key(key: &str) -> bool {
    if key.len() < 11 || !key.is_char_boundary(10) {
        return false;
    }
    let bytes = key.as_bytes();
    let part = |s: &str| {
        s == "----"
            || (s.len() == 4
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    };
    bytes.get(4) == Some(&b':')
        && bytes.get(9) == Some(&b':')
        && key.is_char_boundary(4)
        && key.is_char_boundary(5)
        && key.is_char_boundary(9)
        && part(&key[..4])
        && part(&key[5..9])
        && key[10..].chars().count() <= 128
        && !key[10..].chars().any(char::is_control)
        && key[10..].trim() == &key[10..]
        && key[10..].to_lowercase() == key[10..]
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationFile {
    pub schema_version: u32,
    pub default: Calibration,
    pub devices: BTreeMap<String, Calibration>,
}
impl Default for CalibrationFile {
    fn default() -> Self {
        Self {
            schema_version: 1,
            default: Calibration::default(),
            devices: BTreeMap::new(),
        }
    }
}
impl CalibrationFile {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.devices.len() > 32 {
            return Err(Error(
                "Unsupported calibration schema or more than 32 controller profiles".into(),
            ));
        }
        self.default.validate()?;
        for (key, profile) in &self.devices {
            if !valid_key(key) {
                return Err(Error("Invalid controller model key".into()));
            }
            profile.validate()?;
        }
        Ok(())
    }
    pub fn profile(&self, key: &str) -> Calibration {
        self.devices.get(key).copied().unwrap_or(self.default)
    }
    pub fn set(&mut self, key: String, profile: Calibration) -> Result<()> {
        let mut next = self.clone();
        next.devices.insert(key, profile);
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self> {
        let profiles: Self = storage::read(path)?;
        profiles.validate()?;
        Ok(profiles)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        storage::save(self, path)
    }
}
#[derive(Clone, Debug)]
pub struct Capture {
    count: u32,
    sum: f64,
    minimum: f64,
    maximum: f64,
}
impl Default for Capture {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.,
            minimum: 1.,
            maximum: -1.,
        }
    }
}
impl Capture {
    pub fn push(&mut self, raw: f32) {
        if !raw.is_finite() {
            return;
        }
        let raw = f64::from(raw.clamp(-1., 1.));
        if self.count < u32::MAX {
            self.count += 1;
            self.sum += raw;
        }
        self.minimum = self.minimum.min(raw);
        self.maximum = self.maximum.max(raw);
    }
    pub fn len(&self) -> usize {
        self.count as usize
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    fn bounds(&self) -> Result<(f64, f64)> {
        if self.count < 16 {
            return Err(Error("Collect at least 16 valid samples".into()));
        }
        Ok((self.minimum, self.maximum))
    }
    pub fn center(&self, current: Calibration) -> Result<Calibration> {
        let (lo, hi) = self.bounds()?;
        if hi - lo > 0.12 {
            return Err(Error(
                "Stick moved too much; release it and capture center again".into(),
            ));
        }
        let center = self.sum / f64::from(self.count);
        let mut result = Calibration { center, ..current };
        result.validate()?;
        let noise = ((center - lo) / (center - result.minimum))
            .max((hi - center) / (result.maximum - center));
        result.dead_zone = (noise + 0.02)
            .clamp(0.02, 0.5)
            .min(result.saturation - 0.05);
        result.validate()?;
        Ok(result)
    }
    pub fn range(&self, current: Calibration) -> Result<Calibration> {
        let (minimum, maximum) = self.bounds()?;
        if current.center - minimum < 0.2 || maximum - current.center < 0.2 {
            return Err(Error("Sweep the stick fully in both directions".into()));
        }
        let result = Calibration {
            minimum,
            maximum,
            ..current
        };
        result.validate()?;
        Ok(result)
    }
}
