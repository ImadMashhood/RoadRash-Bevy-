use rr_simulation::{
    building_wall::{self, Boundary, Neighbors, Query, Surface},
    collision::Motion,
};

fn boundary(kind: u8) -> Boundary {
    Boundary {
        path: 0,
        sample: 10,
        building: kind == 0,
        road: [-500, 500],
        outer: [-750, 750],
        selectors: [0; 2],
        flags: 0,
        surface: Some(Surface {
            kind,
            base: [-850, 850],
            top: [-1250, 1250],
            height: [-200, -200],
            side_height: [16, 24],
            ..Default::default()
        }),
    }
}
fn query(lateral: i32) -> Query {
    Query {
        progress: 2688,
        previous_progress: 2368,
        lateral: lateral * 256,
        previous_lateral: (lateral + 100) * 256,
        half_width: 7680,
        half_height: 10240,
        fallen_height: 2048,
        suspension: 1024,
        height: 0,
        contact: 74,
        category: 4,
        player: true,
        owner_crashed: false,
    }
}
fn motion() -> Motion {
    Motion {
        speed: 5000,
        lateral: 7000,
        vertical: 0,
        lean: 500,
        crashed: false,
        steering_gain: 0,
        elasticity: 0,
    }
}
fn resolve(
    current: &Boundary,
    previous: &Boundary,
    query: &Query,
    motion: &mut Motion,
) -> building_wall::Contact {
    building_wall::resolve_surface(
        Neighbors {
            current,
            next: current,
            previous,
            previous_next: previous,
        },
        query,
        motion,
    )
}

#[test]
fn mountain_player_respects_cliff_top_while_ai_respects_the_shoulder() {
    let mut wall = boundary(1);
    let mut q = query(1000);
    assert!(!resolve(&wall, &wall, &q, &mut motion()).hit);
    q.player = false;
    assert_eq!(resolve(&wall, &wall, &q, &mut motion()).lateral, 720 * 256);
    q.player = true;
    wall.surface.as_mut().unwrap().height[1] = 400;
    assert_eq!(resolve(&wall, &wall, &q, &mut motion()).lateral, 820 * 256);
}

#[test]
fn jumping_a_rail_keeps_native_crash_order_and_allows_clearance() {
    let mut rail = boundary(255);
    let surface = rail.surface.as_mut().unwrap();
    surface.event_type = 5;
    surface.rail_flags = 30;
    let mut q = query(750);
    q.contact = 0;
    let mut m = motion();
    let hit = resolve(&rail, &rail, &q, &mut m);
    assert!(hit.rail_crash && hit.crash);
    assert_eq!(
        hit.crash_lateral,
        Some(7000),
        "body inherits steering before the bike reflects"
    );
    assert_eq!(m.lateral, -2734);
    q.height = 7168;
    assert!(!resolve(&rail, &rail, &q, &mut motion()).hit);
    q.height = 0;
    q.category = 8;
    q.owner_crashed = true;
    q.half_height = 7680;
    let hit = resolve(&rail, &rail, &q, &mut motion());
    assert!(!hit.hit);
    assert_eq!(hit.lateral, q.lateral);
}

#[test]
fn adjacent_tunnel_panels_do_not_repeat_the_entry_angle_response() {
    let tunnel = boundary(3);
    let mut old = boundary(3);
    let q = query(740);
    let mut m = motion();
    m.lateral = 0;
    assert!(resolve(&tunnel, &old, &q, &mut m).crash);
    old.flags = 64;
    let mut m = motion();
    m.lateral = 0;
    assert!(!resolve(&tunnel, &old, &q, &mut m).crash);
}

#[test]
fn open_and_hill_ground_do_not_invent_a_solid_shoulder() {
    for kind in [4, 255] {
        let road = boundary(kind);
        assert!(!resolve(&road, &road, &query(2000), &mut motion()).hit);
    }
}

#[test]
fn legacy_boundary_bytes_omit_the_optional_surface_profile() {
    let bytes=br#"{"path":0,"building":true,"sample":10,"road":[-500,500],"outer":[-750,750],"selectors":[0,0],"flags":0}"#;
    let boundary: Boundary = serde_json::from_slice(bytes).unwrap();
    assert!(boundary.surface.is_none());
    assert_eq!(serde_json::to_vec(&boundary).unwrap(), bytes);
}
