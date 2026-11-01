//! Bidirectional recovery navigation over actual road-node connections.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Portal {
    pub end: bool,
    pub target: u32,
    pub target_end: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Path {
    pub length: i64,
    pub branch: u32,
    pub selected_start: Option<i64>,
    pub portals: Vec<Portal>,
    #[serde(default)]
    pub rises: Vec<i8>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Topology {
    pub paths: BTreeMap<u32, Path>,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub path: u32,
    pub progress: i64,
    pub lateral: i32,
    pub height: i32,
    pub heading: i32,
}
impl Topology {
    pub fn straight(length: i64) -> Self {
        Self {
            paths: BTreeMap::from([(
                0,
                Path {
                    length,
                    branch: 0,
                    selected_start: Some(0),
                    portals: vec![],
                    rises: vec![],
                },
            )]),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.paths.is_empty() || self.paths.len() > 4096 {
            return Err(Error::Invalid("recovery topology count"));
        }
        for (id, p) in &self.paths {
            if p.selected_start
                .is_some_and(|v| v.unsigned_abs() > 256_000_000)
                || p.length <= 0
                || p.length > 256_000_000
                || p.rises.len() > 1_000_000
                || (!p.rises.is_empty() && p.rises.len() as i64 * 256 < p.length)
                || p.portals.len() > 8
                || p.portals.iter().collect::<BTreeSet<_>>().len() != p.portals.len()
            {
                return Err(Error::Invalid("recovery path/portal bounds"));
            }
            for edge in &p.portals {
                if edge.end == edge.target_end {
                    return Err(Error::Invalid("recovery portal orientation"));
                }
                let to = self
                    .paths
                    .get(&edge.target)
                    .ok_or(Error::Invalid("dangling recovery portal"))?;
                if !to.portals.contains(&Portal {
                    end: edge.target_end,
                    target: *id,
                    target_end: edge.end,
                }) {
                    return Err(Error::Invalid("nonreciprocal recovery portal"));
                }
            }
        }
        Ok(())
    }
    pub fn valid(&self, l: Location) -> bool {
        self.paths
            .get(&l.path)
            .is_some_and(|p| (0..=p.length).contains(&l.progress))
            && l.lateral.unsigned_abs() <= 256_000_000
            && l.height.unsigned_abs() <= 100_000_000
    }
    pub fn grade(&self, l: Location) -> i32 {
        self.paths
            .get(&l.path)
            .and_then(|p| {
                p.rises
                    .get((l.progress.max(0) >> 8) as usize)
                    .or_else(|| p.rises.last())
            })
            .map_or(0, |v| i32::from(*v))
    }
    pub fn projected(&self, l: Location) -> Option<i64> {
        Some(self.paths.get(&l.path)?.selected_start? + l.progress)
    }
    pub fn locate(&self, distance: i64, lateral: i32, height: i32) -> Option<Location> {
        self.paths
            .iter()
            .filter_map(|(id, p)| {
                let start = p.selected_start?;
                ((start..start + p.length).contains(&distance)).then_some(Location {
                    path: *id,
                    progress: distance - start,
                    lateral,
                    height,
                    heading: 0,
                })
            })
            .next()
            .or_else(|| {
                self.paths
                    .iter()
                    .filter_map(|(id, p)| {
                        let start = p.selected_start?;
                        (distance == start + p.length).then_some(Location {
                            path: *id,
                            progress: p.length,
                            lateral,
                            height,
                            heading: 0,
                        })
                    })
                    .next()
            })
    }
    /// Shortest portal-count route, sorted node/endpoint ties. No proximity shortcut across roads.
    pub fn leg(&self, from: u32, to: u32) -> Option<Portal> {
        if from == to {
            return None;
        }
        let mut queue = VecDeque::from([from]);
        let mut visited = BTreeSet::from([from]);
        let mut first = BTreeMap::new();
        while let Some(at) = queue.pop_front() {
            let mut edges = self.paths.get(&at)?.portals.clone();
            edges.sort_by_key(|e| (e.target, e.end, e.target_end));
            for e in edges {
                if visited.insert(e.target) {
                    let leg = if at == from { e } else { *first.get(&at)? };
                    first.insert(e.target, leg);
                    if e.target == to {
                        return Some(leg);
                    }
                    queue.push_back(e.target);
                }
            }
        }
        None
    }
    pub fn connected(&self, a: u32, b: u32) -> bool {
        self.paths.contains_key(&a)
            && self.paths.contains_key(&b)
            && (a == b || self.leg(a, b).is_some())
    }
    /// Motion crossing follows a planned first portal when supplied; otherwise prefer the loaded route.
    pub fn advance(&self, l: &mut Location, delta: i64, planned: Option<Portal>) -> Result<()> {
        if !self.valid(*l) {
            return Err(Error::Invalid("invalid recovery location"));
        }
        let mut next = l.progress.saturating_add(delta);
        let mut plan = planned;
        for _ in 0..=self.paths.len() {
            let p = &self.paths[&l.path];
            if (0..=p.length).contains(&next)
                && !(delta > 0 && next == p.length && p.portals.iter().any(|e| e.end))
            {
                l.progress = next;
                return Ok(());
            }
            let end = next >= p.length;
            let chosen = plan
                .take()
                .filter(|e| e.end == end && p.portals.contains(e))
                .or_else(|| {
                    p.portals
                        .iter()
                        .filter(|e| e.end == end)
                        .min_by_key(|e| (self.paths[&e.target].selected_start.is_none(), e.target))
                        .copied()
                });
            let Some(e) = chosen else {
                l.progress = next.clamp(0, p.length);
                return Ok(());
            };
            let over = if end { next - p.length } else { -next };
            l.path = e.target;
            if over == 0 {
                l.progress = if e.target_end {
                    self.paths[&e.target].length
                } else {
                    0
                };
                return Ok(());
            }
            next = if e.target_end {
                self.paths[&e.target].length - over
            } else {
                over
            };
        }
        Err(Error::Invalid("recovery traversal bound"))
    }
    /// Integrates every traversed Q8 interval, including backward motion and portals.
    pub fn advance_with_rise(
        &self,
        l: &mut Location,
        delta: i64,
        planned: Option<Portal>,
    ) -> Result<i32> {
        if delta.unsigned_abs() > 10000 {
            return Err(Error::Invalid("recovery movement budget"));
        }
        let original = l.path;
        let direction = delta.signum();
        let mut rise = 0i32;
        for _ in 0..delta.unsigned_abs() {
            let before = *l;
            self.advance(
                l,
                direction,
                if l.path == original { planned } else { None },
            )?;
            if before.path == l.path && before.progress == l.progress {
                break;
            }
            rise += if direction > 0 {
                self.grade(before)
            } else {
                -self.grade(*l)
            };
        }
        Ok(rise)
    }
    pub fn from_course(
        course: &rr_course::Course,
        route: &rr_course::route::Route,
        start_sample: u32,
    ) -> Result<Self> {
        use rr_course::NodeKind;
        let mut paths = BTreeMap::new();
        for (id, n) in &course.nodes {
            if n.kind == NodeKind::Road {
                paths.insert(
                    *id,
                    Path {
                        length: i64::from(n.length) * 256,
                        branch: id.wrapping_mul(2).wrapping_add(1),
                        selected_start: None,
                        portals: vec![],
                        rises: course.sections[n.link as usize]
                            .samples
                            .iter()
                            .take(n.length as usize)
                            .map(|s| s.rise)
                            .collect(),
                    },
                );
            }
        }
        for (i, p) in route.points.iter().enumerate() {
            let path = paths
                .get_mut(&p.node)
                .ok_or(Error::Invalid("selected source path"))?;
            if path.selected_start.is_none() {
                path.selected_start = Some((i as i64 - i64::from(start_sample)) * 256);
            }
            path.branch = p.branch;
        }
        fn roads(
            course: &rr_course::Course,
            at: u32,
            seen: &mut BTreeSet<u32>,
            out: &mut Vec<u32>,
        ) -> Result<()> {
            if !seen.insert(at) {
                return Err(Error::Invalid("source recovery topology cycle"));
            }
            let n = course
                .nodes
                .get(&at)
                .ok_or(Error::Invalid("source path link"))?;
            match n.kind {
                NodeKind::Road => out.push(at),
                NodeKind::Split => {
                    roads(course, n.next, &mut seen.clone(), out)?;
                    roads(course, n.previous, &mut seen.clone(), out)?;
                }
                NodeKind::Merge => roads(course, n.link, seen, out)?,
                NodeKind::End => {}
                NodeKind::Start => return Err(Error::Invalid("forward recovery link to start")),
            }
            Ok(())
        }
        for (id, n) in &course.nodes {
            if n.kind != NodeKind::Road {
                continue;
            }
            let mut targets = vec![];
            roads(course, n.next, &mut BTreeSet::new(), &mut targets)?;
            targets.sort();
            targets.dedup();
            for to in targets {
                paths.get_mut(id).unwrap().portals.push(Portal {
                    end: true,
                    target: to,
                    target_end: false,
                });
                paths
                    .get_mut(&to)
                    .ok_or(Error::Invalid("road target"))?
                    .portals
                    .push(Portal {
                        end: false,
                        target: *id,
                        target_end: true,
                    });
            }
        }
        let t = Self { paths };
        t.validate()?;
        Ok(t)
    }
}
