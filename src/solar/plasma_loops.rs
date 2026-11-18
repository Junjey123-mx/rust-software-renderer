//! External plasma loops of The King's Crown in the solar scene: magnetic
//! arcs that leave the star at one foot point, rise outside its silhouette
//! and fall back into it at another. The laboratory six shader
//! (`celestial::crown`) paints its loops inside the disk; these are
//! geometry, described here in the star's object space so they can be
//! built once and drawn with the star's own transform.

use crate::geometry::Vertex;
use crate::pipeline::direction_from_angles;
use crate::shading::math::hex;
use crate::shading::sphere::east_north;
use crate::solar::SPHERE_RADIUS;
use raylib::math::Vector3;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

/// Quads along each arc and around its tube: few enough that the crown's
/// arcs together cost about as much to transform as one small body.
pub const SEGMENTS: usize = 16;
pub const SIDES: usize = 6;
/// Triangle corners of one arc's tube.
pub const VERTICES_PER_ARC: usize = SEGMENTS * SIDES * 6;
/// Depth of the feet below the surface, in body radii: the open ends of a
/// tube stay hidden inside the faceted sphere (its nearest vertex is at
/// 0.97 of the radius).
pub const FOOT_DEPTH: f32 = 0.12;
/// Corner color of the tubes, seen only in wireframe: the crown's gold.
pub const WIRE_COLOR: Vector3 = hex(0xFFC63A);

/// One magnetic arc. Directions are on the unit sphere of the star's object
/// space; heights and widths are in body radii.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlasmaArc {
    /// Midpoint between the feet on the surface, radians, with the
    /// convention of `direction_from_angles` (longitude 0 faces +Z).
    pub longitude: f32,
    pub latitude: f32,
    /// Direction of the line between the feet, radians from east to north.
    pub tilt: f32,
    /// Angle between the feet seen from the center, radians.
    pub span: f32,
    /// Apex height above the surface.
    pub height: f32,
    /// Tube radius at the apex.
    pub width: f32,
    /// Shift along the plasma ramp: below zero redder, above whiter.
    pub heat: f32,
    /// Emission scale.
    pub intensity: f32,
    /// Phase of the pulse and the flow, radians.
    pub phase: f32,
}

impl PlasmaArc {
    /// Unit direction of the midpoint between the feet.
    pub fn center(&self) -> Vector3 {
        direction_from_angles(self.longitude, self.latitude)
    }

    /// Unit directions of the two feet, half the span on each side of the
    /// center along the tilt.
    pub fn feet(&self) -> [Vector3; 2] {
        let c = self.center();
        let (east, north) = east_north(c);
        let chord = east * self.tilt.cos() + north * self.tilt.sin();
        let (s, k) = (self.span / 2.0).sin_cos();
        [c * k - chord * s, c * k + chord * s]
    }

    /// The arc's plane, which passes through the star's center: the whole
    /// centerline lies in it.
    pub fn basis(&self) -> ArcBasis {
        let [start, end] = self.feet();
        let normal = start.cross(end).normalized();
        ArcBasis {
            start,
            side: normal.cross(start),
            normal,
        }
    }

    /// Distance of the centerline from the star's center at `t` (0 and 1
    /// at the buried feet, the apex at 0.5), in body radii.
    pub fn lift(&self, t: f32) -> f32 {
        1.0 - FOOT_DEPTH + (FOOT_DEPTH + self.height) * (PI * t).sin()
    }

    /// Tube radius at `t`, in body radii: thinner at the feet.
    pub fn width_at(&self, t: f32) -> f32 {
        self.width * (0.65 + 0.35 * (PI * t).sin())
    }

    /// Centerline point at `t` in the object space of a body of `radius`:
    /// the great circle between the feet, raised by `lift`.
    pub fn point(&self, t: f32, radius: f32) -> Vector3 {
        self.basis().direction(self.span * t) * (radius * self.lift(t))
    }

    /// The tube around the centerline for a body of `radius`: `SEGMENTS`
    /// rings of `SIDES` corners, perpendicular to the curve, joined by
    /// outward-facing triangles (one `Vertex` per corner). The ends stay
    /// open, buried in the star.
    pub fn tube(&self, radius: f32) -> Vec<Vertex> {
        let basis = self.basis();
        let rise = (FOOT_DEPTH + self.height) * PI;
        let rings: Vec<[Vector3; SIDES]> = (0..=SEGMENTS)
            .map(|i| {
                let t = i as f32 / SEGMENTS as f32;
                let angle = self.span * t;
                let (dir, across) = (basis.direction(angle), basis.direction(angle + PI / 2.0));
                let center = dir * (radius * self.lift(t));
                // Derivative of `point`; the ring spans the plane normal
                // and the in-plane normal of the curve.
                let tangent = across * (self.span * self.lift(t)) + dir * (rise * (PI * t).cos());
                let out = basis.normal.cross(tangent).normalized();
                let w = radius * self.width_at(t);
                std::array::from_fn(|j| {
                    let (s, c) = (TAU * j as f32 / SIDES as f32).sin_cos();
                    center + (out * c + basis.normal * s) * w
                })
            })
            .collect();
        let mut mesh = Vec::with_capacity(VERTICES_PER_ARC);
        for pair in rings.windows(2) {
            for j in 0..SIDES {
                let k = (j + 1) % SIDES;
                let (a, b, c, d) = (pair[0][j], pair[0][k], pair[1][k], pair[1][j]);
                for p in [a, b, c, a, c, d] {
                    mesh.push(Vertex::new_with_color(p, WIRE_COLOR));
                }
            }
        }
        mesh
    }
}

/// Orthonormal basis of an arc's plane: `start` is the first foot, `side`
/// the in-plane direction a quarter turn towards the second foot and
/// `normal` the plane normal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcBasis {
    pub start: Vector3,
    pub side: Vector3,
    pub normal: Vector3,
}

impl ArcBasis {
    /// Unit direction `angle` radians from the first foot towards the
    /// second.
    pub fn direction(&self, angle: f32) -> Vector3 {
        let (s, c) = angle.sin_cos();
        self.start * c + self.side * s
    }
}

/// A star's arcs and their tubes, built once for the course sphere
/// (`SPHERE_RADIUS`) and shared by every clone of the scene.
#[derive(Debug, Clone)]
pub struct PlasmaLoops {
    pub arcs: Vec<PlasmaArc>,
    /// `VERTICES_PER_ARC` corners per arc, in arc order.
    pub mesh: Arc<Vec<Vertex>>,
}

impl PlasmaLoops {
    pub fn new(arcs: &[PlasmaArc]) -> Self {
        Self {
            arcs: arcs.to_vec(),
            mesh: Arc::new(arcs.iter().flat_map(|a| a.tube(SPHERE_RADIUS)).collect()),
        }
    }

    /// The King's Crown's arcs.
    pub fn crown() -> Self {
        Self::new(&CROWN_ARCS)
    }

    /// Tube of arc `index`.
    pub fn arc_mesh(&self, index: usize) -> &[Vertex] {
        &self.mesh[index * VERTICES_PER_ARC..(index + 1) * VERTICES_PER_ARC]
    }

    /// Farthest reach of any tube from the star's center, in body radii (1
    /// without arcs): the bounding sphere of the star with its loops.
    pub fn extent(&self) -> f32 {
        self.arcs
            .iter()
            .map(|a| 1.0 + a.height + a.width)
            .fold(1.0, f32::max)
    }
}

const fn deg(d: f32) -> f32 {
    d * PI / 180.0
}

/// The crown's arcs: six around the star, one every 60 degrees of longitude
/// on alternating active belts, so whatever side faces the camera about two
/// of them stand near the limb and show their whole arch against the sky.
pub const CROWN_ARCS: [PlasmaArc; 6] = [
    PlasmaArc {
        longitude: deg(20.0),
        latitude: deg(22.0),
        tilt: deg(10.0),
        span: deg(34.0),
        height: 0.42,
        width: 0.065,
        heat: 0.10,
        intensity: 1.15,
        phase: 0.0,
    },
    PlasmaArc {
        longitude: deg(80.0),
        latitude: deg(-28.0),
        tilt: deg(70.0),
        span: deg(28.0),
        height: 0.32,
        width: 0.055,
        heat: -0.05,
        intensity: 1.0,
        phase: 1.7,
    },
    PlasmaArc {
        longitude: deg(145.0),
        latitude: deg(12.0),
        tilt: deg(-35.0),
        span: deg(40.0),
        height: 0.55,
        width: 0.075,
        heat: 0.05,
        intensity: 1.1,
        phase: 3.1,
    },
    PlasmaArc {
        longitude: deg(200.0),
        latitude: deg(34.0),
        tilt: deg(55.0),
        span: deg(24.0),
        height: 0.28,
        width: 0.05,
        heat: -0.10,
        intensity: 0.95,
        phase: 4.4,
    },
    PlasmaArc {
        longitude: deg(260.0),
        latitude: deg(-14.0),
        tilt: deg(-10.0),
        span: deg(36.0),
        height: 0.48,
        width: 0.065,
        heat: 0.15,
        intensity: 1.2,
        phase: 5.6,
    },
    PlasmaArc {
        longitude: deg(320.0),
        latitude: deg(-36.0),
        tilt: deg(30.0),
        span: deg(30.0),
        height: 0.36,
        width: 0.06,
        heat: 0.0,
        intensity: 1.05,
        phase: 2.4,
    },
];
