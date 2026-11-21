//! Tangent-space normal mapping in the software renderer. A normal map is
//! an RGB texture whose texels store unit normals as `rgb = n * 0.5 + 0.5`
//! (flat ground is the blue `(128, 128, 255)`). The fragment shader reads
//! it with bilinear filtering at texture coordinates of the surface point
//! and takes the texel's normal to object space through the TBN frame:
//! tangent `T` along `u` (east), bitangent `B` along `v` (north) and the
//! surface normal `N`, so `n = T * x + B * y + N * z`.
//!
//! The rock map of Clover Haven's mountains (`rock`) is baked once, on
//! first use, from a repeating height map: cracked plates (the borders of
//! periodic Voronoi cells) over a grain of periodic value noise.

use crate::shading::math::smoothstep;
use crate::shading::noise::hash3;
use crate::shading::sphere::tangent_frame;
use raylib::math::Vector3;
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

/// An RGB8 normal map of `size` x `size` texels, row by row, repeating in
/// both directions.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalMap {
    size: usize,
    rgb: Vec<[u8; 3]>,
}

/// A unit normal as the texel color `n * 0.5 + 0.5`.
pub fn encode(n: Vector3) -> [u8; 3] {
    [n.x, n.y, n.z].map(|c| ((c.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8)
}

/// The normal a texel color stores: `rgb * 2 - 1` (not normalized).
pub fn decode(texel: [u8; 3]) -> Vector3 {
    let c = |b: u8| b as f32 / 255.0 * 2.0 - 1.0;
    Vector3::new(c(texel[0]), c(texel[1]), c(texel[2]))
}

impl NormalMap {
    /// Bakes the normal map of `height` over the unit square (it must
    /// repeat with period 1): the slope by central differences between
    /// neighboring texels, wrapping at the edges, times `relief`, so the
    /// texel normal is `normalize(-relief * dh/du, -relief * dh/dv, 1)`.
    pub fn from_height(size: usize, relief: f32, height: impl Fn(f32, f32) -> f32) -> Self {
        let s = size as f32;
        let h: Vec<f32> = (0..size * size)
            .map(|k| height((k % size) as f32 / s, (k / size) as f32 / s))
            .collect();
        let at = |x: usize, y: usize| h[(y % size) * size + x % size];
        let rgb = (0..size * size)
            .map(|k| {
                let (x, y) = (k % size, k / size);
                let du = (at(x + 1, y) - at(x + size - 1, y)) * s / 2.0;
                let dv = (at(x, y + 1) - at(x, y + size - 1)) * s / 2.0;
                encode(Vector3::new(-relief * du, -relief * dv, 1.0).normalized())
            })
            .collect();
        Self { size, rgb }
    }

    /// Texels per side.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The color of texel (`x`, `y`), wrapping around.
    pub fn texel(&self, x: usize, y: usize) -> [u8; 3] {
        self.rgb[(y % self.size) * self.size + x % self.size]
    }

    /// Unit tangent-space normal at texture coordinates (`u`, `v`), one
    /// unit per repetition: the four nearest texels decoded and blended
    /// (bilinear filtering).
    pub fn sample(&self, u: f32, v: f32) -> Vector3 {
        let n = self.size as i64;
        let s = self.size as f32;
        let (x, y) = (u.rem_euclid(1.0) * s - 0.5, v.rem_euclid(1.0) * s - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at =
            |i: i64, j: i64| decode(self.rgb[(j.rem_euclid(n) * n + i.rem_euclid(n)) as usize]);
        let (i, j) = (x0 as i64, y0 as i64);
        let low = at(i, j) * (1.0 - fx) + at(i + 1, j) * fx;
        let high = at(i, j + 1) * (1.0 - fx) + at(i + 1, j + 1) * fx;
        (low * (1.0 - fy) + high * fy).normalized()
    }
}

/// Equirectangular texture coordinates of the unit direction `n`, with the
/// texture repeated `tiles` times around the equator and `tiles / 2` times
/// from pole to pole (square texels on the equator): `u` grows east and
/// `v` north, the directions of `tangent_frame`.
pub fn sphere_uv(n: Vector3, tiles: f32) -> (f32, f32) {
    let u = n.x.atan2(n.z) / TAU;
    let v = n.y.clamp(-1.0, 1.0).asin() / PI + 0.5;
    (u * tiles, v * tiles * 0.5)
}

/// `normal` (the surface normal at the unit direction `n`, object space)
/// perturbed by the tangent-space normal `texel` through the TBN frame:
/// `T` east and `B` north at `n`, made perpendicular to `normal`. With
/// `strength` 0 it is `normal`; with 1, the map's normal.
pub fn perturb(n: Vector3, normal: Vector3, texel: Vector3, strength: f32) -> Vector3 {
    let (east, _) = tangent_frame(n);
    let t = (east - normal * normal.dot(east)).normalized();
    let b = normal.cross(t);
    let mapped = t * texel.x + b * texel.y + normal * texel.z;
    (normal + (mapped - normal) * strength).normalized()
}

/// Texels per side of the rock map.
pub const ROCK_SIZE: usize = 256;
/// Slope multiplier of the rock map's bake.
pub const ROCK_RELIEF: f32 = 0.06;
/// Seed of the rock map's cells and grain.
pub const ROCK_SEED: u32 = 31;
/// Voronoi cells (rock plates) per side of the rock map.
const ROCK_CELLS: i32 = 6;

/// The rock normal map of Clover Haven's mountains, baked on first use.
pub fn rock() -> &'static NormalMap {
    static MAP: OnceLock<NormalMap> = OnceLock::new();
    MAP.get_or_init(|| NormalMap::from_height(ROCK_SIZE, ROCK_RELIEF, rock_height))
}

/// Height of the rock at (`x`, `y`) in the unit square, repeating: plates
/// with a grain of four octaves of value noise, cut by cracks along the
/// cell borders.
pub fn rock_height(x: f32, y: f32) -> f32 {
    let grain: f32 = (0..4)
        .map(|o| {
            let f = 8 << o;
            value_noise(x * f as f32, y * f as f32, f, ROCK_SEED + o as u32) / (1 << o) as f32
        })
        .sum();
    let c = ROCK_CELLS as f32;
    let crack = 1.0 - smoothstep(0.0, 0.2, cell_edge(x * c, y * c, ROCK_CELLS, ROCK_SEED));
    0.5 * grain - 0.45 * crack
}

/// Value noise with a lattice that repeats every `period` cells, in [0, 1].
fn value_noise(x: f32, y: f32, period: i32, seed: u32) -> f32 {
    let (i, j) = (x.floor(), y.floor());
    let (fx, fy) = (x - i, y - j);
    let (i, j) = (i as i32, j as i32);
    let at = |a: i32, b: i32| {
        let h = hash3(
            (i + a).rem_euclid(period),
            (j + b).rem_euclid(period),
            0,
            seed,
        );
        h as f32 / u32::MAX as f32
    };
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let low = at(0, 0) + (at(1, 0) - at(0, 0)) * sx;
    let high = at(0, 1) + (at(1, 1) - at(0, 1)) * sx;
    low + (high - low) * sy
}

/// How far (`x`, `y`) is from a border of the Voronoi cells of one seeded
/// point per lattice cell, repeating every `period` cells: the gap between
/// the two nearest points' distances (0 on a border).
fn cell_edge(x: f32, y: f32, period: i32, seed: u32) -> f32 {
    let (i, j) = (x.floor() as i32, y.floor() as i32);
    let (mut f1, mut f2) = (f32::MAX, f32::MAX);
    for b in -1..=1 {
        for a in -1..=1 {
            let (ci, cj) = (i + a, j + b);
            let h = hash3(ci.rem_euclid(period), cj.rem_euclid(period), 1, seed);
            let px = ci as f32 + (h & 0xffff) as f32 / 65535.0;
            let py = cj as f32 + (h >> 16) as f32 / 65535.0;
            let d = (px - x).hypot(py - y);
            if d < f1 {
                (f1, f2) = (d, f1);
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    f2 - f1
}
