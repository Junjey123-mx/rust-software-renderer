//! Ace Crystal planet: a world of luminous crystals. The ground is a field
//! of angular ridged shards shaded per triangle (every facet its own tint
//! of cyan, celeste, turquoise, white or lilac, with a hard specular glint
//! and an occasional twinkle), and clusters of large hexagonal crystals
//! with pointed tips rise out of it (`crystal_cluster`, baked like the
//! clover's trees). Heights are in planet radii; the relief never moves.

use crate::celestial::{ParamSpec, check_range, find_spec};
use crate::geometry::Mesh;
use crate::pipeline::Transform;
use crate::shading::lighting::{blinn_phong, fresnel, lambert};
use crate::shading::math::{hex, mix3, smoothstep};
use crate::shading::noise::{hash3, ridged};
use crate::solar::planet::clover::planet_params;
use raylib::math::Vector3;
use std::f32::consts::TAU;

/// Height of the tallest ground shard, radii.
pub const ACE_HEIGHT: f32 = 0.05;
/// Height of a crystal cluster at scale 1, planet radii.
pub const CRYSTAL_SIZE: f32 = 0.11;
/// Largest pose scale of a crystal (the hero spires).
pub const CRYSTAL_MAX_SCALE: f32 = 3.4;
/// Model height of the tallest cluster (tip included), model units.
pub const CLUSTER_HEIGHT: f32 = 1.05;
/// Bound of the ground with its crystals over the radius, radii: the
/// tallest spire on the tallest shard.
pub const ACE_RELIEF_BOUND: f32 =
    ACE_HEIGHT + CRYSTAL_MAX_SCALE * CRYSTAL_SIZE * CLUSTER_HEIGHT * 1.05;
/// Cluster variants (`crystal_cluster`).
pub const CRYSTAL_VARIANTS: u8 = 4;
/// Hero spires: the tallest crystals, a handful per planet.
pub const HERO_SPIRES: u32 = 9;

fn unit_hash(seed: u32, i: i32, j: i32, k: i32) -> f32 {
    (hash3(i, j, k, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// Height field of the Ace Crystal ground: sharpened ridged noise, thin
/// crests between angular hollows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AceField {
    /// Height of the tallest shard, radii.
    pub height: f32,
    /// Frequency of the shards on the unit sphere.
    pub frequency: f32,
    pub seed: u32,
}

impl AceField {
    /// Shards up to `ACE_HEIGHT` at frequency 4.5, for `seed`.
    pub fn new(seed: u32) -> Self {
        Self {
            height: ACE_HEIGHT,
            frequency: 4.5,
            seed,
        }
    }

    /// Checks a field built by hand: finite, height not negative,
    /// frequency positive.
    pub fn validate(&self) -> Result<(), String> {
        if !(self.height.is_finite() && self.frequency.is_finite()) {
            return Err("ace field values must be finite".into());
        }
        if self.height < 0.0 || self.frequency <= 0.0 {
            return Err("ace height must be >= 0 and frequency > 0".into());
        }
        Ok(())
    }

    /// Height above the radius along the unit `n`.
    pub fn height(&self, n: Vector3) -> f32 {
        let r = ridged(n * self.frequency, 3, self.seed);
        self.height.max(0.0) * r * r * r
    }

    /// Bound of `height`, radii.
    pub fn max_height(&self) -> f32 {
        self.height.max(0.0)
    }

    /// Digest of every parameter, for geometry keys.
    pub fn digest(&self) -> u64 {
        [self.height.to_bits(), self.frequency.to_bits(), self.seed]
            .iter()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
                (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
            })
    }
}

planet_params! {
    AceParams {
        seed: 41.0, 0.0, 9999.0, "seed of the shards and crystals (rounded)";
        glow: 0.22, 0.0, 1.0, "inner glow of the crystal (lights the night side)";
        sparkle: 0.9, 0.0, 1.0, "strength of the twinkling facets (0 = off)";
        twinkle_speed: 0.7, 0.0, 4.0, "twinkles per second of a facet";
        gloss: 70.0, 4.0, 400.0, "specular exponent of the facets";
    }
}

impl AceParams {
    /// The shards of this material's seed.
    pub fn field(&self) -> AceField {
        AceField::new(self.seed.round().max(0.0) as u32)
    }
}

pub const CYAN: Vector3 = hex(0x35E3FF);
pub const CELESTE: Vector3 = hex(0x9DEBFF);
pub const TURQUOISE: Vector3 = hex(0x1FD8C0);
pub const ICE_WHITE: Vector3 = hex(0xF0FCFF);
pub const LILAC: Vector3 = hex(0xB39BFF);
const DEEP: Vector3 = hex(0x0A3A70);
const AMBIENT: f32 = 0.08;

/// A value in `[0, 1)` for a facet: its normal quantized (every triangle
/// of the relief has its own), so a facet keeps one tint and one twinkle.
fn facet_hash(face_normal: Vector3, seed: u32, salt: i32) -> f32 {
    let q = |v: f32| (v * 211.0).round() as i32;
    unit_hash(
        seed ^ (salt as u32).wrapping_mul(0x9e37_79b9),
        q(face_normal.x),
        q(face_normal.y),
        q(face_normal.z),
    )
}

/// Tint of a facet from its hash: mostly cyan and celeste, then
/// turquoise, white and a few lilac facets.
pub fn facet_tint(h: f32) -> Vector3 {
    match h {
        h if h < 0.34 => CYAN,
        h if h < 0.6 => CELESTE,
        h if h < 0.8 => TURQUOISE,
        h if h < 0.93 => ICE_WHITE,
        _ => LILAC,
    }
}

/// Twinkle of a facet at time `t`: a short flash once per cycle, phase
/// and pace from its hash; only one facet in four ever twinkles.
fn twinkle(h: f32, h2: f32, t: f32, speed: f32) -> f32 {
    if h2 < 0.75 || !t.is_finite() {
        return 0.0;
    }
    let phase = (t * speed * (0.6 + 0.8 * h) + h2 * 17.0).rem_euclid(1.0);
    smoothstep(0.9, 0.97, phase) * (1.0 - smoothstep(0.97, 1.0, phase))
}

/// Shared look of a crystal facet with outward normal `normal`, eye `eye`
/// and light `light` (unit, same space): tinted body lit by the star, an
/// inner glow, a lilac-blue Fresnel rim, a hard white glint and a twinkle.
fn crystal_facet(
    tint: Vector3,
    normal: Vector3,
    light: Vector3,
    eye: Vector3,
    glow: f32,
    gloss: f32,
    flash: f32,
) -> Vector3 {
    let diffuse = lambert(normal, light);
    let rim = fresnel(normal, eye, 3.0);
    let glint = 1.6 * blinn_phong(normal, light, eye, gloss);
    // Light through the crystal: back faces still glow faintly.
    let through = 0.25 * (-normal.dot(light)).max(0.0);
    tint * (AMBIENT + (1.0 - AMBIENT) * diffuse + glow + through)
        + mix3(LILAC, CELESTE, 0.4) * (0.55 * rim)
        + ICE_WHITE * (glint + flash)
}

/// The crystal ground with its per-frame values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedAce {
    pub field: AceField,
    pub params: AceParams,
    /// Unit direction towards the star, object space.
    pub light: Vector3,
    pub time: f32,
}

impl PreparedAce {
    /// Color of the ground at `object_position` on the triangle of flat
    /// normal `face_normal` (object space), eye `eye`: each facet one
    /// tint, darker deep blue in the hollows between the shards.
    pub fn shade(&self, object_position: Vector3, face_normal: Vector3, eye: Vector3) -> Vector3 {
        let n = object_position.normalized();
        let p = &self.params;
        let seed = self.field.seed;
        let h = facet_hash(face_normal, seed, 1);
        let h2 = facet_hash(face_normal, seed, 2);
        let top = (self.field.height(n) / self.field.max_height().max(1e-6)).clamp(0.0, 1.0);
        let tint = mix3(DEEP, facet_tint(h), 0.45 + 0.55 * smoothstep(0.0, 0.5, top));
        let flash = 2.0 * p.sparkle.clamp(0.0, 1.0) * twinkle(h, h2, self.time, p.twinkle_speed);
        crystal_facet(
            tint,
            face_normal,
            self.light,
            eye,
            p.glow * 0.6,
            p.gloss,
            flash,
        )
    }
}

/// The surface crystals (`PropPart::Crystal`): the same facet look with a
/// stronger glow, in the host's object space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrystalShader {
    /// Unit direction towards the star, host object space.
    pub light: Vector3,
    /// Host transform (turns the world view direction into object space).
    pub transform: Transform,
    pub time: f32,
    pub params: AceParams,
}

impl CrystalShader {
    pub fn shade(
        &self,
        object_position: Vector3,
        face_normal: Vector3,
        view_direction: Vector3,
    ) -> Vector3 {
        let eye = self.transform.to_object_dir(view_direction).normalized();
        let p = &self.params;
        let seed = self.params.seed as u32;
        // One tint per crystal face (normal and coarse position).
        let q = |a: f32, b: f32| (a * 9.0).round() as i32 ^ (b * 97.0) as i32;
        let h = unit_hash(
            seed ^ 0xC1C1,
            q(object_position.x, face_normal.x),
            q(object_position.y, face_normal.y),
            q(object_position.z, face_normal.z),
        );
        let tint = if h < 0.45 {
            CYAN
        } else if h < 0.75 {
            CELESTE
        } else if h < 0.9 {
            TURQUOISE
        } else {
            LILAC
        };
        let flash =
            2.5 * p.sparkle.clamp(0.0, 1.0) * twinkle(h, 1.0 - h, self.time, p.twinkle_speed);
        crystal_facet(tint, face_normal, self.light, eye, p.glow, p.gloss, flash)
    }
}

/// Appends a hexagonal crystal to `out`: a prism of radius `r` from 0.12
/// below its foot to `height`, a pointed tip `tip` above it, its axis
/// `axis` (unit) from `foot`. Faces wound outward (as the icosphere).
fn prism(
    out: &mut (Vec<Vector3>, Vec<usize>),
    foot: Vector3,
    axis: Vector3,
    r: f32,
    height: f32,
    tip: f32,
    turn: f32,
) {
    let side = axis.cross(Vector3::new(0.31, 0.95, 0.07)).normalized();
    let front = side.cross(axis);
    let (vertices, indices) = out;
    let base = vertices.len();
    for k in 0..6 {
        let a = turn + TAU * k as f32 / 6.0;
        let ring = (side * a.cos() + front * a.sin()) * r;
        vertices.push(foot + ring - axis * 0.12);
        vertices.push(foot + ring + axis * height);
    }
    vertices.push(foot + axis * (height + tip));
    let apex = base + 12;
    let mut tri = |a: usize, b: usize, c: usize| {
        let (pa, pb, pc) = (vertices[a], vertices[b], vertices[c]);
        let centroid = (pa + pb + pc) / 3.0;
        let along = (centroid - foot).dot(axis);
        let outward = centroid - (foot + axis * along);
        if (pb - pa).cross(pc - pa).dot(outward) >= 0.0 {
            indices.extend([a, b, c]);
        } else {
            indices.extend([a, c, b]);
        }
    };
    for k in 0..6 {
        let (lo, hi) = (base + 2 * k, base + 2 * k + 1);
        let (lo2, hi2) = (base + 2 * ((k + 1) % 6), base + 2 * ((k + 1) % 6) + 1);
        tri(lo, lo2, hi);
        tri(hi, lo2, hi2);
        tri(hi, hi2, apex);
    }
}

/// Crystal cluster `variant` (out of range: wrapped), base at the origin,
/// +Y up, about `CLUSTER_HEIGHT` tall: a big central crystal and three to
/// five smaller ones leaning out around it. 18 triangles per crystal.
pub fn crystal_cluster(variant: u8) -> Mesh {
    let v = (variant % CRYSTAL_VARIANTS) as i32;
    let hash = |k: i32, j: i32| unit_hash(0xACE, v, k, j);
    let mut out = (Vec::new(), Vec::new());
    let up = Vector3::new(0.0, 1.0, 0.0);
    prism(
        &mut out,
        Vector3::zero(),
        up,
        0.13,
        0.72,
        0.3,
        TAU * hash(0, 0),
    );
    let satellites = 3 + (hash(0, 1) * 3.0) as i32;
    for k in 1..=satellites {
        let a = TAU * (k as f32 + 0.35 * hash(k, 0)) / satellites as f32;
        let out_dir = Vector3::new(a.cos(), 0.0, a.sin());
        let lean = 0.35 + 0.45 * hash(k, 1);
        let axis = (up + out_dir * lean).normalized();
        let r = 0.05 + 0.04 * hash(k, 2);
        let h = 0.25 + 0.3 * hash(k, 3);
        prism(
            &mut out,
            out_dir * (0.1 + 0.08 * hash(k, 4)),
            axis,
            r,
            h,
            0.45 * h,
            TAU * hash(k, 5),
        );
    }
    Mesh::new(out.0, out.1).expect("the crystal indices are valid")
}
