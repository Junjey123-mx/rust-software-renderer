//! Clover planet relief: four clover-shaped mountains (three round lobes
//! and a stem, smoothly joined) at the corners of a tetrahedron turned by
//! the seed, rising out of a low fbm seabed. Heights are in planet radii.

use crate::shading::math::smoothstep;
use crate::shading::noise::{fbm, hash3};
use crate::shading::sdf::{P2, circle, smooth_min, triangle};
use crate::solar::planet::field::tangent;
use raylib::math::Vector3;
use std::f32::consts::TAU;

/// Smallest angle between two clover centers (radians, 70 degrees).
pub const MIN_CENTER_ANGLE: f32 = 70.0 * std::f32::consts::PI / 180.0;
/// Typical peak height, radii (each mountain takes 0.75-1.25 of it).
pub const CLOVER_HEIGHT: f32 = 0.12;
/// Share of a mountain's height given to its rocky noise.
const ROCK_NOISE: f32 = 0.12;

/// Height field of the clover planet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloverField {
    /// Unit directions of the four mountains.
    pub centers: [Vector3; 4],
    /// Turn of each clover in its tangent plane, radians.
    pub turns: [f32; 4],
    /// Peak height of each mountain, radii.
    pub heights: [f32; 4],
    /// Radius of a lobe in the tangent plane (gnomonic units).
    pub lobe_r: f32,
    /// Length of the stem below the lobes, same units.
    pub stem: f32,
    /// Water level, radii.
    pub sea: f32,
    pub seed: u32,
}

/// A value in `[0, 1)` from the seed and two indices.
fn unit_hash(seed: u32, i: i32, j: i32) -> f32 {
    (hash3(i, j, 0x51ce, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// `v` turned by `angle` about the unit `axis` (Rodrigues).
fn rotate(v: Vector3, axis: Vector3, angle: f32) -> Vector3 {
    let (s, c) = angle.sin_cos();
    v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c))
}

/// Position of `n` in the plane tangent to the unit `center` (gnomonic
/// projection, `None` at 90 degrees or more so the plane never reaches the
/// far side of the planet).
pub fn tangent_uv(n: Vector3, center: Vector3) -> Option<P2> {
    let d = n.dot(center);
    if d.is_nan() || d <= 1e-3 {
        return None;
    }
    let t1 = tangent(center);
    let t2 = center.cross(t1);
    Some((n.dot(t1) / d, n.dot(t2) / d))
}

/// Signed distance to a clover of lobe radius `lobe_r` (negative inside):
/// three circles at 120 degrees around the origin and a widening stem
/// below, joined with a smooth minimum.
pub fn clover_sdf(p: P2, lobe_r: f32, stem: f32) -> f32 {
    let k = lobe_r * 0.35;
    let mut d = f32::INFINITY;
    for a in [90.0f32, 210.0, 330.0] {
        let a = a.to_radians();
        let c = (a.cos() * lobe_r * 0.95, a.sin() * lobe_r * 0.95);
        d = smooth_min(d, circle(p, c, lobe_r), k);
    }
    let bottom = -(lobe_r + stem);
    let tail = triangle(
        p,
        (0.0, 0.0),
        (-lobe_r * 0.28, bottom),
        (lobe_r * 0.28, bottom),
    );
    smooth_min(d, tail, k * 0.5)
}

impl CloverField {
    /// The clover planet of `seed`: a tetrahedron (109.47 degrees between
    /// centers, so never under `MIN_CENTER_ANGLE`) turned by three hashed
    /// angles, with hashed turns and heights.
    pub fn new(seed: u32) -> Self {
        let mut centers = [
            Vector3::new(1.0, 1.0, 1.0),
            Vector3::new(1.0, -1.0, -1.0),
            Vector3::new(-1.0, 1.0, -1.0),
            Vector3::new(-1.0, -1.0, 1.0),
        ];
        let axes = [
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];
        for c in &mut centers {
            *c = c.normalized();
            for (k, axis) in axes.iter().enumerate() {
                *c = rotate(*c, *axis, TAU * unit_hash(seed, 0, k as i32));
            }
            *c = c.normalized();
        }
        let turns = std::array::from_fn(|i| TAU * unit_hash(seed, 1, i as i32));
        let heights =
            std::array::from_fn(|i| CLOVER_HEIGHT * (0.75 + 0.5 * unit_hash(seed, 2, i as i32)));
        Self {
            centers,
            turns,
            heights,
            lobe_r: 0.16,
            stem: 0.14,
            sea: 0.02,
            seed,
        }
    }

    /// Checks a field built by hand: finite values, unit centers at least
    /// `MIN_CENTER_ANGLE` apart (closer mountains would merge).
    pub fn validate(&self) -> Result<(), String> {
        let finite = self
            .centers
            .iter()
            .all(|c| c.x.is_finite() && c.y.is_finite() && c.z.is_finite())
            && self
                .turns
                .iter()
                .chain(&self.heights)
                .all(|v| v.is_finite())
            && [self.lobe_r, self.stem, self.sea]
                .iter()
                .all(|v| v.is_finite());
        if !finite {
            return Err("clover field values must be finite".into());
        }
        for i in 0..4 {
            for j in i + 1..4 {
                let cos = self.centers[i]
                    .normalized()
                    .dot(self.centers[j].normalized());
                if cos.clamp(-1.0, 1.0).acos() < MIN_CENTER_ANGLE {
                    return Err(format!(
                        "clover centers {i} and {j} are closer than 70 degrees"
                    ));
                }
            }
        }
        Ok(())
    }

    /// The low seabed, everywhere below `sea` (from -0.032 to 0.008 R).
    fn seabed(&self, n: Vector3) -> f32 {
        -0.012 + 0.02 * fbm(n * 3.0, 4, self.seed.wrapping_add(7))
    }

    /// How deep `n` lies inside clover `i`, 0 outside to 1 at a lobe's
    /// core (and the plane coordinates).
    pub fn inside(&self, i: usize, n: Vector3) -> f32 {
        let Some((u, v)) = tangent_uv(n, self.centers[i]) else {
            return 0.0;
        };
        let (s, c) = self.turns[i].sin_cos();
        let p = (u * c - v * s, u * s + v * c);
        smoothstep(
            0.0,
            self.lobe_r * 0.9,
            -clover_sdf(p, self.lobe_r, self.stem),
        )
    }

    /// Height above the radius along the unit `n`.
    pub fn height(&self, n: Vector3) -> f32 {
        let mut h = self.seabed(n);
        for i in 0..4 {
            let inside = self.inside(i, n);
            if inside > 0.0 {
                let rock = 1.0 + ROCK_NOISE * fbm(n * 9.0, 3, self.seed.wrapping_add(i as u32));
                h = h.max(self.heights[i] * inside * rock);
            }
        }
        h
    }

    /// Bound of `height`, radii.
    pub fn max_height(&self) -> f32 {
        let peak = self.heights.iter().copied().fold(0.0, f32::max) * (1.0 + ROCK_NOISE);
        peak.max(self.sea).max(0.008)
    }

    /// Order-independent digest of every parameter, for geometry keys.
    pub fn digest(&self) -> u64 {
        let mut bits: Vec<u32> = self
            .centers
            .iter()
            .flat_map(|c| [c.x, c.y, c.z])
            .chain(self.turns)
            .chain(self.heights)
            .chain([self.lobe_r, self.stem, self.sea])
            .map(f32::to_bits)
            .collect();
        bits.push(self.seed);
        bits.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
            (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
}
