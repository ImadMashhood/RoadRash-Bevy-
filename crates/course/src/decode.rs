use crate::*;
use rr_formats::{
    binary::{range, u16le, u32le},
    resource::Directory,
};
use sha2::{Digest, Sha256};
struct Reader<'a> {
    b: &'a [u8],
    base: u32,
}
impl<'a> Reader<'a> {
    fn fail(&self, at: usize, s: &str) -> Error {
        invalid(format!(
            "course offset 0x{at:x}, file 0x{:x}: {s}",
            u64::from(self.base) + at as u64
        ))
    }
    fn bytes(&self, p: usize, n: usize) -> Result<&'a [u8]> {
        range(self.b, p, n).map_err(|e| self.fail(p, &e.to_string()))
    }
    fn word(&self, p: usize) -> Result<u32> {
        u32le(self.b, p).map_err(|e| self.fail(p, &e.to_string()))
    }
    fn short(&self, p: usize) -> Result<i16> {
        Ok(u16le(self.b, p).map_err(|e| self.fail(p, &e.to_string()))? as i16)
    }
    fn chunk(&self, p: usize, tag: &[u8; 4], header: usize, stride: usize) -> Result<usize> {
        if self.bytes(p, 4)? != tag {
            return Err(self.fail(p, "unexpected chunk tag"));
        }
        let extent = self.word(p + 4)? as usize;
        let n = self.word(p + 8)? as usize;
        if extent < header || n > MAX_SAMPLES || n > (extent - header) / stride {
            return Err(self.fail(p, "chunk count/extent limit"));
        }
        self.bytes(p, extent)?;
        Ok(n)
    }
    fn offset(&self, p: usize) -> u32 {
        self.base + p as u32
    }
}
fn intervals(r: &Reader<'_>, p: usize, sides: bool) -> Result<Vec<Interval>> {
    if p == 0 {
        return Ok(vec![]);
    }
    let stride = if sides { 36 } else { 20 };
    let n = r.chunk(p, if sides { b"DLSR" } else { b"NALR" }, 16, stride)?;
    if n == 0 {
        return Err(r.fail(p, "empty interval table"));
    }
    let fields: Vec<(usize, usize, usize)> = if sides {
        vec![(8, 10, 20), (9, 11, 24), (12, 14, 28), (13, 15, 32)]
    } else {
        vec![(8, 10, 12), (9, 11, 16)]
    };
    let mut result = vec![];
    for i in 0..n {
        let q = p + 16 + i * stride;
        let start = r.word(q)?;
        let end = r.word(q + 4)?;
        if start > end {
            return Err(r.fail(q, "inverted interval"));
        }
        result.push(Interval {
            source_offset: r.offset(q),
            start,
            end,
            first: fields.iter().map(|f| r.b[q + f.0]).collect(),
            last: fields.iter().map(|f| r.b[q + f.1]).collect(),
            slope_q16: fields
                .iter()
                .map(|f| r.word(q + f.2).map(|v| v as i32))
                .collect::<Result<_>>()?,
            flags: if sides {
                [r.b[q + 16], r.b[q + 17]]
            } else {
                [0; 2]
            },
            raw: r.bytes(q, stride)?.to_vec(),
        });
    }
    Ok(result)
}
/// Source interval semantics: no slope on sample zero, exact end bytes, next
/// record entered only at its start, without replacing carried values that tick.
pub fn sample_intervals(records: &[Interval], count: usize) -> Result<Vec<(Vec<i64>, [u8; 2])>> {
    if records.is_empty() || count > MAX_SAMPLES {
        return Err(invalid("interval count/empty table"));
    }
    let fields = records[0].first.len();
    if records.iter().any(|r| {
        r.start > r.end
            || r.first.len() != fields
            || r.last.len() != fields
            || r.slope_q16.len() != fields
    }) {
        return Err(invalid("invalid interval fields"));
    }
    let mut k = 0;
    let mut value: Vec<i64> = records[0]
        .first
        .iter()
        .map(|v| i64::from(*v) << 16)
        .collect();
    let mut out = Vec::with_capacity(count);
    for j in 0..count {
        let rec = &records[k];
        if j > 0 {
            if (j as u64) < u64::from(rec.end) {
                for (v, d) in value.iter_mut().zip(&rec.slope_q16) {
                    *v += i64::from(*d)
                }
            } else if j as u64 == u64::from(rec.end) {
                for (v, e) in value.iter_mut().zip(&rec.last) {
                    *v = i64::from(*e) << 16
                }
            } else if k + 1 < records.len() && u64::from(records[k + 1].start) == j as u64 {
                k += 1
            }
        }
        if value.iter().any(|v| !(0..=0xffffff).contains(v)) {
            return Err(invalid(format!(
                "interval value outside byte range at sample {j}"
            )));
        }
        out.push((value.clone(), records[k].flags));
    }
    Ok(out)
}
fn terrain(r: &Reader<'_>, p: usize, count: usize) -> Result<Terrain> {
    let size = r.word(p + 4)? as usize;
    if size < 16 {
        return Err(r.fail(p, "short terrain"));
    }
    let bytes = r.bytes(p, size)?;
    let tag = std::str::from_utf8(&bytes[..4])
        .map_err(|_| r.fail(p, "terrain tag"))?
        .to_string();
    let t = Reader {
        b: bytes,
        base: r.offset(p),
    };
    let mut out = Terrain {
        tag: tag.clone(),
        source_offset: r.offset(p),
        raw: bytes.to_vec(),
        selectors: vec![],
        endpoint_selectors: None,
        wall_records: [vec![], vec![]],
        hills: vec![],
    };
    let mut selectors = 16;
    match tag.as_str() {
        "DLBR" => {}
        "NTMR" | "DWDR" => {
            t.bytes(0, 24)?;
            selectors = 24;
            for side in 0..2 {
                let n = t.word(12 + side * 4)? as usize;
                if n == 0 || n > MAX_NODES {
                    return Err(t.fail(12 + side * 4, "wall record limit"));
                }
                for _ in 0..n {
                    let q = selectors;
                    t.bytes(q, 16)?;
                    out.wall_records[side].push(Wall {
                        start: t.word(q)?,
                        parameters: t.bytes(q + 4, 8)?.try_into().unwrap(),
                        base_height: t.word(q + 12)? as i32,
                    });
                    selectors += 16
                }
            }
        }
        "LIHR" => {
            let n = t.chunk(0, b"LIHR", 20, 76)?;
            if n == 0 {
                return Err(t.fail(0, "empty hills"));
            }
            for i in 0..n {
                let q = 20 + i * 76;
                let mut points = [[0; 8]; 8];
                for (j, a) in points.iter_mut().enumerate() {
                    *a = t.bytes(q + 12 + j * 8, 8)?.try_into().unwrap();
                    if a[0] > a[1] || (a[3] as i8) > (a[4] as i8) || a[6] == 0 || a[6] > a[7] {
                        return Err(t.fail(q, "invalid hill bounds"));
                    }
                }
                out.hills.push(Hill {
                    start: t.word(q)? as i32,
                    end: t.word(q + 4)? as i32,
                    selectors: t.bytes(q + 8, 2)?.try_into().unwrap(),
                    points,
                });
            }
            return Ok(out);
        }
        _ => return Err(t.fail(0, "unsupported terrain tag")),
    }
    if t.word(8)? as usize != count {
        return Err(t.fail(8, "terrain/path count mismatch"));
    }
    for i in 0..count {
        out.selectors
            .push(t.bytes(selectors + i * 2, 2)?.try_into().unwrap())
    }
    out.endpoint_selectors = r
        .bytes(p + selectors + count * 2, 2)
        .ok()
        .map(|b| b.try_into().unwrap());
    Ok(out)
}
fn section(r: &Reader<'_>, row: usize) -> Result<Section> {
    let mut raw = [0; 13];
    for (i, w) in raw.iter_mut().enumerate() {
        *w = r.word(row + 4 * i)?
    }
    let path = raw[1] as usize;
    let count = r.chunk(path, b"HTPR", 16, 2)?;
    if count == 0 {
        return Err(r.fail(path, "empty path"));
    }
    let lane_intervals = intervals(r, raw[6] as usize, false)?;
    let side_intervals = intervals(r, raw[5] as usize, true)?;
    let mut samples = Vec::with_capacity(count);
    let mut elevation = i32::from(r.short(path + 12)?);
    for i in 0..count {
        let b = r.bytes(path + 16 + i * 2, 2)?;
        samples.push(Sample {
            bend: b[0] as i8,
            rise: b[1] as i8,
            elevation,
            ..Sample::road()
        });
        elevation = elevation
            .checked_add(i32::from(b[1] as i8))
            .ok_or_else(|| r.fail(path, "elevation overflow"))?;
    }
    if !lane_intervals.is_empty() {
        for (s, (q, _)) in samples
            .iter_mut()
            .zip(sample_intervals(&lane_intervals, count)?)
        {
            s.lanes_q8 = [(q[0] / 256) as i32, (q[1] / 256) as i32];
            s.width = s.lanes_q8.map(lane_world)
        }
    }
    if !side_intervals.is_empty() {
        for (s, (q, flags)) in samples
            .iter_mut()
            .zip(sample_intervals(&side_intervals, count)?)
        {
            s.shoulder = [(q[0] >> 16) as i32, (q[1] >> 16) as i32];
            s.side_height = [(q[2] >> 16) as i32, (q[3] >> 16) as i32];
            s.flags = flags
        }
    }
    let mut endpoint = Sample {
        elevation: i32::from(r.short(path + 14)?),
        ..Sample::road()
    };
    if let Ok(b) = r.bytes(path + 16 + count * 2, 2) {
        endpoint.bend = b[0] as i8;
        endpoint.rise = b[1] as i8
    }
    if let Some(a) = lane_intervals.last() {
        endpoint.lanes_q8 = [i32::from(a.last[0]) * 256, i32::from(a.last[1]) * 256];
        endpoint.width = endpoint.lanes_q8.map(lane_world)
    }
    if let Some(a) = side_intervals.last() {
        endpoint.shoulder = [i32::from(a.last[0]), i32::from(a.last[1])];
        endpoint.side_height = [i32::from(a.last[2]), i32::from(a.last[3])];
        endpoint.flags = a.flags
    }
    let mut events = vec![];
    let p = raw[9] as usize;
    if p != 0 {
        let n = r.chunk(p, b"CESR", 20, 12)?;
        for i in 0..n {
            let q = p + 20 + i * 12;
            let start = r.word(q)? as i32;
            let end = r.word(q + 4)? as i32;
            if start > end || events.last().is_some_and(|v: &Event| v.start > start) {
                return Err(r.fail(q, "invalid ordered event interval"));
            }
            events.push(Event {
                source_offset: r.offset(q),
                start,
                end,
                raw_type: r.bytes(q + 8, 1)?[0],
                parameters: r.bytes(q + 9, 3)?.try_into().unwrap(),
            })
        }
    }
    apply_events(&mut samples, &events);
    let mut family_events = vec![];
    let mut family_bias = 0;
    let p = raw[3] as usize;
    if p != 0 {
        let n = r.chunk(p, b"MSRR", 24, 12)?;
        family_bias = i32::from(r.bytes(p + 14, 1)?[0]) - 0x58;
        for i in 0..n {
            let q = p + 24 + i * 12;
            let words = [r.word(q)?, r.word(q + 4)?, r.word(q + 8)?];
            let kind = words[1] as u8;
            let slot = (words[1] >> 8) as u8;
            if !(1..=4).contains(&kind)
                || slot > 3
                || family_events
                    .last()
                    .is_some_and(|v: &FamilyEvent| v.sample > words[0] as i32)
            {
                return Err(r.fail(q, "invalid family event"));
            }
            family_events.push(FamilyEvent {
                source_offset: r.offset(q),
                sample: words[0] as i32,
                kind,
                slot,
                family: words[2] as u16,
                raw: words,
            })
        }
    }
    let mut roadside = vec![];
    let p = raw[2] as usize;
    if p != 0 {
        for i in 0..r.chunk(p, b"BORR", 20, 4)? {
            roadside.push(r.word(p + 20 + i * 4)?)
        }
    }
    let mut scenery = vec![];
    let mut scenery_following = None;
    let p = raw[7] as usize;
    if p != 0 {
        let n = r.chunk(p, b"DZHR", 16, 8)?;
        for i in 0..n {
            scenery.push([r.word(p + 16 + i * 8)?, r.word(p + 20 + i * 8)?])
        }
        scenery_following = r.word(p + 16 + n * 8).ok()
    }
    Ok(Section {
        source_offset: r.offset(row),
        raw,
        samples,
        backward_endpoint: endpoint,
        lane_intervals,
        side_intervals,
        events,
        family_events,
        family_bias,
        roadside,
        scenery,
        scenery_following,
        terrain: if raw[4] == 0 {
            None
        } else {
            Some(terrain(r, raw[4] as usize, count)?)
        },
    })
}
pub fn apply_events(samples: &mut [Sample], events: &[Event]) {
    let mut k = 0;
    for (j, s) in samples.iter_mut().enumerate() {
        if events.is_empty() {
            break;
        }
        if j > 0 && k + 1 < events.len() && i64::from(events[k + 1].start) <= j as i64 {
            k += 1
        }
        let e = &events[k];
        s.event_parameters = e.parameters;
        if (j as i64) < i64::from(e.start) || j as i64 > i64::from(e.end) {
            continue;
        }
        if j > 0 {
            s.event_type = e.raw_type.wrapping_add(1)
        }
        let end = i64::from(e.end);
        let start = i64::from(e.start);
        let j = j as i64;
        s.zone_end = j == end && !matches!(s.event_type, 4 | 5 | 7);
        s.marking_flags = match s.event_type {
            4 => 0x40,
            5 | 7 => 0,
            _ if j == end => 0,
            _ if j == start => 8,
            _ if end - j == 1 => 16,
            1 => {
                let mid = end - 2 * j + start;
                if mid == 0 || mid == 1 {
                    0x22
                } else {
                    0x20 + u8::from(end - j < j - start + 1)
                }
            }
            _ => 0x21,
        };
    }
}
pub fn read(bytes: &[u8]) -> Result<Course> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid("course file limit"));
    }
    let directory = Directory::read(bytes).map_err(|e| invalid(e.to_string()))?;
    let resource = |tag: &str| -> Result<Reader<'_>> {
        let mut all = directory
            .entries
            .iter()
            .filter(|r| r.tag == tag && r.id == 1);
        let a = all
            .next()
            .ok_or_else(|| invalid(format!("missing {tag} resource 1")))?;
        if all.next().is_some() {
            return Err(invalid(format!("duplicate {tag} resource 1")));
        }
        Ok(Reader {
            b: a.payload(bytes).map_err(|e| invalid(e.to_string()))?,
            base: a.offset,
        })
    };
    let s = resource("SGS ")?;
    let n = s.chunk(0, b"SGSR", 16, 52)?;
    if n == 0 || n > MAX_NODES {
        return Err(invalid("section count limit"));
    }
    let mut sections = vec![];
    let mut total = 0;
    for i in 0..n {
        let v = section(&s, 16 + i * 52)?;
        total += v.samples.len();
        if total > MAX_SAMPLES {
            return Err(invalid("aggregate course sample limit"));
        }
        sections.push(v)
    }
    let g = resource("NOD ")?;
    if g.bytes(0, 4)? != b"DONR" || g.word(4)? as usize != g.b.len() {
        return Err(g.fail(0, "node magic/extent"));
    }
    let count = g.word(8)? as usize;
    if count == 0 || count > MAX_NODES {
        return Err(g.fail(8, "node count limit"));
    }
    let root = g.word(16)?;
    let mut nodes = BTreeMap::new();
    let mut pending = vec![root];
    while let Some(at) = pending.pop() {
        if nodes.contains_key(&at) {
            continue;
        }
        let p = at as usize;
        if at < 20 || !at.is_multiple_of(4) {
            return Err(g.fail(p, "invalid node alignment"));
        }
        let k = g.word(p)?;
        let kind = match k {
            0 => NodeKind::Road,
            1 => NodeKind::Split,
            2 => NodeKind::Merge,
            3 => NodeKind::Start,
            4 => NodeKind::End,
            _ => return Err(g.fail(p, "unknown node kind")),
        };
        let words = if k < 3 { 8 } else { 2 };
        let mut raw = vec![];
        for i in 0..words {
            raw.push(g.word(p + i * 4)?)
        }
        let mut node = Node {
            offset: at,
            file_offset: g.offset(p),
            kind,
            next: 0,
            previous: 0,
            link: 0,
            length: 0,
            primary: 0,
            raw,
        };
        if k < 3 {
            node.next = node.raw[2];
            node.previous = node.raw[3];
            node.link = node.raw[4];
            node.length = node.raw[5];
            node.primary = (node.raw[6] >> 8) as u8 as i8;
            pending.extend([node.next, node.previous]);
            if k > 0 {
                pending.push(node.link)
            }
        }
        nodes.insert(at, node);
        if nodes.len() > count {
            return Err(g.fail(p, "node count exceeds header"));
        }
    }
    if nodes.len() != count {
        return Err(g.fail(8, "unreachable nodes or incorrect header count"));
    }
    // Node extents must not alias other records, even when links are aligned.
    let mut end = 20;
    for n in nodes.values() {
        if n.offset < end {
            return Err(g.fail(n.offset as usize, "overlapping node records"));
        }
        end = n.offset + n.raw.len() as u32 * 4
    }
    let course = Course {
        root,
        sections,
        nodes,
        source_sha256: format!("{:x}", Sha256::digest(bytes)),
    };
    course.validate()?;
    Ok(course)
}
