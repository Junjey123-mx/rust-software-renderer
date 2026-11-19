//! The royal crown of The King's Crown in the solar scene ("the Red-Black
//! King"): real geometry sitting on the star's northern cap, made of a
//! flared band, faceted points with ornamental finials and gems set in the
//! band. Everything is described in the star's object space in body radii
//! (the unit sphere), so the crown follows the star's position, spin and
//! scale through the star's own transform.
//!
//! Azimuths use the convention of `direction_from_angles` (0 faces `+Z`).

use crate::geometry::{Mesh, Vertex};
use crate::pipeline::Transform;
use crate::shading::math::{hex, ramp, saturate, smoothstep};
use crate::solar::SPHERE_RADIUS;
use crate::solar::shapes::{End, loft, merge, octahedron, scaled};
use raylib::math::Vector3;
use std::f32::consts::TAU;
use std::sync::Arc;

const UP: Vector3 = Vector3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};

/// Unit outward direction on the equatorial plane at `azimuth`.
pub fn outward(azimuth: f32) -> Vector3 {
    let (s, c) = azimuth.sin_cos();
    Vector3::new(s, 0.0, c)
}

/// Unit direction of increasing azimuth on the equatorial plane.
pub fn along(azimuth: f32) -> Vector3 {
    let (s, c) = azimuth.sin_cos();
    Vector3::new(c, 0.0, -s)
}

/// The band: a closed ring around the star's `Y` axis whose wall flares
/// outwards from bottom to top. Its lower edge sinks into the sphere, so
/// the crown sits on the star instead of floating above it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrownBand {
    /// Latitude of the band's middle on the sphere, radians.
    pub latitude: f32,
    /// Height of the wall.
    pub height: f32,
    /// Radial thickness of the wall.
    pub thickness: f32,
    /// Distance of the wall's middle outside the sphere at `latitude`.
    pub lift: f32,
    /// Top radius minus bottom radius.
    pub flare: f32,
    /// Segments around the ring.
    pub segments: usize,
}

impl CrownBand {
    pub fn bottom(&self) -> f32 {
        self.latitude.sin() - self.height / 2.0
    }

    pub fn top(&self) -> f32 {
        self.latitude.sin() + self.height / 2.0
    }

    /// Radius of the middle of the wall at height `y` (linear flare).
    pub fn radius_at(&self, y: f32) -> f32 {
        let middle = self.latitude.cos() + self.lift;
        middle + self.flare * ((y - self.bottom()) / self.height - 0.5)
    }

    /// Middle of the top rim at `azimuth`.
    pub fn rim(&self, azimuth: f32) -> Vector3 {
        outward(azimuth) * self.radius_at(self.top()) + UP * self.top()
    }

    /// Middle of the outer face at mid height at `azimuth`.
    pub fn outer_face(&self, azimuth: f32) -> Vector3 {
        let y = self.latitude.sin();
        outward(azimuth) * (self.radius_at(y) + self.thickness / 2.0) + UP * y
    }

    /// The closed band: `segments` quadrilateral cross-sections (inner and
    /// outer bottom, outer and inner top) joined by outward quads.
    pub fn mesh(&self) -> Mesh {
        let (y0, y1, t) = (self.bottom(), self.top(), self.thickness / 2.0);
        let (r0, r1) = (self.radius_at(y0), self.radius_at(y1));
        let section = [(r0 - t, y0), (r0 + t, y0), (r1 + t, y1), (r1 - t, y1)];
        let n = self.segments;
        let mut vertices = Vec::with_capacity(n * 4);
        for i in 0..n {
            let o = outward(TAU * i as f32 / n as f32);
            vertices.extend(section.iter().map(|&(r, y)| o * r + UP * y));
        }
        let mut indices = Vec::with_capacity(n * 24);
        for i in 0..n {
            let j = (i + 1) % n;
            for k in 0..4 {
                let l = (k + 1) % 4;
                let (a, b, c, d) = (i * 4 + k, i * 4 + l, j * 4 + l, j * 4 + k);
                indices.extend([a, d, c, a, c, b]);
            }
        }
        Mesh::new(vertices, indices).expect("the band has triangles")
    }
}

/// One point of the crown, rising from the band's top rim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrownSpike {
    pub azimuth: f32,
    /// Length along its axis.
    pub height: f32,
    /// Size along the rim.
    pub width: f32,
    /// Size across the rim.
    pub depth: f32,
    /// Outward tilt of the axis (0 = vertical).
    pub lean: f32,
    /// Size of the ornamental finial on the tip (0 = none): main points
    /// carry one, the small points between them do not.
    pub finial: f32,
}

/// Fraction of the height where a point is widest, and how much wider.
pub const BULGE_AT: f32 = 0.28;
pub const BULGE: f32 = 1.18;
/// How deep a point's base sinks into the rim, so no crack shows.
pub const SINK: f32 = 0.015;

impl CrownSpike {
    pub fn axis(&self) -> Vector3 {
        (UP + outward(self.azimuth) * self.lean).normalized()
    }

    pub fn base(&self, band: &CrownBand) -> Vector3 {
        band.rim(self.azimuth) - UP * SINK
    }

    pub fn tip(&self, band: &CrownBand) -> Vector3 {
        self.base(band) + self.axis() * self.height
    }

    /// Across-the-rim direction perpendicular to the axis.
    fn across(&self) -> Vector3 {
        let (o, a) = (outward(self.azimuth), self.axis());
        (o - a * o.dot(a)).normalized()
    }

    /// Faceted blade: a diamond base, a wider diamond at `BULGE_AT` of the
    /// height and the tip.
    pub fn mesh(&self, band: &CrownBand) -> Mesh {
        let (base, axis) = (self.base(band), self.axis());
        let (side, across) = (along(self.azimuth), self.across());
        let diamond = |center: Vector3, scale: f32| {
            let (w, d) = (self.width / 2.0 * scale, self.depth / 2.0 * scale);
            vec![
                center + side * w,
                center + across * d,
                center - side * w,
                center - across * d,
            ]
        };
        let bulge = base + axis * (self.height * BULGE_AT);
        loft(
            &[diamond(base, 1.0), diamond(bulge, BULGE)],
            End::Cap,
            End::Apex(self.tip(band)),
            base + axis * (self.height * 0.2),
        )
    }

    /// Elongated octahedron around the tip, when the point has a finial.
    pub fn finial_mesh(&self, band: &CrownBand) -> Option<Mesh> {
        (self.finial > 0.0).then(|| {
            let f = self.finial;
            octahedron(
                self.tip(band) + self.axis() * (0.7 * f),
                along(self.azimuth) * f,
                self.across() * f,
                self.axis() * (1.4 * f),
            )
        })
    }
}

/// A faceted gem set halfway into the band's outer face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrownGem {
    pub azimuth: f32,
    /// Half-width along the rim.
    pub size: f32,
}

impl CrownGem {
    pub fn center(&self, band: &CrownBand) -> Vector3 {
        band.outer_face(self.azimuth)
    }

    /// A rhombus seen from outside: taller than wide, shallow outwards.
    pub fn mesh(&self, band: &CrownBand) -> Mesh {
        octahedron(
            self.center(band),
            along(self.azimuth) * self.size,
            UP * (1.3 * self.size),
            outward(self.azimuth) * (0.45 * self.size),
        )
    }
}

/// The whole crown, in body radii.
#[derive(Debug, Clone, PartialEq)]
pub struct CrownSpec {
    pub band: CrownBand,
    pub spikes: Vec<CrownSpike>,
    pub gems: Vec<CrownGem>,
}

/// Main points of the royal crown.
pub const MAIN_POINTS: usize = 7;

impl CrownSpec {
    /// The Red-Black King's crown: seven main points symmetric about the
    /// front (azimuth 0), the front one tallest and the others alternating
    /// in height, small points between them, a gem under every main point
    /// and open gaps through which the star shows.
    pub fn royal() -> Self {
        let step = TAU / MAIN_POINTS as f32;
        let mut spikes = Vec::new();
        let mut gems = Vec::new();
        for k in 0..MAIN_POINTS {
            // Distance from the front, so the crown is mirror-symmetric.
            let m = k.min(MAIN_POINTS - k);
            let azimuth = step * k as f32;
            let (height, width) = [(0.52, 0.17), (0.34, 0.14), (0.42, 0.15), (0.3, 0.13)][m];
            spikes.push(CrownSpike {
                azimuth,
                height,
                width,
                depth: 0.06,
                lean: 0.18,
                finial: if m == 0 { 0.07 } else { 0.055 },
            });
            spikes.push(CrownSpike {
                azimuth: azimuth + step / 2.0,
                height: 0.16,
                width: 0.09,
                depth: 0.045,
                lean: 0.25,
                finial: 0.0,
            });
            gems.push(CrownGem {
                azimuth,
                size: if m == 0 { 0.075 } else { 0.06 },
            });
        }
        Self {
            band: CrownBand {
                latitude: 54f32.to_radians(),
                height: 0.2,
                thickness: 0.04,
                lift: 0.08,
                flare: 0.1,
                segments: 28,
            },
            spikes,
            gems,
        }
    }

    /// Points that carry a finial.
    pub fn main_spikes(&self) -> impl Iterator<Item = &CrownSpike> {
        self.spikes.iter().filter(|s| s.finial > 0.0)
    }
}

/// Corner colors of the two parts, seen only in wireframe.
pub const GOLD_WIRE: Vector3 = hex(0xFFC63A);
pub const JEWEL_WIRE: Vector3 = hex(0xD21F2B);

impl CrownSpec {
    /// Metal of the crown in body radii: the band and every point.
    pub fn gold_mesh(&self) -> Mesh {
        let mut parts = vec![self.band.mesh()];
        parts.extend(self.spikes.iter().map(|s| s.mesh(&self.band)));
        merge(&parts)
    }

    /// Stones of the crown in body radii: the finials and the gems.
    pub fn jewel_mesh(&self) -> Mesh {
        let mut parts: Vec<Mesh> = self
            .spikes
            .iter()
            .filter_map(|s| s.finial_mesh(&self.band))
            .collect();
        parts.extend(self.gems.iter().map(|g| g.mesh(&self.band)));
        merge(&parts)
    }
}

/// The crown built once for the course sphere (`SPHERE_RADIUS`): one
/// expanded vertex list per material, shared by every clone of the scene
/// and drawn with the star's transform, so it never rebuilds per frame.
#[derive(Debug, Clone)]
pub struct RoyalCrown {
    pub spec: CrownSpec,
    pub gold: Arc<Vec<Vertex>>,
    pub jewels: Arc<Vec<Vertex>>,
    /// Farthest corner from the star's center, in body radii.
    pub extent: f32,
    /// Own turn of the crown around the star's axis, radians per simulated
    /// second, on top of the star's spin (0: it turns with the star).
    pub spin: f32,
    /// Heights of the enamel stripe on the band's outer face, object units.
    pub inlay: (f32, f32),
    /// Height of the band's top, object units: jewels above it are finials.
    pub rim: f32,
}

impl RoyalCrown {
    pub fn new(spec: CrownSpec) -> Self {
        let (gold, jewels) = (spec.gold_mesh(), spec.jewel_mesh());
        let extent = gold
            .vertices
            .iter()
            .chain(&jewels.vertices)
            .map(|p| p.length())
            .fold(1.0, f32::max);
        let (middle, half) = (spec.band.latitude.sin(), INLAY * spec.band.height / 2.0);
        Self {
            gold: Arc::new(scaled(&gold, SPHERE_RADIUS).expand_vertices(GOLD_WIRE)),
            jewels: Arc::new(scaled(&jewels, SPHERE_RADIUS).expand_vertices(JEWEL_WIRE)),
            inlay: (
                (middle - half) * SPHERE_RADIUS,
                (middle + half) * SPHERE_RADIUS,
            ),
            rim: spec.band.top() * SPHERE_RADIUS,
            spec,
            extent,
            spin: 0.0,
        }
    }

    pub fn royal() -> Self {
        Self::new(CrownSpec::royal())
    }

    pub fn triangle_count(&self) -> usize {
        (self.gold.len() + self.jewels.len()) / 3
    }

    /// Shading of one part in a frame, with the crown's `transform`.
    pub fn shader(&self, part: CrownPart, transform: Transform, time: f32) -> CrownShader {
        CrownShader {
            part,
            transform,
            time,
            inlay: self.inlay,
            rim: self.rim,
        }
    }
}

/// Share of the band's height covered by the enamel stripe.
pub const INLAY: f32 = 0.56;

/// What the space around the crown looks like in its polished gold, from
/// the dark sky to the star's fire.
pub const GOLD_REFLECTION: [(f32, Vector3); 4] = [
    (0.0, hex(0x3A1208)),
    (0.45, hex(0x9A5A14)),
    (0.75, hex(0xF0A030)),
    (1.0, hex(0xFFE9A8)),
];
/// Gems of the band.
pub const RUBY: [(f32, Vector3); 5] = [
    (0.0, hex(0x2A0410)),
    (0.35, hex(0x7A0A1E)),
    (0.65, hex(0xD21F2B)),
    (0.9, hex(0xFF5A5A)),
    (1.0, hex(0xFFD0C8)),
];
/// Finials on the points: darker, towards wine.
pub const GARNET: [(f32, Vector3); 4] = [
    (0.0, hex(0x1E0208)),
    (0.4, hex(0x5A0A1C)),
    (0.75, hex(0xA0142C)),
    (1.0, hex(0xFF6A7A)),
];
pub const GOLD: Vector3 = hex(0xE8B04A);
pub const ENAMEL: Vector3 = hex(0x5A0A1C);
pub const IVORY: Vector3 = hex(0xFFF2C4);

/// Which part of the crown a draw holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrownPart {
    Gold,
    Jewels,
}

/// Procedural materials of the crown, lit by the star it sits on: polished
/// gold that mirrors the star's fire where its faces turn towards it, a
/// corinto enamel stripe on the band, rubies set in the band and garnet
/// finials on the points, both slowly shimmering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrownShader {
    pub part: CrownPart,
    /// The crown's transform (the star's, plus its own spin).
    pub transform: Transform,
    pub time: f32,
    pub inlay: (f32, f32),
    pub rim: f32,
}

impl CrownShader {
    /// Color at object-space point `p` of a face with object-space normal
    /// `face_normal`, seen along the unit world direction to the eye.
    pub fn shade(&self, p: Vector3, face_normal: Vector3, view_world: Vector3) -> Vector3 {
        let n = face_normal.normalized();
        let v = self.transform.to_object_dir(view_world);
        let to_star = -p.normalized();
        let lit = n.dot(to_star).max(0.0);
        // The eye ray mirrored by the face: towards the star it shows fire.
        let mirrored = (n * (2.0 * n.dot(v)) - v).dot(to_star);
        let glow = smoothstep(-0.1, 0.8, mirrored);
        let fresnel = (1.0 - n.dot(v).abs()).powi(4);
        let sky = 0.5 + 0.5 * n.y;
        match self.part {
            CrownPart::Gold => {
                let radial = Vector3::new(p.x, 0.0, p.z).normalized();
                let on_stripe = p.y > self.inlay.0 && p.y < self.inlay.1 && n.dot(radial) > 0.6;
                if on_stripe {
                    ENAMEL * (0.55 + 0.6 * lit + 0.35 * sky)
                        + hex(0xFF7A5A) * (0.45 * glow.powi(3))
                        + IVORY * (0.3 * fresnel)
                } else {
                    let sheen = n.dot(v).max(0.0).powi(2);
                    GOLD * (0.15 + 0.4 * lit + 0.2 * sky + 0.45 * sheen)
                        + ramp(&GOLD_REFLECTION, glow) * 0.85
                        + IVORY * (0.45 * fresnel)
                }
            }
            CrownPart::Jewels => {
                let phase = 7.0 * (p.x + p.z) + 3.0 * p.y;
                let shimmer = 0.88 + 0.12 * (2.0 * self.time + phase).sin();
                let facet = saturate((0.3 + 0.5 * lit + 0.35 * glow + 0.2 * sky) * shimmer);
                let stone = if p.y > self.rim {
                    ramp(&GARNET, facet)
                } else {
                    ramp(&RUBY, facet)
                };
                stone
                    + IVORY * (0.8 * smoothstep(0.92, 0.99, mirrored))
                    + RUBY[3].1 * (0.3 * fresnel)
            }
        }
    }
}
