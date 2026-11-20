//! Surface props of a relief planet (trees, rocks): poses scattered over a
//! Fibonacci lattice with seeded jitter, kept where a seeded density noise,
//! the land, the slope and the spacing allow. Same seed, same poses.

use crate::shading::noise::{fbm, hash3};
use crate::solar::planet::HeightField;
use crate::solar::planet::field::NORMAL_EPS;
use raylib::math::Vector3;
use std::f32::consts::TAU;

/// Kind of surface model a pose places.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropKind {
    Tree,
    Rock,
}

/// Where props of one kind may stand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterRule {
    pub kind: PropKind,
    /// Lattice points tried over the whole sphere.
    pub candidates: u32,
    /// Frequency of the density noise on the unit sphere.
    pub density_freq: f32,
    /// Density noise (in `[-1, 1]`) a point must exceed.
    pub density_threshold: f32,
    /// Largest slope (1 - cos to the vertical) a prop stands on.
    pub max_slope: f32,
    /// Smallest angle between two props, radians.
    pub min_spacing: f32,
    /// Only above the sea.
    pub land_only: bool,
}

impl ScatterRule {
    /// Clover forest: hundreds of trees on the gentle land.
    pub const TREES: Self = Self {
        kind: PropKind::Tree,
        candidates: 6000,
        density_freq: 9.0,
        density_threshold: -0.15,
        max_slope: 0.2,
        min_spacing: 0.012,
        land_only: true,
    };
    /// Boulders on steeper, higher ground.
    pub const ROCKS: Self = Self {
        kind: PropKind::Rock,
        candidates: 3000,
        density_freq: 14.0,
        density_threshold: 0.1,
        max_slope: 0.6,
        min_spacing: 0.02,
        land_only: true,
    };
}

/// One prop on the planet, in its object space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropPose {
    /// Unit direction from the center to the foot.
    pub dir: Vector3,
    /// Unit "up" of the model: the field's normal at the foot.
    pub up: Vector3,
    /// Turn about `up`, radians.
    pub yaw: f32,
    /// Uniform scale of the model.
    pub scale: f32,
    pub kind: PropKind,
}

/// A value in `[0, 1)` from the seed and two indices.
fn unit_hash(seed: u32, i: u32, j: i32) -> f32 {
    (hash3(i as i32, j, 0x9e01, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// Point `i` of a Fibonacci lattice of `count` points, unit sphere.
fn lattice(i: u32, count: u32) -> Vector3 {
    let y = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
    let r = (1.0 - y * y).max(0.0).sqrt();
    let a = i as f32 * 2.399_963_2;
    Vector3::new(r * a.cos(), y, r * a.sin())
}

/// Poses of `rule` on `field` for `seed`. Never panics: a rule nothing can
/// satisfy (`max_slope` 0, no land) gives no props.
pub fn scatter(field: &HeightField, rule: &ScatterRule, seed: u32) -> Vec<PropPose> {
    let count = rule.candidates.max(1);
    // Lattice spacing: the jitter moves each point within its own cell.
    let cell = (4.0 * std::f32::consts::PI / count as f32).sqrt();
    let sea = field.sea_level().unwrap_or(f32::MIN);
    let cos_spacing = rule.min_spacing.cos();
    let mut poses: Vec<PropPose> = Vec::new();
    for i in 0..count {
        let base = lattice(i, count);
        let t1 = base.cross(Vector3::new(0.31, 0.95, 0.07)).normalized();
        let t2 = base.cross(t1);
        let (ju, jv) = (unit_hash(seed, i, 0) - 0.5, unit_hash(seed, i, 1) - 0.5);
        let n = (base + (t1 * ju + t2 * jv) * cell).normalized();
        if fbm(n * rule.density_freq, 3, seed) <= rule.density_threshold {
            continue;
        }
        if rule.land_only && field.height(n) <= sea {
            continue;
        }
        let up = field.normal(n, NORMAL_EPS);
        let slope = 1.0 - up.dot(n);
        if slope.is_nan() || slope >= rule.max_slope {
            continue;
        }
        // ponytail: O(accepted) scan per candidate (~6000 x a few hundred);
        // a spatial grid if the counts grow by 10x.
        if poses.iter().any(|p| p.dir.dot(n) > cos_spacing) {
            continue;
        }
        poses.push(PropPose {
            dir: n,
            up,
            yaw: TAU * unit_hash(seed, i, 2),
            scale: 0.8 + 0.4 * unit_hash(seed, i, 3),
            kind: rule.kind,
        });
    }
    poses
}
