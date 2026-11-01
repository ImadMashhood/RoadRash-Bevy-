use crate::{
    Result,
    catalog::Catalog,
    race::{Mode, Outcome, Summary},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Career {
    pub character: u8,
    #[serde(default)]
    pub alias: String,
    pub level: u8,
    pub qualified: u8,
    pub bike: u8,
    pub cash: u64,
    pub condition: u16,
    pub maximum_condition: u16,
    pub completed: bool,
    revision: u64,
    next_race: u64,
    last_settled: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settlement {
    pub race_id: u64,
    pub outcome: Outcome,
    pub place: u8,
    pub prize: u64,
    pub penalty: u64,
    pub game_over: bool,
    pub level_advanced: bool,
    pub finale: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purchase {
    Bought,
    AlreadyOwned,
}
impl Career {
    pub fn new(catalog: &Catalog, character: u8) -> Result<Self> {
        let c = catalog.character(character)?;
        Ok(Self {
            character,
            alias: String::new(),
            level: 1,
            qualified: 0,
            bike: c.bikes[0],
            cash: u64::from(c.starting_cash),
            condition: 100,
            maximum_condition: 100,
            completed: false,
            revision: 0,
            next_race: 1,
            last_settled: 0,
        })
    }
    pub fn validate(&self, catalog: &Catalog) -> Result<()> {
        catalog.character(self.character)?;
        catalog.bike(self.bike)?;
        if !(1..=5).contains(&self.level)
            || self.alias.len() > 19
            || self.alias.chars().any(char::is_control)
            || self.qualified > 31
            || self.condition > self.maximum_condition
            || self.maximum_condition == 0
            || self.maximum_condition > 10000
            || self.next_race == 0
            || self.last_settled >= self.next_race
            || self.cash > u32::MAX as u64
            || self.revision == u64::MAX
            || self.next_race == u64::MAX
        {
            return Err("invalid career record".into());
        }
        Ok(())
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    fn confirm(&self, expected: u64) -> Result<()> {
        if self.revision != expected {
            Err("stale confirmation; review the current quote".into())
        } else {
            Ok(())
        }
    }
    pub fn allocate_race(&mut self) -> Result<u64> {
        let id = self.next_race;
        self.next_race = self
            .next_race
            .checked_add(1)
            .ok_or("race identifier exhausted")?;
        Ok(id)
    }
    pub fn purchase(&mut self, catalog: &Catalog, bike: u8, expected: u64) -> Result<Purchase> {
        self.validate(catalog)?;
        self.confirm(expected)?;
        let price = u64::from(catalog.bike(bike)?.price);
        let trade = u64::from(catalog.bike(self.bike)?.price / 2);
        if bike == self.bike {
            return Ok(Purchase::AlreadyOwned);
        }
        let available = self.cash.checked_add(trade).ok_or("cash overflow")?;
        if available < price {
            return Err("insufficient funds including trade-in".into());
        }
        let mut next = self.clone();
        next.cash = available - price;
        next.bike = bike;
        next.condition = next.maximum_condition;
        next.revision += 1;
        next.validate(catalog)?;
        *self = next;
        Ok(Purchase::Bought)
    }
    /// Explicit modern optional repair policy: ceil(tenth price * missing fraction).
    pub fn repair_quote(&self, catalog: &Catalog) -> Result<u64> {
        self.validate(catalog)?;
        Ok((u64::from(catalog.bike(self.bike)?.price / 10)
            * u64::from(self.maximum_condition - self.condition))
        .div_ceil(u64::from(self.maximum_condition)))
    }
    pub fn repair(&mut self, catalog: &Catalog, expected: u64) -> Result<u64> {
        self.confirm(expected)?;
        let cost = self.repair_quote(catalog)?;
        if self.cash < cost {
            return Err("insufficient funds for repair".into());
        }
        if cost == 0 && self.condition == self.maximum_condition {
            return Ok(0);
        }
        self.cash -= cost;
        self.condition = self.maximum_condition;
        self.revision += 1;
        Ok(cost)
    }
    /// The monotonic race ledger makes replaying/revisiting a result a no-op across serialization.
    pub fn settle(&mut self, catalog: &Catalog, summary: &Summary) -> Result<Option<Settlement>> {
        self.validate(catalog)?;
        summary.validate()?;
        let identity = summary.identity();
        identity.validate()?;
        if identity.mode == Mode::Thrash {
            return Ok(None);
        }
        if identity.id <= self.last_settled {
            return Ok(None);
        }
        if identity.id >= self.next_race
            || identity.id + 1 != self.next_race
            || identity.level != self.level
            || identity.bike != self.bike
        {
            return Err("result does not belong to the current career race".into());
        }
        let mut next = self.clone();
        let mut report = Settlement {
            race_id: identity.id,
            outcome: summary.outcome(),
            place: summary.place(),
            prize: 0,
            penalty: 0,
            game_over: false,
            level_advanced: false,
            finale: false,
        };
        match summary.outcome() {
            Outcome::Won | Outcome::Lost => {
                report.prize = u64::from(
                    catalog.prizes[usize::from(summary.place().saturating_sub(1).min(14))],
                ) * u64::from(next.level);
                next.cash = next
                    .cash
                    .checked_add(report.prize)
                    .ok_or("prize overflow")?;
                if summary.outcome() == Outcome::Won {
                    next.qualified |= 1 << (identity.course - 1);
                }
                next.condition = summary
                    .bike_condition()
                    .clamp(0, i32::from(next.maximum_condition))
                    as u16;
            }
            Outcome::Arrested | Outcome::Wrecked => {
                report.penalty = if summary.outcome() == Outcome::Arrested {
                    u64::from(next.level) * 400
                } else {
                    u64::from(catalog.bike(next.bike)?.price / 10)
                };
                if report.penalty <= next.cash {
                    next.cash -= report.penalty;
                    next.condition = next.maximum_condition;
                } else {
                    report.game_over = true;
                    let reset = Self::new(catalog, next.character)?;
                    next.level = reset.level;
                    next.qualified = 0;
                    next.bike = reset.bike;
                    next.cash = reset.cash;
                    next.condition = reset.condition;
                    next.completed = false;
                }
            }
            Outcome::Quit | Outcome::TimedOut => {}
        }
        if next.qualified == 31 {
            next.qualified = 0;
            if next.level < 5 {
                next.level += 1;
                report.level_advanced = true;
            } else {
                next.completed = true;
                report.finale = true;
            }
        }
        next.last_settled = identity.id;
        next.revision += 1;
        next.validate(catalog)?;
        *self = next;
        Ok(Some(report))
    }
    /// Versioned in-memory serialization contract coordinated with Phase 17's disk durability work.
    pub fn encode(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&serde_json::json!({"schema_version":1,"career":self}))
            .map_err(|e| e.to_string())
    }
    pub fn decode(bytes: &[u8], catalog: &Catalog) -> Result<Self> {
        if bytes.len() > 8192 {
            return Err("career record too large".into());
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Envelope {
            schema_version: u8,
            career: Career,
        }
        let e: Envelope = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if e.schema_version != 1 {
            return Err("unsupported career schema".into());
        }
        e.career.validate(catalog)?;
        Ok(e.career)
    }
}
