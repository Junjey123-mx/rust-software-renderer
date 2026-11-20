//! A planet's landmark: one large model standing on the surface (the Hearts
//! pyramid), baked into the host's mesh space at load, upright on the
//! field's normal. A direction that falls off a plateau moves to the
//! nearest one.

use crate::geometry::{Mesh, Vertex};
use crate::pipeline::Transform;
use crate::solar::attach::prop_meshes::{heart_disc_mesh, pyramid_mesh};
use crate::solar::attach::props::{PropKind, PropPose, frame_matrix};
use crate::solar::planet::HeightField;
use crate::solar::planet::field::{NORMAL_EPS, tangent};
use crate::solar::shapes::frame_matrix as axes_matrix;
use raylib::math::{Matrix, Vector3};
use std::sync::Arc;

/// Height of the Hearts pyramid, planet radii.
pub const PYRAMID_HEIGHT: f32 = 0.45;
/// Steps of the Hearts pyramid.
pub const PYRAMID_LEVELS: u8 = 5;
/// Share of the model's height sunk below the ground, so its base never
/// floats over a lower terrace.
const SINK: f32 = 0.06;
/// Farthest a landmark moves to find a plateau, radians.
const SEARCH: f32 = 0.6;
/// Width of the heart disc, planet radii.
pub const DISC_SIZE: f32 = 0.22;
/// Height of the disc's center over the pyramid's top, planet radii.
pub const DISC_LIFT: f32 = 0.17;
/// Turn of the disc about the surface normal, radians per second.
pub const DISC_SPIN: f32 = 0.8;
/// Outline points of the disc (4 * 20 + 80 = 160 triangles).
const DISC_SAMPLES: u32 = 20;

#[derive(Debug, Clone)]
pub struct Landmark {
    /// Unit direction of its foot (on a plateau).
    pub dir: Vector3,
    /// Its "up": the field's normal at the foot.
    pub up: Vector3,
    /// The structure and its neon edges, the host's mesh space.
    pub mesh: Arc<Vec<Vertex>>,
    pub edges: Arc<Vec<Vertex>>,
    /// Center of its flat top, the host's mesh space (where the heart disc
    /// sits).
    pub top_local: Vector3,
    /// Farthest vertex from the host center, the host's mesh units.
    pub reach: f32,
    /// Bounding sphere of the structure and its disc, the host's mesh
    /// space.
    pub bound_center: Vector3,
    pub bound_radius: f32,
    /// The heart disc in its own frame (`heart_disc_mesh`), drawn with
    /// `disc_matrix`.
    pub disc: Arc<Vec<Vertex>>,
    /// Center of the disc, the host's mesh space.
    pub disc_center: Vector3,
    /// Disc width, the host's mesh units.
    pub disc_size: f32,
}

/// The plateau point nearest to the unit `dir` (itself when it is one):
/// rings of growing angle, 32 points each, out to `SEARCH`.
pub fn nearest_plateau(field: &HeightField, dir: Vector3) -> Option<Vector3> {
    let dir = dir.normalized();
    if field.is_plateau(dir) {
        return Some(dir);
    }
    let (t1, t2) = (tangent(dir), dir.cross(tangent(dir)));
    (1..=(SEARCH / 0.01) as usize).find_map(|k| {
        let r = k as f32 * 0.01;
        (0..32).find_map(|j| {
            let a = j as f32 / 32.0 * std::f32::consts::TAU;
            let n = (dir * r.cos() + (t1 * a.cos() + t2 * a.sin()) * r.sin()).normalized();
            field.is_plateau(n).then_some(n)
        })
    })
}

impl Landmark {
    /// The Hearts pyramid near `dir` on `field`, on a host mesh of radius
    /// `radius` (object units): `PYRAMID_HEIGHT` radii tall, sunk `SINK`
    /// of its height. An error when no plateau lies within `SEARCH`.
    pub fn pyramid(field: &HeightField, dir: Vector3, radius: f32) -> Result<Self, String> {
        let dir = nearest_plateau(field, dir).ok_or("no plateau near the landmark")?;
        let up = field.normal(dir, NORMAL_EPS);
        let pose = PropPose {
            dir,
            up,
            yaw: 0.0,
            scale: 1.0,
            kind: PropKind::Building,
        };
        let size = PYRAMID_HEIGHT * radius;
        let map = frame_matrix(&pose, field, radius, size);
        let place = |m: &Mesh| -> Vec<Vertex> {
            m.indices
                .iter()
                .map(|&i| {
                    let p = m.vertices[i] - Vector3::new(0.0, SINK, 0.0);
                    Vertex::new_with_color(map(p), Vector3::one())
                })
                .collect()
        };
        let (body, edges) = pyramid_mesh(PYRAMID_LEVELS);
        let (mesh, edges) = (place(&body), place(&edges));
        let disc_size = DISC_SIZE * radius;
        let top_local = map(Vector3::new(0.0, 1.0 - SINK, 0.0));
        let disc_center = top_local + up * (DISC_LIFT * radius);
        // The disc turns: its whole sweep counts (a sphere of 0.75 of its
        // width holds the heart and the halo).
        let sweep = 0.75 * disc_size;
        let reach = mesh
            .iter()
            .chain(&edges)
            .map(|v| v.position.length())
            .fold(disc_center.length() + sweep, f32::max);
        let bound_center =
            mesh.iter().fold(Vector3::zero(), |a, v| a + v.position) / mesh.len().max(1) as f32;
        let bound_radius = mesh
            .iter()
            .chain(&edges)
            .map(|v| (v.position - bound_center).length())
            .fold((disc_center - bound_center).length() + sweep, f32::max);
        let disc = heart_disc_mesh(DISC_SAMPLES)?.expand_vertices(Vector3::one());
        Ok(Self {
            dir,
            up,
            mesh: Arc::new(mesh),
            edges: Arc::new(edges),
            top_local,
            reach,
            bound_center,
            bound_radius,
            disc: Arc::new(disc),
            disc_center,
            disc_size,
        })
    }

    /// World matrix of the disc on a host placed with `host` at `time`:
    /// upright over the pyramid's top along the surface normal, turning
    /// `DISC_SPIN` rad/s about it, and turning with the host.
    pub fn disc_matrix(&self, host: &Transform, time: f32) -> Matrix {
        let up = host.to_world_dir(self.up).normalized();
        let x0 = host.to_world_dir(tangent(self.up)).normalized();
        let z0 = x0.cross(up);
        let (s, c) = (DISC_SPIN * time)
            .rem_euclid(std::f32::consts::TAU)
            .sin_cos();
        let x = x0 * c + z0 * s;
        let z = x.cross(up);
        let k = self.disc_size * host.scale;
        let origin = host.position + host.to_world_dir(self.disc_center) * host.scale;
        axes_matrix(x * k, up * k, z * k, origin)
    }

    /// The light direction `world_light` in the disc's own frame.
    pub fn disc_light(&self, host: &Transform, time: f32, world_light: Vector3) -> Vector3 {
        let m = self.disc_matrix(host, time);
        let axes = [
            Vector3::new(m.m0, m.m1, m.m2),
            Vector3::new(m.m4, m.m5, m.m6),
            Vector3::new(m.m8, m.m9, m.m10),
        ];
        Vector3::new(
            axes[0].dot(world_light),
            axes[1].dot(world_light),
            axes[2].dot(world_light),
        )
        .normalized()
    }
}
