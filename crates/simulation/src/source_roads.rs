//! Compact route-context ground/cache identities for steered physical roads.
use crate::{Error, Result, handling::Ground, topology::Topology};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_progress: Option<i64>,
    pub choices: Vec<u8>,
    /// Selected physical path -> global source sample at its beginning.
    pub selected: BTreeMap<u32, u32>,
    /// Per-path sorted [local sample, global sample, road offset, road mask].
    pub identities: BTreeMap<u32, Vec<[i32; 4]>>,
}
impl Profile {
    pub fn identity(&self, path: u32, local: u32) -> Option<[i32; 4]> {
        let samples = self.identities.get(&path)?;
        samples
            .binary_search_by_key(&local, |v| v[0] as u32)
            .ok()
            .map(|i| samples[i])
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fork {
    pub bit: u8,
    pub halves: [u32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Roads {
    pub version: u8,
    pub initial: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lanes_q8: Option<BTreeMap<u32, Vec<[i32; 2]>>>,
    /// Ground in road-relative coordinates, once per physical node.
    pub ground: BTreeMap<u32, Vec<Ground>>,
    pub forks: BTreeMap<u32, Fork>,
    pub profiles: Vec<Profile>,
}
impl Roads {
    pub fn validate(&self, t: &Topology) -> Result<()> {
        if self.version != 1
            || self.profiles.is_empty()
            || self.profiles.len() > 64
            || usize::from(self.initial) >= self.profiles.len()
            || self.ground.len() != t.paths.len()
            || self.forks.len() > 6
        {
            return Err(Error::Invalid("physical road profile budget"));
        }
        let mut total = 0usize;
        for (path, ground) in &self.ground {
            let p = t
                .paths
                .get(path)
                .ok_or(Error::Invalid("physical ground path"))?;
            total = total.checked_add(ground.len()).ok_or(Error::Overflow)?;
            if p.length % 256 != 0
                || ground.len() != ((p.length + 255) / 256) as usize
                || total > 1_000_000
                || ground.iter().any(|g| {
                    g.left > g.right
                        || g.left_shoulder > g.left
                        || g.right_shoulder < g.right
                        || g.kind > 4
                        || !(-128..=127).contains(&g.rise)
                        || !(-128..=127).contains(&g.bend)
                        || [
                            g.left,
                            g.right,
                            g.left_shoulder,
                            g.right_shoulder,
                            g.height,
                            g.movement,
                        ]
                        .iter()
                        .any(|v| v.unsigned_abs() > 1_000_000)
                })
            {
                return Err(Error::Invalid("physical ground bounds"));
            }
        }
        if let Some(lanes) = &self.lanes_q8
            && (lanes.len() != self.ground.len()
                || lanes.iter().any(|(path, values)| {
                    self.ground
                        .get(path)
                        .is_none_or(|g| g.len() != values.len())
                        || values.iter().flatten().any(|n| !(0..=65536).contains(n))
                }))
        {
            return Err(Error::Invalid("source lane geometry"));
        }
        let bits = self.profiles[0].choices.len();
        let mut choices = std::collections::BTreeSet::new();
        let mut records = 0usize;
        let mut physical_offsets = BTreeMap::new();
        if bits > 6 || self.profiles.len() != 1usize << bits {
            return Err(Error::Invalid("physical road choices"));
        }
        let origin = self
            .origin(t)
            .ok_or(Error::Invalid("physical route origin"))?;
        for (index, profile) in self.profiles.iter().enumerate() {
            if profile
                .choices
                .iter()
                .enumerate()
                .fold(0usize, |mask, (bit, value)| {
                    mask | (usize::from(*value) << bit)
                })
                != index
            {
                return Err(Error::Invalid("physical route mask order"));
            }
            if profile.choices.len() != bits
                || profile.choices.iter().any(|v| *v > 1)
                || !choices.insert(&profile.choices)
                || profile.selected.is_empty()
                || profile.selected.len() > t.paths.len()
                || profile.identities.len() > t.paths.len()
            {
                return Err(Error::Invalid("physical route profile"));
            }
            for (path, start) in &profile.selected {
                if *start > 1_000_000 || !self.ground.contains_key(path) {
                    return Err(Error::Invalid("physical route path"));
                }
            }
            for (path, samples) in &profile.identities {
                for identity in samples {
                    if physical_offsets
                        .insert((*path, identity[0]), identity[2])
                        .is_some_and(|old| old != identity[2])
                    {
                        return Err(Error::Invalid("inconsistent physical road offset"));
                    }
                }
                let ground = self
                    .ground
                    .get(path)
                    .ok_or(Error::Invalid("physical identity path"))?;
                records = records.checked_add(samples.len()).ok_or(Error::Overflow)?;
                if records > 8_000_000
                    || samples.len() > ground.len()
                    || samples.windows(2).any(|v| v[0][0] >= v[1][0])
                    || samples.iter().any(|v| {
                        v[0] < 0
                            || v[0] as usize >= ground.len()
                            || !(0..=1_000_000).contains(&v[1])
                            || v[2].unsigned_abs() > 1_000_000
                            || !(1..=3).contains(&v[3])
                    })
                {
                    return Err(Error::Invalid("physical route identities"));
                }
            }
            let mut ordered = profile
                .selected
                .iter()
                .map(|(path, start)| (*start, *path))
                .collect::<Vec<_>>();
            ordered.sort();
            let mut cursor = 0u32;
            let mut previous = None;
            for (start, path) in ordered {
                if start != cursor
                    || previous.is_some_and(|from| {
                        !t.paths[&from]
                            .portals
                            .iter()
                            .any(|p| p.end && p.target == path)
                    })
                {
                    return Err(Error::Invalid("disconnected physical route sequence"));
                }
                cursor = cursor
                    .checked_add(self.ground[&path].len() as u32)
                    .ok_or(Error::Overflow)?;
                previous = Some(path);
                if cursor > 1_000_000 {
                    return Err(Error::Invalid("physical route sample budget"));
                }
            }
            if profile
                .finish_progress
                .is_some_and(|finish| finish < 0 || finish > i64::from(cursor) * 256 - origin)
            {
                return Err(Error::Invalid("physical route finish"));
            }
            for (path, start) in &profile.selected {
                for local in 0..self.ground[path].len() {
                    if profile
                        .identity(*path, local as u32)
                        .is_none_or(|i| i[1] as u32 != start + local as u32)
                    {
                        return Err(Error::Invalid("incomplete physical selected route"));
                    }
                }
            }
        }
        for (path, start) in &self.profiles[usize::from(self.initial)].selected {
            if t.paths[path].selected_start != Some(i64::from(*start) * 256 - origin) {
                return Err(Error::Invalid("initial physical projection"));
            }
        }
        for (from, fork) in &self.forks {
            let p = t
                .paths
                .get(from)
                .ok_or(Error::Invalid("physical fork path"))?;
            if usize::from(fork.bit) >= bits
                || fork.halves[0] == fork.halves[1]
                || fork
                    .halves
                    .iter()
                    .any(|target| !p.portals.iter().any(|e| e.end && e.target == *target))
            {
                return Err(Error::Invalid("physical fork portal"));
            }
            for profile in &self.profiles {
                if let Some(start) = profile.selected.get(from) {
                    let target = fork.halves[usize::from(profile.choices[usize::from(fork.bit)])];
                    if profile.selected.get(&target)
                        != Some(&(*start + self.ground[from].len() as u32))
                    {
                        return Err(Error::Invalid("physical fork choice path"));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn ground(&self, index: u8) -> Result<Vec<Ground>> {
        let profile = self
            .profiles
            .get(usize::from(index))
            .ok_or(Error::Invalid("physical route index"))?;
        let length = profile
            .selected
            .iter()
            .map(|(path, start)| *start as usize + self.ground[path].len())
            .max()
            .ok_or(Error::Invalid("empty physical route"))?;
        if length > 1_000_000 {
            return Err(Error::Invalid("physical route length"));
        }
        let mut out = vec![Ground::default(); length];
        let mut filled = vec![false; length];
        for (path, start) in &profile.selected {
            for (local, base) in self.ground[path].iter().enumerate() {
                let offset = profile
                    .identity(*path, local as u32)
                    .ok_or(Error::Invalid("missing physical road ground"))?[2];
                let mut ground = *base;
                ground.left += offset;
                ground.right += offset;
                ground.left_shoulder += offset;
                ground.right_shoulder += offset;
                let at = *start as usize + local;
                if std::mem::replace(&mut filled[at], true) {
                    return Err(Error::Invalid("overlapping physical route"));
                }
                out[at] = ground;
            }
        }
        if filled.contains(&false) {
            return Err(Error::Invalid("physical route gap"));
        }
        Ok(out)
    }
}

impl Roads {
    /// Enumerate all bounded source branch contexts; ground types are assigned by the asset adapter.
    pub fn from_course(
        course: &rr_course::Course,
        initial: &rr_course::route::Route,
    ) -> Result<Self> {
        let splits = course.splits();
        if splits.len() > 6 {
            return Err(Error::Invalid("source split budget"));
        }
        let mut ground = BTreeMap::new();
        let mut lanes_q8 = BTreeMap::new();
        let mut forks = BTreeMap::new();
        for (&path, node) in &course.nodes {
            if node.kind != rr_course::NodeKind::Road {
                continue;
            }
            lanes_q8.insert(
                path,
                course.sections[node.link as usize]
                    .samples
                    .iter()
                    .take(node.length as usize)
                    .map(|s| s.lanes_q8)
                    .collect(),
            );
            ground.insert(
                path,
                course.sections[node.link as usize]
                    .samples
                    .iter()
                    .take(node.length as usize)
                    .map(|s| Ground {
                        left: -s.width[0],
                        right: s.width[1],
                        left_shoulder: -s.width[0] - s.shoulder[0],
                        right_shoulder: s.width[1] + s.shoulder[1],
                        flags: s.marking_flags,
                        rise: i32::from(s.rise),
                        bend: i32::from(s.bend),
                        ..Ground::default()
                    })
                    .collect(),
            );
            let next = &course.nodes[&node.next];
            if next.kind == rr_course::NodeKind::Split {
                let bit = splits
                    .iter()
                    .position(|id| *id == next.offset)
                    .ok_or(Error::Invalid("missing source fork"))?;
                forks.insert(
                    path,
                    Fork {
                        bit: bit as u8,
                        halves: [next.next, next.previous],
                    },
                );
            }
        }
        let initial = initial
            .choices
            .iter()
            .enumerate()
            .fold(0u8, |mask, (bit, value)| mask | (*value << bit));
        let mut profiles = vec![];
        for mask in 0..1usize << splits.len() {
            let choices = (0..splits.len())
                .map(|bit| ((mask >> bit) & 1) as u8)
                .collect::<Vec<_>>();
            let route = course
                .route(&choices)
                .map_err(|_| Error::Invalid("source route construction"))?;
            let mut selected = BTreeMap::new();
            let mut physical: BTreeMap<u32, BTreeMap<u32, [i32; 4]>> = BTreeMap::new();
            let mut insert = |path, local: u32, sample: usize, offset, road: i32| -> Result<()> {
                let samples = physical.entry(path).or_default();
                if let Some(old) = samples.get_mut(&local) {
                    if old[1] != sample as i32 || old[2] != offset {
                        return Err(Error::Invalid("conflicting physical route identity"));
                    }
                    old[3] |= road;
                } else {
                    samples.insert(local, [local as i32, sample as i32, offset, road]);
                }
                Ok(())
            };
            for (sample, p) in route.points.iter().enumerate() {
                selected
                    .entry(p.node)
                    .or_insert((sample - p.local as usize) as u32);
                insert(
                    p.node,
                    p.local,
                    sample,
                    p.offset,
                    p.dual.as_ref().map_or(3, |d| 1 << d.half),
                )?;
                if let Some(d) = &p.dual {
                    insert(d.node, d.local, sample, d.offset, 1 << (1 - d.half))?;
                }
            }
            let identities = physical
                .into_iter()
                .map(|(path, samples)| (path, samples.into_values().collect()))
                .collect();
            profiles.push(Profile {
                finish_progress: None,
                choices,
                selected,
                identities,
            });
        }
        Ok(Self {
            version: 1,
            initial,
            lanes_q8: Some(lanes_q8),
            ground,
            forks,
            profiles,
        })
    }
    pub fn origin(&self, topology: &Topology) -> Option<i64> {
        self.profiles[usize::from(self.initial)]
            .selected
            .iter()
            .find_map(|(path, start)| {
                Some(i64::from(*start) * 256 - topology.paths.get(path)?.selected_start?)
            })
    }
    pub fn location(
        &self,
        index: u8,
        distance: i64,
        origin: i64,
        lateral: i32,
        height: i32,
    ) -> Option<crate::topology::Location> {
        let global = distance.checked_add(origin)?;
        let profile = self.profiles.get(usize::from(index))?;
        profile
            .selected
            .iter()
            .find_map(|(path, start)| {
                let start = i64::from(*start) * 256;
                let length = self.ground.get(path)?.len() as i64 * 256;
                ((start..start + length).contains(&global)).then_some(crate::topology::Location {
                    path: *path,
                    progress: global - start,
                    lateral,
                    height,
                    heading: 0,
                })
            })
            .or_else(|| {
                profile.selected.iter().find_map(|(path, start)| {
                    let start = i64::from(*start) * 256;
                    let length = self.ground.get(path)?.len() as i64 * 256;
                    (global == start + length).then_some(crate::topology::Location {
                        path: *path,
                        progress: length,
                        lateral,
                        height,
                        heading: 0,
                    })
                })
            })
    }
}
impl Roads {
    pub fn topology(&self, index: u8, base: &Topology) -> Result<Topology> {
        let profile = self
            .profiles
            .get(usize::from(index))
            .ok_or(Error::Invalid("physical route context"))?;
        let origin = self
            .origin(base)
            .ok_or(Error::Invalid("physical route origin"))?;
        let mut result = base.clone();
        for (path, p) in &mut result.paths {
            p.selected_start = profile
                .selected
                .get(path)
                .map(|start| i64::from(*start) * 256 - origin);
        }
        Ok(result)
    }
    pub fn fork(
        &self,
        index: u8,
        at: crate::topology::Location,
        lateral: i32,
        planned: Option<crate::topology::Portal>,
    ) -> Result<(u8, Option<crate::topology::Portal>)> {
        let Some(fork) = self.forks.get(&at.path) else {
            return Ok((index, None));
        };
        let profile = self
            .profiles
            .get(usize::from(index))
            .ok_or(Error::Invalid("physical fork context"))?;
        let mut choices = profile.choices.clone();
        choices[usize::from(fork.bit)] = 0;
        let left = self
            .profiles
            .iter()
            .find(|p| p.choices == choices)
            .ok_or(Error::Invalid("missing left physical route"))?;
        let identity = left
            .identity(fork.halves[0], 0)
            .ok_or(Error::Invalid("physical split edge"))?;
        let edge = identity[2] + self.ground[&fork.halves[0]][0].right;
        let half = planned
            .and_then(|p| fork.halves.iter().position(|target| *target == p.target))
            .unwrap_or(usize::from((lateral >> 8) >= edge));
        choices[usize::from(fork.bit)] = half as u8;
        let index = self
            .profiles
            .iter()
            .position(|p| p.choices == choices)
            .ok_or(Error::Invalid("steered physical route"))? as u8;
        Ok((
            index,
            Some(crate::topology::Portal {
                end: true,
                target: fork.halves[half],
                target_end: false,
            }),
        ))
    }
}

impl Roads {
    pub fn set_finishes(
        &mut self,
        course: &rr_course::Course,
        level: u8,
        topology: &Topology,
    ) -> Result<()> {
        let origin = self
            .origin(topology)
            .ok_or(Error::Invalid("physical finish origin"))?;
        for profile in &mut self.profiles {
            let route = course
                .route(&profile.choices)
                .map_err(|_| Error::Invalid("physical finish route"))?;
            let finish = route
                .finish(course, level)
                .map_err(|_| Error::Invalid("missing physical finish"))?;
            profile.finish_progress = Some(finish as i64 * 256 - origin);
        }
        Ok(())
    }
}
impl Roads {
    pub fn physical_identity(&self, path: u32, local: u32) -> Option<[i32; 4]> {
        self.profiles
            .iter()
            .find_map(|profile| profile.identity(path, local))
    }
    pub fn physical_delta(
        topology: &Topology,
        from: crate::topology::Location,
        to: crate::topology::Location,
    ) -> Option<i64> {
        if from.path == to.path {
            return Some(to.progress - from.progress);
        }
        let portal = topology
            .paths
            .get(&from.path)?
            .portals
            .iter()
            .find(|p| p.target == to.path)?;
        Some(if portal.end {
            topology.paths[&from.path].length - from.progress + to.progress
        } else {
            -from.progress - (topology.paths[&to.path].length - to.progress)
        })
    }
}
impl Roads {
    pub fn advance_contact(
        &self,
        index: u8,
        mut at: crate::topology::Location,
        delta: i64,
        lateral: i32,
        base: &Topology,
    ) -> Result<(u8, crate::topology::Location)> {
        let length = base
            .paths
            .get(&at.path)
            .ok_or(Error::Invalid("contact physical road"))?
            .length;
        let (index, planned) = if delta > 0 && at.progress.saturating_add(delta) >= length {
            self.fork(index, at, lateral, None)?
        } else {
            (index, None)
        };
        self.topology(index, base)?
            .advance(&mut at, delta, planned)?;
        Ok((index, at))
    }
}
