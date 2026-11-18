//! External plasma loops of The King's Crown in the solar scene: magnetic
//! arcs that leave the star at one foot point, rise outside its silhouette
//! and fall back into it at another. The laboratory six shader
//! (`celestial::crown`) paints its loops inside the disk; these are
//! geometry, described here in the star's object space so they can be
//! built once and drawn with the star's own transform.

use crate::celestial::crown::PLASMA_RAMP;
use crate::geometry::Vertex;
use crate::pipeline::{NormalSource, ShadedOptions, direction_from_angles};
use crate::shading::math::{hex, ramp, saturate, smoothstep};
use crate::shading::noise::fbm;
use crate::shading::sphere::east_north;
use crate::solar::SPHERE_RADIUS;
use raylib::math::Vector3;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

/// Quads along each arc and around its tube: the six crown arcs add 840
/// triangles (the course sphere has 192), still smoother than the star's
/// own silhouette; the shader rounds the tube, not the facets.
pub const SEGMENTS: usize = 14;
pub const SIDES: usize = 5;
/// Triangle corners of one arc's tube.
pub const VERTICES_PER_ARC: usize = SEGMENTS * SIDES * 6;
/// Depth of the feet below the surface, in body radii: the open ends of a
/// tube stay hidden inside the faceted sphere (its nearest vertex is at
/// 0.97 of the radius).
pub const FOOT_DEPTH: f32 = 0.12;
/// Corner color of the tubes, seen only in wireframe: the crown's gold.
pub const WIRE_COLOR: Vector3 = hex(0xFFC63A);
/// Seconds per brightness pulse of an arc.
pub const PULSE_PERIOD: f32 = 5.0;
/// Bright knots along each arc and how fast they run from the first foot
/// to the second, knots per second.
pub const FLOW_KNOTS: f32 = 3.0;
pub const FLOW_SPEED: f32 = 0.35;
/// Strands around each tube and the turns they make from foot to foot.
pub const STRANDS: f32 = 3.0;
pub const TWISTS: f32 = 2.0;
/// Noise cells of the plasma grain along an arc.
pub const GRAIN: f32 = 14.0;

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
        self.curve(&self.basis(), t, radius).0
    }

    /// Centerline point at `t` and its derivative with respect to `t`.
    pub fn curve(&self, basis: &ArcBasis, t: f32, radius: f32) -> (Vector3, Vector3) {
        let angle = self.span * t;
        let (dir, across) = (basis.direction(angle), basis.direction(angle + PI / 2.0));
        let rise = (FOOT_DEPTH + self.height) * PI * (PI * t).cos();
        (
            dir * (radius * self.lift(t)),
            (across * (self.span * self.lift(t)) + dir * rise) * radius,
        )
    }

    /// The tube around the centerline for a body of `radius`: `SEGMENTS`
    /// rings of `SIDES` corners, perpendicular to the curve, joined by
    /// outward-facing triangles (one `Vertex` per corner). The ends stay
    /// open, buried in the star.
    pub fn tube(&self, radius: f32) -> Vec<Vertex> {
        let basis = self.basis();
        let rings: Vec<[Vector3; SIDES]> = (0..=SEGMENTS)
            .map(|i| {
                let t = i as f32 / SEGMENTS as f32;
                // The ring spans the plane normal and the in-plane normal
                // of the curve.
                let (center, tangent) = self.curve(&basis, t, radius);
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

    /// What shading this arc needs in a frame at `time` on a body of
    /// `radius`: its plane and the animation phases, computed once.
    pub fn shader(&self, time: f32, radius: f32) -> ArcShader {
        ArcShader {
            arc: *self,
            basis: self.basis(),
            radius,
            pulse: 0.85 + 0.15 * (TAU * time / PULSE_PERIOD + self.phase).sin(),
            flow: FLOW_SPEED * time,
        }
    }
}

/// Shaded-stage options of the tubes: closed and wound outwards, so back
/// faces are culled; flat normals, since the shader derives its own.
pub fn plasma_options(threads: usize) -> ShadedOptions {
    ShadedOptions {
        cull_back_faces: true,
        normal_source: NormalSource::Face,
        threads,
    }
}

/// One arc ready to shade a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcShader {
    pub arc: PlasmaArc,
    pub basis: ArcBasis,
    pub radius: f32,
    /// Brightness pulse of this frame, around 1.
    pub pulse: f32,
    /// Distance the knots have run, in knots.
    pub flow: f32,
}

impl ArcShader {
    /// Emission of the tube at object-space point `p`, seen along the unit
    /// object-space direction `eye`. The position along the arc comes from
    /// the angle of `p` in the arc's plane; the offset from the centerline
    /// gives a smooth tube normal whatever the facets, so the core facing
    /// the eye burns gold to white and the outline cools to crimson. Its
    /// angle around the centerline draws strands that twist along the arc
    /// like a magnetic flux rope; knots of plasma run between the feet, the
    /// feet glow hotter and the whole arc pulses.
    pub fn shade(&self, p: Vector3, eye: Vector3) -> Vector3 {
        let (arc, b) = (&self.arc, &self.basis);
        // Nearest centerline point: a guess from the polar angle of `p`,
        // one Newton step along the curve, then the offset without its
        // component along the axis.
        let guess = p.dot(b.side).atan2(p.dot(b.start)).clamp(0.0, arc.span) / arc.span;
        let (c, d) = arc.curve(b, guess, self.radius);
        let t = (guess + (p - c).dot(d) / d.dot(d)).clamp(0.0, 1.0);
        let (c, d) = arc.curve(b, t, self.radius);
        let mut offset = p - c;
        offset -= d * (offset.dot(d) / d.dot(d));
        let len = offset.length();
        let facing = if len > 1e-6 {
            (offset.dot(eye) / len).abs()
        } else {
            1.0
        };
        let core = facing * facing;
        let out = b.normal.cross(d) / d.length();
        let around = offset.dot(b.normal).atan2(offset.dot(out));
        let strands = 0.5 + 0.5 * (STRANDS * around - TAU * TWISTS * t + arc.phase).cos();
        let wave = 0.5 + 0.5 * (TAU * (FLOW_KNOTS * t - self.flow) + arc.phase).sin();
        let knots = smoothstep(0.55, 1.0, wave);
        let foot = 1.0 - smoothstep(0.0, 0.2, t.min(1.0 - t));
        // Grain in tube coordinates (around as a circle, so it never
        // seams), carried along the arc by the flow.
        let (sa, ca) = around.sin_cos();
        let grain = fbm(
            Vector3::new(1.5 * ca, 1.5 * sa, GRAIN * t - 2.0 * self.flow),
            2,
            arc.phase.to_bits(),
        );
        let heat = (0.12
            + 0.38 * facing
            + 0.3 * core
            + 0.22 * strands
            + 0.25 * knots
            + 0.15 * foot
            + 0.5 * grain)
            * self.pulse
            + arc.heat;
        let glow = arc.intensity
            * (0.65 + 0.35 * facing)
            * (0.7 + 0.3 * strands)
            * (1.0 + 0.25 * knots + 0.6 * grain);
        ramp(&PLASMA_RAMP, saturate(heat)) * glow
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
        width: 0.05,
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
        width: 0.042,
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
        width: 0.058,
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
        width: 0.04,
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
        width: 0.05,
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
        width: 0.046,
        heat: 0.0,
        intensity: 1.05,
        phase: 2.4,
    },
];
