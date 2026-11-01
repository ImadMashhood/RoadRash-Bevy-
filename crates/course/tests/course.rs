//! Entirely authored binary fixture, not bytes extracted from original media.
use rr_course::{decode::sample_intervals, export::seams, route::*, *};
fn word(b: &mut [u8], p: usize, v: u32) {
    b[p..p + 4].copy_from_slice(&v.to_le_bytes())
}
fn chunk(b: &mut Vec<u8>, tag: &[u8; 4], header: usize, stride: usize, n: usize) -> usize {
    let p = b.len();
    b.resize(p + header + stride * n, 0);
    b[p..p + 4].copy_from_slice(tag);
    word(b, p + 4, (header + stride * n) as u32);
    word(b, p + 8, n as u32);
    p
}
fn fixture() -> Vec<u8> {
    let mut s = vec![];
    chunk(&mut s, b"SGSR", 16, 52, 4);
    for (section, count) in [4, 200, 200, 10].into_iter().enumerate() {
        let row = 16 + section * 52;
        let p = chunk(&mut s, b"HTPR", 16, 2, count);
        word(&mut s, row + 4, p as u32);
        s[p + 12..p + 14]
            .copy_from_slice(&(if section == 0 { 100i16 } else { 999i16 }).to_le_bytes());
        s[p + 14..p + 16].copy_from_slice(&321i16.to_le_bytes());
        for j in 0..count {
            s[p + 16 + 2 * j] = if section == 2 { 1 } else { 0 };
            s[p + 17 + 2 * j] = if j % 2 == 0 { 2 } else { 254 };
        }
        let lanes = chunk(&mut s, b"NALR", 16, 20, 1);
        word(&mut s, row + 24, lanes as u32);
        word(&mut s, lanes + 20, count as u32);
        let widths = match section {
            1 => [2, 1],
            2 => [1, 3],
            _ => [2, 3],
        };
        s[lanes + 24..lanes + 28].copy_from_slice(&[widths[0], widths[1], widths[0], widths[1]]);
        let sides = chunk(&mut s, b"DLSR", 16, 36, 1);
        word(&mut s, row + 20, sides as u32);
        word(&mut s, sides + 20, count as u32);
        s[sides + 24..sides + 32].copy_from_slice(&[10, 20, 10, 20, 0, 7, 0, 7]);
        s[sides + 32] = 0x80;
        s[sides + 33] = 0x13;
        let terrain = chunk(&mut s, b"DLBR", 16, 2, count);
        word(&mut s, row + 16, terrain as u32);
        s[terrain + 16..terrain + 16 + count * 2].fill(255);
        if section == 3 {
            let e = chunk(&mut s, b"CESR", 20, 12, 5);
            word(&mut s, row + 36, e as u32);
            for k in 0..5 {
                word(&mut s, e + 20 + k * 12, (k + 1) as u32);
                word(&mut s, e + 24 + k * 12, (k + 1) as u32);
                s[e + 28 + k * 12] = 6;
                s[e + 29 + k * 12] = (k + 1) as u8;
            }
        }
    }
    let size = s.len();
    word(&mut s, 4, size as u32);
    let mut g = vec![0; 228];
    g[..4].copy_from_slice(b"DONR");
    word(&mut g, 4, 228);
    word(&mut g, 8, 8);
    word(&mut g, 16, 20);
    let rows = [
        (20, 0, 60, 52, 0, 4),
        (52, 3, 0, 0, 0, 0),
        (60, 1, 92, 124, 20, 15360),
        (92, 0, 156, 60, 1, 200),
        (124, 0, 156, 60, 2, 200),
        (156, 2, 92, 124, 188, 15360),
        (188, 0, 220, 156, 3, 10),
        (220, 4, 0, 0, 0, 0),
    ];
    for (at, k, next, prev, link, len) in rows {
        word(&mut g, at, k);
        if k < 3 {
            word(&mut g, at + 8, next);
            word(&mut g, at + 12, prev);
            word(&mut g, at + 16, link);
            word(&mut g, at + 20, len);
        }
    }
    let table = 24usize;
    let start = 24 + 16 + 64;
    let mut b = vec![0; start];
    b[..4].copy_from_slice(b"CRSR");
    b[12..16].copy_from_slice(b"LBTR");
    word(&mut b, 16, table as u32);
    b[table..table + 4].copy_from_slice(b"LBTR");
    word(&mut b, table + 8, 2);
    let gs = g.len();
    for (i, (tag, offset, len)) in [(b"NOD ", start, gs), (b"SGS ", start + gs, s.len())]
        .into_iter()
        .enumerate()
    {
        let at = table + 16 + i * 32;
        word(&mut b, at, u32::from_be_bytes(*tag));
        word(&mut b, at + 4, 1);
        word(&mut b, at + 8, offset as u32);
        word(&mut b, at + 12, len as u32)
    }
    b.extend(g);
    b.extend(s);
    b
}
fn c() -> Course {
    Course::read(&fixture()).unwrap()
}
fn rec(start: u32, end: u32, first: u8, last: u8, slope: i32) -> Interval {
    Interval {
        source_offset: 0,
        start,
        end,
        first: vec![first],
        last: vec![last],
        slope_q16: vec![slope],
        flags: [0; 2],
        raw: vec![],
    }
}
#[test]
fn typed_graph_retains_source_offsets_and_rejects_dangling_section() {
    let b = fixture();
    let c = Course::read(&b).unwrap();
    assert_eq!(c.nodes.len(), 8);
    assert_eq!(c.sections.len(), 4);
    assert_eq!(c.nodes[&92].file_offset, 104 + 92);
    let mut bad = b.clone();
    word(&mut bad, 104 + 92 + 16, 999);
    let e = Course::read(&bad).unwrap_err().to_string();
    assert!(e.contains("node 0x5c") && e.contains("section 999"));
    let mut bad = b;
    word(&mut bad, 104 + 92 + 8, 0x100000);
    assert!(
        Course::read(&bad)
            .unwrap_err()
            .to_string()
            .contains("offset 0x100000")
    );
}
#[test]
fn backward_links_are_intentional_but_forward_cycles_and_bad_roots_reject() {
    let course = c();
    course.validate().unwrap();
    let mut v = course.clone();
    v.nodes.get_mut(&92).unwrap().next = 20;
    assert!(v.validate().unwrap_err().to_string().contains("cycle"));
    let mut v = course.clone();
    v.root = 123;
    assert!(v.validate().unwrap_err().to_string().contains("root"));
    let mut v = course;
    v.nodes.get_mut(&188).unwrap().next = 52;
    assert!(
        v.validate()
            .unwrap_err()
            .to_string()
            .contains("backward terminal")
    );
}
#[test]
fn every_truncated_authored_container_is_rejected_without_panics() {
    let b = fixture();
    for length in 0..b.len() {
        assert!(Course::read(&b[..length]).is_err(), "length {length}");
    }
}
#[test]
fn graph_count_kind_alias_and_dimension_limits_reject() {
    let b = fixture();
    for (p, value) in [
        (104 + 8, 4097),
        (104 + 20, 5),
        (104 + 60 + 8, 24),
        (104 + 92 + 20, u32::MAX),
        (104 + 16, 21),
    ] {
        let mut v = b.clone();
        word(&mut v, p, value);
        assert!(Course::read(&v).is_err(), "{p}");
    }
}
#[test]
fn split_merge_choices_preserve_section_identity_and_dual_window() {
    let course = c();
    for choice in 0..2 {
        let r = course.route(&[choice]).unwrap();
        assert_eq!(r.points.len(), 214);
        assert_eq!(r.points[4].section, 1 + u32::from(choice));
        assert_eq!(r.points[4].event, 2);
        assert_eq!(r.points[123].event, 6);
        assert_eq!(r.points[203].dual.as_ref().unwrap().separation, 0);
        assert_eq!(r.points[204].event, 1);
        assert_eq!(r.points[204].branch, 0);
        assert!(r.points[4].branch > 0);
        assert!(r.points[204].dual.is_none());
        for p in &r.points {
            if let Some(d) = &p.dual {
                assert_eq!(
                    d.path,
                    course.sections[d.section as usize].samples[d.local as usize]
                );
            }
        }
    }
    assert!(course.route(&[]).is_err());
    assert!(course.route(&[2]).is_err());
}
#[test]
fn signed_rise_integrates_and_source_header_heights_remain_distinct() {
    let course = c();
    let r = course.route(&[0]).unwrap();
    assert_eq!(r.points[3].route_elevation, 102);
    assert_eq!(r.points[4].route_elevation, 100);
    assert_eq!(r.points[4].path.elevation, 999);
    for pair in r.points.windows(2) {
        assert_eq!(
            pair[1].route_elevation - pair[0].route_elevation,
            i64::from(pair[0].path.rise)
        );
    }
    assert_eq!(course.sections[1].samples[1].elevation, 1001);
    assert_eq!(course.sections[1].samples[2].elevation, 999);
    let other = course.route(&[1]).unwrap();
    assert_eq!(r.points[4].path.bend, 0);
    assert_eq!(other.points[4].path.bend, 1);
}
#[test]
fn lane_units_asymmetric_shoulders_and_direct_endpoint_survive() {
    let course = c();
    assert_eq!(course.sections[0].samples[0].width, [500, 750]);
    assert_eq!(course.sections[1].samples[0].width, [500, 250]);
    let s = &course.sections[0].samples[0];
    assert_eq!(s.shoulder, [10, 20]);
    assert_eq!(s.side_height, [0, 7]);
    assert_eq!(s.flags, [0x80, 0x13]);
    assert_eq!(lane_world(1), 0);
    assert_eq!(lane_world(-1), 0);
    assert_eq!(lane_world(257), 250);
    assert_eq!(course.sections[0].backward_endpoint.elevation, 321);
    assert_eq!(course.sections[0].backward_endpoint.width, [500, 750]);
    assert!(course.sections[0].terrain.as_ref().unwrap().raw.len() > 16);
}
#[test]
fn interval_first_sample_exact_end_gap_and_entering_tick_match_rules() {
    let intervals = [rec(0, 2, 2, 4, 65536), rec(4, 6, 99, 6, 65536)];
    let v = sample_intervals(&intervals, 8).unwrap();
    let values: Vec<_> = v.iter().map(|v| v.0[0] / 65536).collect();
    assert_eq!(values, [2, 3, 4, 4, 4, 5, 6, 6]);
    let adjacent = [rec(0, 2, 1, 2, 65536), rec(2, 4, 7, 8, 65536)];
    assert_eq!(
        sample_intervals(&adjacent, 5)
            .unwrap()
            .iter()
            .map(|v| v.0[0] / 65536)
            .collect::<Vec<_>>(),
        [1, 2, 2, 2, 2]
    );
    assert!(sample_intervals(&[rec(3, 2, 0, 0, 0)], 2).is_err());
    assert!(sample_intervals(&[rec(0, 10, 255, 255, 65536)], 3).is_err());
}
#[test]
fn event_start_interior_penultimate_end_and_beyond_are_explicit() {
    let e = Event {
        source_offset: 1,
        start: 1,
        end: 5,
        raw_type: 0,
        parameters: [2, 3, 4],
    };
    let mut samples = vec![Sample::road(); 7];
    rr_course::decode::apply_events(&mut samples, &[e]);
    assert_eq!(
        samples.iter().map(|s| s.event_type).collect::<Vec<_>>(),
        [0, 1, 1, 1, 1, 1, 0]
    );
    assert_eq!(samples[1].marking_flags, 8);
    assert_eq!(samples[3].marking_flags, 0x22);
    assert_eq!(samples[4].marking_flags, 16);
    assert!(samples[5].zone_end);
    assert!(!samples[6].zone_end);
    let mut samples = vec![Sample::road(); 3];
    rr_course::decode::apply_events(
        &mut samples,
        &[Event {
            source_offset: 0,
            start: 0,
            end: 2,
            raw_type: 6,
            parameters: [1, 0, 0],
        }],
    );
    assert_eq!(samples[0].event_type, 0);
    assert_eq!(samples[1].event_type, 7);
    assert!(!samples[2].zone_end);
}
#[test]
fn all_levels_resolve_first_finish_event_start() {
    let course = c();
    for half in 0..2 {
        let r = course.route(&[half]).unwrap();
        for level in 1..=5 {
            assert_eq!(r.finish(&course, level).unwrap(), 204 + level as usize)
        }
        assert!(r.finish(&course, 0).is_err());
        assert!(r.finish(&course, 6).is_err());
    }
}
#[test]
fn cursor_roundtrips_every_fraction_and_exact_terminal_without_overrun() {
    let course = c();
    for half in 0..2 {
        let r = course.route(&[half]).unwrap();
        let mut cur = Cursor::new(&r, 0).unwrap();
        for q in 0..r.len_q8() {
            assert_eq!(cur.position_q8, q);
            cur.point(&r).unwrap();
            cur.advance(&r, 1).unwrap();
        }
        assert!(cur.point(&r).is_err());
        assert_eq!(cur.endpoint(&course, &r).unwrap().elevation, 321);
        let before = cur.clone();
        assert!(cur.advance(&r, 1).is_err());
        assert_eq!(cur, before);
        cur.advance(&r, -r.len_q8()).unwrap();
        assert_eq!(cur.position_q8, 0);
        assert!(cur.advance(&r, -1).is_err());
        assert!(Cursor::new(&r, i64::MAX).is_err());
    }
}
#[test]
fn seam_edges_and_continuous_geometry_do_not_teleport() {
    let course = c();
    for half in 0..2 {
        let r = course.route(&[half]).unwrap();
        for s in seams(&r).unwrap() {
            assert_eq!(s.retained_edge_error, 0);
            let before = Cursor::new(&r, s.index as i64 * 256 - 1)
                .unwrap()
                .geometry(&r)
                .unwrap();
            let at = Cursor::new(&r, s.index as i64 * 256)
                .unwrap()
                .geometry(&r)
                .unwrap();
            assert!((at.elevation - before.elevation).abs() <= 2. / 256.);
            for side in 0..2 {
                assert!((at.side_height[side] - before.side_height[side]).abs() <= 7. / 256.);
                assert!((at.width[side] - before.width[side]).abs() <= 500. / 256.);
            }
        }
    }
}
#[test]
fn placement_ids_are_stable_unique_and_level_gates_are_independent() {
    let mut course = c();
    let s = &mut course.sections[1];
    s.roadside = vec![(15 << 24) | (1 << 16) | (2 << 8) | 1];
    s.family_events = vec![FamilyEvent {
        source_offset: 0,
        sample: 0,
        kind: 1,
        slot: 0,
        family: 42,
        raw: [0; 3],
    }];
    s.scenery = vec![
        [0x01900100, 0x1fff],
        [0x01900200, 0x2001],
        [0x81900301, 0x1000],
    ];
    let a = scenery::place(&course, 1, 1).unwrap();
    let b = scenery::place(&course, 1, 1).unwrap();
    assert_eq!(a.instances, b.instances);
    let ids: std::collections::BTreeSet<_> = a.instances.iter().map(|o| &o.id).collect();
    assert_eq!(ids.len(), a.instances.len());
    assert!(a.instances.iter().all(|o| o.family == Some(42)));
    assert!(
        a.instances
            .iter()
            .any(|o| o.kind == "billboard" && o.lateral == Some(-1 + (250 - 500) / 2))
    );
    assert!(
        a.instances
            .iter()
            .any(|o| o.kind == "pedestrian" && o.lateral.is_none())
    );
    course.sections[1].scenery.push([0x81f0000b, 0x1000]);
    let cops = scenery::place(&course, 1, 1).unwrap();
    assert!(
        cops.instances
            .iter()
            .any(|o| o.kind == "police" && o.family.is_none() && o.lateral.is_none())
    );
    let harder = scenery::place(&course, 2, 1).unwrap();
    assert!(
        harder
            .instances
            .iter()
            .any(|o| o.kind == "billboard" && o.selector == 2)
    );
    assert!(!harder.instances.iter().any(|o| o.kind == "pedestrian"));
    assert!(scenery::place(&course, 0, 1).is_err());
}
#[test]
fn bounded_mutations_of_graph_and_nested_tables_never_panic() {
    let original = fixture();
    let mut seed = 0x524f414452415348u64;
    for _ in 0..3000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut b = original.clone();
        let p = seed as usize % (b.len() - 4);
        word(&mut b, p, (seed >> 32) as u32);
        assert!(std::panic::catch_unwind(|| Course::read(&b)).is_ok());
    }
}

#[test]
fn scenery_positions_and_identity_stay_fixed_when_camera_crosses_seams() {
    let mut course = c();
    for s in &mut course.sections {
        s.roadside = vec![(15 << 24) | (1 << 16) | (15 << 12) | 1];
    }
    let placed = scenery::place(&course, 1, 7).unwrap();
    for half in 0..2 {
        let route = course.route(&[half]).unwrap();
        let fixed = route.instances(&placed);
        assert!(!fixed.is_empty());
        for seam in &route.transitions {
            let mut cursor = Cursor::new(&route, seam.index as i64 * 256 - 1).unwrap();
            let before = route.instances(&placed);
            cursor.advance(&route, 2).unwrap();
            let after = route.instances(&placed);
            assert_eq!(before, after);
            assert_eq!(fixed, after);
        }
        for o in fixed {
            let p = &route.points[o.route_sample];
            assert_eq!(p.section, o.section);
            assert_eq!(p.local, o.local);
            assert_eq!(p.branch, o.branch);
        }
    }
}
#[test]
fn isolated_scenery_ordering_budget_rejects_excessive_quadratic_work() {
    let mut course = c();
    course.sections[0].scenery = vec![[0, 0]; 4097];
    assert!(
        scenery::place(&course, 1, 1)
            .unwrap_err()
            .to_string()
            .contains("budget")
    );
}

#[test]
fn bend_separation_is_a_second_integral_while_rise_changes_height() {
    let course = c();
    let r = course.route(&[0]).unwrap();
    let s = &r.points[4..8];
    assert_eq!(
        s.iter()
            .map(|p| p.dual.as_ref().unwrap().separation)
            .collect::<Vec<_>>(),
        [0, 1, 3, 6]
    );
    assert_eq!(
        s.iter()
            .map(|p| p.dual.as_ref().unwrap().separation_delta)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        s.iter().map(|p| p.route_elevation).collect::<Vec<_>>(),
        [100, 102, 100, 102]
    );
}
fn with_terrain(hill: bool) -> Vec<u8> {
    let mut b = fixture();
    let start = u32::from_le_bytes(b[80..84].try_into().unwrap()) as usize;
    let offset = b.len() - start;
    let mut t = vec![];
    if hill {
        chunk(&mut t, b"LIHR", 20, 76, 1);
        word(&mut t, 20, 0);
        word(&mut t, 24, (-6i32) as u32);
        for i in 0..8 {
            t[32 + i * 8..40 + i * 8].copy_from_slice(&[0, 4, 1, 254, 2, 1, 1, 3]);
        }
    } else {
        chunk(&mut t, b"NTMR", 24, 16, 2);
        t.resize(64, 255);
        word(&mut t, 4, 64);
        word(&mut t, 8, 4);
        word(&mut t, 12, 1);
        word(&mut t, 16, 1);
        word(&mut t, 24, 0);
        word(&mut t, 36, (-300i32) as u32);
        word(&mut t, 40, 0);
        word(&mut t, 52, 50);
    }
    word(&mut b, start + 16 + 16, offset as u32);
    b.extend(t);
    let len = b.len() - start;
    word(&mut b, 84, len as u32);
    word(&mut b, start + 4, len as u32);
    b
}
#[test]
fn terrain_signed_hills_cliff_bases_and_unknown_parameters_are_preserved() {
    let b = with_terrain(true);
    let hill = Course::read(&b).unwrap().sections[0]
        .terrain
        .clone()
        .unwrap();
    assert_eq!(hill.hills[0].start, 0);
    assert_eq!(hill.hills[0].end, -6);
    assert_eq!(hill.hills[0].points[0][3] as i8, -2);
    let mut bad = b;
    let last = bad.len();
    bad[last - 2] = 0;
    assert!(Course::read(&bad).is_err());
    let walls = Course::read(&with_terrain(false)).unwrap().sections[0]
        .terrain
        .clone()
        .unwrap();
    assert_eq!(walls.wall_records[0][0].base_height, -300);
    assert_eq!(walls.wall_records[1][0].base_height, 50);
    assert_eq!(walls.selectors.len(), 4);
    assert!(!walls.raw.is_empty());
}

#[test]
fn section_end_initializers_are_distinct_from_forward_boundary_samples() {
    let course = c();
    for half in 0..2 {
        let r = course.route(&[half]).unwrap();
        for t in &r.transitions {
            let mut q = Cursor::new(&r, t.index as i64 * 256).unwrap();
            assert_eq!(q.section_endpoint(&course, &r).unwrap().elevation, 321);
            let next = q.point(&r).unwrap().section;
            q.advance(&r, -1).unwrap();
            assert_ne!(q.point(&r).unwrap().section, next);
            assert!(q.section_endpoint(&course, &r).is_err());
            q.advance(&r, 1).unwrap();
            assert_eq!(q.point(&r).unwrap().section, next);
        }
        assert!(
            Cursor::new(&r, 256)
                .unwrap()
                .section_endpoint(&course, &r)
                .is_err()
        );
    }
}

#[test]
fn exporter_rejects_source_overlap_before_creating_output() {
    let root = std::env::temp_dir().join(format!(
        "rr-course-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let out = root.join("generated");
    let e = rr_course::export::inspect(&root, &out).unwrap_err();
    assert!(e.to_string().contains("outside immutable"));
    assert!(!out.exists());
    std::fs::remove_dir(&root).unwrap();
}
