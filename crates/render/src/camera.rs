use crate::Result;
use rr_course::{Sample, route::Route};
use serde::Serialize;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Framing {
    #[default]
    Classic,
    Wide,
}
#[derive(Clone, Debug, Serialize)]
pub struct CameraSpec {
    pub width: u32,
    pub height: u32,
    pub horizon: f64,
    pub camera_height: f64,
    pub near_q8: f64,
    pub slices: usize,
    pub player_depth_q8: f64,
    pub integer_scale: bool,
}
impl Default for CameraSpec {
    fn default() -> Self {
        Self::new(Framing::Classic, false)
    }
}
impl CameraSpec {
    pub fn new(framing: Framing, integer_scale: bool) -> Self {
        Self {
            width: if framing == Framing::Wide { 852 } else { 640 },
            height: 480,
            horizon: 180.,
            camera_height: 150.,
            near_q8: 1.,
            slices: 33,
            // The first road boundary is at 256; the player's sample is
            // one boundary farther on (the source camera trails by 256).
            player_depth_q8: 512.,
            integer_scale,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !(320..=1920).contains(&self.width)
            || !(240..=1080).contains(&self.height)
            || !self.horizon.is_finite()
            || !(0.0..self.height as f64).contains(&self.horizon)
            || !self.camera_height.is_finite()
            || !(1.0..=1000.).contains(&self.camera_height)
            || !self.near_q8.is_finite()
            || !(1.0..=256.).contains(&self.near_q8)
            || !(2..=65).contains(&self.slices)
            || !self.player_depth_q8.is_finite()
            || !(256.0..=1024.0).contains(&self.player_depth_q8)
        {
            return Err("invalid classic camera specification".into());
        }
        Ok(())
    }
    pub fn viewport(&self, physical: [u32; 2]) -> Option<([u32; 2], [u32; 2])> {
        if physical.contains(&0) {
            return None;
        }
        let mut scale = (f64::from(physical[0]) / f64::from(self.width))
            .min(f64::from(physical[1]) / f64::from(self.height));
        if self.integer_scale && scale >= 1. {
            scale = scale.floor()
        }
        let extent = [
            (f64::from(self.width) * scale).floor().max(1.) as u32,
            (f64::from(self.height) * scale).floor().max(1.) as u32,
        ];
        Some((
            [(physical[0] - extent[0]) / 2, (physical[1] - extent[1]) / 2],
            extent,
        ))
    }
    pub fn hud_origin(&self) -> [u32; 2] {
        [self.width.saturating_sub(640) / 2 + 16, 16]
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub position_q8: f64,
    pub lateral: f64,
    pub heading: i32,
}
impl Default for View {
    fn default() -> Self {
        Self {
            position_q8: 12800.,
            lateral: 0.,
            heading: 0,
        }
    }
}
/// Schema 3 reads source Q8 positions directly. Legacy scenarios retain the
/// original authored bridge (8 mm per source Q8, 10 mm per lane unit).
pub fn snapshot_view(
    previous: &rr_simulation::Snapshot,
    current: &rr_simulation::Snapshot,
    alpha: f32,
    start_q8: f64,
) -> View {
    let a = if alpha.is_finite() {
        f64::from(alpha.clamp(0., 1.))
    } else {
        0.
    };
    let mix = |x: f64, y: f64| x + (y - x) * a;
    if let (Some(old), Some(new)) = (
        previous
            .actors
            .iter()
            .find(|a| a.id.0 == 1)
            .and_then(|a| a.rider.as_ref()),
        current
            .actors
            .iter()
            .find(|a| a.id.0 == 1)
            .and_then(|a| a.rider.as_ref()),
    ) {
        return View {
            position_q8: start_q8 + mix(old.bike.distance_q8 as f64, new.bike.distance_q8 as f64),
            lateral: mix(f64::from(old.bike.lateral), f64::from(new.bike.lateral)) / 256.,
            heading: 0,
        };
    }
    View {
        position_q8: start_q8 + mix(previous.distance_mm as f64, current.distance_mm as f64) / 8.,
        lateral: mix(
            f64::from(previous.lateral_mm),
            f64::from(current.lateral_mm),
        ) / 10.,
        heading: 0,
    }
}
#[derive(Clone)]
pub struct DepthTable(pub Vec<i32>);
impl DepthTable {
    pub fn source(exe: &rr_formats::pe::SupportedExe<'_>) -> Result<Self> {
        let b = exe.read_va(0x46f098, 4352 * 4)?;
        let values = b
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| i32::from_le_bytes(*c))
            .collect::<Vec<_>>();
        if values[0] != 163840
            || values[256] != 32768
            || values.windows(2).any(|p| p[1] <= 0 || p[1] > p[0])
        {
            return Err("invalid supported projection depth table".into());
        }
        Ok(Self(values))
    }
    pub fn authored() -> Self {
        Self(
            (0..4352)
                .map(|i| {
                    if i == 0 {
                        163840
                    } else {
                        (16777216 / (i * 2)).min(163840)
                    }
                })
                .collect(),
        )
    }
    pub fn scale(&self, z: f64) -> f64 {
        if !z.is_finite() || z < 0. {
            return 0.;
        }
        let i = (z / 2.).floor() as usize;
        self.0.get(i).copied().unwrap_or(0) as f64 / 65536. * 2.
    }
}
#[derive(Clone, Debug)]
pub struct Surface {
    pub half: u8,
    pub section: u32,
    pub local: u32,
    pub path: Sample,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub scale: f64,
}
#[derive(Clone, Debug)]
pub struct Slice {
    pub index: usize,
    pub own: u8,
    pub roads: Vec<Surface>,
}
#[derive(Clone, Copy, Default)]
struct Track {
    x: f64,
    y: f64,
    dx: f64,
}
pub fn project(
    route: &Route,
    spec: &CameraSpec,
    depths: &DepthTable,
    view: &View,
) -> Result<Vec<Slice>> {
    spec.validate()?;
    if !view.position_q8.is_finite()
        || view.position_q8 < 0.
        || view.position_q8 >= route.len_q8() as f64
        || !view.lateral.is_finite()
        || view.lateral.abs() > 1e6
    {
        return Err("camera outside finite sampled route".into());
    }
    let first = (view.position_q8 / 256.).floor() as usize;
    let fraction = (view.position_q8 % 256.) / 256.;
    let base = &route.points[first];
    let camera_x = f64::from(base.offset) + view.lateral;
    let mut tracks = [Track::default(); 2];
    let mut previous_dual = false;
    let mut previous_half = 0;
    let mut slices = vec![];
    for (n, p) in route
        .points
        .iter()
        .enumerate()
        .skip(first)
        .take(spec.slices)
    {
        let offset = n - first;
        let dual = p.dual.as_ref();
        let own = dual.map_or(0, |d| d.half);
        if offset == 0 {
            let st = Track {
                x: 0.,
                y: -fraction * f64::from(p.path.rise),
                dx: f64::from(p.path.bend) * (1. - fraction),
            };
            tracks[own as usize] = st;
            if let Some(d) = dual {
                let sign = if own == 0 { 1. } else { -1. };
                tracks[1 - own as usize] = Track {
                    x: sign * f64::from(d.separation),
                    y: -fraction * f64::from(d.path.rise),
                    dx: st.dx + sign * f64::from(d.separation_delta),
                }
            }
        } else {
            if let Some(d) = dual.filter(|_| !previous_dual) {
                tracks[1] = tracks[0];
                if p.event != 2 {
                    if own == 0 {
                        tracks[1].x += f64::from(d.separation);
                        tracks[1].dx += f64::from(d.separation_delta)
                    } else {
                        tracks[0].x = tracks[1].x - f64::from(d.separation);
                        tracks[0].dx = tracks[1].dx - f64::from(d.separation_delta)
                    }
                }
            } else if dual.is_none() && previous_dual && previous_half == 1 && p.event != 1 {
                tracks[0] = tracks[1]
            }
            for (half, st) in tracks
                .iter_mut()
                .enumerate()
                .take(if dual.is_some() { 2 } else { 1 })
            {
                let path = if half == own as usize {
                    &p.path
                } else {
                    &dual.unwrap().path
                };
                st.dx += f64::from(path.bend);
            }
        }
        let z = 256. - fraction * 256. + offset as f64 * 256.;
        let mut roads = vec![];
        for (half, st) in tracks
            .iter_mut()
            .enumerate()
            .take(if dual.is_some() { 2 } else { 1 })
        {
            let (section, local, path, center) = if half == own as usize {
                (p.section, p.local, &p.path, p.offset)
            } else {
                let d = dual.unwrap();
                (d.section, d.local, &d.path, d.offset)
            };
            roads.push(Surface {
                half: half as u8,
                section,
                local,
                path: path.clone(),
                x: st.x + f64::from(center) - camera_x,
                y: st.y,
                z,
                scale: depths.scale(z),
            });
            st.x += st.dx;
            st.y += f64::from(path.rise);
        }
        slices.push(Slice {
            index: n,
            own,
            roads,
        });
        previous_dual = dual.is_some();
        previous_half = own;
    }
    Ok(slices)
}

/// Project linked recovery entities into disposable drawing proxies. Canonical IDs
/// and physical state remain in the simulation; the high-bit bike ID is render-only.
pub fn encounter_snapshot(source: &rr_simulation::Snapshot) -> rr_simulation::Snapshot {
    use rr_simulation::recovery::Phase;
    let mut view = source.clone();
    view.actors.clear();
    for original in &source.actors {
        let Some(unit) = &original.encounter else {
            view.actors.push(original.clone());
            continue;
        };
        let Some(body) = &unit.recovery.body else {
            view.actors.push(original.clone());
            continue;
        };
        if unit.recovery.phase != Phase::Mounting {
            let mut bike = original.clone();
            bike.id.0 |= 1u64 << 63;
            if let Some(rider) = &mut bike.rider {
                rider.frame = unit.recovery.bike_frame;
                rider.mirror = unit.recovery.bike_mirror;
                rider.roll = 0;
            }
            view.actors.push(bike);
        }
        let Some(progress) = body.projected else {
            continue;
        };
        let mut rider_view = original.clone();
        rider_view.position.progress.0 = progress * 8;
        rider_view.position.lateral.0 = i64::from(body.location.lateral) * 10 / 256;
        rider_view.position.section = rr_simulation::SectionId(body.location.path);
        rider_view.position.branch = rr_simulation::BranchId(body.branch);
        if let Some(rider) = &mut rider_view.rider {
            rider.bike = body.motion.clone();
            rider.frame = body.frame;
            rider.mirror = body.mirror;
            rider.view_flags = body.view_flags;
            rider.roll = body.roll;
        }
        if original.id.0 == 1 {
            view.distance_mm = (progress.max(0) * 8) as u64;
            view.lateral_mm = (i64::from(body.location.lateral) * 10 / 256) as i32;
        }
        view.actors.push(rider_view);
    }
    view
}

pub fn interpolated_snapshot(
    previous: &rr_simulation::Snapshot,
    current: &rr_simulation::Snapshot,
    alpha: f32,
) -> rr_simulation::Snapshot {
    let a = if alpha.is_finite() {
        f64::from(alpha.clamp(0., 1.))
    } else {
        0.
    };
    let mut out = current.clone();
    for actor in &mut out.actors {
        if let Some(old) = previous.actors.iter().find(|p| {
            p.id == actor.id
                && (p.position.route, p.position.branch, p.position.section)
                    == (
                        actor.position.route,
                        actor.position.branch,
                        actor.position.section,
                    )
        }) {
            if let (Some(old_rider), Some(rider)) = (&old.rider, &mut actor.rider) {
                rider.bike.lateral = (f64::from(old_rider.bike.lateral)
                    + (f64::from(rider.bike.lateral) - f64::from(old_rider.bike.lateral)) * a)
                    .round() as i32;
                rider.bike.height = (f64::from(old_rider.bike.height)
                    + (f64::from(rider.bike.height) - f64::from(old_rider.bike.height)) * a)
                    .round() as i32;
            }
            actor.position.lateral.0 = (old.position.lateral.0 as f64
                + (actor.position.lateral.0 as f64 - old.position.lateral.0 as f64) * a)
                .round() as i64;
            actor.position.progress.0 = (old.position.progress.0 as f64
                + (actor.position.progress.0 as f64 - old.position.progress.0 as f64) * a)
                .round() as i64;
        }
    }
    for agent in &mut out.road_users {
        if let Some(old) = previous
            .road_users
            .iter()
            .find(|old| old.id == agent.id && old.location.path == agent.location.path)
        {
            agent.location.progress = (old.location.progress as f64
                + (agent.location.progress - old.location.progress) as f64 * a)
                .round() as i64;
            agent.location.lateral = (f64::from(old.location.lateral)
                + f64::from(agent.location.lateral - old.location.lateral) * a)
                .round() as i32;
            if let (Some(old), Some(current)) = (old.projected, agent.projected) {
                agent.projected = Some((old as f64 + (current - old) as f64 * a).round() as i64);
            }
        }
    }
    out.distance_mm = (previous.distance_mm as f64
        + (current.distance_mm as f64 - previous.distance_mm as f64) * a)
        .round() as u64;
    out.lateral_mm = (f64::from(previous.lateral_mm)
        + (f64::from(current.lateral_mm) - f64::from(previous.lateral_mm)) * a)
        .round() as i32;
    out
}

/// A presentation proxy for the player; other actors may only compare local
/// progress within the same route/branch/section. A later race adapter must
/// provide cross-section visibility rather than treating unrelated local Zs
/// as global distance.
/// A selected-route projection remains valid across section boundaries; unknown
/// off-route locations have no projection and are never treated as a shortcut.
pub fn road_user_depth(
    snapshot: &rr_simulation::Snapshot,
    agent: &rr_simulation::population::Agent,
    player_depth: f64,
) -> Option<f64> {
    let player = snapshot.actors.iter().find(|a| a.id.0 == 1)?;
    Some(player_depth + agent.projected? as f64 - player.position.progress.0 as f64 / 8.)
}

pub fn relative_actor_depth(
    snapshot: &rr_simulation::Snapshot,
    actor: &rr_simulation::ActorSnapshot,
    player_depth: f64,
) -> Option<f64> {
    let player = snapshot.actors.iter().find(|a| a.id.0 == 1)?;
    if actor.id == player.id {
        return Some(player_depth);
    }
    let a = &actor.position;
    let p = &player.position;
    if (a.route, a.branch) != (p.route, p.branch)
        || (a.section != p.section && (actor.encounter.is_none() || player.encounter.is_none()))
    {
        return None;
    }
    Some(player_depth + (a.progress.0 as f64 - p.progress.0 as f64) / 8.)
}

#[cfg(test)]
mod recovery_tests {
    #[test]
    fn detached_proxies_follow_body_and_preserve_the_bike_without_domain_writes() {
        use rr_simulation::{Command, Simulation, encounter::Setup, fixtures, recovery::Impact};
        let c = fixtures::riding();
        let setup = Setup::authored(&c).unwrap();
        let mut sim = Simulation::with_encounters(1, c, setup).unwrap();
        sim.step(Command {
            impact: Some(Impact {
                rider_velocity: Some([5000, 0, 0]),
                bike_velocity: Some([0, 0, 0]),
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
        for _ in 0..5 {
            sim.step(Command::default()).unwrap();
        }
        let hash = sim.state_hash();
        let canonical = sim.snapshot();
        let view = super::encounter_snapshot(&canonical);
        assert_eq!(view.actors.len(), 2);
        let body = view.actors.iter().find(|a| a.id.0 == 1).unwrap();
        let bike = view
            .actors
            .iter()
            .find(|a| a.id.0 == (1u64 << 63) | 1)
            .unwrap();
        assert!(
            body.rider.as_ref().unwrap().bike.distance_q8
                > bike.rider.as_ref().unwrap().bike.distance_q8
        );
        assert_eq!(
            bike.rider.as_ref().unwrap().bike,
            canonical.actors[0].rider.as_ref().unwrap().bike
        );
        let camera = super::snapshot_view(&view, &view, 1., 12800.);
        assert_eq!(
            camera.position_q8,
            12800. + body.rider.as_ref().unwrap().bike.distance_q8 as f64
        );
        assert_eq!(sim.state_hash(), hash);
        assert_eq!(sim.snapshot(), canonical);
    }
}

#[cfg(test)]
mod population_tests {
    #[test]
    fn road_users_interpolate_in_copies_without_stepping_physics() {
        use rr_simulation::{population::*, topology::*, *};
        let mut s = Simulation::with_population(
            1,
            fixtures::riding(),
            encounter::Setup::authored(&fixtures::riding()).unwrap(),
            Setup {
                traffic: false,
                periodic_police: false,
                ..Default::default()
            },
        )
        .unwrap();
        s.spawn_road_user(&Spawn {
            id: 1,
            location: Location {
                progress: 2000,
                lateral: 64000,
                ..Default::default()
            },
            kind: Kind::Vehicle,
            family: None,
            selector: 0,
            subtype: 0,
            speed: 8000,
            direction: 1,
        })
        .unwrap();
        let before = s.snapshot();
        s.advance().unwrap();
        let current = s.snapshot();
        let hash = s.state_hash();
        let middle = super::interpolated_snapshot(&before, &current, 0.5);
        let progress = middle.road_users[0].location.progress;
        assert!(
            progress > before.road_users[0].location.progress
                && progress < current.road_users[0].location.progress
        );
        assert_eq!(s.state_hash(), hash);
        assert_eq!(s.snapshot(), current);
        let mut across = current.road_users[0].clone();
        across.location.path = 999;
        assert!(super::road_user_depth(&current, &across, 512.).is_some());
        across.projected = None;
        assert_eq!(super::road_user_depth(&current, &across, 512.), None);
    }
}
