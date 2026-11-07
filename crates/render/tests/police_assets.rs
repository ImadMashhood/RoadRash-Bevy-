use rr_render::rider::RiderAssets;
use serde_json::{Value, json};

#[test]
fn police_actor_uses_own_sheet_and_skips_linked_road_user_duplicate() {
    use rr_render::{CameraSpec, camera::View, fixture, raster::Texture, source::SpriteAsset};
    use rr_simulation::{ActorId, Simulation, encounter, fixtures, population};
    use std::sync::Arc;
    let mut scene = fixture::scene("straight").unwrap();
    let sprite = |color| SpriteAsset {
        texture: Texture::solid(color),
        logical: [20., 30.],
        pivot: [0.5, 1.],
    };
    scene.assets.police_rider = Some(Arc::new(RiderAssets {
        bodies: vec![Some(sprite([5, 245, 155, 255])); 64],
        shadows: vec![None; 64],
        color: 0xde,
    }));
    scene.assets.police = vec![Some(sprite([245, 5, 155, 255])); 64];
    let config = fixtures::riding();
    let setup = encounter::Setup::authored(&config).unwrap();
    let topology = setup.topology.clone();
    let mut snapshot = Simulation::with_encounters(1, config, setup)
        .unwrap()
        .snapshot();
    snapshot.actors[0].encounter.as_mut().unwrap().police = true;
    let spec = CameraSpec::default();
    let view = View {
        position_q8: 50. * 256.,
        ..Default::default()
    };
    let render = |snapshot: &rr_simulation::Snapshot, scene: &rr_render::Scene| {
        scene
            .draw(&spec, &view, snapshot, false)
            .unwrap()
            .0
            .image
            .pixels
    };
    let expected = render(&snapshot, &scene);
    assert!(expected.as_chunks::<4>().0.contains(&[5, 245, 155, 255]));
    let mut population = population::World::new(population::Setup::default());
    let id = population
        .spawn(
            &population::Spawn {
                id: 1,
                kind: population::Kind::Police,
                location: snapshot.actors[0]
                    .encounter
                    .as_ref()
                    .unwrap()
                    .recovery
                    .bike_location,
                family: None,
                selector: 0,
                subtype: 0,
                speed: 0,
                direction: 1,
            },
            0,
            &topology,
        )
        .unwrap()
        .unwrap();
    population.agents.get_mut(&id).unwrap().rider = Some(ActorId(1));
    snapshot.road_users = population.agents.into_values().collect();
    assert_eq!(render(&snapshot, &scene), expected);
    snapshot.road_users[0].rider = None;
    assert_ne!(
        render(&snapshot, &scene),
        expected,
        "unlinked legacy cop still draws"
    );
    snapshot.road_users.clear();
    Arc::get_mut(scene.assets.police_rider.as_mut().unwrap())
        .unwrap()
        .bodies
        .fill(None);
    assert!(
        !render(&snapshot, &scene)
            .as_chunks::<4>()
            .0
            .contains(&[230, 45, 60, 255]),
        "empty cop frame must not draw the generic rider fallback"
    );
    scene.assets.police_foot_rider = Some(Arc::new(RiderAssets {
        bodies: vec![Some(sprite([155, 245, 5, 255])); 400],
        shadows: vec![None; 400],
        color: 6,
    }));
    let config = fixtures::riding();
    let setup = encounter::Setup::authored(&config).unwrap();
    let mut falling = Simulation::with_encounters(1, config, setup).unwrap();
    falling
        .step(rr_simulation::Command {
            impact: Some(rr_simulation::recovery::Impact::default()),
            ..Default::default()
        })
        .unwrap();
    let mut body = falling.snapshot().actors[0]
        .encounter
        .as_ref()
        .unwrap()
        .recovery
        .body
        .clone()
        .unwrap();
    for code in 36..=41 {
        body.animation.selected = Some((code, 0));
        snapshot.actors[0].encounter.as_mut().unwrap().recovery.body = Some(body.clone());
        snapshot.actors[0].rider.as_mut().unwrap().frame = 289;
        let before = serde_json::to_value(&snapshot).unwrap();
        let image = render(&snapshot, &scene);
        assert!(
            image.as_chunks::<4>().0.contains(&[155, 245, 5, 255]),
            "foot clip {code} must use the upright source rider bank"
        );
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
    }
}

#[test]
#[ignore = "requires private native police sheets and reference observations"]
fn native_police_body_shadow_assets() {
    let data = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_DATA").unwrap());
    let bank = RiderAssets::load_police(&data).unwrap();
    let mut rows = vec![];
    for (number, sprites) in [(1, &bank.bodies), (3, &bank.shadows)] {
        assert_eq!(sprites.len(), 45);
        for (frame, sprite) in sprites.iter().enumerate() {
            let Some(sprite) = sprite else {
                rows.push(json!([number, frame, 0]));
                continue;
            };
            let hash = sprite
                .texture
                .image
                .pixels
                .iter()
                .fold(14695981039346656037u64, |hash, byte| {
                    (hash ^ u64::from(*byte)).wrapping_mul(1099511628211)
                });
            rows.push(json!([
                number,
                frame,
                1,
                sprite.texture.image.width,
                sprite.texture.image.height,
                sprite.logical[0] as i32,
                sprite.logical[1] as i32,
                (sprite.pivot[0] * sprite.logical[0]).round() as i32,
                (sprite.pivot[1] * sprite.logical[1]).round() as i32,
                format!("{hash:016x}")
            ]));
        }
    }
    let output = std::path::PathBuf::from(std::env::var_os("RR_NATIVE_OUTPUT").unwrap());
    std::fs::write(
        output.join("police-assets-rust.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
    let expected: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("RR_NATIVE_POLICE_ASSETS").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(json!(rows), expected);
}
