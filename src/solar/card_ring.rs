//! The ring of diamond playing cards around House of Cards: one shared
//! low-poly card (a thin slab with chamfered corners) and a procedural
//! material that paints the diamond suit on its face, so no texture is
//! loaded and every card reuses the same vertices.
//!
//! Card space: the card's center at the origin, its height along `+Y`, its
//! width along `X` and its face looking along `+Z` (the back along `-Z`).

use crate::geometry::Mesh;
use crate::shading::math::{hex, mix3, smoothstep};
use crate::solar::shapes::{End, loft};
use raylib::math::Vector3;

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
