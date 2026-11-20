//! Neon traffic of the Hearts planet. The ships reuse the player's
//! spacecraft mesh (the same `Arc`, never a copy) and switch to a
//! procedural dart under `DART_PX`.

use crate::geometry::{Mesh, Vertex};
use crate::solar::SolarMeshes;
use crate::solar::ship::{MESH_CENTER, MESH_RADIUS, ShipState};
use raylib::math::{Matrix, Vector3};
use std::sync::Arc;

/// Projected radius under which a neon ship is drawn as its dart, pixels.
pub const DART_PX: f32 = 30.0;
/// Size of a neon ship relative to the player's.
pub const NEON_SIZE: f32 = 0.4;

/// Pushes the triangle `[a, b, c]` wound to face away from `inside`.
fn push_out(m: &mut Mesh, [a, b, c]: [Vector3; 3], inside: Vector3) {
    let i = m.vertices.len();
    m.vertices.extend([a, b, c]);
    let centroid = (a + b + c) / 3.0;
    if (b - a).cross(c - a).dot(centroid - inside) > 0.0 {
        m.indices.extend([i, i + 1, i + 2]);
    } else {
        m.indices.extend([i, i + 2, i + 1]);
    }
}

/// The far stand-in of a neon ship (34 triangles), in the spacecraft's own
/// object space so one model matrix serves both: nose to `-Z`, `+Y` up,
/// centered on `MESH_CENTER` and inside `MESH_RADIUS`. A delta hull of
/// diamond sections, a tail fin and a canopy.
pub fn dart_mesh() -> Mesh {
    let c = MESH_CENTER;
    let at = |x: f32, y: f32, z: f32| c + Vector3::new(x, y, z);
    let mut m = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    // (z, half width, half height) of each section, nose to tail.
    let sections = [(-1.8, 0.6, 0.35), (0.6, 2.0, 0.5), (2.4, 3.0, 0.25)];
    let ring =
        |(z, w, h): (f32, f32, f32)| [at(w, 0.0, z), at(0.0, h, z), at(-w, 0.0, z), at(0.0, -h, z)];
    let nose = at(0.0, 0.0, -3.6);
    let first = ring(sections[0]);
    for k in 0..4 {
        push_out(&mut m, [nose, first[k], first[(k + 1) % 4]], c);
    }
    for pair in sections.windows(2) {
        let (a, b) = (ring(pair[0]), ring(pair[1]));
        for k in 0..4 {
            let n = (k + 1) % 4;
            push_out(&mut m, [a[k], b[k], b[n]], c);
            push_out(&mut m, [a[k], b[n], a[n]], c);
        }
    }
    let tail = ring(sections[2]);
    push_out(&mut m, [tail[0], tail[1], tail[2]], at(0.0, 0.0, 0.0));
    push_out(&mut m, [tail[0], tail[2], tail[3]], at(0.0, 0.0, 0.0));
    // Tail fin: a thin triangular prism over the back of the hull.
    let fin = |x: f32| [at(x, 0.45, 0.9), at(x, 1.6, 2.4), at(x, 0.25, 2.4)];
    let (l, r) = (fin(-0.08), fin(0.08));
    let fin_center = at(0.0, 0.8, 1.9);
    push_out(&mut m, l, fin_center);
    push_out(&mut m, r, fin_center);
    for k in 0..3 {
        let n = (k + 1) % 3;
        push_out(&mut m, [l[k], r[k], r[n]], fin_center);
        push_out(&mut m, [l[k], r[n], l[n]], fin_center);
    }
    // Canopy: a low four-sided pyramid on the hull.
    let top = at(0.0, 0.75, -0.4);
    let base = [
        at(0.35, 0.3, -1.0),
        at(0.35, 0.4, 0.2),
        at(-0.35, 0.4, 0.2),
        at(-0.35, 0.3, -1.0),
    ];
    for k in 0..4 {
        push_out(
            &mut m,
            [top, base[k], base[(k + 1) % 4]],
            at(0.0, 0.3, -0.4),
        );
    }
    m
}

/// One neon ship: the player's spacecraft mesh and the shared dart, at
/// `NEON_SIZE` of the player's size.
#[derive(Debug, Clone)]
pub struct NeonShip {
    pub mesh: Arc<Vec<Vertex>>,
    pub dart: Arc<Vec<Vertex>>,
    /// Model scale: its radius over `MESH_RADIUS`.
    pub scale: f32,
}

impl NeonShip {
    /// A ship of the player's meshes, `radius` world units across its
    /// bounding sphere.
    pub fn new(meshes: &SolarMeshes, radius: f32) -> Self {
        Self {
            mesh: Arc::clone(&meshes.ship),
            dart: Arc::clone(&meshes.dart),
            scale: radius / MESH_RADIUS,
        }
    }

    /// Bounding radius, world units.
    pub fn radius(&self) -> f32 {
        MESH_RADIUS * self.scale
    }

    /// The mesh to draw at `radius_px` pixels of projected radius.
    pub fn lod(&self, radius_px: f32) -> &Arc<Vec<Vertex>> {
        if radius_px < DART_PX {
            &self.dart
        } else {
            &self.mesh
        }
    }

    /// Model matrix with its center at `position`, nose along `forward`
    /// and top along `up` (the spacecraft's convention, `ShipState`).
    pub fn matrix(&self, position: Vector3, forward: Vector3, up: Vector3) -> Matrix {
        let mut s = ShipState::new(position, self.scale);
        s.forward = forward;
        s.up = up;
        s.renormalize();
        s.model_matrix()
    }
}

/// Dark hull with a neon rim: the edge color grows where the surface turns
/// away from the eye (Fresnel), over a dim lit body. `light` and `view` are
/// unit directions in the ship's object space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NeonShader {
    pub body: Vector3,
    pub edge: Vector3,
    pub light: Vector3,
    pub view: Vector3,
    /// Rim exponent (higher: a thinner rim).
    pub fresnel: f32,
}

impl NeonShader {
    pub fn shade(&self, face_normal: Vector3) -> Vector3 {
        let lit = face_normal.dot(self.light).max(0.0);
        let rim = (1.0 - face_normal.dot(self.view).abs())
            .clamp(0.0, 1.0)
            .powf(self.fresnel.max(0.1));
        self.body * (0.15 + 0.85 * lit) + self.edge * rim
    }
}
