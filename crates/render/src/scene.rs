use crate::{Result, camera::*, raster::*, source::*};
use rr_course::{Course, Sample, route::Route};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug)]
pub struct SceneOptions {
    pub course: String,
    pub level: u8,
    pub branch_mask: u32,
    pub seed: u32,
}
impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            course: "CITY".into(),
            level: 1,
            branch_mask: 0,
            seed: 1,
        }
    }
}
#[derive(Clone, Debug)]
pub struct TerrainSample {
    pub kind: TerrainKind,
    pub selectors: [u8; 2],
    pub points: [Vec<[f64; 2]>; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TerrainKind {
    None,
    Building,
    Hill,
    Wall,
}
#[derive(Clone)]
pub struct Object {
    pub id: String,
    pub section: u32,
    pub local: u32,
    pub lateral: f64,
    pub size: f64,
    pub mirror: bool,
    pub asset: SpriteAsset,
}
pub struct Scene {
    pub course: Course,
    pub route: Route,
    pub options: SceneOptions,
    pub assets: SourceAssets,
    pub terrain: Vec<Vec<TerrainSample>>,
    pub family_slots: Vec<Vec<[u16; 4]>>,
    pub objects: BTreeMap<(u32, u32), Vec<Object>>,
    pub notes: BTreeSet<String>,
    pub headings: Vec<i32>,
}
#[derive(Default, Serialize)]
pub struct RenderReceipt {
    pub rider_bounds: Option<[f64; 4]>,
    pub stats: Statistics,
    pub source_samples: usize,
    pub terrain_categories: BTreeMap<String, usize>,
    pub position_q8: f64,
    pub lateral: f64,
    pub width: u32,
    pub height: u32,
    pub framing_policy: String,
}
struct Random(u32);
impl Random {
    fn draw(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        (self.0 >> 16) & 32767
    }
    fn range(&mut self, a: i32, b: i32) -> i32 {
        a + (self.draw() % (b - a + 1) as u32) as i32
    }
}
#[derive(Clone, Copy, Default)]
struct HillPoint {
    start: i32,
    end: i32,
    x: i32,
    height: i32,
    dx: i32,
    dh: i32,
}
impl Scene {
    pub fn new(course: Course, assets: SourceAssets, options: SceneOptions) -> Result<Self> {
        if !(1..=5).contains(&options.level)
            || course.splits().len() > 16
            || options.branch_mask >= (1 << course.splits().len())
        {
            return Err("invalid scene level/branch selection".into());
        }
        let choices = (0..course.splits().len())
            .map(|i| ((options.branch_mask >> i) & 1) as u8)
            .collect::<Vec<_>>();
        let route = course.route(&choices)?;
        let mut headings = vec![];
        let mut heading = 0i32;
        for p in &route.points {
            headings.push(heading);
            heading = heading.wrapping_add(i32::from(p.path.bend).wrapping_mul(65536));
        }
        let mut family_slots = vec![];
        for s in &course.sections {
            let mut slots = [0; 4];
            let mut next = 0;
            let mut table = vec![];
            for sample in 0..s.samples.len() {
                let at = (sample as i64 + 33 + i64::from(s.family_bias)).max(0);
                while let Some(e) = s.family_events.get(next) {
                    if i64::from(e.sample) > at {
                        break;
                    }
                    if e.kind == 1 || e.kind == 3 {
                        slots[e.slot as usize] = e.family
                    }
                    next += 1;
                }
                table.push(slots)
            }
            family_slots.push(table)
        }
        let terrain = terrain_geometry(&course, options.seed);
        let placed = rr_course::scenery::place(&course, options.level, options.seed)?;
        let mut objects: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let mut notes = assets.notes.clone();
        for o in placed.instances {
            let (Some(lateral), Some(family)) = (o.lateral, o.family) else {
                continue;
            };
            if let Some(asset) = assets.families.get(&family).and_then(|f| {
                f.sprite(
                    if o.kind == "billboard" { 4 } else { 0 },
                    usize::from(o.selector & 63),
                )
            }) {
                objects
                    .entry((o.section, o.sample))
                    .or_default()
                    .push(Object {
                        id: o.id,
                        section: o.section,
                        local: o.sample,
                        lateral: f64::from(lateral),
                        size: f64::from(o.size) / 4.,
                        mirror: o.mirror,
                        asset: asset.clone(),
                    });
            } else {
                notes.insert(format!(
                    "unresolved static scenery family {family} selector {}",
                    o.selector
                ));
            }
        }
        for (si, section) in course.sections.iter().enumerate() {
            for (j, path) in section.samples.iter().enumerate() {
                let geometry = &terrain[si][j];
                if geometry.kind == TerrainKind::None {
                    continue;
                }
                for side in 0..2 {
                    let selector = geometry.selectors[side];
                    if selector == 255 {
                        continue;
                    }
                    let family = family_slots[si][j][usize::from(selector >> 6)];
                    let category = 1;
                    let available = assets
                        .families
                        .get(&family)
                        .and_then(|f| f.material(category, usize::from(selector & 63), None));
                    if available.is_none() {
                        notes.insert(format!("optional terrain family {family} category {category} selector {} unresolved; native ground fallback",selector&63));
                    }
                    let shoulder = path.flags[side];
                    if shoulder != 255 {
                        let id = family_slots[si][j][usize::from(shoulder >> 6)];
                        if assets
                            .families
                            .get(&id)
                            .and_then(|f| f.material(5, usize::from(shoulder & 63), Some(0)))
                            .is_none()
                        {
                            notes.insert(format!("optional shoulder family {id} selector {} unresolved; native ground fallback",shoulder&63));
                        }
                    }
                }
            }
        }
        Ok(Self {
            course,
            route,
            options,
            assets,
            terrain,
            family_slots,
            objects,
            notes,
            headings,
        })
    }
    fn material(
        &self,
        s: &Surface,
        selector: u8,
        category: usize,
        frame: Option<usize>,
    ) -> Option<&Material> {
        if selector == 255 {
            return None;
        }
        let id =
            self.family_slots[s.section as usize][s.local as usize][usize::from(selector >> 6)];
        self.assets
            .families
            .get(&id)?
            .material(category, usize::from(selector & 63), frame)
    }
    pub fn draw(
        &self,
        spec: &CameraSpec,
        view: &View,
        snapshot: &rr_simulation::Snapshot,
        enhanced: bool,
    ) -> Result<(Frame, RenderReceipt)> {
        self.draw_columns(spec, view, snapshot, enhanced, 0..spec.width)
    }
    /// Independent vertical stripes retain ground-anchor occlusion within each
    /// stripe. Every worker uses the same draw order and absolute pixel math.
    pub fn draw_parallel(
        &self,
        spec: &CameraSpec,
        view: &View,
        snapshot: &rr_simulation::Snapshot,
        enhanced: bool,
        workers: usize,
    ) -> Result<(Frame, RenderReceipt)> {
        let workers = workers.clamp(1, 8).min((spec.width / 32).max(1) as usize);
        if workers == 1 {
            return self.draw(spec, view, snapshot, enhanced);
        }
        std::thread::scope(|scope| {
            let mut jobs = vec![];
            for worker in 0..workers {
                let columns = (spec.width * worker as u32 / workers as u32)
                    ..(spec.width * (worker + 1) as u32 / workers as u32);
                let region = columns.clone();
                jobs.push((
                    columns,
                    scope.spawn(move || {
                        self.draw_columns(spec, view, snapshot, enhanced, region)
                            .map_err(|error| error.to_string())
                    }),
                ));
            }
            let (_, first) = jobs.remove(0);
            let (mut frame, mut receipt) = first.join().map_err(|_| "Raster worker panicked")??;
            for (columns, job) in jobs {
                let (stripe, _) = job.join().map_err(|_| "Raster worker panicked")??;
                for y in 0..spec.height {
                    let a = (y * spec.width + columns.start) as usize;
                    let b = (y * spec.width + columns.end) as usize;
                    frame.image.pixels[a * 4..b * 4]
                        .copy_from_slice(&stripe.image.pixels[a * 4..b * 4]);
                    frame.depth[a..b].copy_from_slice(&stripe.depth[a..b]);
                }
                frame.stats.pixels += stripe.stats.pixels;
            }
            receipt.stats = frame.stats;
            Ok((frame, receipt))
        })
    }
    fn draw_columns(
        &self,
        spec: &CameraSpec,
        view: &View,
        snapshot: &rr_simulation::Snapshot,
        enhanced: bool,
        columns: std::ops::Range<u32>,
    ) -> Result<(Frame, RenderReceipt)> {
        let slices = project(&self.route, spec, &self.assets.depths, view)?;
        let mut frame = Frame::new(spec.width, spec.height)?;
        frame.columns = columns;
        frame.clear([112, 150, 196, 255]);
        let first = (view.position_q8 / 256.).floor() as usize;
        let heading = self.headings[first]
            .wrapping_add(
                (f64::from(self.route.points[first].path.bend) * (view.position_q8 % 256.) * 256.)
                    as i32,
            )
            .wrapping_add(view.heading);
        draw_horizon(&mut frame, &self.assets.horizon, heading);
        let mut receipt = RenderReceipt {
            source_samples: slices.len(),
            position_q8: view.position_q8,
            lateral: view.lateral,
            width: spec.width,
            height: spec.height,
            framing_policy: "Hor+: fixed vertical scale; centered classic-safe HUD".into(),
            ..Default::default()
        };
        let mut billboards = vec![];
        for pair in slices.windows(2).rev() {
            let near = &pair[0];
            let far = &pair[1];
            for a in &far.roads {
                let b = near
                    .roads
                    .iter()
                    .find(|r| r.half == a.half)
                    .unwrap_or(&near.roads[near.own as usize]);
                let t = &self.terrain[b.section as usize][b.local as usize];
                let ft = &self.terrain[a.section as usize][a.local as usize];
                *receipt
                    .terrain_categories
                    .entry(format!("{:?}", t.kind))
                    .or_default() += 1;
                let q = |surface: &Surface, x: f64, y: f64, u: f64, v: f64| Vertex {
                    x: surface.x + x,
                    y: surface.y + y,
                    z: surface.z,
                    u,
                    v,
                };
                // A far ground sheet sits below local shoulders/roads; z testing clips it at hills.
                if b.half == near.own {
                    let grass = &self.assets.ground[0];
                    let width = 16000.;
                    frame.quad(
                        [
                            q(a, -width, -40., 0., 0.),
                            q(a, width, -40., 20., 0.),
                            q(b, width, -40., 20., 1.),
                            q(b, -width, -40., 0., 1.),
                        ],
                        grass,
                        spec,
                        &self.assets.depths,
                    );
                }
                for side in 0..2 {
                    let sign = if side == 0 { -1. } else { 1. };
                    let edge = sign * f64::from(b.path.width[side]);
                    let far_edge = sign * f64::from(a.path.width[side]);
                    let outer = edge + sign * f64::from(b.path.shoulder[side]);
                    let far_outer = far_edge + sign * f64::from(a.path.shoulder[side]);
                    let sh = f64::from(b.path.side_height[side]);
                    let fsh = f64::from(a.path.side_height[side]);
                    let shoulder = self
                        .material(b, b.path.flags[side], 5, Some(0))
                        .map(|m| &m.texture)
                        .unwrap_or(&self.assets.ground[1]);
                    frame.quad(
                        [
                            q(a, far_edge, 0., 0., 0.),
                            q(a, far_outer, fsh, 1., 0.),
                            q(b, outer, sh, 1., 1.),
                            q(b, edge, 0., 0., 1.),
                        ],
                        shoulder,
                        spec,
                        &self.assets.depths,
                    );
                    match t.kind {
                        TerrainKind::Hill | TerrainKind::Wall => {
                            let mut x = outer;
                            let mut y = sh;
                            let mut fx = far_outer;
                            let mut fy = fsh;
                            let fallback = &self.assets.ground[0];
                            for (band, point) in t.points[side].iter().enumerate() {
                                let other = ft.points[side].get(band).copied().unwrap_or(*point);
                                let nx = edge + sign * point[0];
                                let nfx = far_edge + sign * other[0];
                                let material = self
                                    .material(
                                        b,
                                        t.selectors[side],
                                        1,
                                        Some(if t.kind == TerrainKind::Wall {
                                            usize::from(band == 2)
                                        } else {
                                            band
                                        }),
                                    )
                                    .map(|m| &m.texture)
                                    .unwrap_or(fallback);
                                frame.quad(
                                    [
                                        q(a, fx, fy, 0., 0.),
                                        q(a, nfx, other[1], 1., 0.),
                                        q(b, nx, point[1], 1., 1.),
                                        q(b, x, y, 0., 1.),
                                    ],
                                    material,
                                    spec,
                                    &self.assets.depths,
                                );
                                x = nx;
                                y = point[1];
                                fx = nfx;
                                fy = other[1];
                            }
                            if t.kind == TerrainKind::Wall
                                && t.points[side].iter().any(|p| p[1] < 0.)
                            {
                                frame.quad(
                                    [
                                        q(a, far_outer, -f64::from(a.path.elevation), 0., 0.),
                                        q(
                                            a,
                                            far_outer + sign * 6000.,
                                            -f64::from(a.path.elevation),
                                            8.,
                                            0.,
                                        ),
                                        q(
                                            b,
                                            outer + sign * 6000.,
                                            -f64::from(b.path.elevation),
                                            8.,
                                            1.,
                                        ),
                                        q(b, outer, -f64::from(b.path.elevation), 0., 1.),
                                    ],
                                    &self.assets.water,
                                    spec,
                                    &self.assets.depths,
                                );
                            }
                        }
                        TerrainKind::Building => {
                            if let Some(material) = self.material(b, t.selectors[side], 1, None) {
                                let height = material.height;
                                if height >= 17. {
                                    let x = outer;
                                    let fx = far_outer;
                                    frame.quad(
                                        [
                                            q(a, fx, fsh, 0., 0.),
                                            q(a, fx, fsh + height, 0., 1.),
                                            q(b, x, sh + height, 1., 1.),
                                            q(b, x, sh, 1., 0.),
                                        ],
                                        &material.texture,
                                        spec,
                                        &self.assets.depths,
                                    );
                                    let roof = height / 2.;
                                    frame.quad(
                                        [
                                            q(a, fx, fsh + height, 0., 0.),
                                            q(a, fx + sign * roof, fsh + height, 1., 0.),
                                            q(b, x + sign * roof, sh + height, 1., 1.),
                                            q(b, x, sh + height, 0., 1.),
                                        ],
                                        &material.texture,
                                        spec,
                                        &self.assets.depths,
                                    );
                                }
                            }
                        }
                        TerrainKind::None => {}
                    }
                }
                let count = ((b.path.lanes_q8[0] + 255) / 256 + (b.path.lanes_q8[1] + 255) / 256)
                    .clamp(1, 16);
                let left = -f64::from(b.path.width[0]);
                let fl = -f64::from(a.path.width[0]);
                let right = f64::from(b.path.width[1]);
                let fr = f64::from(a.path.width[1]);
                for lane in 0..count {
                    let t0 = f64::from(lane) / f64::from(count);
                    let t1 = f64::from(lane + 1) / f64::from(count);
                    let material = road_material(&b.path, lane, count);
                    let phase = if b.path.marking_flags & 0x10 != 0 {
                        0.
                    } else {
                        (near.index % 2) as f64
                    };
                    let texture = &self.assets.road[material];
                    frame.quad(
                        [
                            q(a, fl + (fr - fl) * t0, 0., 0., phase),
                            q(a, fl + (fr - fl) * t1, 0., 1., phase),
                            q(b, left + (right - left) * t1, 0., 1., phase + 1.),
                            q(b, left + (right - left) * t0, 0., 0., phase + 1.),
                        ],
                        texture,
                        spec,
                        &self.assets.depths,
                    );
                }
                if let Some(objects) = self.objects.get(&(b.section, b.local)) {
                    for o in objects {
                        let ground = ground_height(t, &b.path, o.lateral);
                        billboards.push(Billboard {
                            id: o.id.clone(),
                            class: SpriteClass::Scenery,
                            x: contact_anchor(
                                b,
                                a,
                                b.z + 128.,
                                o.lateral,
                                [ground, ground_height(ft, &a.path, o.lateral)],
                                spec,
                                &self.assets.depths,
                            )[0],
                            ground: contact_anchor(
                                b,
                                a,
                                b.z + 128.,
                                o.lateral,
                                [ground, ground_height(ft, &a.path, o.lateral)],
                                spec,
                                &self.assets.depths,
                            )[1],
                            z: b.z + 128.,
                            width: o.asset.logical[0] * o.size,
                            height: o.asset.logical[1] * o.size,
                            pivot: o.asset.pivot,
                            texture: o.asset.texture.clone(),
                            mirror: o.mirror,
                            rotation: 0.,
                            clip_ground: true,
                        });
                    }
                }
            }
        }
        if snapshot.actors.iter().any(|a| a.id.0 == 1) {
            for agent in &snapshot.road_users {
                let Some(z) = crate::camera::road_user_depth(snapshot, agent, spec.player_depth_q8)
                else {
                    continue;
                };
                if z <= 256. || z > 8192. {
                    continue;
                }
                use rr_simulation::population::{AgentState, Kind};
                let sprite = match agent.kind {
                    Kind::Vehicle => self
                        .assets
                        .vehicles
                        .get(usize::from(agent.selector))
                        .and_then(|v| {
                            v.get(if agent.state == AgentState::Crossing {
                                1
                            } else if agent.direction < 0 {
                                0
                            } else {
                                2
                            })
                        })
                        .and_then(Option::as_ref),
                    Kind::Pedestrian => agent
                        .family
                        .and_then(|id| self.assets.families.get(&id))
                        .and_then(|f| {
                            f.pedestrian_frames
                                .get(&agent.selector)
                                .and_then(|frames| frames.get(agent.frame))
                                .and_then(Option::as_ref)
                        }),
                    Kind::Police => self.assets.police.get(agent.frame).and_then(Option::as_ref),
                };
                let Some(sprite) = sprite else {
                    continue;
                };
                let library = match agent.kind {
                    Kind::Pedestrian => self
                        .assets
                        .pedestrian_appearances
                        .iter()
                        .find(|a| Some(a.family) == agent.family && a.selector == agent.selector)
                        .map(|a| &a.animation),
                    Kind::Police => self.assets.police_animation.as_deref(),
                    Kind::Vehicle => None,
                };
                let bank = usize::from(agent.kind == Kind::Police);
                let view_flags = library
                    .and_then(|lib| {
                        let (code, index) = agent.animation.selected?;
                        let frame = lib.banks.get(bank)?.get(code)?.as_ref()?.get(index)?;
                        lib.select(bank, frame, 0, 0)
                            .ok()
                            .map(|(cell, _)| cell.flags)
                    })
                    .unwrap_or(0);
                let size = (if view_flags & 0x10000 != 0 { 2. } else { 1. })
                    * (if view_flags & 0x20000 != 0 { 4. } else { 1. });
                let anchor = actor_anchor(
                    self,
                    &slices,
                    z,
                    f64::from(agent.location.lateral) / 256.,
                    spec,
                    &self.assets.depths,
                );
                billboards.push(Billboard {
                    id: format!("population/{:020}", agent.id),
                    class: if agent.kind == Kind::Vehicle {
                        SpriteClass::Traffic
                    } else {
                        SpriteClass::Racer
                    },
                    x: anchor[0],
                    ground: anchor[1],
                    z,
                    width: sprite.logical[0] * 2. * size,
                    height: sprite.logical[1] * 2. * size,
                    pivot: sprite.pivot,
                    texture: sprite.texture.clone(),
                    mirror: (agent.direction < 0 && agent.kind == Kind::Pedestrian)
                        ^ (view_flags & 1 != 0),
                    rotation: if agent.state == AgentState::Hit {
                        std::f64::consts::FRAC_PI_2
                    } else {
                        0.
                    },
                    clip_ground: true,
                });
            }
        }
        for actor in &snapshot.actors {
            let Some(z) = relative_actor_depth(snapshot, actor, spec.player_depth_q8) else {
                continue;
            };
            if actor.id.0 == 1 || z > spec.near_q8 {
                let animation_assets = actor
                    .encounter
                    .as_ref()
                    .and_then(|u| self.assets.palettes.get(&u.color))
                    .or(self.assets.animations.as_ref());
                let animated = actor
                    .rider
                    .as_ref()
                    .and_then(|r| animation_assets?.body(r.frame));
                let sprite = animated.unwrap_or(&self.assets.rider);
                let mirrored = actor.rider.as_ref().is_some_and(|r| r.mirror);
                let pivot = if mirrored {
                    [1. - sprite.pivot[0], sprite.pivot[1]]
                } else {
                    sprite.pivot
                };
                let actor_scale = actor.rider.as_ref().map_or(1., |r| {
                    let a = if r.view_flags & 0x10000 != 0 { 2. } else { 1. };
                    a * if r.view_flags & 0x20000 != 0 { 4. } else { 1. }
                });
                let texture = if enhanced && animated.is_none() {
                    self.assets.enhanced.as_ref().unwrap_or(&sprite.texture)
                } else {
                    &sprite.texture
                };
                let lateral = if actor.id.0 == 1 {
                    view.lateral
                } else {
                    actor
                        .rider
                        .as_ref()
                        .map_or(actor.position.lateral.0 as f64 / 10., |r| {
                            f64::from(r.bike.lateral) / 256.
                        })
                };
                let mut anchor = actor_anchor(self, &slices, z, lateral, spec, &self.assets.depths);
                if let Some(rider) = &actor.rider {
                    if let Some(shadow) = animation_assets.and_then(|a| a.shadow(rider.frame)) {
                        billboards.push(Billboard {
                            id: format!("shadow/{:020}", actor.id.0),
                            class: SpriteClass::Shadow,
                            x: anchor[0],
                            ground: anchor[1],
                            z,
                            width: shadow.logical[0] * 2. * actor_scale,
                            height: shadow.logical[1] * 2. * actor_scale,
                            pivot: if rider.mirror {
                                [1. - shadow.pivot[0], shadow.pivot[1]]
                            } else {
                                shadow.pivot
                            },
                            texture: shadow.texture.clone(),
                            mirror: rider.mirror,
                            rotation: 0.,
                            clip_ground: false,
                        });
                    }
                    anchor[1] += f64::from(rider.bike.height) / 256.;
                }
                // Source empty BOB slots mean no body draw, never a stale fallback pose.
                if actor.rider.is_some() && self.assets.animations.is_some() && animated.is_none() {
                    continue;
                }
                if actor.id.0 == 1 {
                    let scale = self.assets.depths.scale(z);
                    let [width, height] = sprite.logical.map(|n| n * 2. * scale * actor_scale);
                    let x = spec.width as f64 / 2. + anchor[0] * scale;
                    let y = spec.horizon + (spec.camera_height - anchor[1]) * scale;
                    let left = x - width * pivot[0];
                    let top = y - height * pivot[1];
                    receipt.rider_bounds = Some([left, top, left + width, top + height]);
                }
                billboards.push(Billboard {
                    id: format!("actor/{:020}", actor.id.0),
                    class: SpriteClass::Racer,
                    x: anchor[0],
                    ground: anchor[1],
                    z,
                    // Source actor size 512: (size >> 6) / 4 = 2.
                    // Logical dimensions come from RRAN, never bitmap size.
                    width: sprite.logical[0] * 2. * actor_scale,
                    height: sprite.logical[1] * 2. * actor_scale,
                    pivot,
                    texture: texture.clone(),
                    mirror: actor.rider.as_ref().is_some_and(|r| r.mirror),
                    rotation: actor.rider.as_ref().map_or(0., |r| {
                        f64::from(r.roll) * std::f64::consts::TAU / 16777216.
                    }),
                    clip_ground: true,
                });
            }
        }
        draw_billboards(&mut frame, &mut billboards, spec, &self.assets.depths);
        draw_hud(&mut frame, spec, snapshot);
        receipt.stats = frame.stats;
        Ok((frame, receipt))
    }
}
pub fn road_material(path: &Sample, lane: i32, _count: i32) -> usize {
    let mut panel = lane;
    if panel > 0 {
        let left = (path.lanes_q8[0] + 255) / 256;
        if panel == left {
            panel = 2
        } else if path.lanes_q8[0] & 255 == 0 || panel > 1 {
            panel = if path.lanes_q8[1] & 255 == 0 {
                1
            } else {
                i32::from(panel < path.lanes_q8[1] / 256 + left)
            }
        } else {
            panel = 0
        }
    }
    let flags = path.marking_flags;
    if flags != 0 && flags & 0x40 == 0 {
        if flags & 0x18 != 0 {
            return (2 + panel) as usize;
        }
        if flags & 0x20 != 0 {
            return 1;
        }
    }
    (5 + panel) as usize
}
pub fn draw_horizon(frame: &mut Frame, texture: &Texture, heading: i32) {
    let quarter = heading / 4;
    let scroll = -((quarter.wrapping_add(quarter / 4)) >> 15);
    let width = frame.image.width;
    let image = &texture.image;
    let xoff = (i64::from(image.width) - i64::from(width)) / 2;
    for y in 0..320.min(frame.image.height) {
        for x in frame.columns.clone() {
            let sx =
                (i64::from(x) + xoff - i64::from(scroll)).rem_euclid(i64::from(image.width)) as u32;
            let sy = y.min(image.height - 1);
            let source = ((sy * image.width + sx) * 4) as usize;
            let dest = ((y * width + x) * 4) as usize;
            frame.image.pixels[dest..dest + 4].copy_from_slice(&image.pixels[source..source + 4]);
        }
    }
}
pub fn ground_height(t: &TerrainSample, path: &Sample, x: f64) -> f64 {
    let side = usize::from(x > 0.);
    let sign = if side == 0 { -1. } else { 1. };
    let distance = x * sign - f64::from(path.width[side]);
    if distance <= 0. {
        return 0.;
    }
    let mut previous = [
        f64::from(path.shoulder[side]),
        f64::from(path.side_height[side]),
    ];
    if distance <= previous[0] {
        return previous[1] * distance / previous[0].max(1.);
    }
    for point in &t.points[side] {
        if distance <= point[0] {
            let span = point[0] - previous[0];
            return if span.abs() < 1e-6 {
                point[1]
            } else {
                previous[1]
                    + (point[1] - previous[1]) * ((distance - previous[0]) / span).clamp(0., 1.)
            };
        }
        previous = *point
    }
    previous[1]
}
fn draw_hud(frame: &mut Frame, spec: &CameraSpec, snapshot: &rr_simulation::Snapshot) {
    let origin = spec.hud_origin();
    let digits = [
        0b111101101101111u16,
        0b010110010010111,
        0b111001111100111,
        0b111001111001111,
        0b101101111001001,
        0b111100111001111,
        0b111100111101111,
        0b111001001001001,
        0b111101111101111,
        0b111101111001111,
    ];
    let number = snapshot.speed_mm_per_tick.to_string();
    for (i, ch) in number.chars().enumerate() {
        let Some(digit) = ch.to_digit(10) else {
            continue;
        };
        for y in 0..5 {
            for x in 0..3 {
                if digits[digit as usize] & (1 << (14 - y * 3 - x)) != 0 {
                    for dy in 0..3 {
                        for dx in 0..3 {
                            let px = origin[0] + i as u32 * 12 + x * 3 + dx;
                            let py = origin[1] + y * 3 + dy;
                            let p = ((py * frame.image.width + px) * 4) as usize;
                            frame.image.pixels[p..p + 4].copy_from_slice(&[245, 240, 220, 255]);
                        }
                    }
                }
            }
        }
    }
}

/// Source wall/hill parameter walk on a private presentation RNG stream.
/// This precomputes a stable course view; it never consumes simulation RNG.
pub fn terrain_geometry(course: &Course, seed: u32) -> Vec<Vec<TerrainSample>> {
    let mut random = Random(seed);
    let mut out = vec![];
    for section in &course.sections {
        let mut samples = vec![];
        let mut wall_record = [0usize; 2];
        let mut wall_values = [[0i32; 3]; 2];
        let mut hill_record = 0usize;
        let mut points = [HillPoint::default(); 8];
        if let Some(t) = &section.terrain {
            if !t.wall_records[0].is_empty() {
                for (side, v) in wall_values.iter_mut().enumerate() {
                    let p = t.wall_records[side][0].parameters;
                    *v = [i32::from(p[2]), i32::from(p[7]) * 2, i32::from(p[4]) * 2]
                }
            }
            if let Some(h) = t.hills.first() {
                for (i, v) in points.iter_mut().enumerate() {
                    let p = h.points[i];
                    v.x = random.range(i32::from(p[0]) * 32, i32::from(p[1]) * 32);
                    v.height = random.range(i32::from(p[3] as i8) * 16, i32::from(p[4] as i8) * 16);
                }
            }
        }
        for (j, path) in section.samples.iter().enumerate() {
            let mut geom = TerrainSample {
                kind: TerrainKind::None,
                selectors: [255; 2],
                points: [vec![], vec![]],
            };
            if let Some(t) = &section.terrain {
                geom.selectors = t.selectors.get(j).copied().unwrap_or([255; 2]);
                match t.tag.as_str() {
                    "DLBR" => geom.kind = TerrainKind::Building,
                    "NTMR" | "DWDR" => {
                        geom.kind = TerrainKind::Wall;
                        for side in 0..2 {
                            let records = &t.wall_records[side];
                            if j > 0
                                && wall_record[side] + 1 < records.len()
                                && records[wall_record[side] + 1].start <= j as u32
                            {
                                wall_record[side] += 1
                            }
                            let r = &records[wall_record[side]];
                            let p = r.parameters;
                            let v = &mut wall_values[side];
                            if j > 0 {
                                v[0] = (v[0] + random.range(-i32::from(p[1]), i32::from(p[1])))
                                    .clamp(0, i32::from(p[2]) * 2);
                                v[1] = (v[1] + random.range(-i32::from(p[6]), i32::from(p[6])))
                                    .clamp(0, i32::from(p[7]) * 4);
                                v[2] = (v[2] + random.range(-i32::from(p[3]), i32::from(p[3])))
                                    .clamp(0, i32::from(p[4]) * 4);
                            }
                            let height = r.base_height + v[1];
                            let height = if height < 1 { -path.elevation } else { height };
                            let shoulder = path.shoulder[side];
                            let a = shoulder + v[0];
                            let b = shoulder + v[2] + (i32::from(p[2]) + i32::from(p[5]) * 2) * 2;
                            geom.points[side] = vec![
                                [f64::from(a), 0.],
                                [f64::from(b), f64::from(height)],
                                [f64::from(b + 800), f64::from(height)],
                            ];
                        }
                    }
                    "LIHR" => {
                        geom.kind = TerrainKind::Hill;
                        if j > 0 && hill_record + 1 < t.hills.len() {
                            if hill_record + 1 < t.hills.len()
                                && t.hills[hill_record + 1].start <= j as i32
                            {
                                hill_record += 1
                            }
                            let record = &t.hills[hill_record];
                            for (i, v) in points.iter_mut().enumerate() {
                                let p = record.points[i];
                                let at = j as i32;
                                if !(v.end > at && at > v.start) {
                                    let len = random
                                        .range(i32::from(p[6]), i32::from(p[7]))
                                        .min(record.end - at);
                                    v.start = at;
                                    v.end = at + len;
                                    if len == 0 {
                                        break;
                                    }
                                    let mut dx =
                                        random.range(-i32::from(p[2]) * 8, i32::from(p[2]) * 8);
                                    let target = v.x + dx * len;
                                    if target < i32::from(p[0]) * 32 {
                                        dx = (i32::from(p[0]) * 32 - v.x) / len
                                    } else if target > i32::from(p[1]) * 32 {
                                        dx = (i32::from(p[1]) * 32 - v.x) / len
                                    }
                                    v.dx = dx;
                                    let mut dh =
                                        random.range(-i32::from(p[5]) * 8, i32::from(p[5]) * 8);
                                    let target = v.height + dh * len;
                                    if target < i32::from(p[3] as i8) * 16 {
                                        dh = (i32::from(p[3] as i8) * 16 - v.height) / len
                                    } else if target > i32::from(p[4] as i8) * 16 {
                                        dh = (i32::from(p[4] as i8) * 16 - v.height) / len
                                    }
                                    v.dh = dh;
                                }
                            }
                            for p in &mut points {
                                p.x = p.x.wrapping_add(p.dx);
                                p.height = p.height.wrapping_add(p.dh);
                            }
                        }
                        geom.selectors = t.hills[hill_record].selectors;
                        for side in 0..2 {
                            geom.points[side] = (0..3)
                                .map(|i| {
                                    let p = points[side * 4 + i];
                                    [
                                        f64::from(path.shoulder[side] + p.x),
                                        if path.marking_flags != 0 && path.marking_flags & 0x40 == 0
                                        {
                                            0.
                                        } else {
                                            f64::from(p.height)
                                        },
                                    ]
                                })
                                .collect()
                        }
                    }
                    _ => {}
                }
            }
            samples.push(geom);
        }
        out.push(samples);
    }
    out
}

/// Bind a sprite foot to the same projected strip/depth convention as the road.
/// Source lookup scales are nonlinear near the camera, so interpolating raw
/// heights alone is insufficient for ground contact.
pub fn contact_anchor(
    near: &Surface,
    far: &Surface,
    z: f64,
    lateral: f64,
    ground: [f64; 2],
    spec: &CameraSpec,
    depths: &DepthTable,
) -> [f64; 2] {
    let inv = 1. / z;
    let beta = ((inv - 1. / near.z) / (1. / far.z - 1. / near.z)).clamp(0., 1.);
    let scale = depths.scale(z);
    if scale <= 0. {
        return [near.x + lateral, near.y + ground[0]];
    }
    let x0 = (near.x + lateral) * near.scale;
    let x1 = (far.x + lateral) * far.scale;
    let y0 = (spec.camera_height - near.y - ground[0]) * near.scale;
    let y1 = (spec.camera_height - far.y - ground[1]) * far.scale;
    [
        (x0 + (x1 - x0) * beta) / scale,
        spec.camera_height - (y0 + (y1 - y0) * beta) / scale,
    ]
}
fn actor_anchor(
    scene: &Scene,
    slices: &[Slice],
    z: f64,
    lateral: f64,
    spec: &CameraSpec,
    depths: &DepthTable,
) -> [f64; 2] {
    for pair in slices.windows(2) {
        if pair[0].roads[0].z <= z && z <= pair[1].roads[0].z {
            let a = &pair[0].roads[pair[0].own as usize];
            let b = &pair[1].roads[pair[1].own as usize];
            let near = ground_height(
                &scene.terrain[a.section as usize][a.local as usize],
                &a.path,
                lateral,
            );
            let far = ground_height(
                &scene.terrain[b.section as usize][b.local as usize],
                &b.path,
                lateral,
            );
            return contact_anchor(a, b, z, lateral, [near, far], spec, depths);
        }
    }
    let a = &slices[0].roads[slices[0].own as usize];
    [a.x + lateral, a.y]
}
