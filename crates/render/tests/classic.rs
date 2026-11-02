use rr_course::{Course, Hill, Node, NodeKind, Sample, Section, Terrain, Wall};
use rr_render::{camera::*, raster::*, scene::*, source::*};
use std::collections::BTreeMap;
fn course(mut samples: Vec<Sample>) -> Course {
    if samples.is_empty() {
        samples = vec![Sample::road(); 128]
    }
    let len = samples.len() as u32;
    let node = |offset, kind, next, previous, link, length| Node {
        offset,
        file_offset: offset,
        kind,
        next,
        previous,
        link,
        length,
        primary: 0,
        raw: vec![0; 8],
    };
    Course {
        root: 20,
        sections: vec![Section {
            source_offset: 0,
            raw: [0; 13],
            backward_endpoint: samples.last().unwrap().clone(),
            samples,
            lane_intervals: vec![],
            side_intervals: vec![],
            events: vec![],
            family_events: vec![],
            family_bias: 0,
            roadside: vec![],
            scenery: vec![],
            scenery_following: None,
            terrain: None,
        }],
        nodes: BTreeMap::from([
            (20, node(20, NodeKind::Road, 60, 52, 0, len)),
            (52, node(52, NodeKind::Start, 0, 0, 0, 0)),
            (60, node(60, NodeKind::End, 0, 0, 0, 0)),
        ]),
        source_sha256: "authored".into(),
    }
}
fn scene() -> Scene {
    Scene::new(
        course(vec![]),
        SourceAssets::authored(),
        SceneOptions::default(),
    )
    .unwrap()
}
fn snap() -> rr_simulation::Snapshot {
    rr_simulation::Simulation::default().snapshot()
}
fn view() -> View {
    View {
        position_q8: 50. * 256.,
        ..Default::default()
    }
}
fn sprite(id: &str, class: SpriteClass, color: [u8; 4], depth: f64) -> Billboard {
    Billboard {
        id: id.into(),
        class,
        x: 0.,
        ground: 0.,
        z: depth,
        width: 80.,
        height: 100.,
        pivot: [0.5, 1.],
        texture: Texture::solid(color),
        mirror: false,
        rotation: 0.,
        clip_ground: true,
    }
}
#[test]
fn camera_spec_declares_classic_projection_and_validates_bad_parameters() {
    let s = CameraSpec::default();
    assert_eq!(
        (s.width, s.height, s.horizon, s.camera_height, s.slices),
        (640, 480, 180., 150., 33)
    );
    for v in [f64::NAN, f64::INFINITY, -1., 5000.] {
        let mut bad = s.clone();
        bad.camera_height = v;
        assert!(bad.validate().is_err());
    }
    let mut bad = s;
    bad.width = 10000;
    assert!(bad.validate().is_err());
}
#[test]
fn straight_bend_crest_valley_depth_is_monotonic_and_bounded() {
    for mode in 0..4 {
        let mut samples = vec![Sample::road(); 128];
        for (i, p) in samples.iter_mut().enumerate() {
            p.bend = if mode == 1 {
                if i % 10 < 5 { 60 } else { -60 }
            } else {
                0
            };
            p.rise = match mode {
                2 => {
                    if i % 20 < 10 {
                        50
                    } else {
                        -50
                    }
                }
                3 => {
                    if i % 20 < 10 {
                        -50
                    } else {
                        50
                    }
                }
                _ => 0,
            };
        }
        let c = course(samples);
        let r = c.route(&[]).unwrap();
        for fraction in [0., 1., 127.5, 255.] {
            let v = View {
                position_q8: 50. * 256. + fraction,
                ..Default::default()
            };
            let out = project(&r, &CameraSpec::default(), &DepthTable::authored(), &v).unwrap();
            assert_eq!(out.len(), 33);
            assert!(out.windows(2).all(|p| p[0].roads[0].z < p[1].roads[0].z));
            assert!(out.iter().all(|s| {
                s.roads
                    .iter()
                    .all(|r| r.x.is_finite() && r.y.is_finite() && r.z >= 1. && r.scale > 0.)
            }));
        }
    }
}
#[test]
fn projection_known_straight_slice_has_exact_scale_and_edges() {
    let r = course(vec![]).route(&[]).unwrap();
    let out = project(
        &r,
        &CameraSpec::default(),
        &DepthTable::authored(),
        &View {
            position_q8: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out[0].roads[0].z, 256.);
    assert_eq!(out[0].roads[0].scale, 2.);
    assert_eq!(out[1].roads[0].scale, 1.);
    assert_eq!(out[1].roads[0].x, 0.);
    assert_eq!(out[1].roads[0].path.width, [500, 500]);
}
#[test]
fn camera_out_of_range_and_nonfinite_values_fail_cleanly() {
    let r = course(vec![]).route(&[]).unwrap();
    for p in [-1., f64::NAN, f64::INFINITY, r.len_q8() as f64] {
        assert!(
            project(
                &r,
                &CameraSpec::default(),
                &DepthTable::authored(),
                &View {
                    position_q8: p,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn horizon_wraps_at_source_period_and_steering_stays_in_bounds() {
    let pixels = (0..720 * 320)
        .flat_map(|i| {
            [
                (i % 720 % 256) as u8,
                (i % 720 / 256) as u8,
                (i / 720) as u8,
                255,
            ]
        })
        .collect();
    let texture = Texture::new(
        rr_formats::pixels::Rgba {
            width: 720,
            height: 320,
            pixels,
        },
        true,
    )
    .unwrap();
    for width in [640, 852] {
        let mut a = Frame::new(width, 480).unwrap();
        let mut b = Frame::new(width, 480).unwrap();
        draw_horizon(&mut a, &texture, 0);
        draw_horizon(&mut b, &texture, 4 * 32768 * 720 * 4 / 5);
        assert_eq!(a.image, b.image);
        for heading in [i32::MIN, i32::MAX, -1, 1] {
            draw_horizon(&mut b, &texture, heading);
            assert_eq!(b.image.pixels.len(), width as usize * 480 * 4);
        }
    }
}
#[test]
fn source_terrain_categories_are_distinct_seeded_and_renderable() {
    let mut c = course(vec![]);
    let base = Terrain {
        tag: "DLBR".into(),
        source_offset: 0,
        raw: vec![],
        selectors: vec![[255; 2]; 128],
        endpoint_selectors: None,
        wall_records: [vec![], vec![]],
        hills: vec![],
    };
    c.sections[0].terrain = Some(base.clone());
    assert_eq!(terrain_geometry(&c, 1)[0][0].kind, TerrainKind::Building);
    let mut wall = base.clone();
    wall.tag = "NTMR".into();
    wall.wall_records = [
        vec![Wall {
            start: 0,
            parameters: [0, 1, 10, 1, 20, 1, 1, 10],
            base_height: -1000,
        }],
        vec![Wall {
            start: 0,
            parameters: [0, 1, 10, 1, 20, 1, 1, 10],
            base_height: 150,
        }],
    ];
    c.sections[0].terrain = Some(wall);
    c.sections[0].samples[0].elevation = 100;
    let geometry = terrain_geometry(&c, 1);
    assert_eq!(geometry[0][0].kind, TerrainKind::Wall);
    assert!(geometry[0][0].points[0].iter().any(|p| p[1] < 0.));
    let mut hill = base;
    hill.tag = "LIHR".into();
    hill.hills = vec![
        Hill {
            start: 0,
            end: 127,
            selectors: [1, 2],
            points: [[2, 8, 1, 255, 5, 1, 4, 12]; 8],
        },
        Hill {
            start: 128,
            end: 256,
            selectors: [2, 3],
            points: [[4, 12, 1, 0, 8, 1, 4, 12]; 8],
        },
    ];
    c.sections[0].terrain = Some(hill);
    let a = terrain_geometry(&c, 42);
    let b = terrain_geometry(&c, 42);
    assert_eq!(a[0][0].kind, TerrainKind::Hill);
    assert_eq!(a[0][0].points, b[0][0].points);
    assert_ne!(a[0][0].points, terrain_geometry(&c, 2)[0][0].points);
    let s = Scene::new(c, SourceAssets::authored(), SceneOptions::default()).unwrap();
    let (f, r) = s
        .draw(&CameraSpec::default(), &view(), &snap(), false)
        .unwrap();
    assert!(f.stats.pixels > 0);
    assert!(r.terrain_categories.contains_key("Hill"));
}
#[test]
fn road_event_materials_change_on_the_exact_sample() {
    let mut p = Sample::road();
    assert_eq!((road_material(&p, 0, 4), road_material(&p, 2, 4)), (5, 7));
    p.marking_flags = 8;
    assert_eq!((road_material(&p, 0, 4), road_material(&p, 2, 4)), (2, 4));
    p.marking_flags = 0x20;
    assert_eq!(road_material(&p, 0, 4), 1);
    p.marking_flags = 0x40;
    assert_eq!(road_material(&p, 0, 4), 5);
    p.marking_flags = 0;
    assert_eq!(road_material(&p, 0, 4), 5);
    let mut source = vec![Sample::road(); 60];
    rr_course::decode::apply_events(
        &mut source,
        &[rr_course::Event {
            source_offset: 0,
            start: 50,
            end: 55,
            raw_type: 0,
            parameters: [0; 3],
        }],
    );
    assert_eq!(
        [49, 50, 51, 55, 56].map(|j| road_material(&source[j], 0, 4)),
        [5, 2, 1, 5, 5]
    );
}
#[test]
fn equal_depth_sprite_ties_are_independent_of_registration_order() {
    let spec = CameraSpec::default();
    let d = DepthTable::authored();
    let mut sprites = vec![
        sprite("02", SpriteClass::Racer, [255, 0, 0, 255], 512.),
        sprite("01", SpriteClass::Scenery, [0, 255, 0, 255], 512.),
        sprite("03", SpriteClass::Traffic, [0, 0, 255, 255], 512.),
    ];
    let mut a = Frame::new(640, 480).unwrap();
    a.clear([0, 0, 0, 255]);
    draw_billboards(&mut a, &mut sprites, &spec, &d);
    sprites.reverse();
    let mut b = Frame::new(640, 480).unwrap();
    b.clear([0, 0, 0, 255]);
    draw_billboards(&mut b, &mut sprites, &spec, &d);
    assert_eq!(a.image, b.image);
    assert_eq!(
        &a.image.pixels[(250 * 640 + 320) * 4..(250 * 640 + 320) * 4 + 4],
        &[255, 0, 0, 255]
    );
}
#[test]
fn transparent_texels_do_not_write_depth_or_obscure_other_sprites() {
    let mut f = Frame::new(640, 480).unwrap();
    f.clear([9, 8, 7, 255]);
    let mut b = vec![
        sprite("far", SpriteClass::Racer, [0, 200, 0, 255], 512.),
        sprite("near", SpriteClass::Racer, [255, 0, 0, 0], 300.),
    ];
    draw_billboards(
        &mut f,
        &mut b,
        &CameraSpec::default(),
        &DepthTable::authored(),
    );
    assert_eq!(
        &f.image.pixels[(280 * 640 + 320) * 4..(280 * 640 + 320) * 4 + 4],
        &[0, 200, 0, 255]
    );
}
#[test]
fn near_and_screen_clipping_degenerates_and_behind_camera_are_bounded() {
    let spec = CameraSpec::default();
    let depth = DepthTable::authored();
    let mut f = Frame::new(640, 480).unwrap();
    let t = Texture::solid([255; 4]);
    let vertex = |x, y, z| Vertex {
        x,
        y,
        z,
        u: 0.,
        v: 0.,
    };
    f.polygon(
        &[
            vertex(-40., 100., -20.),
            vertex(40., 100., 512.),
            vertex(0., 0., 512.),
        ],
        &t,
        &spec,
        &depth,
    );
    assert_eq!(f.stats.clipped, 1);
    for p in [
        [vertex(0., 0., 0.); 3],
        [vertex(0., 0., 512.); 3],
        [vertex(f64::NAN, 0., 1.); 3],
        [vertex(1e12, 0., 1.); 3],
    ] {
        f.polygon(&p, &t, &spec, &depth)
    }
    let pixels = f.stats.pixels;
    let mut behind = vec![sprite("behind", SpriteClass::Scenery, [255; 4], -1.)];
    draw_billboards(&mut f, &mut behind, &spec, &depth);
    assert_eq!(f.stats.pixels, pixels);
    assert!(!f.depth.iter().any(|p| p.is_nan()));
    assert!(f.stats.degenerate >= 3);
    let v = clip_near(
        &[
            vertex(-1., 0., -1.),
            vertex(1., 0., 10.),
            vertex(0., 1., 10.),
        ],
        1.,
    );
    assert_eq!(v.len(), 4);
    assert!(v.iter().all(|p| p.z >= 1.));
}
#[test]
fn perspective_sprite_scale_preserves_contact_and_logical_pivot() {
    let s = CameraSpec::default();
    let d = DepthTable::authored();
    for depth in [256., 512., 1024.] {
        let mut f = Frame::new(640, 480).unwrap();
        f.clear([0; 4]);
        let mut list = vec![sprite("rider", SpriteClass::Racer, [255; 4], depth)];
        draw_billboards(&mut f, &mut list, &s, &d);
        let ys = f
            .image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter(|(_, c)| c[0] == 255)
            .map(|(i, _)| i / 640)
            .collect::<Vec<_>>();
        let foot = 180. + 150. * d.scale(depth);
        assert!((*ys.iter().max().unwrap() as f64 + 1. - foot).abs() <= 1.);
        let height = *ys.iter().max().unwrap() - *ys.iter().min().unwrap() + 1;
        assert!((height as f64 - 100. * d.scale(depth)).abs() <= 1.);
    }
}
#[test]
fn aspect_and_integer_scaling_never_stretch_the_logical_surface() {
    let mut s = CameraSpec::default();
    for size in [[640, 480], [1920, 1080], [800, 1280], [320, 240]] {
        let (origin, extent) = s.viewport(size).unwrap();
        assert!(origin[0] + extent[0] <= size[0] && origin[1] + extent[1] <= size[1]);
        assert!((extent[0] as f64 / extent[1] as f64 - 4. / 3.).abs() < 0.01);
    }
    s.integer_scale = true;
    assert_eq!(s.viewport([1920, 1080]).unwrap(), ([320, 60], [1280, 960]));
    assert!(s.viewport([0, 480]).is_none());
}
#[test]
fn wide_view_is_hor_plus_with_classic_safe_hud_and_identical_gameplay() {
    let s = scene();
    let simulation = rr_simulation::Simulation::default();
    let before = simulation.state_hash();
    let classic = CameraSpec::default();
    let wide = CameraSpec::new(Framing::Wide, false);
    let v = view();
    let a = project(&s.route, &classic, &s.assets.depths, &v).unwrap();
    let b = project(&s.route, &wide, &s.assets.depths, &v).unwrap();
    assert_eq!(a[0].roads[0].scale, b[0].roads[0].scale);
    assert_eq!(wide.hud_origin()[0] - classic.hud_origin()[0], 106);
    s.draw(&classic, &v, &simulation.snapshot(), false).unwrap();
    s.draw(&wide, &v, &simulation.snapshot(), false).unwrap();
    assert_eq!(simulation.state_hash(), before);
}
#[test]
fn interpolation_changes_only_owned_visual_copies_and_is_bounded() {
    let mut simulation = rr_simulation::Simulation::default();
    let old = simulation.snapshot();
    for _ in 0..10 {
        simulation
            .step(rr_simulation::Command {
                throttle: true,
                steer: 1,
                ..Default::default()
            })
            .unwrap();
    }
    let now = simulation.snapshot();
    let hash = simulation.state_hash();
    let a = snapshot_view(&old, &now, 0., 12800.);
    let b = snapshot_view(&old, &now, 0.5, 12800.);
    let c = snapshot_view(&old, &now, 1., 12800.);
    assert!(a.position_q8 < b.position_q8 && b.position_q8 < c.position_q8);
    assert!(a.lateral < b.lateral && b.lateral < c.lateral);
    let visual = interpolated_snapshot(&old, &now, 0.5);
    assert_ne!(visual, now);
    assert_eq!(simulation.state_hash(), hash);
    assert_eq!(
        interpolated_snapshot(&old, &now, f32::NAN).distance_mm,
        old.distance_mm
    );
}
#[test]
fn repeated_fixed_camera_pixels_have_no_masked_regions() {
    let s = scene();
    let spec = CameraSpec::default();
    let (a, _) = s.draw(&spec, &view(), &snap(), false).unwrap();
    let (b, _) = s.draw(&spec, &view(), &snap(), false).unwrap();
    assert_eq!(a.image, b.image);
    assert!(a.stats.pixels > 0);
    assert!(a.image.pixels.iter().filter(|c| **c != 0).count() > 10000);
}

#[test]
fn nonlinear_source_near_scales_keep_the_rider_visible_and_on_the_road() {
    let mut s = scene();
    for (i, v) in s.assets.depths.0.iter_mut().enumerate().take(257) {
        *v = if i <= 128 {
            163840
        } else {
            163840 - ((i - 128) * 131072 / 128) as i32
        };
    }
    let spec = CameraSpec::default();
    let snapshot = snap();
    let (mut empty, _) = (snapshot.clone(), 0);
    empty.actors.clear();
    let (with, _) = s.draw(&spec, &view(), &snapshot, false).unwrap();
    let (without, _) = s.draw(&spec, &view(), &empty, false).unwrap();
    assert!(
        with.image
            .pixels
            .iter()
            .zip(&without.image.pixels)
            .filter(|(a, b)| a != b)
            .count()
            > 1000
    );
    let rows = with
        .image
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, p)| **p == [230, 45, 60, 255])
        .map(|(i, _)| i / 640)
        .collect::<Vec<_>>();
    assert!(!rows.is_empty());
    let slices = project(&s.route, &spec, &s.assets.depths, &view()).unwrap();
    let near = &slices[0].roads[0];
    let far = &slices[1].roads[0];
    let anchor = contact_anchor(
        near,
        far,
        spec.player_depth_q8,
        0.,
        [0., 0.],
        &spec,
        &s.assets.depths,
    );
    let foot = spec.horizon
        + (spec.camera_height - anchor[1]) * s.assets.depths.scale(spec.player_depth_q8);
    assert!((*rows.iter().max().unwrap() as f64 + 1. - foot).abs() <= 1.);
}

#[test]
fn pixel_diff_marks_actual_changes_without_a_mask() {
    let expected = rr_formats::pixels::Rgba {
        width: 2,
        height: 1,
        pixels: vec![0, 10, 20, 255, 40, 50, 60, 255],
    };
    let mut actual = expected.clone();
    actual.pixels[4] = 70;
    let (count, diff) = rr_render::suite::image_diff(&expected, &actual).unwrap();
    assert_eq!(count, 1);
    assert_eq!(&diff.pixels[4..8], &[30, 0, 0, 255]);
}
#[test]
fn output_guard_rejects_immutable_install_before_creating_files() {
    let root = std::env::temp_dir().join(format!(
        "rr-render-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    assert!(rr_render::suite::output_path(&root, &root.join("captures")).is_err());
    assert!(!root.join("captures").exists());
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn native_source_mips_reduce_minification_without_changing_geometry() {
    let mut t = Texture::solid([255; 4]);
    let mip = rr_formats::pixels::Rgba {
        width: 1,
        height: 1,
        pixels: vec![100, 100, 100, 255],
    };
    t.image = std::sync::Arc::new(rr_formats::pixels::Rgba {
        width: 8,
        height: 8,
        pixels: vec![255; 8 * 8 * 4],
    });
    t.mipmaps = vec![t.image.clone(), std::sync::Arc::new(mip)];
    let mut frame = Frame::new(640, 480).unwrap();
    let v = |x, y| Vertex {
        x,
        y,
        z: 512.,
        u: if x < 0. { 0. } else { 1. },
        v: if y > 0. { 0. } else { 1. },
    };
    frame.quad(
        [v(-1., 100.), v(1., 100.), v(1., 98.), v(-1., 98.)],
        &t,
        &CameraSpec::default(),
        &DepthTable::authored(),
    );
    assert!(
        frame
            .image
            .pixels
            .as_chunks::<4>()
            .0
            .contains(&[100, 100, 100, 255])
    );
}

#[test]
fn authored_visual_checkpoints_match_reviewed_golden_hashes() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../../docs/fixtures/render-goldens.json")).unwrap();
    for name in rr_render::fixture::CASES {
        let scene = rr_render::fixture::scene(name).unwrap();
        let (frame, _) = scene
            .draw(&CameraSpec::default(), &view(), &snap(), false)
            .unwrap();
        assert_eq!(
            serde_json::json!(rr_render::suite::hash(&frame.image.pixels)),
            expected["rgba_sha256"][name],
            "authored checkpoint {name}"
        );
    }
}

#[test]
fn static_scenery_aliases_pixels_without_losing_view_specific_hotspots() {
    let mut bytes = vec![0u8; 292];
    let word = |b: &mut [u8], at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    for (at, v) in [
        (0, 0x5252414e),
        (4, 20),
        (8, 112),
        (20, 0x52524644),
        (24, 92),
        (112, 0x414e494d),
        (116, 20),
        (128, 3),
        (32, 212),
        (92, 284),
        (220, 0xbc614e),
        (232, 1),
        (252, 2),
        (256, 2),
        (260, 1),
        (276, 32),
        (284, 1),
        (288, 2),
    ] {
        word(&mut bytes, at, v)
    }
    bytes[228..230].copy_from_slice(&2i16.to_le_bytes());
    bytes[230..232].copy_from_slice(&2i16.to_le_bytes());
    bytes[280..284].copy_from_slice(&[0, 1, 1, 0]);
    let mut palette = rr_formats::pixels::Palette([[0; 4]; 256]);
    palette.0[1] = [200, 20, 30, 0];
    let sprite = rr_render::source::scenery_sprite(&bytes, 0, &palette)
        .unwrap()
        .unwrap();
    assert_eq!(sprite.logical, [2., 2.]);
    assert_eq!(sprite.pivot, [0.5, 1.]);
    assert_eq!(sprite.texture.image.pixels[3], 0);
    assert_eq!(sprite.texture.image.pixels[7], 255);
    for n in 0..bytes.len() {
        assert!(
            std::panic::catch_unwind(|| rr_render::source::scenery_sprite(
                &bytes[..n],
                0,
                &palette
            ))
            .is_ok()
        );
    }
    word(&mut bytes, 32, u32::MAX);
    assert!(rr_render::source::scenery_sprite(&bytes, 0, &palette).is_err());
}

#[test]
fn rider_and_scenery_ground_follow_the_rendered_shoulder_ramp() {
    let mut p = Sample::road();
    p.shoulder = [80, 80];
    p.side_height = [40, 40];
    let terrain = TerrainSample {
        kind: TerrainKind::Building,
        selectors: [255; 2],
        points: [vec![], vec![]],
    };
    assert_eq!(ground_height(&terrain, &p, 0.), 0.);
    assert_eq!(ground_height(&terrain, &p, 500.), 0.);
    assert_eq!(ground_height(&terrain, &p, 540.), 20.);
    assert_eq!(ground_height(&terrain, &p, -540.), 20.);
    assert_eq!(ground_height(&terrain, &p, 580.), 40.);
}

#[test]
fn actor_projection_never_compares_unrelated_branch_local_coordinates() {
    let snapshot = snap();
    let player = &snapshot.actors[0];
    let mut other = player.clone();
    other.id = rr_simulation::ActorId(2);
    other.position.progress.0 += 80;
    assert_eq!(relative_actor_depth(&snapshot, &other, 420.), Some(430.));
    other.position.branch = rr_simulation::BranchId(7);
    assert_eq!(relative_actor_depth(&snapshot, &other, 420.), None);
    let mut changed = snapshot.clone();
    changed.actors[0].position.branch = rr_simulation::BranchId(8);
    changed.actors[0].position.progress.0 = 300;
    assert_eq!(
        interpolated_snapshot(&snapshot, &changed, 0.5).actors[0]
            .position
            .progress
            .0,
        300
    );
    let mut reset = snapshot.clone();
    reset.distance_mm = 9000;
    reset.actors[0].position.progress.0 = 10;
    assert_eq!(
        relative_actor_depth(&reset, &reset.actors[0], 420.),
        Some(420.)
    );
}

#[test]
fn rran_rider_registration_does_not_use_bitmap_dimensions() {
    let mut bytes = vec![0u8; 72];
    for (at, word) in [
        (0, 0x5252414eu32),
        (4, 20),
        (20, 0x52524644),
        (24, 36),
        (36, 64),
        (48, 29),
        (52, 64),
        (64, 19),
        (68, 52),
    ] {
        bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
    }
    let (logical, pivot) = rr_render::source::rider_frame_registration(&bytes, 0).unwrap();
    assert_eq!(logical, [29., 64.]);
    assert_eq!(pivot, [19. / 29., 52. / 64.]);
    assert!(rr_render::source::rider_frame_registration(&bytes, 1).is_err());
    for end in 0..bytes.len() {
        assert!(
            rr_render::source::rider_frame_registration(&bytes[..end], 0).is_err(),
            "prefix {end}"
        );
    }
    bytes[48..52].copy_from_slice(&0u32.to_le_bytes());
    assert!(rr_render::source::rider_frame_registration(&bytes, 0).is_err());
}

#[test]
fn non_bottom_hotspot_keeps_full_sprite_below_ground_without_ignoring_hills() {
    let spec = CameraSpec::default();
    let depths = DepthTable::authored();
    let mut frame = Frame::new(640, 480).unwrap();
    frame.clear([0; 4]);
    for y in 0..480 {
        for x in 0..640 {
            frame.depth[y * 640 + x] = if y < 330 { 600. } else { 450. };
        }
    }
    let mut b = sprite("registered", SpriteClass::Racer, [255; 4], 512.);
    b.width = 58.;
    b.height = 128.;
    b.pivot = [19. / 29., 52. / 64.];
    draw_billboards(&mut frame, &mut [b.clone()], &spec, &depths);
    let positions = frame
        .image
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, p)| p[0] == 255)
        .map(|(i, _)| (i % 640, i / 640))
        .collect::<Vec<_>>();
    assert_eq!(positions.iter().map(|p| p.0).min(), Some(282));
    assert_eq!(positions.iter().map(|p| p.0).max(), Some(339));
    assert_eq!(positions.iter().map(|p| p.1).min(), Some(226));
    assert_eq!(positions.iter().map(|p| p.1).max(), Some(353));
    frame.clear([0; 4]);
    frame.depth.fill(400.);
    draw_billboards(&mut frame, &mut [b], &spec, &depths);
    assert_eq!(frame.stats.pixels, 0, "nearer hill occludes the sprite");
}

#[test]
fn cliff_surfaces_keep_landscape_art_when_tunnel_art_is_available() {
    use rr_formats::family::FamilyNode;
    use std::sync::Arc;
    let mut scene = rr_render::fixture::scene("wall").unwrap();
    let family = scene.assets.families.get_mut(&1).unwrap();
    let mut definition = (*family.definition).clone();
    // Category 3 is tunnel geometry, deliberately bright magenta. Cliff
    // landscape remains in category 1, with the authored checker texture.
    definition
        .nodes
        .iter_mut()
        .find(|n| n.offset == 0)
        .unwrap()
        .children[3] = 300;
    definition.nodes.push(FamilyNode {
        offset: 300,
        tag: 1,
        children: vec![400],
        raw_header: vec![],
    });
    definition.nodes.push(FamilyNode {
        offset: 400,
        tag: 0x434c4750,
        children: vec![],
        raw_header: vec![0; 32],
    });
    definition.nodes.sort_by_key(|n| n.offset);
    family.definition = Arc::new(definition);
    family.groups.insert(
        400,
        rr_render::source::Material {
            end_texture: Texture::solid([255, 0, 255, 255]),
            texture: Texture::solid([255, 0, 255, 255]),
            height: 300.,
        },
    );
    let (frame, _) = scene
        .draw(&CameraSpec::default(), &view(), &snap(), false)
        .unwrap();
    assert!(
        !frame
            .image
            .pixels
            .as_chunks::<4>()
            .0
            .contains(&[255, 0, 255, 255])
    );
    assert!(
        frame
            .image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] == 150 && p[1] == 90 && p[2] == 55),
        "landscape checker is visible"
    );
}

#[test]
fn repeated_capture_can_replace_its_existing_output_file() {
    let dir = std::env::temp_dir().join(format!("rr-existing-capture-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("install")).unwrap();
    let file = dir.join("frame.png");
    std::fs::write(&file, b"old capture").unwrap();
    let out = rr_render::suite::output_path(&dir.join("install"), &file).unwrap();
    std::fs::write(out, b"new capture").unwrap();
    assert_eq!(std::fs::read(file).unwrap(), b"new capture");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn animation_frame_pivots_palette_and_ground_shadow_survive_airborne_transition() {
    use rr_simulation::{
        animation::{Player, Pose, Weapon},
        handling::Bike,
        riding::Rider,
    };
    let mut scene = scene();
    let sprite = |color, logical, pivot| SpriteAsset {
        texture: Texture::solid(color),
        logical,
        pivot,
    };
    scene.assets.animations = Some(std::sync::Arc::new(rr_render::rider::RiderAssets {
        bodies: vec![
            Some(sprite([245, 5, 100, 255], [20., 40.], [0.25, 0.75])),
            Some(sprite([5, 245, 100, 255], [40., 60.], [0.75, 0.5])),
        ],
        shadows: vec![Some(sprite([0, 0, 0, 128], [80., 8.], [0.5, 0.5])); 2],
        color: 0,
    }));
    let mut snapshot = snap();
    snapshot.actors[0].rider = Some(Rider {
        actions: None,
        bike: Bike::default(),
        animation: Player::default(),
        pose: Pose::Ride,
        weapon: Weapon::Fists,
        frame: 0,
        mirror: false,
        view_flags: 0,
        roll: 0,
        flags: 0,
        contact_frame: false,
        pose_ticks: 0,
    });
    let spec = CameraSpec::default();
    let (base, receipt) = scene.draw(&spec, &view(), &snapshot, false).unwrap();
    let bounds = receipt.rider_bounds.unwrap();
    let original_bounds = bounds;
    assert_eq!(bounds[2] - bounds[0], 40.);
    assert_eq!(bounds[3] - bounds[1], 80.);
    snapshot.actors[0].rider.as_mut().unwrap().bike.height = 30 * 256;
    let (air, _) = scene.draw(&spec, &view(), &snapshot, false).unwrap();
    // Observe a body-free patch around the ground anchor, comparing against an
    // otherwise identical draw without a shadow. This cannot pass on HUD pixels.
    let saved_shadow = scene.assets.animations.as_ref().unwrap().shadows.clone();
    std::sync::Arc::get_mut(scene.assets.animations.as_mut().unwrap())
        .unwrap()
        .shadows = vec![None; 2];
    snapshot.actors[0].rider.as_mut().unwrap().bike.height = 0;
    let (without_shadow, _) = scene.draw(&spec, &view(), &snapshot, false).unwrap();
    std::sync::Arc::get_mut(scene.assets.animations.as_mut().unwrap())
        .unwrap()
        .shadows = saved_shadow;
    let region = |f: &Frame| {
        f.image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                ((240..300).contains(&(i % 640)) && (320..340).contains(&(i / 640))).then_some(*c)
            })
            .collect::<Vec<_>>()
    };
    assert_ne!(
        region(&base),
        region(&without_shadow),
        "shadow must actually contribute pixels near the ground anchor"
    );
    assert_eq!(region(&base), region(&air));
    let r = snapshot.actors[0].rider.as_mut().unwrap();
    r.frame = 1;
    r.bike.height = 0;
    let (next, receipt) = scene.draw(&spec, &view(), &snapshot, false).unwrap();
    let bounds = receipt.rider_bounds.unwrap();
    assert_eq!(bounds[2] - bounds[0], 80.);
    assert_eq!(bounds[3] - bounds[1], 120.);
    assert_ne!(base.image.pixels, next.image.pixels);
    let anchor = |bounds: [f64; 4], pivot: [f64; 2]| {
        [
            bounds[0] + (bounds[2] - bounds[0]) * pivot[0],
            bounds[1] + (bounds[3] - bounds[1]) * pivot[1],
        ]
    };
    assert_eq!(
        anchor(bounds, [0.75, 0.5]),
        anchor(original_bounds, [0.25, 0.75])
    );
    snapshot.actors[0].rider.as_mut().unwrap().mirror = true;
    let (_, mirrored) = scene.draw(&spec, &view(), &snapshot, false).unwrap();
    assert_eq!(
        anchor(mirrored.rider_bounds.unwrap(), [0.25, 0.5]),
        anchor(original_bounds, [0.25, 0.75])
    );
    snapshot.actors[0].rider.as_mut().unwrap().frame = 2;
    let (_, empty) = scene.draw(&spec, &view(), &snapshot, false).unwrap();
    assert!(
        empty.rider_bounds.is_none(),
        "empty source slot must not fall back to an unrelated rider frame"
    );
}
#[test]
fn billboard_rotation_preserves_zero_angle_and_changes_nonzero_pose() {
    let spec = CameraSpec::default();
    let depths = DepthTable::authored();
    let mut a = Frame::new(640, 480).unwrap();
    let mut b = Frame::new(640, 480).unwrap();
    let mut c = Frame::new(640, 480).unwrap();
    let mut s = sprite("rider", SpriteClass::Racer, [255, 0, 255, 255], 512.);
    draw_billboards(&mut a, &mut [s.clone()], &spec, &depths);
    s.rotation = 0.;
    draw_billboards(&mut b, &mut [s.clone()], &spec, &depths);
    assert_eq!(a.image.pixels, b.image.pixels);
    s.rotation = 0.2;
    draw_billboards(&mut c, &mut [s], &spec, &depths);
    assert_ne!(a.image.pixels, c.image.pixels);
}

#[test]
fn parallel_vertical_stripes_preserve_every_pixel_depth_and_draw_statistic() {
    for name in rr_render::fixture::CASES {
        let scene = rr_render::fixture::scene(name).unwrap();
        let snapshot = rr_simulation::Simulation::default().snapshot();
        for framing in [rr_render::Framing::Classic, rr_render::Framing::Wide] {
            let spec = rr_render::CameraSpec::new(framing, false);
            let view = rr_render::View::default();
            let (serial, receipt) = scene.draw(&spec, &view, &snapshot, false).unwrap();
            for workers in [2, 3, 6] {
                let (parallel, actual) = scene
                    .draw_parallel(&spec, &view, &snapshot, false, workers)
                    .unwrap();
                assert_eq!(
                    serial.image.pixels, parallel.image.pixels,
                    "{name} {framing:?} {workers}"
                );
                assert_eq!(serial.depth, parallel.depth, "{name} {workers}");
                assert_eq!(
                    serde_json::to_value(&receipt).unwrap(),
                    serde_json::to_value(&actual).unwrap(),
                    "{name} {workers}"
                );
            }
        }
    }
}

#[test]
fn buildings_draw_exposed_textured_ends_and_solid_roofs() {
    let mut scene = rr_render::fixture::scene("building").unwrap();
    // A tall building behind a short one must expose the first CLGP cell on
    // its end, independently of its facade and the palette-colored roof.
    for row in &mut scene.buildings[0][..54] {
        for p in row.iter_mut().flatten() {
            p.height = 100;
            p.depth = 50;
        }
    }
    let (frame, _) = scene
        .draw(&CameraSpec::default(), &view(), &snap(), false)
        .unwrap();
    let pixels = frame.image.pixels.as_chunks::<4>().0;
    assert!(
        pixels.iter().filter(|p| **p == [165, 85, 45, 255]).count() > 20,
        "textured building end missing"
    );
    assert!(pixels.contains(&[95, 95, 90, 255]), "solid roof missing");
    assert!(pixels.contains(&[150, 90, 55, 255]), "facade missing");
}
