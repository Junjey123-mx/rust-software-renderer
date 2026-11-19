//! The ring of diamond playing cards around House of Cards: one shared
//! low-poly card (a thin slab with chamfered corners) and a procedural
//! material that paints the diamond suit on its face, so no texture is
//! loaded and every card reuses the same vertices.
//!
//! Card space: the card's center at the origin, its height along `+Y`, its
//! width along `X` and its face looking along `+Z` (the back along `-Z`).

use crate::geometry::{Mesh, Vertex};
use crate::shading::math::{hex, mix3, smoothstep};
use crate::shading::sphere::rotate_axis;
use crate::solar::shapes::{End, frame_matrix, loft};
use crate::solar::skybox::random01;
use raylib::math::{Matrix, Vector3};
use std::f32::consts::TAU;
use std::sync::Arc;

/// Card size in card units: height 1, the 2.5 x 3.5 proportion of a
/// playing card, and a thin slab.
pub const CARD_HEIGHT: f32 = 1.0;
pub const CARD_WIDTH: f32 = 0.714;
pub const CARD_THICKNESS: f32 = 0.035;
/// Cut of each corner along both edges.
pub const CHAMFER: f32 = 0.07;

/// Outline of the face: a rectangle with its four corners cut, in order
/// around the card.
pub fn card_outline() -> [(f32, f32); 8] {
    let (w, h, c) = (CARD_WIDTH / 2.0, CARD_HEIGHT / 2.0, CHAMFER);
    [
        (w - c, -h),
        (w, -h + c),
        (w, h - c),
        (w - c, h),
        (-w + c, h),
        (-w, h - c),
        (-w, -h + c),
        (-w + c, -h),
    ]
}

/// The card slab: the outline at the back and at the face joined by the
/// edge, both caps closed (28 triangles).
pub fn card_mesh() -> Mesh {
    let ring = |z: f32| {
        card_outline()
            .iter()
            .map(|&(x, y)| Vector3::new(x, y, z))
            .collect::<Vec<_>>()
    };
    let t = CARD_THICKNESS / 2.0;
    loft(&[ring(-t), ring(t)], End::Cap, End::Cap, Vector3::zero())
}

/// Which side of the card a face belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardSide {
    Face,
    Back,
    Edge,
}

impl CardSide {
    /// From a card-space face normal.
    pub fn of(normal: Vector3) -> Self {
        let z = normal.normalized().z;
        if z > 0.5 {
            Self::Face
        } else if z < -0.5 {
            Self::Back
        } else {
            Self::Edge
        }
    }
}

/// The deck's colors, from the gas giant's palette.
pub const CARD_DARK: Vector3 = hex(0x1A0A20);
pub const LATTICE: Vector3 = hex(0x3A1450);
pub const BORDER: Vector3 = hex(0x6B0F1A);
pub const EDGE: Vector3 = hex(0x4E0E27);
pub const MAGENTA: Vector3 = hex(0xC13CFF);
pub const PINK: Vector3 = hex(0xFF4FB8);
pub const BLUSH: Vector3 = hex(0xFFC6E8);
pub const VIOLET: Vector3 = hex(0x52106A);

/// Diamond distance: below 1 inside a rhombus of half-axes `a` and `b`
/// centered at `(cx, cy)`.
fn rhombus(u: f32, v: f32, cx: f32, cy: f32, a: f32, b: f32) -> f32 {
    (u - cx).abs() / a + (v - cy).abs() / b
}

/// Albedo and self-emission at card-space point `(x, y)` of a side: the
/// face carries a large neon diamond and two corner pips inside a corinto
/// border over dark violet; the back a violet lattice with a smaller
/// diamond medallion; the edge is corinto.
pub fn card_pattern(side: CardSide, x: f32, y: f32) -> (Vector3, f32) {
    let (u, v) = (x / (CARD_WIDTH / 2.0), y / (CARD_HEIGHT / 2.0));
    if side == CardSide::Edge {
        return (EDGE, 0.0);
    }
    if u.abs() > 0.88 || v.abs() > 0.91 {
        return (BORDER, 0.1);
    }
    let lattice = {
        let (a, b) = ((u * 4.0).fract().abs() - 0.5, (v * 5.0).fract().abs() - 0.5);
        a.abs() + b.abs() < 0.12
    };
    match side {
        CardSide::Face => {
            let d = rhombus(u, v, 0.0, 0.0, 0.5, 0.62);
            if d < 1.0 {
                let core = 1.0 - d;
                let neon = mix3(MAGENTA, PINK, smoothstep(0.0, 0.5, core));
                let rim = smoothstep(0.88, 0.97, d);
                return (
                    mix3(neon, BLUSH, 0.15 * smoothstep(0.6, 1.0, core) + rim),
                    0.4,
                );
            }
            let pip =
                rhombus(u, v, -0.62, 0.74, 0.13, 0.17).min(rhombus(u, v, 0.62, -0.74, 0.13, 0.17));
            if pip < 1.0 {
                return (PINK, 0.35);
            }
            (if lattice { LATTICE } else { CARD_DARK }, 0.0)
        }
        _ => {
            if rhombus(u, v, 0.0, 0.0, 0.3, 0.38) < 1.0 {
                return (MAGENTA, 0.25);
            }
            (if lattice { MAGENTA * 0.6 } else { VIOLET }, 0.05)
        }
    }
}

/// One card ready to shade a frame: the star's light in card space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardShader {
    pub light: Vector3,
}

impl CardShader {
    /// Lit by the star on its own side, the diamond glowing like the gas
    /// giant's neon storms even in shadow.
    pub fn shade(&self, p: Vector3, face_normal: Vector3) -> Vector3 {
        let n = face_normal.normalized();
        let (albedo, glow) = card_pattern(CardSide::of(n), p.x, p.y);
        albedo * (0.3 + 0.5 * n.dot(self.light).max(0.0) + glow)
    }
}

/// Layout of the ring, in radii of the planet it circles, so the ring
/// grows with the planet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardRingSpec {
    pub cards: usize,
    /// Distance from the planet's center to the cards' centers.
    pub radius: f32,
    /// Largest radial offset of a card from `radius`.
    pub spread: f32,
    /// Largest offset of a card off the ring plane.
    pub thickness: f32,
    /// Card height (its long side lies along the radius).
    pub card_size: f32,
    /// Tilt of the ring plane from the orbital plane, radians.
    pub inclination: f32,
    /// Azimuth of the axis the ring tilts around, radians.
    pub node: f32,
    /// Mean tilt of the cards' faces from the ring plane towards the
    /// direction of travel, radians, and its largest variation.
    pub tilt: f32,
    pub tilt_spread: f32,
    /// Angular speed of the whole ring around the planet, radians per
    /// simulated second: one speed for every card, so their gaps hold.
    pub orbit_speed: f32,
    /// Largest rock of a card about its long side, radians.
    pub flutter: f32,
    pub seed: u32,
}

impl Default for CardRingSpec {
    /// Twenty cards about 1.85 radii out, so the outer edge stays within
    /// the 2.2 radii the system reserves around House of Cards.
    fn default() -> Self {
        Self {
            cards: 20,
            radius: 1.85,
            spread: 0.06,
            thickness: 0.04,
            card_size: 0.3,
            inclination: 24f32.to_radians(),
            node: 0.6,
            tilt: 40f32.to_radians(),
            tilt_spread: 12f32.to_radians(),
            orbit_speed: 0.25,
            flutter: 7f32.to_radians(),
            seed: 7,
        }
    }
}

/// Fixed character of one card, drawn once from the ring's seed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardSlot {
    /// Angle around the ring at time 0, radians.
    pub phase: f32,
    /// Distance from the planet's center, planet radii.
    pub radius: f32,
    /// Offset along the ring's normal, planet radii.
    pub height: f32,
    /// Tilt of the face, radians.
    pub tilt: f32,
    /// Its own flutter: angular speed (radians per second) and phase.
    pub flutter_speed: f32,
    pub flutter_phase: f32,
}

/// Placement of one card in the world: unit axes of card space and the
/// card height in world units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardFrame {
    pub origin: Vector3,
    pub x: Vector3,
    pub y: Vector3,
    pub z: Vector3,
    pub size: f32,
}

impl CardFrame {
    /// Model matrix of the shared card mesh.
    pub fn matrix(&self) -> Matrix {
        frame_matrix(
            self.x * self.size,
            self.y * self.size,
            self.z * self.size,
            self.origin,
        )
    }

    /// A world direction in card space (rotation only).
    pub fn to_card_dir(&self, d: Vector3) -> Vector3 {
        Vector3::new(d.dot(self.x), d.dot(self.y), d.dot(self.z))
    }
}

/// The cards around one planet: their seeded slots and the one card mesh
/// they all share.
#[derive(Debug, Clone)]
pub struct CardRing {
    pub spec: CardRingSpec,
    pub slots: Vec<CardSlot>,
    pub mesh: Arc<Vec<Vertex>>,
}

/// Corner color of the card mesh, seen only in wireframe.
pub const CARD_WIRE: Vector3 = PINK;

impl CardRing {
    /// Evenly spaced cards, each nudged by the seed: its angle by at most
    /// 15 % of a slot, its radius, height and tilt within the spec.
    pub fn new(spec: CardRingSpec) -> Self {
        let mut state = spec.seed;
        let mut signed = || 2.0 * random01(&mut state) - 1.0;
        let step = TAU / spec.cards as f32;
        let slots = (0..spec.cards)
            .map(|i| CardSlot {
                phase: step * (i as f32 + 0.15 * signed()),
                radius: spec.radius + spec.spread * signed(),
                height: spec.thickness * signed(),
                tilt: spec.tilt + spec.tilt_spread * signed(),
                flutter_speed: 0.7 + 0.25 * signed(),
                flutter_phase: TAU * (0.5 + 0.5 * signed()),
            })
            .collect();
        Self {
            spec,
            slots,
            mesh: Arc::new(card_mesh().expand_vertices(CARD_WIRE)),
        }
    }

    /// The ring of House of Cards.
    pub fn diamonds() -> Self {
        Self::new(CardRingSpec::default())
    }

    /// Unit normal of the ring plane: `+Y` tilted by the inclination
    /// around the node axis.
    pub fn normal(&self) -> Vector3 {
        let (s, c) = self.spec.node.sin_cos();
        rotate_axis(
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(s, 0.0, c),
            self.spec.inclination,
        )
    }

    /// Farthest corner of a card from its center, planet radii.
    pub fn card_reach(&self) -> f32 {
        Vector3::new(CARD_WIDTH, CARD_HEIGHT, CARD_THICKNESS).length() / 2.0 * self.spec.card_size
    }

    /// Farthest reach of a card from the planet's center, planet radii.
    pub fn extent(&self) -> f32 {
        self.spec.radius + self.spec.spread + self.spec.thickness + self.card_reach()
    }

    /// Card `index` at simulated `time` (the scene clock's, so a paused
    /// clock freezes the ring): carried along the ring at the ring's speed
    /// and rocking about its long side at its own.
    pub fn pose(&self, index: usize, center: Vector3, radius: f32, time: f32) -> CardFrame {
        let slot = &self.slots[index];
        let flutter = self.spec.flutter * (slot.flutter_speed * time + slot.flutter_phase).sin();
        self.frame(index, center, radius, self.spec.orbit_speed * time, flutter)
    }

    /// Card `index` around a planet at `center` of `radius`, at `angle`
    /// radians along the ring from its slot and with `flutter` radians
    /// added to its tilt: the long side along the radius and the face
    /// turned from the ring's normal towards the direction of travel.
    pub fn frame(
        &self,
        index: usize,
        center: Vector3,
        radius: f32,
        angle: f32,
        flutter: f32,
    ) -> CardFrame {
        let slot = &self.slots[index];
        let normal = self.normal();
        let (s, c) = self.spec.node.sin_cos();
        let e1 = Vector3::new(s, 0.0, c);
        let e2 = normal.cross(e1);
        let (sa, ca) = (slot.phase + angle).sin_cos();
        let radial = e1 * ca + e2 * sa;
        let travel = normal.cross(radial);
        let (st, ct) = (slot.tilt + flutter).sin_cos();
        let face = normal * ct + travel * st;
        CardFrame {
            origin: center + (radial * slot.radius + normal * slot.height) * radius,
            x: radial.cross(face),
            y: radial,
            z: face,
            size: self.spec.card_size * radius,
        }
    }
}
