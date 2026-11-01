use crate::{
    Result,
    camera::{CameraSpec, DepthTable},
};
use rr_formats::pixels::Rgba;
use serde::Serialize;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct Texture {
    pub image: Arc<Rgba>,
    pub repeat: bool,
    pub mipmaps: Vec<Arc<Rgba>>,
}
impl Texture {
    pub fn solid(c: [u8; 4]) -> Self {
        Self {
            image: Arc::new(Rgba {
                width: 1,
                height: 1,
                pixels: c.to_vec(),
            }),
            repeat: true,
            mipmaps: vec![],
        }
    }
    pub fn new(image: Rgba, repeat: bool) -> Result<Self> {
        image.validate()?;
        Ok(Self {
            image: Arc::new(image),
            repeat,
            mipmaps: vec![],
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub u: f64,
    pub v: f64,
}
impl Vertex {
    fn mix(a: Self, b: Self, t: f64) -> Self {
        Self {
            x: a.x + (b.x - a.x) * t,
            y: a.y + (b.y - a.y) * t,
            z: a.z + (b.z - a.z) * t,
            u: a.u + (b.u - a.u) * t,
            v: a.v + (b.v - a.v) * t,
        }
    }
}
#[derive(Clone, Copy, Default, Debug, Serialize)]
pub struct Statistics {
    pub submitted: usize,
    pub clipped: usize,
    pub degenerate: usize,
    pub pixels: usize,
    pub sprites: usize,
}
pub struct Frame {
    pub image: Rgba,
    pub depth: Vec<f32>,
    pub stats: Statistics,
    // Below a 2.5D sprite's hotspot, road pixels must not cut off the
    // bitmap extension. Probe occlusion at its ground anchor instead.
    sprite_ground_row: Option<u32>,
    pub(crate) columns: std::ops::Range<u32>,
}
impl Frame {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 || width > 1920 || height > 1080 {
            return Err("frame bounds".into());
        }
        Ok(Self {
            image: Rgba {
                width,
                height,
                pixels: vec![0; width as usize * height as usize * 4],
            },
            depth: vec![f32::INFINITY; width as usize * height as usize],
            stats: Statistics::default(),
            sprite_ground_row: None,
            columns: 0..width,
        })
    }
    pub fn clear(&mut self, c: [u8; 4]) {
        for p in self.image.pixels.as_chunks_mut::<4>().0 {
            p.copy_from_slice(&c)
        }
        self.depth.fill(f32::INFINITY);
        self.stats = Statistics::default();
    }
    pub fn quad(
        &mut self,
        vertices: [Vertex; 4],
        texture: &Texture,
        spec: &CameraSpec,
        depths: &DepthTable,
    ) {
        self.polygon(&vertices, texture, spec, depths)
    }
    pub fn polygon(
        &mut self,
        vertices: &[Vertex],
        texture: &Texture,
        spec: &CameraSpec,
        depths: &DepthTable,
    ) {
        self.stats.submitted += 1;
        if vertices.len() < 3
            || vertices.len() > 16
            || vertices.iter().any(|v| {
                ![v.x, v.y, v.z, v.u, v.v]
                    .iter()
                    .all(|n| n.is_finite() && n.abs() < 1e9)
            })
        {
            self.stats.degenerate += 1;
            return;
        }
        let clipped = clip_near(vertices, spec.near_q8);
        if clipped.len() != vertices.len() || vertices.iter().any(|v| v.z < spec.near_q8) {
            self.stats.clipped += 1
        }
        if clipped.len() < 3 {
            return;
        }
        let projected: Vec<_> = clipped
            .iter()
            .map(|v| {
                let scale = depths.scale(v.z);
                [
                    f64::from(spec.width) / 2. + v.x * scale,
                    spec.horizon + (spec.camera_height - v.y) * scale,
                    v.z,
                    v.u,
                    v.v,
                ]
            })
            .collect();
        for i in 1..projected.len() - 1 {
            self.triangle([projected[0], projected[i], projected[i + 1]], texture)
        }
    }
    fn triangle(&mut self, p: [[f64; 5]; 3], texture: &Texture) {
        let span = |axis: usize| {
            p.iter().map(|v| v[axis]).fold(f64::NEG_INFINITY, f64::max)
                - p.iter().map(|v| v[axis]).fold(f64::INFINITY, f64::min)
        };
        let ratio = (span(3) * f64::from(texture.image.width) / span(0).max(1.))
            .max(span(4) * f64::from(texture.image.height) / span(1).max(1.));
        let level = if ratio > 1. {
            ratio.log2().floor() as usize
        } else {
            0
        };
        let image = texture
            .mipmaps
            .get(level.min(texture.mipmaps.len().saturating_sub(1)))
            .unwrap_or(&texture.image);

        let edge = |a: [f64; 5], b: [f64; 5], x: f64, y: f64| {
            (x - a[0]) * (b[1] - a[1]) - (y - a[1]) * (b[0] - a[0])
        };
        let area = edge(p[0], p[1], p[2][0], p[2][1]);
        if !area.is_finite() || area.abs() < 1e-8 {
            self.stats.degenerate += 1;
            return;
        }
        let min_x = p
            .iter()
            .map(|v| v[0])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .clamp(0., f64::from(self.image.width)) as u32;
        let max_x = p
            .iter()
            .map(|v| v[0])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .clamp(0., f64::from(self.image.width)) as u32;
        let min_y = p
            .iter()
            .map(|v| v[1])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .clamp(0., f64::from(self.image.height)) as u32;
        let max_y = p
            .iter()
            .map(|v| v[1])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .clamp(0., f64::from(self.image.height)) as u32;
        for y in min_y..max_y {
            for x in min_x.max(self.columns.start)..max_x.min(self.columns.end) {
                let fx = f64::from(x) + 0.5;
                let fy = f64::from(y) + 0.5;
                let w = [
                    edge(p[1], p[2], fx, fy) / area,
                    edge(p[2], p[0], fx, fy) / area,
                    edge(p[0], p[1], fx, fy) / area,
                ];
                if w.iter().any(|v| *v < -1e-8) {
                    continue;
                }
                let inv = w.iter().enumerate().map(|(i, w)| w / p[i][2]).sum::<f64>();
                if inv <= 0. || !inv.is_finite() {
                    continue;
                }
                let z = (1. / inv) as f32;
                let index = (y * self.image.width + x) as usize;
                let occlusion_index = self
                    .sprite_ground_row
                    .filter(|row| y >= *row)
                    .map_or(index, |row| (row * self.image.width + x) as usize);
                if z > self.depth[occlusion_index] + 0.01 {
                    continue;
                }
                let uv = |axis| {
                    w.iter()
                        .enumerate()
                        .map(|(i, w)| w * p[i][axis] / p[i][2])
                        .sum::<f64>()
                        / inv
                };
                let texel = |v: f64, size: u32| {
                    let v = if texture.repeat {
                        v.rem_euclid(1.)
                    } else {
                        v.clamp(0., 1. - f64::EPSILON)
                    };
                    (v * f64::from(size)).floor().clamp(0., f64::from(size - 1)) as u32
                };
                let tx = texel(uv(3), image.width);
                let ty = texel(uv(4), image.height);
                let source = ((ty * image.width + tx) * 4) as usize;
                let pixel = &image.pixels[source..source + 4];
                if pixel[3] == 0 {
                    continue;
                }
                let dst = &mut self.image.pixels[index * 4..index * 4 + 4];
                if pixel[3] == 255 {
                    dst.copy_from_slice(pixel)
                } else {
                    let a = u32::from(pixel[3]);
                    for c in 0..3 {
                        dst[c] = ((u32::from(pixel[c]) * a + u32::from(dst[c]) * (255 - a) + 127)
                            / 255) as u8
                    }
                    dst[3] = 255
                }
                self.depth[index] = z;
                self.stats.pixels += 1;
            }
        }
    }
}
pub fn clip_near(vertices: &[Vertex], near: f64) -> Vec<Vertex> {
    if vertices.is_empty() {
        return vec![];
    }
    let mut result = vec![];
    let mut previous = *vertices.last().unwrap();
    for &current in vertices {
        let a = previous.z >= near;
        let b = current.z >= near;
        if a != b {
            let t = (near - previous.z) / (current.z - previous.z);
            result.push(Vertex::mix(previous, current, t))
        }
        if b {
            result.push(current)
        }
        previous = current;
    }
    result
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpriteClass {
    Shadow,
    Scenery,
    Traffic,
    Racer,
}
#[derive(Clone)]
pub struct Billboard {
    pub id: String,
    pub class: SpriteClass,
    pub x: f64,
    pub ground: f64,
    pub z: f64,
    pub width: f64,
    pub height: f64,
    pub pivot: [f64; 2],
    pub texture: Texture,
    pub mirror: bool,
    pub rotation: f64,
    pub clip_ground: bool,
}
pub fn draw_billboards(
    frame: &mut Frame,
    billboards: &mut [Billboard],
    spec: &CameraSpec,
    depths: &DepthTable,
) {
    billboards.sort_by(|a, b| {
        b.z.total_cmp(&a.z)
            .then(a.class.cmp(&b.class))
            .then(a.id.cmp(&b.id))
    });
    for b in billboards {
        if b.z < spec.near_q8 || !b.z.is_finite() || b.width <= 0. || b.height <= 0. {
            continue;
        }
        let x = b.x - b.pivot[0] * b.width;
        let y = b.ground - (1. - b.pivot[1]) * b.height;
        let ground_row = spec.horizon + (spec.camera_height - b.ground) * depths.scale(b.z);
        frame.sprite_ground_row =
            (b.clip_ground && ground_row.is_finite() && ground_row < spec.height as f64)
                .then(|| (ground_row - 1.).max(0.).floor() as u32);
        let (u0, u1) = if b.mirror { (1., 0.) } else { (0., 1.) };
        let mut vertices = [
            Vertex {
                x,
                y: y + b.height,
                z: b.z,
                u: u0,
                v: 0.,
            },
            Vertex {
                x: x + b.width,
                y: y + b.height,
                z: b.z,
                u: u1,
                v: 0.,
            },
            Vertex {
                x: x + b.width,
                y,
                z: b.z,
                u: u1,
                v: 1.,
            },
            Vertex {
                x,
                y,
                z: b.z,
                u: u0,
                v: 1.,
            },
        ];
        if b.rotation != 0. && b.rotation.is_finite() {
            let (sin, cos) = b.rotation.sin_cos();
            for v in &mut vertices {
                let dx = v.x - b.x;
                let dy = v.y - b.ground;
                v.x = b.x + dx * cos - dy * sin;
                v.y = b.ground + dx * sin + dy * cos;
            }
        }
        frame.quad(vertices, &b.texture, spec, depths);
        frame.stats.sprites += 1;
        frame.sprite_ground_row = None;
    }
}
