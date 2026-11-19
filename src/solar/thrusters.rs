//! Exhaust of the spacecraft: one shared low-poly flame placed at each
//! nozzle of `Laboratorio3.obj` and stretched per frame, so the engines are
//! real geometry in the depth buffer and no vertex is rebuilt.
//!
//! Flame space: the base, a unit square like the nozzle mouths, at `z = 0`;
//! the flame swells, then narrows to its tip at `z = 1`, along `+Z` (the
//! ship's back).

use crate::geometry::{Mesh, Vertex};
use crate::solar::ShipState;
use crate::solar::shapes::{End, frame_matrix, loft};
use raylib::math::{Matrix, Vector3};
use std::f32::consts::TAU;
use std::sync::Arc;

/// A nozzle mouth of the ship mesh, object space: the center of its exit
/// plane and the half side of its square inner opening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nozzle {
    pub center: Vector3,
    pub half: f32,
}

/// The three nozzles, measured on `Laboratorio3.obj` ("NaveBimotor"): the
/// main engine on the axis and one on each wing pod, all opening towards
/// `+Z`.
pub const NOZZLES: [Nozzle; 3] = [
    Nozzle {
        center: Vector3 {
            x: 0.0,
            y: 0.0,
            z: 1.8,
        },
        half: 0.2705,
    },
    Nozzle {
        center: Vector3 {
            x: -2.45,
            y: 0.0,
            z: 1.57,
        },
        half: 0.2176,
    },
    Nozzle {
        center: Vector3 {
            x: 2.45,
            y: 0.0,
            z: 1.57,
        },
        half: 0.2176,
    },
];

/// How far inside the nozzle the flame starts, object units, so no gap
/// shows between the mouth and the fire.
pub const INSET: f32 = 0.05;

/// Rings of the flame: (distance along the flame, radius) after the square
/// base, each an octagon.
const PROFILE: [(f32, f32); 3] = [(0.12, 1.05), (0.4, 0.8), (0.72, 0.42)];

/// The flame: the square base (corners and edge midpoints), three octagons
/// and the tip (62 triangles, closed).
pub fn flame_mesh() -> Mesh {
    let square: Vec<Vector3> = (0..8)
        .map(|i| {
            let (s, c) = (i as f32 * TAU / 8.0).sin_cos();
            let k = c.abs().max(s.abs());
            Vector3::new(c / k, s / k, 0.0)
        })
        .collect();
    let mut rings = vec![square];
    rings.extend(PROFILE.iter().map(|&(z, r)| {
        (0..8)
            .map(|i| {
                let (s, c) = (i as f32 * TAU / 8.0).sin_cos();
                Vector3::new(c * r, s * r, z)
            })
            .collect()
    }));
    loft(
        &rings,
        End::Cap,
        End::Apex(Vector3::new(0.0, 0.0, 1.0)),
        Vector3::new(0.0, 0.0, 0.3),
    )
}

/// Corner color of the flame mesh, seen only in wireframe.
pub const FLAME_WIRE: Vector3 = Vector3 {
    x: 1.0,
    y: 0.6,
    z: 0.15,
};

/// The flame shared by every nozzle.
#[derive(Debug, Clone)]
pub struct Thrusters {
    pub mesh: Arc<Vec<Vertex>>,
}

impl Default for Thrusters {
    fn default() -> Self {
        Self {
            mesh: Arc::new(flame_mesh().expand_vertices(FLAME_WIRE)),
        }
    }
}

/// Model matrix of the flame at `nozzle` of `ship`, `length` object units
/// long: its base on the nozzle mouth (`INSET` inside), its axis along the
/// ship's back and its size following the ship's scale.
pub fn flame_matrix(ship: &ShipState, nozzle: &Nozzle, length: f32) -> Matrix {
    let s = ship.scale;
    let base = ship.to_world(nozzle.center - Vector3::new(0.0, 0.0, INSET));
    frame_matrix(
        ship.right() * (nozzle.half * s),
        ship.up * (nozzle.half * s),
        -ship.forward * ((length + INSET) * s),
        base,
    )
}
