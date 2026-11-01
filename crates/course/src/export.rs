//! Local-only route explorer and metadata coverage export. No game art is emitted.
use crate::{route::*, *};
use serde_json::{Value, json};
use std::{fs, path::Path};
pub const COURSES: [(&str, &str); 5] = [
    ("CITY", "City"),
    ("HIWAY", "Sierra Nevada"),
    ("MEDLY", "Peninsula"),
    ("NAPA", "Napa"),
    ("CANYN", "Pacific Highway"),
];
#[derive(Debug, Serialize)]
pub struct Seam {
    pub index: usize,
    pub node: u32,
    pub kind: TransitionKind,
    pub half: u8,
    pub retained_edge_error: i64,
    pub continuous_geometry_passed: bool,
    pub source_elevation_reset: i32,
    pub route_elevation_continuity_error: i64,
    pub shoulder_height_delta: [i32; 2],
    pub before: (u32, u32),
    pub at: (u32, u32),
    pub after: (u32, u32),
}
pub fn seams(route: &Route) -> Result<Vec<Seam>> {
    let mut out = vec![];
    for t in &route.transitions {
        if t.index == 0 || t.index + 1 >= route.points.len() {
            return Err(invalid("seam has no before/after sample"));
        }
        let before = &route.points[t.index - 1];
        let at = &route.points[t.index];
        let after = &route.points[t.index + 1];
        let half = match t.kind {
            TransitionKind::Split => t.half,
            TransitionKind::Merge => before.dual.as_ref().map_or(t.half, |d| d.half),
        };
        let side = usize::from(half);
        let sign = if side == 0 { -1 } else { 1 };
        let edge = |p: &RoutePoint| i64::from(p.offset) + sign * i64::from(p.path.width[side]);
        let low = Cursor::new(route, t.index as i64 * 256 - 1)?.geometry(route)?;
        let high = Cursor::new(route, t.index as i64 * 256)?.geometry(route)?;
        let close = |a: f64, b: f64| (a - b).abs() < 1e-8;
        let mut smooth = close(
            high.elevation - low.elevation,
            f64::from(before.path.rise) / 256.,
        );
        smooth &= close(
            high.center - low.center,
            f64::from(at.offset - before.offset) / 256.,
        );
        for side in 0..2 {
            smooth &= close(
                high.width[side] - low.width[side],
                f64::from(at.path.width[side] - before.path.width[side]) / 256.,
            );
            smooth &= close(
                high.shoulder[side] - low.shoulder[side],
                f64::from(at.path.shoulder[side] - before.path.shoulder[side]) / 256.,
            );
            smooth &= close(
                high.side_height[side] - low.side_height[side],
                f64::from(at.path.side_height[side] - before.path.side_height[side]) / 256.,
            );
        }
        out.push(Seam {
            continuous_geometry_passed: smooth,
            index: t.index,
            node: t.node,
            kind: t.kind,
            half,
            retained_edge_error: edge(at) - edge(before),
            route_elevation_continuity_error: at.route_elevation
                - before.route_elevation
                - i64::from(before.path.rise),
            source_elevation_reset: at.path.elevation
                - before.path.elevation
                - i32::from(before.path.rise),
            shoulder_height_delta: std::array::from_fn(|i| {
                at.path.side_height[i] - before.path.side_height[i]
            }),
            before: (before.section, before.local),
            at: (at.section, at.local),
            after: (after.section, after.local),
        });
    }
    Ok(out)
}
fn compact_point(p: &RoutePoint) -> Value {
    json!([
        p.node,
        p.section,
        p.local,
        p.branch,
        p.offset,
        p.event,
        p.dual.as_ref().map(|d| json!([
            d.half,
            d.node,
            d.section,
            d.local,
            d.offset,
            d.separation,
            d.separation_delta
        ])),
        p.route_elevation
    ])
}
fn write_json(p: &Path, v: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(v).map_err(|e| invalid(e.to_string()))?;
    fs::write(p, bytes).map_err(|e| invalid(e.to_string()))
}
pub fn inspect(data: &Path, out: &Path) -> Result<()> {
    let input = data.canonicalize().map_err(|e| invalid(e.to_string()))?;
    if out
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(invalid("course output cannot contain parent traversal"));
    }
    let absolute = std::path::absolute(out).map_err(|e| invalid(e.to_string()))?;
    let mut existing = absolute.as_path();
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| invalid("output has no existing ancestor"))?;
    }
    let output = existing
        .canonicalize()
        .map_err(|e| invalid(e.to_string()))?
        .join(
            absolute
                .strip_prefix(existing)
                .map_err(|e| invalid(e.to_string()))?,
        );
    if input.starts_with(&output) || output.starts_with(&input) {
        return Err(invalid(
            "course output must be outside immutable source tree",
        ));
    }
    fs::create_dir_all(&output).map_err(|e| invalid(e.to_string()))?;
    let exe = fs::read(input.join("ROADRASH.EXE")).map_err(|e| invalid(e.to_string()))?;
    let exe = rr_formats::pe::SupportedExe::read(&exe).map_err(|e| invalid(e.to_string()))?;
    let grid = exe
        .read_va(0x464218, 8)
        .map_err(|e| invalid(e.to_string()))?;
    let offline_start = u32::from_le_bytes(grid[4..8].try_into().unwrap());
    let expected: Value = serde_json::from_str(include_str!(
        "../../../docs/fixtures/course-expectations.json"
    ))
    .map_err(|e| invalid(e.to_string()))?;
    let mut total_exported = 0usize;
    let families = fs::read(input.join("DATA/FAMILIES.RSC")).map_err(|e| invalid(e.to_string()))?;
    let families =
        rr_formats::resource::Directory::read(&families).map_err(|e| invalid(e.to_string()))?;
    let family_ids: std::collections::BTreeSet<_> = families
        .entries
        .iter()
        .filter(|r| r.tag == "FAM ")
        .map(|r| r.id)
        .collect();
    let mut reports = vec![];
    for (stem, name) in COURSES {
        let bytes = fs::read(input.join(format!("DATA/COURSES/{stem}.CRS")))
            .map_err(|e| invalid(e.to_string()))?;
        let c = Course::read(&bytes)?;
        let split_count = c.splits().len();
        if split_count > 8 {
            return Err(invalid("offline branch enumeration limit"));
        }
        let mut routes = vec![];
        let mut route_reports = vec![];
        let mut visited = std::collections::BTreeSet::new();
        let mut max_edge_error = 0;
        let mut all_seams = vec![];
        for mask in 0..1usize << split_count {
            let choices: Vec<u8> = (0..split_count).map(|i| ((mask >> i) & 1) as u8).collect();
            let r = c.route(&choices)?;
            total_exported += r.points.len();
            if total_exported > 2_000_000 {
                return Err(invalid("offline export sample budget"));
            }
            let finishes = (1..=5)
                .map(|l| r.finish(&c, l))
                .collect::<Result<Vec<_>>>()?;
            let expectation = expected["courses"][stem]
                .as_array()
                .and_then(|a| a.iter().find(|e| e["choices"] == json!(choices)))
                .ok_or_else(|| invalid("missing pinned census expectation"))?;
            if expectation["samples"] != json!(r.points.len())
                || expectation["finishes"] != json!(finishes)
            {
                return Err(invalid(format!(
                    "{stem} branch {mask} disagrees with pinned branch census"
                )));
            }
            for p in &r.points {
                visited.insert(p.section);
            }
            if r.points.iter().any(|p| p.event == 1) {
                for pair in r.points.windows(2) {
                    if pair[1].event == 1 && pair[0].dual.as_ref().is_none_or(|d| d.separation != 0)
                    {
                        return Err(invalid(format!(
                            "{stem} branch {mask} merge separation did not close"
                        )));
                    }
                }
            }
            let seam = seams(&r)?;
            for s in &seam {
                max_edge_error = max_edge_error.max(s.retained_edge_error.abs());
                if s.retained_edge_error.abs() > 1
                    || s.route_elevation_continuity_error != 0
                    || !s.continuous_geometry_passed
                {
                    return Err(invalid(format!(
                        "{stem} branch {mask}: geometry seam failed"
                    )));
                }
            }
            // Exercise exact bounds and every seam in both directions before exporting.
            let mut cursor = Cursor::new(&r, 0)?;
            for p in &r.points {
                if cursor.point(&r)? != p {
                    return Err(invalid("forward cursor sample mismatch"));
                }
                cursor.advance(&r, 256)?;
            }
            cursor.endpoint(&c, &r)?;
            for p in r.points.iter().rev() {
                cursor.advance(&r, -256)?;
                if cursor.point(&r)? != p {
                    return Err(invalid("reverse cursor sample mismatch"));
                }
            }

            for t in &r.transitions {
                cursor = Cursor::new(&r, t.index as i64 * 256 - 1)?;
                let before = cursor.clone();
                cursor.advance(&r, 2)?;
                cursor.advance(&r, -2)?;
                if cursor != before {
                    return Err(invalid("seam cursor round-trip mismatch"));
                }
            }
            cursor.advance(&r, r.len_q8() - cursor.position_q8)?;
            cursor.endpoint(&c, &r)?;
            cursor.advance(&r, -r.len_q8())?;
            route_reports.push(json!({"branch_mask":mask,"choices":choices,"samples":r.points.len(),"finish_samples_by_level":finishes,"terminal_node":r.terminal,"seams":seam,"visited_sections":r.points.iter().map(|p|p.section).collect::<std::collections::BTreeSet<_>>(),"cursor_forward_reverse":"passed at every whole sample and seam fractions","pinned_census":"matched"}));
            all_seams.push(seams(&r)?);
            routes.push(json!({"choices":choices,"points":r.points.iter().map(compact_point).collect::<Vec<_>>(),"finishes":finishes,"transitions":r.transitions}));
        }
        let placed = (1..=5)
            .map(|l| crate::scenery::place(&c, l, 1))
            .collect::<Result<Vec<_>>>()?;
        let unresolved_by_level: Vec<_> = placed
            .iter()
            .map(|p| {
                p.instances
                    .iter()
                    .filter(|i| {
                        i.family
                            .is_some_and(|f| !family_ids.contains(&u32::from(f)))
                    })
                    .count()
            })
            .collect();
        let missing_families: std::collections::BTreeSet<_> = placed
            .iter()
            .flat_map(|p| {
                p.instances
                    .iter()
                    .filter(|i| {
                        i.family
                            .is_some_and(|f| !family_ids.contains(&u32::from(f)))
                    })
                    .filter_map(|i| i.family)
            })
            .collect();
        let source_record_counts = json!({"events":c.sections.iter().map(|s|s.events.len()).sum::<usize>(),"family_requests":c.sections.iter().map(|s|s.family_events.len()).sum::<usize>(),"roadside_profiles":c.sections.iter().map(|s|s.roadside.len()).sum::<usize>(),"scenery_records":c.sections.iter().map(|s|s.scenery.len()).sum::<usize>()});
        let terrain_counts = c.sections.iter().filter_map(|s| s.terrain.as_ref()).fold(
            BTreeMap::new(),
            |mut a, t| {
                *a.entry(t.tag.clone()).or_insert(0usize) += 1;
                a
            },
        );
        let unsupported = vec![
            "Random hill/wall geometry is retained as typed source parameters; full race RNG terrain integration belongs to road rendering/gameplay.",
            "Family selectors/IDs are reconstructed; billboard artwork and family animation rendering are later presentation work.",
            "Offline scenery uses an isolated seed-1 stream; full original terrain/scenery/race random-call interleaving is not claimed.",
        ];
        let report = json!({"course":stem,"name":name,"sha256":c.source_sha256,"sections":c.sections.len(),"nodes":c.nodes.len(),"splits":split_count,"branch_routes":routes.len(),"source_samples":c.sections.iter().map(|s|s.samples.len()).sum::<usize>(),"visited_sections":visited,"unvisited_sections":(0..c.sections.len()as u32).filter(|s|!visited.contains(s)).collect::<Vec<_>>(),"source_record_counts":source_record_counts,"terrain_counts":terrain_counts,"unresolved_family_instances_by_level":unresolved_by_level,"missing_family_ids":missing_families,"placements_by_level":placed.iter().map(|p|p.instances.len()).collect::<Vec<_>>(),"suppressed_placements_by_level":placed.iter().map(|p|p.suppressed).collect::<Vec<_>>(),"max_retained_edge_error_world_units":max_edge_error,"routes":route_reports,"unsupported":unsupported});
        write_json(
            &output.join(format!("{stem}.json")),
            &json!({"schema_version":1,"course":stem,"name":name,"offline_start_q8":offline_start,"definition":c,"routes":routes,"placements":placed,"seams":all_seams}),
        )?;
        reports.push(report);
    }
    write_json(
        &output.join("coverage.json"),
        &json!({"schema_version":1,"outcome":"passed","course_decoder_version":"rr-course-1","reference_revision":"7aecf6044f2dd773b925b325823f30d648d61c89","seed":1,"offline_start_q8":offline_start,"courses":reports}),
    )?;
    fs::write(output.join("index.html"), include_str!("viewer.html"))
        .map_err(|e| invalid(e.to_string()))?;
    println!(
        "Validated five courses; inspector: {}",
        output.join("index.html").display()
    );
    Ok(())
}
