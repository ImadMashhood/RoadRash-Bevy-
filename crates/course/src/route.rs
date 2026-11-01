use crate::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TransitionKind {
    Split,
    Merge,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Transition {
    pub index: usize,
    pub node: u32,
    pub kind: TransitionKind,
    pub half: u8,
    pub other_node: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoutePoint {
    pub node: u32,
    pub section: u32,
    pub local: u32,
    pub branch: u32,
    pub path: Sample,
    pub offset: i32,
    /// Continuous integral of signed rise along this selected path.
    pub route_elevation: i64,
    pub dual: Option<Dual>,
    pub event: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Dual {
    pub half: u8,
    pub node: u32,
    pub section: u32,
    pub local: u32,
    pub path: Sample,
    pub offset: i32,
    pub separation: i32,
    pub separation_delta: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Route {
    pub choices: Vec<u8>,
    pub points: Vec<RoutePoint>,
    pub transitions: Vec<Transition>,
    pub terminal: u32,
}
fn narrow(v: i64) -> i32 {
    v as i32
}
fn shift(v: i32) -> i32 {
    lane_world(v)
}
impl Course {
    pub fn route(&self, choices: &[u8]) -> Result<Route> {
        self.validate()?;
        let splits = self.splits();
        if choices.len() != splits.len() || choices.iter().any(|v| *v > 1) {
            return Err(invalid(
                "one binary choice required per split (sorted node offsets)",
            ));
        }
        let decisions: BTreeMap<_, _> = splits.into_iter().zip(choices.iter().copied()).collect();
        let mut points = vec![];
        let mut transitions = vec![];
        let mut at = self.root;
        let mut previous = 0;
        let mut branch = 0;
        let mut visited = std::collections::BTreeSet::new();
        loop {
            if !visited.insert(at) {
                return Err(invalid(format!("route cycle at node 0x{at:x}")));
            }
            let n = &self.nodes[&at];
            match n.kind {
                NodeKind::Road => {
                    if points.len() + n.length as usize > MAX_SAMPLES {
                        return Err(invalid("route sample limit"));
                    }
                    for local in 0..n.length {
                        points.push(RoutePoint {
                            node: at,
                            section: n.link,
                            local,
                            branch,
                            path: self.sections[n.link as usize].samples[local as usize].clone(),
                            offset: 0,
                            route_elevation: 0,
                            dual: None,
                            event: 0,
                        })
                    }
                    previous = at;
                    at = n.next
                }
                NodeKind::Split => {
                    let half = decisions[&at];
                    let target = if half == 0 { n.next } else { n.previous };
                    transitions.push(Transition {
                        index: points.len(),
                        node: at,
                        kind: TransitionKind::Split,
                        half,
                        other_node: if half == 0 { n.previous } else { n.next },
                    });
                    branch = at * 2 + u32::from(half) + 1;
                    at = target
                }
                NodeKind::Merge => {
                    let half = if previous == n.next {
                        0
                    } else if previous == n.previous {
                        1
                    } else {
                        return Err(invalid(format!(
                            "merge 0x{at:x} entered from unrelated node 0x{previous:x}"
                        )));
                    };
                    transitions.push(Transition {
                        index: points.len(),
                        node: at,
                        kind: TransitionKind::Merge,
                        half,
                        other_node: if half == 0 { n.previous } else { n.next },
                    });
                    branch = 0;
                    at = n.link
                }
                NodeKind::End => break,
                NodeKind::Start => return Err(invalid("forward path reached start terminal")),
            }
        }
        if points.is_empty() {
            return Err(invalid("empty route"));
        }
        let mut out = Route {
            choices: choices.to_vec(),
            points,
            transitions,
            terminal: at,
        };
        let mut height = i64::from(out.points[0].path.elevation);
        for p in &mut out.points {
            p.route_elevation = height;
            height += i64::from(p.path.rise);
        }
        self.annotate(&mut out)?;
        Ok(out)
    }
    fn branch_points(&self, mut at: u32, reverse: bool) -> Result<Vec<RoutePoint>> {
        let mut chunks = vec![];
        let mut total = 0usize;
        let mut seen = std::collections::BTreeSet::new();
        loop {
            if !seen.insert(at) {
                return Err(invalid("branch cycle"));
            }
            let n = &self.nodes[&at];
            if n.kind != NodeKind::Road {
                break;
            }
            total += n.length as usize;
            if total > MAX_SAMPLES {
                return Err(invalid("alternate branch sample limit"));
            }
            let mut chunk = vec![];
            for local in 0..n.length {
                chunk.push(RoutePoint {
                    node: at,
                    section: n.link,
                    local,
                    branch: 0,
                    path: self.sections[n.link as usize].samples[local as usize].clone(),
                    offset: 0,
                    route_elevation: 0,
                    dual: None,
                    event: 0,
                })
            }
            chunks.push(chunk);
            at = if reverse { n.previous } else { n.next };
        }
        if reverse {
            chunks.reverse()
        }
        let out: Vec<_> = chunks.into_iter().flatten().collect();
        if out.is_empty() {
            return Err(invalid("empty split/merge branch"));
        }
        Ok(out)
    }
    fn annotate(&self, r: &mut Route) -> Result<()> {
        const WINDOW: usize = 81;
        let mut center = 0i32;
        let mut other_center = 0i32;
        let mut delta = 0i32;
        let mut separation = 0i32;
        let mut half = 0u8;
        let mut alternate: Vec<RoutePoint> = vec![];
        let mut cursor = 0usize;
        let mut split_active = false;
        let mut dual = false;
        let mut backward: Vec<(usize, i32, i32)> = vec![];
        let mut merge_index = None;
        for i in 0..r.points.len() {
            let transition = r.transitions.iter().find(|t| t.index == i);
            if let Some(t) = transition {
                if i == 0 {
                    return Err(invalid("split/merge requires an incoming road sample"));
                }
                let before = &r.points[i - 1].path;
                let current = &r.points[i].path;
                match t.kind {
                    TransitionKind::Split => {
                        alternate = self.branch_points(t.other_node, false)?;
                        cursor = 0;
                        half = t.half;
                        let (h0, h1) = if half == 0 {
                            (current, &alternate[0].path)
                        } else {
                            (&alternate[0].path, current)
                        };
                        let left = center.wrapping_add(shift(h0.lanes_q8[0] - before.lanes_q8[0]));
                        let right = center.wrapping_add(shift(before.lanes_q8[1] - h1.lanes_q8[1]));
                        (center, other_center) = if half == 0 {
                            (left, right)
                        } else {
                            (right, left)
                        };
                        delta = i32::from(h1.bend) - i32::from(h0.bend);
                        separation = 0;
                        dual = true;
                        split_active = true;
                        merge_index = None;
                        r.points[i].event = 2
                    }
                    TransitionKind::Merge => {
                        half = t.half;
                        center = center.wrapping_add(if half == 0 {
                            shift(current.lanes_q8[0] - before.lanes_q8[0])
                        } else {
                            shift(before.lanes_q8[1] - current.lanes_q8[1])
                        });
                        dual = false;
                        split_active = false;
                        merge_index = None;
                        r.points[i].event = 1
                    }
                }
            } else if dual {
                if split_active
                    && (r.points[i].node != r.points[i - 1].node
                        || r.points[i].local > WINDOW as u32)
                {
                    dual = false;
                    split_active = false;
                    half = 0
                } else {
                    cursor = (cursor + 1).min(alternate.len() - 1);
                    let (a, b) = if half == 0 {
                        (&r.points[i].path, &alternate[cursor].path)
                    } else {
                        (&alternate[cursor].path, &r.points[i].path)
                    };
                    separation = separation.wrapping_add(delta);
                    delta = delta.wrapping_add(i32::from(b.bend) - i32::from(a.bend));
                }
            } else {
                let node = &self.nodes[&r.points[i].node];
                if self.nodes[&node.next].kind == NodeKind::Merge
                    && node.length - r.points[i].local == WINDOW as u32
                {
                    let t = r
                        .transitions
                        .iter()
                        .find(|t| t.kind == TransitionKind::Merge && t.index > i)
                        .ok_or_else(|| invalid("missing merge window target"))?;
                    if t.index != i + WINDOW || t.index >= r.points.len() {
                        return Err(invalid("merge window/route mismatch"));
                    }
                    half = t.half;
                    alternate = self.branch_points(t.other_node, true)?;
                    backward.clear();
                    let mut differences = vec![];
                    for distance in 0..WINDOW {
                        let other = alternate.len().saturating_sub(distance + 1);
                        let own = &r.points[t.index - 1 - distance].path;
                        let alt = &alternate[other].path;
                        differences.push((
                            other,
                            if half == 0 {
                                i32::from(alt.bend) - i32::from(own.bend)
                            } else {
                                i32::from(own.bend) - i32::from(alt.bend)
                            },
                        ));
                    }
                    let mut d = -differences[0].1;
                    let mut x = 0i32;
                    for (other, bend) in differences {
                        backward.push((other, d, x));
                        d = d.wrapping_sub(bend);
                        x = x.wrapping_sub(d)
                    }
                    let own = &r.points[t.index - 1].path;
                    let alt = &alternate.last().unwrap().path;
                    let (a, b) = if half == 0 { (own, alt) } else { (alt, own) };
                    let out = &r.points[t.index].path;
                    let left = shift(a.lanes_q8[0] - out.lanes_q8[0]);
                    let right = shift(out.lanes_q8[1] - b.lanes_q8[1]);
                    other_center = center.wrapping_add(if half == 0 {
                        right - left
                    } else {
                        left - right
                    });
                    dual = true;
                    split_active = false;
                    merge_index = Some(t.index);
                    r.points[i].event = 6;
                }
            }
            if dual && let Some(end) = merge_index {
                (cursor, delta, separation) = backward[end - 1 - i]
            }
            r.points[i].offset = center;
            if dual {
                let p = &alternate[cursor];
                r.points[i].dual = Some(Dual {
                    half,
                    node: p.node,
                    section: p.section,
                    local: p.local,
                    path: p.path.clone(),
                    offset: other_center,
                    separation,
                    separation_delta: delta,
                })
            }
        }
        Ok(())
    }
}
impl Route {
    pub fn finish(&self, c: &Course, level: u8) -> Result<usize> {
        if !(1..=5).contains(&level) {
            return Err(invalid("level must be 1..5"));
        }
        self.points
            .iter()
            .position(|p| {
                c.sections[p.section as usize].events.iter().any(|e| {
                    e.raw_type == 6
                        && e.parameters[0] == level
                        && i64::from(e.start) == i64::from(p.local)
                })
            })
            .ok_or_else(|| invalid(format!("no level-{level} finish on selected route")))
    }
    pub fn len_q8(&self) -> i64 {
        self.points.len() as i64 * 256
    }
    /// Section-local dual geometry at the front/back slice; source-unit offsets.
    pub fn edges(&self, index: usize) -> Result<([i64; 2], Option<[i64; 2]>)> {
        let p = self
            .points
            .get(index)
            .ok_or_else(|| invalid("route sample outside sampled path"))?;
        let own = [
            i64::from(p.offset) - i64::from(p.path.width[0]),
            i64::from(p.offset) + i64::from(p.path.width[1]),
        ];
        let other = p.dual.as_ref().map(|d| {
            let shift = if d.half == 0 {
                d.separation
            } else {
                d.separation.wrapping_neg()
            };
            let x = i64::from(d.offset) + i64::from(shift);
            [
                x - i64::from(d.path.width[0]),
                x + i64::from(d.path.width[1]),
            ]
        });
        Ok((own, other))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Cursor {
    pub position_q8: i64,
}
impl Cursor {
    pub fn new(r: &Route, position_q8: i64) -> Result<Self> {
        if !(0..=r.len_q8()).contains(&position_q8) {
            return Err(invalid("cursor outside sampled path"));
        }
        Ok(Self { position_q8 })
    }
    pub fn advance(&mut self, r: &Route, delta_q8: i64) -> Result<()> {
        let next = self
            .position_q8
            .checked_add(delta_q8)
            .ok_or_else(|| invalid("cursor position overflow"))?;
        if !(0..=r.len_q8()).contains(&next) {
            return Err(invalid("cursor step outside sampled path"));
        }
        self.position_q8 = next;
        Ok(())
    }
    pub fn point<'a>(&self, r: &'a Route) -> Result<&'a RoutePoint> {
        r.points
            .get((self.position_q8 / 256) as usize)
            .ok_or_else(|| invalid("cursor at terminal endpoint; use endpoint()"))
    }
    pub fn endpoint<'a>(&self, c: &'a Course, r: &Route) -> Result<&'a Sample> {
        if self.position_q8 != r.len_q8() {
            return Err(invalid("cursor is not at terminal endpoint"));
        }
        let last = r.points.last().ok_or_else(|| invalid("empty route"))?;
        Ok(&c.sections[last.section as usize].backward_endpoint)
    }
    pub fn interpolated_elevation(&self, c: &Course, r: &Route) -> Result<i32> {
        if self.position_q8 == r.len_q8() {
            return Ok(self.endpoint(c, r)?.elevation);
        }
        let p = self.point(r)?;
        Ok(narrow(
            i64::from(p.path.elevation) + i64::from(p.path.rise) * (self.position_q8 % 256) / 256,
        ))
    }
}

/// Continuous inspection geometry. Raw source headers and discrete flags remain
/// on Sample; blending is a declared presentation policy, not recovered physics.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Geometry {
    pub center: f64,
    pub elevation: f64,
    pub width: [f64; 2],
    pub shoulder: [f64; 2],
    pub side_height: [f64; 2],
}
impl Cursor {
    pub fn geometry(&self, r: &Route) -> Result<Geometry> {
        if self.position_q8 < 0 || self.position_q8 > r.len_q8() {
            return Err(invalid("geometry cursor outside sampled path"));
        }
        let i = (self.position_q8 / 256) as usize;
        let last = r.points.last().ok_or_else(|| invalid("empty route"))?;
        if i == r.points.len() {
            return Ok(Geometry {
                center: f64::from(last.offset),
                elevation: (last.route_elevation + i64::from(last.path.rise)) as f64,
                width: last.path.width.map(f64::from),
                shoulder: last.path.shoulder.map(f64::from),
                side_height: last.path.side_height.map(f64::from),
            });
        }
        let a = &r.points[i];
        let b = r.points.get(i + 1).unwrap_or(a);
        let t = (self.position_q8 % 256) as f64 / 256.;
        let lerp = |a: i32, b: i32| f64::from(a) + (f64::from(b) - f64::from(a)) * t;
        Ok(Geometry {
            center: lerp(a.offset, b.offset),
            elevation: a.route_elevation as f64 + f64::from(a.path.rise) * t,
            width: std::array::from_fn(|j| lerp(a.path.width[j], b.path.width[j])),
            shoulder: std::array::from_fn(|j| lerp(a.path.shoulder[j], b.path.shoulder[j])),
            side_height: std::array::from_fn(|j| {
                lerp(a.path.side_height[j], b.path.side_height[j])
            }),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldInstance {
    pub id: String,
    pub route_sample: usize,
    pub branch: u32,
    pub section: u32,
    pub local: u32,
    pub lateral_world: i64,
    pub route_elevation: i64,
}
impl Route {
    /// Resolve source-local instances once. Camera movement only selects visibility;
    /// it never regenerates IDs, random placement or world positions.
    pub fn instances(&self, placed: &crate::scenery::Placements) -> Vec<WorldInstance> {
        let mut locations = BTreeMap::<(u32, u32), Vec<usize>>::new();
        for (i, p) in self.points.iter().enumerate() {
            locations.entry((p.section, p.local)).or_default().push(i)
        }
        let mut out = vec![];
        for item in &placed.instances {
            let Some(lateral) = item.lateral else {
                continue;
            };
            if let Some(indices) = locations.get(&(item.section, item.sample)) {
                for &i in indices {
                    let p = &self.points[i];
                    out.push(WorldInstance {
                        id: item.id.clone(),
                        route_sample: i,
                        branch: p.branch,
                        section: p.section,
                        local: p.local,
                        lateral_world: i64::from(p.offset) + i64::from(lateral),
                        route_elevation: p.route_elevation,
                    });
                }
            }
        }
        out.sort_by(|a, b| a.route_sample.cmp(&b.route_sample).then(a.id.cmp(&b.id)));
        out
    }
}

impl Cursor {
    /// Direct backwards initializer at any node end, kept separate from the
    /// forward cache's half-open sample convention and continuous inspection view.
    pub fn section_endpoint<'a>(&self, c: &'a Course, r: &Route) -> Result<&'a Sample> {
        if self.position_q8 <= 0 || self.position_q8 > r.len_q8() || self.position_q8 % 256 != 0 {
            return Err(invalid("not a section endpoint"));
        }
        let i = (self.position_q8 / 256) as usize;
        let prior = &r.points[i - 1];
        if r.points.get(i).is_some_and(|p| p.node == prior.node) {
            return Err(invalid("not a section endpoint"));
        }
        Ok(&c.sections[prior.section as usize].backward_endpoint)
    }
}
