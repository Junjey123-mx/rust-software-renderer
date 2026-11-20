//! Waterfalls of the clover planet: a ribbon that leaves a lobe's edge where
//! it drops most towards the sea and runs down the field's gradient until
//! it meets the water, a little above the slope. Positions are in planet
//! radii (the planet's radius is 1).

use crate::geometry::{Mesh, Vertex};
use crate::shading::lighting::lambert;
use crate::shading::math::{hex, mix3, smoothstep};
use crate::shading::noise::{gradient3, hash3};
use crate::solar::planet::CloverField;
use crate::solar::planet::field::tangent;
use raylib::math::Vector3;
use std::f32::consts::TAU;
use std::sync::Arc;

/// Width of a ribbon, radii.
pub const WATERFALL_WIDTH: f32 = 0.02;
/// Height of the ribbon over the ground, radii.
pub const WATERFALL_GAP: f32 = 0.004;
/// Segments of a ribbon (two triangles each).
pub const WATERFALL_SEGMENTS: usize = 16;
/// Step of the gradient descent, radians.
const STEP: f32 = 0.002;
/// Azimuths searched around a clover for the start.
const AZIMUTHS: usize = 48;
/// Seeded restarts when the descent stalls above the sea.
const RESTARTS: u32 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct Waterfall {
    /// Center of the ribbon's first rung, on the lobe's edge.
    pub top: Vector3,
    /// Center of its last rung, at the sea.
    pub bottom: Vector3,
    pub width: f32,
    pub mesh: Arc<Mesh>,
}

fn unit_hash(seed: u32, i: i32, j: i32) -> f32 {
    (hash3(i, j, 0x7a7e, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// The unit `n` moved `angle` radians along the unit tangent `dir`.
fn walk(n: Vector3, dir: Vector3, angle: f32) -> Vector3 {
    (n * angle.cos() + dir * angle.sin()).normalized()
}

/// Gradient of the height at the unit `n`, in the tangent plane.
fn gradient(field: &CloverField, n: Vector3) -> Vector3 {
    const EPS: f32 = 1e-3;
    let t1 = tangent(n);
    let t2 = n.cross(t1);
    let at = |d: Vector3| field.height((n + d * EPS).normalized());
    t1 * ((at(t1) - at(-t1)) / (2.0 * EPS)) + t2 * ((at(t2) - at(-t2)) / (2.0 * EPS))
}

/// The start on clover `center`: for each azimuth, the coast and the point
/// a third of the way back towards the center; the one standing highest
/// over the sea (seeded jitter breaks ties) wins.
fn start(field: &CloverField, center: usize, seed: u32) -> Option<Vector3> {
    let c = field.centers[center];
    let (t1, t2) = (tangent(c), c.cross(tangent(c)));
    let mut best: Option<(f32, Vector3)> = None;
    for k in 0..AZIMUTHS {
        let a = TAU * k as f32 / AZIMUTHS as f32;
        let dir = t1 * a.cos() + t2 * a.sin();
        let Some(coast) = (1..300)
            .map(|s| s as f32 * STEP)
            .find(|&r| !field.is_land(walk(c, dir, r)))
        else {
            continue;
        };
        let p = walk(c, dir, coast * 2.0 / 3.0);
        let drop = field.height(p) - field.sea;
        let score = drop * (1.0 + 0.1 * unit_hash(seed, center as i32, k as i32));
        if drop > 0.0 && best.is_none_or(|(s, _)| score > s) {
            best = Some((score, p));
        }
    }
    best.map(|(_, p)| p)
}

/// Steepest descent from `from` to the sea: the unit directions, or `None`
/// when it stalls above the water.
fn descend(field: &CloverField, from: Vector3) -> Option<Vec<Vector3>> {
    let mut path = vec![from];
    let mut n = from;
    for _ in 0..2000 {
        if !field.is_land(n) {
            return Some(path);
        }
        let g = gradient(field, n);
        if g.length() < 1e-4 {
            return None;
        }
        n = walk(n, -g.normalized(), STEP);
        path.push(n);
    }
    None
}

/// `count + 1` points evenly spaced along the polyline `path`.
fn resample(path: &[Vector3], count: usize) -> Vec<Vector3> {
    let lengths: Vec<f32> = path.windows(2).map(|w| (w[1] - w[0]).length()).collect();
    let total: f32 = lengths.iter().sum();
    (0..=count)
        .map(|i| {
            let mut left = total * i as f32 / count as f32;
            for (k, &l) in lengths.iter().enumerate() {
                if left <= l || k + 1 == lengths.len() {
                    let t = if l > 0.0 { (left / l).min(1.0) } else { 0.0 };
                    return (path[k] + (path[k + 1] - path[k]) * t).normalized();
                }
                left -= l;
            }
            path[0]
        })
        .collect()
}

/// The waterfall of clover `center`: from the lobe's edge with the largest
/// drop down the gradient to the sea, `WATERFALL_GAP` over the ground. A
/// descent that stalls above the water restarts from a seeded jitter; an
/// error when no start reaches the sea (no floating ribbon).
pub fn waterfall_from(field: &CloverField, center: usize, seed: u32) -> Result<Waterfall, String> {
    if center >= field.centers.len() {
        return Err(format!("no clover {center}"));
    }
    let top = start(field, center, seed).ok_or("no lobe edge above the sea")?;
    let (t1, t2) = (tangent(top), top.cross(tangent(top)));
    let path = (0..=RESTARTS)
        .find_map(|r| {
            let from = if r == 0 {
                top
            } else {
                let a = TAU * unit_hash(seed, 100 + center as i32, r as i32);
                walk(top, t1 * a.cos() + t2 * a.sin(), 4.0 * STEP)
            };
            descend(field, from)
        })
        .ok_or("the descent never reaches the sea")?;
    let points = resample(&path, WATERFALL_SEGMENTS);
    let lift = |n: Vector3| n * (1.0 + field.height(n).max(field.sea) + WATERFALL_GAP);
    let mut vertices = Vec::with_capacity(2 * points.len());
    for (i, &n) in points.iter().enumerate() {
        let next = points[(i + 1).min(points.len() - 1)];
        let prev = points[i.saturating_sub(1)];
        let along = next - prev;
        let side = along.cross(n).normalized() * (WATERFALL_WIDTH / 2.0);
        let p = lift(n);
        vertices.push(lift((p - side).normalized()));
        vertices.push(lift((p + side).normalized()));
    }
    let mut indices = Vec::with_capacity(6 * WATERFALL_SEGMENTS);
    for i in 0..WATERFALL_SEGMENTS {
        let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        // Wound to face away from the planet.
        indices.extend([a, b, c, b, d, c]);
    }
    Ok(Waterfall {
        top: lift(points[0]),
        bottom: lift(points[WATERFALL_SEGMENTS]),
        width: WATERFALL_WIDTH,
        mesh: Arc::new(Mesh { vertices, indices }),
    })
}

/// Bands sliding down a ribbon per second (the default flow).
pub const FLOW_SPEED: f32 = 0.8;
/// Bands along a whole ribbon.
pub const FLOW_BANDS: f32 = 7.0;
/// Strength of the foam where the ribbon meets the sea.
pub const BASE_FOAM: f32 = 0.85;

const FALL_DEEP: Vector3 = hex(0x1C6FB8);
const FALL_LIGHT: Vector3 = hex(0x7FD3F5);
const FALL_FOAM: Vector3 = hex(0xF2FAFF);

/// Water running down a ribbon: light bands slide from the top to the
/// bottom at `speed` bands per second over `bands` bands, broken by a
/// little noise, with foam at the base. Opaque, lit by the star with a
/// moderate glow. `top`, `bottom` and `light` live in the mesh's space;
/// `time` is the scene clock (frozen while paused).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterfallShader {
    pub top: Vector3,
    pub bottom: Vector3,
    pub speed: f32,
    pub bands: f32,
    pub foam: f32,
    pub light: Vector3,
    pub time: f32,
}

impl Waterfall {
    /// Its shader at `time` with the default flow.
    pub fn shader(&self, light: Vector3, time: f32) -> WaterfallShader {
        WaterfallShader {
            top: self.top,
            bottom: self.bottom,
            speed: FLOW_SPEED,
            bands: FLOW_BANDS,
            foam: BASE_FOAM,
            light,
            time,
        }
    }
}

impl WaterfallShader {
    /// Where `p` lies along the ribbon: 0 at the top, 1 at the bottom
    /// (its projection on the top-bottom chord; the ribbon coordinate is
    /// recovered from the object position).
    pub fn along(&self, p: Vector3) -> f32 {
        let chord = self.bottom - self.top;
        let len2 = chord.dot(chord);
        if len2.is_nan() || len2 <= 0.0 {
            return 0.0;
        }
        ((p - self.top).dot(chord) / len2).clamp(0.0, 1.0)
    }

    pub fn shade(&self, p: Vector3, face_normal: Vector3) -> Vector3 {
        let v = self.along(p);
        // The time term is wrapped to one band first, so a long session
        // keeps its precision.
        let shift = (self.speed * self.time).rem_euclid(1.0);
        let wobble = 0.12 * gradient3(p * 60.0, 0x7a7e);
        let flow = (v * self.bands - shift + wobble).rem_euclid(1.0);
        let streak = smoothstep(0.55, 0.85, flow) * (1.0 - smoothstep(0.85, 1.0, flow));
        let water = mix3(FALL_DEEP, FALL_LIGHT, streak);
        let foam = self.foam * smoothstep(0.7, 1.0, v);
        let color = mix3(water, FALL_FOAM, foam.clamp(0.0, 1.0));
        color * (0.25 + 0.6 * lambert(face_normal, self.light) + 0.15)
    }
}

/// One waterfall ready to draw, in the host's mesh space.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedWaterfall {
    /// The clover it falls from.
    pub mountain: usize,
    /// The ribbon scaled to the host's mesh (top, bottom and width too).
    pub fall: Waterfall,
    /// Its triangles, one `Vertex` per corner.
    pub soup: Vec<Vertex>,
    /// Bounding sphere of the ribbon.
    pub bound_center: Vector3,
    pub bound_radius: f32,
    /// Outward direction at its middle, for the far-side test.
    pub normal: Vector3,
}

/// The waterfalls of a clover planet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WaterfallSet {
    pub falls: Vec<PlacedWaterfall>,
}

impl WaterfallSet {
    /// One fall per mountain of `field` (`per_mountain` above 1 counts as
    /// 1: a single steepest edge per mountain; 0 gives none), scaled to a
    /// host mesh of radius `radius`. A mountain whose descent finds no sea
    /// gets no fall.
    pub fn for_clover(field: &CloverField, per_mountain: u8, seed: u32, radius: f32) -> Self {
        if per_mountain == 0 {
            return Self::default();
        }
        let falls = (0..field.centers.len())
            .filter_map(|m| {
                let w = waterfall_from(field, m, seed).ok()?;
                let mesh = Mesh {
                    vertices: w.mesh.vertices.iter().map(|&v| v * radius).collect(),
                    indices: w.mesh.indices.clone(),
                };
                let soup = mesh.expand_vertices(Vector3::one());
                let bound_center = (w.top + w.bottom) * (0.5 * radius);
                let bound_radius = mesh
                    .vertices
                    .iter()
                    .map(|&v| (v - bound_center).length())
                    .fold(0.0, f32::max);
                Some(PlacedWaterfall {
                    mountain: m,
                    fall: Waterfall {
                        top: w.top * radius,
                        bottom: w.bottom * radius,
                        width: w.width * radius,
                        mesh: Arc::new(mesh),
                    },
                    soup,
                    bound_center,
                    bound_radius,
                    normal: bound_center.normalized(),
                })
            })
            .collect();
        Self { falls }
    }

    /// Farthest vertex from the host center, mesh units (0 without falls).
    pub fn reach(&self) -> f32 {
        self.falls
            .iter()
            .flat_map(|f| f.fall.mesh.vertices.iter())
            .map(|v| v.length())
            .fold(0.0, f32::max)
    }
}
