#![forbid(unsafe_code)]
//! Source-unit course reconstruction, independent of Bevy and frame timing.
pub mod decode;
pub mod export;
pub mod route;
pub mod scenery;
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn invalid(s: impl Into<String>) -> Error {
    Error(s.into())
}
pub const MAX_SAMPLES: usize = 1_000_000;
pub const MAX_NODES: usize = 4096;
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Sample {
    pub bend: i8,
    pub rise: i8,
    pub elevation: i32,
    pub lanes_q8: [i32; 2],
    pub width: [i32; 2],
    pub shoulder: [i32; 2],
    pub side_height: [i32; 2],
    pub flags: [u8; 2],
    pub event_type: u8,
    pub event_parameters: [u8; 3],
    pub marking_flags: u8,
    pub zone_end: bool,
}
impl Sample {
    pub fn road() -> Self {
        Self {
            lanes_q8: [512; 2],
            width: [500; 2],
            ..Self::default()
        }
    }
}
/// Fixed source lane conversion, truncating toward zero. No physical metre claim.
pub fn lane_world(q8: i32) -> i32 {
    (i64::from(q8) * 250 / 256) as i32
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Interval {
    pub source_offset: u32,
    pub start: u32,
    pub end: u32,
    pub first: Vec<u8>,
    pub last: Vec<u8>,
    pub slope_q16: Vec<i32>,
    pub flags: [u8; 2],
    pub raw: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Event {
    pub source_offset: u32,
    pub start: i32,
    pub end: i32,
    pub raw_type: u8,
    pub parameters: [u8; 3],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FamilyEvent {
    pub source_offset: u32,
    pub sample: i32,
    pub kind: u8,
    pub slot: u8,
    pub family: u16,
    pub raw: [u32; 3],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Terrain {
    pub tag: String,
    pub source_offset: u32,
    pub raw: Vec<u8>,
    pub selectors: Vec<[u8; 2]>,
    pub endpoint_selectors: Option<[u8; 2]>,
    pub wall_records: [Vec<Wall>; 2],
    pub hills: Vec<Hill>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Wall {
    pub start: u32,
    pub parameters: [u8; 8],
    pub base_height: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Hill {
    pub start: i32,
    pub end: i32,
    pub selectors: [u8; 2],
    pub points: [[u8; 8]; 8],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Section {
    pub source_offset: u32,
    pub raw: [u32; 13],
    pub samples: Vec<Sample>,
    pub backward_endpoint: Sample,
    pub lane_intervals: Vec<Interval>,
    pub side_intervals: Vec<Interval>,
    pub events: Vec<Event>,
    pub family_events: Vec<FamilyEvent>,
    pub family_bias: i32,
    pub roadside: Vec<u32>,
    pub scenery: Vec<[u32; 2]>,
    pub scenery_following: Option<u32>,
    pub terrain: Option<Terrain>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NodeKind {
    Road,
    Split,
    Merge,
    Start,
    End,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Node {
    pub offset: u32,
    pub file_offset: u32,
    pub kind: NodeKind,
    pub next: u32,
    pub previous: u32,
    pub link: u32,
    pub length: u32,
    pub primary: i8,
    pub raw: Vec<u32>,
}
impl Node {
    pub fn forward(&self) -> Vec<u32> {
        match self.kind {
            NodeKind::Road => vec![self.next],
            NodeKind::Split => vec![self.next, self.previous],
            NodeKind::Merge => vec![self.link],
            _ => vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Course {
    pub root: u32,
    pub sections: Vec<Section>,
    pub nodes: BTreeMap<u32, Node>,
    pub source_sha256: String,
}
impl Course {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        decode::read(bytes)
    }
    pub fn splits(&self) -> Vec<u32> {
        self.nodes
            .values()
            .filter(|n| n.kind == NodeKind::Split)
            .map(|n| n.offset)
            .collect()
    }
    pub fn validate(&self) -> Result<()> {
        if self.nodes.len() > MAX_NODES
            || self.sections.len() > MAX_NODES
            || self.sections.is_empty()
            || self.sections.iter().map(|s| s.samples.len()).sum::<usize>() > MAX_SAMPLES
        {
            return Err(invalid("course graph/sample budget"));
        }
        for (key, node) in &self.nodes {
            if *key != node.offset {
                return Err(invalid("node key/offset mismatch"));
            }
        }

        if !self.nodes.contains_key(&self.root) {
            return Err(invalid(format!("missing root node 0x{:x}", self.root)));
        }
        for n in self.nodes.values() {
            let at = || format!("node 0x{:x} (file 0x{:x})", n.offset, n.file_offset);
            if matches!(n.kind, NodeKind::Road | NodeKind::Split | NodeKind::Merge) {
                for target in [n.next, n.previous]
                    .into_iter()
                    .chain((n.kind != NodeKind::Road).then_some(n.link))
                {
                    if !self.nodes.contains_key(&target) {
                        return Err(invalid(format!("{} dangling link 0x{target:x}", at())));
                    }
                }
            }
            if n.kind == NodeKind::Road
                && (n.length == 0
                    || self
                        .sections
                        .get(n.link as usize)
                        .is_none_or(|s| n.length as usize > s.samples.len()))
            {
                return Err(invalid(format!(
                    "{} invalid section {} / length {}",
                    at(),
                    n.link,
                    n.length
                )));
            }
        }
        // Bidirectional references are intentional. Only forward travel must be acyclic.
        let mut colors = BTreeMap::<u32, u8>::new();
        let mut stack = vec![(self.root, false)];
        let mut end = false;
        while let Some((id, exit)) = stack.pop() {
            if exit {
                colors.insert(id, 2);
                continue;
            }
            match colors.get(&id) {
                Some(1) => return Err(invalid(format!("forward cycle at node 0x{id:x}"))),
                Some(2) => continue,
                _ => {}
            }
            colors.insert(id, 1);
            stack.push((id, true));
            let n = &self.nodes[&id];
            if n.kind == NodeKind::End {
                end = true
            }
            if n.kind == NodeKind::Start {
                return Err(invalid(format!(
                    "forward route reaches backward terminal 0x{id:x}"
                )));
            }
            for next in n.forward().into_iter().rev() {
                stack.push((next, false))
            }
        }
        if !end {
            return Err(invalid("no reachable forward terminal"));
        }
        for n in self.nodes.values() {
            if n.kind != NodeKind::Start && !colors.contains_key(&n.offset) {
                return Err(invalid(format!(
                    "unreachable forward node 0x{:x}",
                    n.offset
                )));
            }
        }
        Ok(())
    }
}
