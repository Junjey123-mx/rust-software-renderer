//! Exhaust of the spacecraft: one shared low-poly flame placed at each
//! nozzle of `Laboratorio3.obj` and stretched per frame, so the engines are
//! real geometry in the depth buffer and no vertex is rebuilt.
//!
//! Flame space: the base, a unit square like the nozzle mouths, at `z = 0`;
//! the flame swells, then narrows to its tip at `z = 1`, along `+Z` (the
//! ship's back).

use crate::geometry::{Mesh, Vertex};
use crate::shading::math::{hex, mix3, smoothstep};
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

/// Flame length, object units: the idle flame, and what full thrust adds.
pub const IDLE_LENGTH: f32 = 0.45;
pub const THRUST_LENGTH: f32 = 1.5;
/// Wing pods burn shorter than the main engine.
pub const POD_SHARE: f32 = 0.8;
/// Afterburner: how much longer and brighter a fully boosted flame is.
pub const BOOST_STRETCH: f32 = 1.3;
pub const BOOST_GLOW: f32 = 0.45;

/// Flicker phase of nozzle `index`, so the engines never pulse in step.
pub fn nozzle_phase(index: usize) -> f32 {
    index as f32 * 2.1
}

/// Length of the flame at nozzle `index` for the ship's engine level at
/// simulated `time`: idle plus thrust, stretched by the afterburner as the
/// boost ramps, flickering a few percent at its own phase (frozen with the
/// clock, so a paused frame is stable).
pub fn flame_length(ship: &ShipState, index: usize, time: f32) -> f32 {
    let phase = nozzle_phase(index);
    let flicker =
        1.0 + 0.07 * (31.0 * time + phase).sin() + 0.04 * (57.0 * time + 2.0 * phase).sin();
    let share = if index == 0 { 1.0 } else { POD_SHARE };
    let afterburner = 1.0 + BOOST_STRETCH * ship.boost;
    (IDLE_LENGTH + THRUST_LENGTH * ship.thrust) * share * afterburner * flicker
}

/// Exhaust colors from the nozzle to the tip.
pub const FLAME_CORE: Vector3 = hex(0xFFF1B8);
pub const FLAME_AMBER: Vector3 = hex(0xFFB22E);
pub const FLAME_ORANGE: Vector3 = hex(0xFF6414);
pub const FLAME_TIP: Vector3 = hex(0xA61E0A);
/// Afterburner colors over the same stops.
pub const AFTER_CORE: Vector3 = hex(0xE6FFFF);
pub const AFTER_CYAN: Vector3 = hex(0x3FE3FF);
pub const AFTER_BLUE: Vector3 = hex(0x2A6BFF);
pub const AFTER_TIP: Vector3 = hex(0x4A1EB4);

/// The exhaust `boost` of the way from `warm` to `blue`, channel by
/// channel: blue rises first, green in the middle and red fades last, so
/// the flame passes through peach and lavender-white instead of the green
/// a straight mix of amber and cyan gives.
pub fn afterburner_mix(warm: Vector3, blue: Vector3, boost: f32) -> Vector3 {
    let lerp = |a: f32, b: f32, k: f32| a + (b - a) * k;
    Vector3::new(
        lerp(warm.x, blue.x, smoothstep(0.5, 1.0, boost)),
        lerp(warm.y, blue.y, smoothstep(0.25, 0.75, boost)),
        lerp(warm.z, blue.z, smoothstep(0.0, 0.5, boost)),
    )
}

/// Color `t` of the way down a flame through four stops.
fn gradient(stops: [Vector3; 4], t: f32) -> Vector3 {
    if t < 0.3 {
        mix3(stops[0], stops[1], smoothstep(0.0, 0.3, t))
    } else if t < 0.65 {
        mix3(stops[1], stops[2], smoothstep(0.3, 0.65, t))
    } else {
        mix3(stops[2], stops[3], smoothstep(0.65, 1.0, t))
    }
}

/// One flame ready to shade a frame: the flame-space normal transform
/// (inverse-transpose columns of its model matrix, for the view-facing
/// glow), the simulated time, the nozzle's phase and the boost level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlameShader {
    pub normal_axes: [Vector3; 3],
    pub time: f32,
    pub phase: f32,
    pub boost: f32,
}

impl FlameShader {
    /// The flame of nozzle `index`, `length` object units long.
    pub fn new(ship: &ShipState, index: usize, length: f32, time: f32) -> Self {
        let (nozzle, s) = (NOZZLES[index], ship.scale);
        let side = nozzle.half * s;
        Self {
            normal_axes: [
                ship.right() / side,
                ship.up / side,
                -ship.forward / ((length + INSET) * s),
            ],
            time,
            phase: nozzle_phase(index),
            boost: ship.boost,
        }
    }

    /// Emissive (no light): white-hot at the nozzle through amber and
    /// orange to a dark red tip, brightest where the flame faces the eye
    /// and pulsing in bands that run down the plume. The afterburner blends
    /// in as the boost ramps: white-cyan through blue to a violet tip, and
    /// brighter.
    pub fn shade(&self, p: Vector3, face_normal: Vector3, view_world: Vector3) -> Vector3 {
        let [x, y, z] = self.normal_axes;
        let n = (x * face_normal.x + y * face_normal.y + z * face_normal.z).normalized();
        let facing = n.dot(view_world).abs();
        let t = p.z.clamp(0.0, 1.0);
        let warm = gradient([FLAME_CORE, FLAME_AMBER, FLAME_ORANGE, FLAME_TIP], t);
        let blue = gradient([AFTER_CORE, AFTER_CYAN, AFTER_BLUE, AFTER_TIP], t);
        let color = afterburner_mix(warm, blue, self.boost);
        let pulse = 1.0 + 0.15 * (18.0 * t - 22.0 * self.time + self.phase).sin();
        color * ((0.55 + 0.65 * facing) * pulse * (1.0 + BOOST_GLOW * self.boost))
    }
}
