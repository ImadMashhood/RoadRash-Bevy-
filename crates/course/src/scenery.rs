use crate::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Instance {
    pub id: String,
    pub section: u32,
    pub sample: u32,
    pub source_record: u32,
    pub ordinal: u32,
    pub kind: String,
    pub selector: u8,
    pub family: Option<u16>,
    /// Actor requests have no fixed lateral position until gameplay spawning.
    pub lateral: Option<i32>,
    pub size: u8,
    pub mirror: bool,
    pub raw: [u32; 2],
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Placements {
    pub instances: Vec<Instance>,
    pub suppressed: usize,
    pub rng_state: u32,
}
struct Random(u32);
impl Random {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        (self.0 >> 16) & 32767
    }
}
/// Isolated deterministic reconstruction stream. Full race RNG interleaving is
/// deliberately not asserted by this offline inspector.
pub fn place(course: &Course, level: u8, seed: u32) -> Result<Placements> {
    if !(1..=5).contains(&level) {
        return Err(invalid("placement level must be 1..5"));
    }
    course.validate()?;
    if course
        .sections
        .iter()
        .map(|s| s.scenery.len())
        .sum::<usize>()
        > 4096
    {
        return Err(invalid("scenery pairwise ordering budget"));
    }
    let mut random = Random(seed);
    let mut out = Placements::default();
    for (si, s) in course.sections.iter().enumerate() {
        let mut residency = Vec::with_capacity(s.samples.len());
        let mut slots = [0u16; 4];
        let mut next = 0;
        for sample in 0..s.samples.len() {
            let at = (sample as i64 + 33 + i64::from(s.family_bias)).max(0);
            while let Some(e) = s.family_events.get(next) {
                if i64::from(e.sample) > at {
                    break;
                }
                if e.slot > 3 {
                    return Err(invalid("family slot outside 0..3"));
                }
                if e.kind == 1 || e.kind == 3 {
                    slots[e.slot as usize] = e.family
                }
                next += 1;
            }
            residency.push(slots);
        }
        let mut push = |sample: usize,
                        record: usize,
                        ordinal: usize,
                        kind: &str,
                        selector: u8,
                        lateral: Option<i32>,
                        size: u8,
                        mirror: bool,
                        raw: [u32; 2]|
         -> Result<()> {
            if out.instances.len() >= MAX_SAMPLES {
                return Err(invalid("placement count limit"));
            }
            out.instances.push(Instance {
                id: format!(
                    "rr:course/{}/section/{si}/{kind}/{record}/{ordinal}",
                    course.source_sha256
                ),
                section: si as u32,
                sample: sample as u32,
                source_record: record as u32,
                ordinal: ordinal as u32,
                kind: kind.into(),
                selector,
                family: if kind == "police" {
                    None
                } else {
                    Some(residency[sample][(selector >> 6) as usize])
                },
                lateral,
                size,
                mirror,
                raw,
            });
            Ok(())
        };
        let mut record = 0;
        let mut start = 0;
        let mut end = 0;
        for (j, path) in s.samples.iter().enumerate() {
            if s.roadside.is_empty() {
                break;
            }
            let duration = |w: u32| {
                let period = if w & 0x0f000000 != 0 {
                    (w >> 12) & 15
                } else {
                    0
                };
                (period + 1) * ((w >> 24) & 15)
            };
            if j == 0 {
                end = duration(s.roadside[0]) as usize
            } else if j > end {
                record += 1;
                if record >= s.roadside.len() {
                    break;
                }
                start = j;
                end = j + duration(s.roadside[record]) as usize
            }
            let w = s.roadside[record];
            let selector = (w >> 16) as u8;
            let period = if w & 0x0f000000 != 0 {
                ((w >> 12) & 15) as usize
            } else {
                0
            };
            if selector & 63 == 63 || !(j - start).is_multiple_of(period + 1) {
                continue;
            }
            let left =
                (w & 0x10000000 != 0) ^ (w & 0x80000000 != 0 && (j - start).is_multiple_of(2));
            let spacing = if w & 0x0f000000 != 0 {
                if w & 0x20000000 != 0 {
                    ((w & 15) + 4) * 256
                } else {
                    (w & 15) * 64
                }
            } else {
                let v = ((w & 0xf000) >> 8) + (w & 15);
                if w & 0x20000000 != 0 {
                    (v + 32) * 32
                } else {
                    v * 8
                }
            } as i32;
            let mut x = if left {
                -path.width[0] + 224 - spacing
            } else {
                path.width[1] - 224 + spacing
            };
            for ordinal in 0..((w >> 8) & 7) + 1 {
                push(
                    j,
                    record,
                    j * 8 + ordinal as usize,
                    "roadside",
                    selector,
                    Some(x),
                    ((w >> 4) & 15) as u8,
                    w & 0x40000000 != 0,
                    [w, 0],
                )?;
                let step = if w & 0x800 != 0 {
                    spacing / 2 + (random.next() % ((spacing - spacing / 2 + 1) as u32)) as i32
                } else {
                    spacing
                };
                x += if left { -step } else { step }
            }
        }
        let mut events = vec![];
        let mut base = 0u32;
        for (i, [w, detail]) in s.scenery.iter().copied().enumerate() {
            base = base
                .checked_add((w >> 24) & 127)
                .ok_or_else(|| invalid("scenery position overflow"))?;
            let sample = base + random.next() % (((w & 0xf0000) >> 15) + 1);
            let required = (detail >> 12) & 7;
            let mut enabled = required == 0 || required == u32::from(level);
            let probability = ((w >> 20) & 15) as i32
                + if w & 0x80000000 != 0 && w & 15 >= 11 {
                    i32::from(level) - 5
                } else {
                    i32::from(level) - 1
                };
            if enabled && probability < (random.next() % 10) as i32 {
                enabled = false
            }
            events.push((sample, i, w, detail, enabled));
        }
        // Physical pairwise exchange order is observable for equal positions.
        for i in 0..events.len() {
            for j in i + 1..events.len() {
                if events[j].0 < events[i].0 {
                    events.swap(i, j)
                }
            }
        }
        let mut per_sample = BTreeMap::<u32, u8>::new();
        for (sample, i, w, detail, enabled) in events {
            let selector = (w >> 8) as u8;
            if !enabled || selector & 63 == 63 || sample as usize >= s.samples.len() {
                out.suppressed += 1;
                continue;
            }
            let actor = w & 0x80000000 != 0;
            let kind = if actor {
                if w & 15 < 11 { "pedestrian" } else { "police" }
            } else {
                "billboard"
            };
            if !actor {
                let count = per_sample.entry(sample).or_default();
                if *count >= 2 {
                    out.suppressed += 1;
                    continue;
                }
                *count += 1
            }
            let displacement = (((detail & 0xfff) as i32) << 20) >> 20;
            let path = &s.samples[sample as usize];
            let lateral = (path.width[1] - path.width[0]) / 2 + displacement;
            push(
                sample as usize,
                i,
                0,
                kind,
                selector,
                if actor { None } else { Some(lateral) },
                8,
                w & 0x80 != 0,
                [w, detail],
            )?;
        }
    }
    out.rng_state = random.0;
    Ok(out)
}
