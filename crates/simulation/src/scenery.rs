//! Versioned source scenery rules and authoritative loading/marking state.
use crate::{
    Error, Result,
    topology::{Location, Topology},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoxRules {
    /// 0..15: DZHR billboard nibble; 16: BORR slot.
    pub class: u8,
    pub road_mask: u8,
    pub sample: u32,
    pub selector: u8,
    pub ordinal: u32,
    pub box_count: u16,
    /// Frame-two indexed texture flags. None leaves the native ring-slot word intact.
    pub texture_flags: Option<u32>,
}
impl BoxRules {
    pub fn validate(&self) -> Result<()> {
        if self.class > 16
            || self.road_mask == 0
            || self.road_mask > 3
            || self.sample > 1_000_000
            || self.box_count == 0
            || self.box_count > 4096
        {
            return Err(Error::Invalid("source scenery metadata"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    pub sample: i64,
    pub selector: u8,
    pub tick: u64,
    pub frame: u8,
    pub display_frame: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RoadSpan {
    pub path: u32,
    pub first: u32,
    pub last: u32,
    pub road_mask: u8,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_route: Option<u8>,
    pub loaded: BTreeSet<(u32, u32)>,
    pub slot_flags: BTreeMap<u32, Vec<u32>>,
    pub pending_camera: Option<Location>,
    pub camera_back: i32,
    pub frame_follow: Option<Location>,
    pub object_roads: BTreeMap<u64, u8>,
    pub resident: Option<BTreeSet<(u32, u32)>>,
}
impl State {
    pub fn physical_window(
        topology: &Topology,
        at: Location,
        front: i64,
        back: i64,
        physical: &BTreeMap<(u32, u32), Vec<usize>>,
    ) -> BTreeSet<(u32, u32)> {
        let mut found = BTreeSet::new();
        let mut queue = VecDeque::from([
            (at.path, at.progress, front, true),
            (at.path, at.progress, back, false),
        ]);
        let mut best = BTreeMap::new();
        while let Some((path, start, reach, forward)) = queue.pop_front() {
            if reach < 0 || best.get(&(path, forward)).is_some_and(|v| *v >= reach) {
                continue;
            }
            best.insert((path, forward), reach);
            let road = &topology.paths[&path];
            let (low, high) = if forward {
                (start, (start + reach).min(road.length))
            } else {
                ((start - reach).max(0), start)
            };
            let low = low.div_euclid(256) as u32;
            let high = high.div_euclid(256).min((road.length - 1) / 256) as u32;
            if low <= high {
                found.extend(
                    physical
                        .range((path, low)..=(path, high))
                        .map(|(key, _)| *key),
                );
            }
            let used = if forward { road.length - start } else { start };
            if reach < used {
                continue;
            }
            for portal in road.portals.iter().filter(|p| p.end == forward) {
                let target = &topology.paths[&portal.target];
                queue.push_back((
                    portal.target,
                    if portal.target_end { target.length } else { 0 },
                    reach - used,
                    forward,
                ));
            }
        }
        found
    }
    /// Source boxes become queryable on the tick after their frame was prepared.
    pub fn apply_pending(
        &mut self,
        topology: &Topology,
        physical: &BTreeMap<(u32, u32), Vec<usize>>,
    ) -> Vec<(u32, u32)> {
        let Some(at) = self.pending_camera.take() else {
            return vec![];
        };
        let window = Self::physical_window(
            topology,
            at,
            0x1e00,
            at.progress.rem_euclid(256) + 256,
            physical,
        );
        window
            .into_iter()
            .filter(|k| self.loaded.insert(*k))
            .collect()
    }
}
