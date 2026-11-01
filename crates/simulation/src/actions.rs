//! Source nitro scalar state and explicit player-only action policy (schema 6).
use crate::{Command, Error, Result};
use rr_formats::{
    binary::{range, u32le},
    resource::Directory,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub recharge: i32,
    pub multiplier_q8: i32,
    pub maximum: i32,
    pub charges: i32,
    pub minimum: i32,
    pub tail: i32,
}
impl Config {
    pub fn read(bytes: &[u8], bike: u32) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let directory = Directory::read(bytes)?;
        let entry = directory
            .entries
            .iter()
            .find(|r| r.tag == "SPEC" && r.id == bike)
            .ok_or("Missing nitro bike specification")?;
        let data = range(bytes, entry.offset as usize, entry.length as usize)?;
        let word = |index: usize| u32le(data, index * 4).map(|n| n as i32);
        let value = Self {
            recharge: word(83)?,
            multiplier_q8: word(84)?,
            maximum: word(85)?,
            charges: word(86)?,
            minimum: word(87)?,
            tail: word(88)?,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(self) -> Result<()> {
        if !(0..=256).contains(&self.recharge)
            || !(0..=1024).contains(&self.multiplier_q8)
            || !(0..=65535).contains(&self.maximum)
            || !(0..=1000).contains(&self.charges)
            || !(0..=self.maximum).contains(&self.minimum)
            || !(0..=65535).contains(&self.tail)
        {
            return Err(Error::Invalid("nitro configuration"));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    pub charge_q8: i32,
    pub charges: i32,
    pub active: bool,
    pub dismount_held: bool,
}
impl State {
    pub fn new(config: Config) -> Self {
        Self {
            charge_q8: config.maximum * 256,
            charges: config.charges,
            active: false,
            dismount_held: false,
        }
    }
    /// One source think tick: recharge/drain first, then activation and force selection.
    /// Recovery/failed bikes cannot activate; airborne boosting is source-compatible; releasing nitro alone retains
    /// an active charge until throttle release, depletion, crash or finish.
    pub fn tick(&mut self, config: Config, input: Command, eligible: bool) -> i32 {
        if self.active {
            self.charge_q8 -= 256;
            if self.charge_q8 < 1 {
                self.active = false;
            }
        } else if self.charge_q8 / 256 < config.maximum {
            self.charge_q8 += config.recharge;
        }
        if !eligible || !input.throttle || input.brake {
            self.active = false;
        } else if input.nitro
            && !self.active
            && self.charges > 0
            && self.charge_q8 / 256 >= config.minimum
        {
            self.charges -= 1;
            self.active = true;
        }
        if self.active {
            config.multiplier_q8
        } else {
            256
        }
    }
}
