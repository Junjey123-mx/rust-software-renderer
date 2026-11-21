//! Surface props of a relief planet (trees, rocks): poses scattered over a
//! Fibonacci lattice with seeded jitter, kept where a seeded density noise,
//! the land, the slope and the spacing allow. Same seed, same poses.

use crate::geometry::{Mesh, Vertex};
use crate::shading::math::hex;
use crate::shading::noise::{fbm, hash3};
use crate::solar::attach::prop_meshes::{
    TOWER_VARIANTS, TREE_VARIANTS, rock_mesh, tower_mesh, tree_parts,
};
use crate::solar::planet::HeightField;
use crate::solar::planet::ace::{CRYSTAL_VARIANTS, crystal_cluster};
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
    /// A neon building of the Hearts city.
    Building,
    /// A crystal cluster of Ace Crystal.
    Crystal,
}

/// Which ground a prop may stand on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScatterMask {
    /// Anywhere the slope allows.
    Anywhere,
    /// Only above the sea.
    LandOnly,
    /// Only on flat terraces (`HeightField::is_plateau`).
    Plateau,
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
    /// Which ground counts.
    pub mask: ScatterMask,
    /// A cone left empty: unit direction and angular radius (radians),
    /// such as a landmark's footprint.
    pub exclude: Option<(Vector3, f32)>,
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
        mask: ScatterMask::LandOnly,
        exclude: None,
    };
    /// Boulders on steeper, higher ground.
    pub const ROCKS: Self = Self {
        kind: PropKind::Rock,
        candidates: 3000,
        density_freq: 14.0,
        density_threshold: 0.1,
        max_slope: 0.6,
        min_spacing: 0.02,
        mask: ScatterMask::LandOnly,
        exclude: None,
    };
    /// Hearts city: districts of buildings on the flat terraces.
    pub const BUILDINGS: Self = Self {
        kind: PropKind::Building,
        candidates: 1500,
        density_freq: 5.0,
        density_threshold: 0.05,
        max_slope: 0.05,
        min_spacing: 0.04,
        mask: ScatterMask::Plateau,
        exclude: None,
    };
    /// Ace Crystal: crystal clusters over most of the ground.
    pub const CRYSTALS: Self = Self {
        kind: PropKind::Crystal,
        candidates: 2400,
        density_freq: 3.5,
        density_threshold: 0.0,
        max_slope: 0.6,
        min_spacing: 0.085,
        mask: ScatterMask::Anywhere,
        exclude: None,
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
        let ground = match rule.mask {
            ScatterMask::Anywhere => true,
            ScatterMask::LandOnly => field.height(n) > sea,
            ScatterMask::Plateau => field.is_plateau(n),
        };
        if !ground || rule.exclude.is_some_and(|(dir, r)| n.dot(dir) > r.cos()) {
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

/// Size of the surface models on their planet, in planet radii (about
/// the height of a tree).
pub const PROP_SIZE: f32 = 0.05;

/// Sectors of a prop layer: the 12 vertices of the icosahedron, each prop
/// going to the nearest one.
pub const SECTOR_COUNT: usize = 12;
/// Below this projected radius of the host (pixels) no prop is drawn.
pub const PROP_MIN_PX: f32 = 60.0;

/// Whether a patch on a planet is hidden behind it: every point within
/// `angle` radians of the unit `dir` and at most `top` radii from the
/// center lies past the horizon of an eye `eye` radii away along the unit
/// `to_eye` (infinitely far under the orthographic camera), the planet
/// occluding as a sphere of `occluder` radii. Conservative: a patch whose
/// center is past the horizon but whose near edge (or a tall model on it)
/// still shows is kept.
pub fn beyond_horizon(
    to_eye: Vector3,
    eye: f32,
    dir: Vector3,
    angle: f32,
    top: f32,
    occluder: f32,
) -> bool {
    if eye.is_nan() || eye <= occluder || occluder <= 0.0 {
        return false;
    }
    let horizon = (occluder / eye).acos() + (occluder / top.max(occluder)).acos();
    dir.dot(to_eye).clamp(-1.0, 1.0).acos() - angle > horizon
}

/// Colored part of a model: each part is its own layer, since the shaded
/// stage carries no vertex color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropPart {
    TreeTrunk,
    TreeCanopy,
    Rock,
    BuildingBody,
    /// The buildings' neon edges (emissive).
    BuildingEdge,
    /// Ace Crystal's crystals (their own faceted shader,
    /// `planet::ace::CrystalShader`).
    Crystal,
}

impl PropPart {
    pub fn color(self) -> Vector3 {
        match self {
            Self::TreeTrunk => hex(0x6B4A2E),
            Self::TreeCanopy => hex(0x2E8B3E),
            Self::Rock => hex(0x8A8178),
            Self::BuildingBody => hex(0x3A1F5C),
            Self::BuildingEdge => hex(0xFF2E63),
            Self::Crystal => crate::solar::planet::ace::CYAN,
        }
    }

    /// Share of the color that glows instead of taking the light.
    pub fn glow(self) -> f32 {
        match self {
            Self::BuildingEdge => 1.0,
            _ => 0.0,
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
    /// Unit direction of every baked prop (where vehicles must not go).
    pub sites: Vec<Vector3>,
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
            PropKind::Building => {
                let variant = (pose.scale * 97.0) as u8 % TOWER_VARIANTS;
                let (body, edges) = tower_mesh(variant);
                add(PropPart::BuildingBody, sector, &body, &map);
                add(PropPart::BuildingEdge, sector, &edges, &map);
            }
            PropKind::Crystal => {
                let variant = (pose.scale * 97.0) as u8 % CRYSTAL_VARIANTS;
                add(PropPart::Crystal, sector, &crystal_cluster(variant), &map);
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
    PropLayers {
        layers,
        sites: poses.iter().map(|p| p.dir).collect(),
    }
}
