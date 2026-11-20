//! Surface props of a relief planet (trees, rocks): poses scattered over a
//! Fibonacci lattice with seeded jitter, kept where a seeded density noise,
//! the land, the slope and the spacing allow. Same seed, same poses.

use crate::geometry::{Mesh, Vertex};
use crate::shading::math::hex;
use crate::shading::noise::{fbm, hash3};
use crate::solar::attach::prop_meshes::{TREE_VARIANTS, rock_mesh, tree_parts};
use crate::solar::planet::HeightField;
use crate::solar::planet::field::NORMAL_EPS;
use crate::solar::planet::icosphere::icosphere;
use raylib::math::Vector3;
use std::f32::consts::TAU;
use std::sync::Arc;

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

/// Sectors of a prop layer: the 12 vertices of the icosahedron, each prop
/// going to the nearest one.
pub const SECTOR_COUNT: usize = 12;
/// Below this projected radius of the host (pixels) no prop is drawn.
pub const PROP_MIN_PX: f32 = 60.0;
/// A sector is drawn while its normal faces the eye at least this much
/// (cosine; slightly negative keeps the sectors on the horizon).
pub const SECTOR_FACING: f32 = -0.3;

/// Colored part of a model: each part is its own layer, since the shaded
/// stage carries no vertex color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropPart {
    TreeTrunk,
    TreeCanopy,
    Rock,
}

impl PropPart {
    pub fn color(self) -> Vector3 {
        match self {
            Self::TreeTrunk => hex(0x6B4A2E),
            Self::TreeCanopy => hex(0x2E8B3E),
            Self::Rock => hex(0x8A8178),
        }
    }
}

/// The props of one part inside one sector, baked into the planet's
/// object space as a single mesh.
#[derive(Debug, Clone)]
pub struct Sector {
    pub mesh: Arc<Vec<Vertex>>,
    /// Bounding sphere of the mesh, object space.
    pub bound_center: Vector3,
    pub bound_radius: f32,
    /// Unit direction of the sector (its icosahedron vertex).
    pub normal: Vector3,
    /// Props baked into it.
    pub props: usize,
}

/// One part's sectors (only the ones that hold props).
#[derive(Debug, Clone)]
pub struct PropLayer {
    pub part: PropPart,
    pub sectors: Vec<Sector>,
}

/// Every baked layer of a planet.
#[derive(Debug, Clone, Default)]
pub struct PropLayers {
    pub layers: Vec<PropLayer>,
}

impl PropLayers {
    /// Meshes to draw with every sector in view.
    pub fn sector_count(&self) -> usize {
        self.layers.iter().map(|l| l.sectors.len()).sum()
    }

    /// Reach of the layers from the planet center, object units.
    pub fn reach(&self) -> f32 {
        self.layers
            .iter()
            .flat_map(|l| &l.sectors)
            .map(|s| s.bound_center.length() + s.bound_radius)
            .fold(0.0, f32::max)
    }
}

/// Maps a model point (base at the origin, +Y up) onto the planet: the
/// foot on the surface of `field` (planet radius `radius`, object units),
/// +Y along the pose's `up`, turned by its yaw and scaled by `size` times
/// its scale.
pub fn frame_matrix(
    pose: &PropPose,
    field: &HeightField,
    radius: f32,
    size: f32,
) -> impl Fn(Vector3) -> Vector3 {
    let up = pose.up;
    let side = up.cross(Vector3::new(0.31, 0.95, 0.07)).normalized();
    let front = side.cross(up);
    let (s, c) = pose.yaw.sin_cos();
    let (x_axis, z_axis) = (side * c + front * s, front * c - side * s);
    let foot = pose.dir * (radius * field.surface_radius(pose.dir));
    let k = size * pose.scale;
    move |p: Vector3| foot + (x_axis * p.x + up * p.y + z_axis * p.z) * k
}

/// Bakes `poses` into one layer per part, each split into the sectors that
/// hold props (`SECTOR_COUNT` at most). Trees take a variant from the
/// pose's scale; every rock is its own seeded boulder.
pub fn bake(
    poses: &[PropPose],
    field: &HeightField,
    radius: f32,
    size: f32,
    seed: u32,
) -> PropLayers {
    let axes = icosphere(0).expect("level 0 is in range").positions;
    let mut buckets: Vec<((PropPart, usize), Vec<Vertex>, usize)> = Vec::new();
    let mut add = |part: PropPart, sector: usize, mesh: &Mesh, map: &dyn Fn(Vector3) -> Vector3| {
        let index = match buckets
            .iter()
            .position(|(key, _, _)| *key == (part, sector))
        {
            Some(i) => i,
            None => {
                buckets.push(((part, sector), Vec::new(), 0));
                buckets.len() - 1
            }
        };
        let (_, vertices, count) = &mut buckets[index];
        for &i in &mesh.indices {
            vertices.push(Vertex::new_with_color(
                map(mesh.vertices[i]),
                Vector3::one(),
            ));
        }
        *count += 1;
    };
    for (i, pose) in poses.iter().enumerate() {
        let sector = (0..axes.len())
            .max_by(|&a, &b| pose.dir.dot(axes[a]).total_cmp(&pose.dir.dot(axes[b])))
            .unwrap_or(0);
        let map = frame_matrix(pose, field, radius, size);
        match pose.kind {
            PropKind::Tree => {
                let variant = (pose.scale * 97.0) as u8 % TREE_VARIANTS;
                let [trunk, canopy] = tree_parts(variant);
                add(PropPart::TreeTrunk, sector, &trunk, &map);
                add(PropPart::TreeCanopy, sector, &canopy, &map);
            }
            PropKind::Rock => {
                let rock = rock_mesh(seed ^ (i as u32).wrapping_mul(0x9e37_79b9));
                add(PropPart::Rock, sector, &rock, &map);
            }
        }
    }
    let mut layers: Vec<PropLayer> = Vec::new();
    for ((part, sector), vertices, props) in buckets {
        let center = vertices.iter().fold(Vector3::zero(), |a, v| a + v.position)
            / vertices.len().max(1) as f32;
        let bound_radius = vertices
            .iter()
            .map(|v| (v.position - center).length())
            .fold(0.0, f32::max);
        let baked = Sector {
            mesh: Arc::new(vertices),
            bound_center: center,
            bound_radius,
            normal: axes[sector],
            props,
        };
        match layers.iter_mut().find(|l| l.part == part) {
            Some(layer) => layer.sectors.push(baked),
            None => layers.push(PropLayer {
                part,
                sectors: vec![baked],
            }),
        }
    }
    PropLayers { layers }
}
