//! Shared pixel-coverage conventions for the filled triangle modes.
//!
//! Screen space: `x` right, `y` down, valid pixels `0 <= x < width`,
//! `0 <= y < height`. The sample point of pixel `(x, y)` is its center
//! `(x + 0.5, y + 0.5)`. All intermediate math is `f64`.
//!
//! Edge function: `E(A, B, P) = (Px - Ax) * (By - Ay) - (Py - Ay) * (Bx - Ax)`.
//! A triangle is normalised so that `E(A, B, C) > 0`; a point is covered when
//! all three edges accept it under the top-left rule:
//! `E > 0`, or `E == 0` and the edge goes down (`By > Ay`) or is horizontal
//! going left (`By == Ay && Bx < Ax`). Shared edges between adjacent
//! triangles are therefore owned by exactly one of them.
//!
//! Both fill algorithms use these helpers; neither is implemented in terms
//! of the other.

use crate::geometry::Vertex;

/// Framebuffer size the rasterizer clips against. Independent of raylib.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RasterBounds {
    pub width: u32,
    pub height: u32,
}

impl RasterBounds {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

pub type Point = (f64, f64);

pub fn vertex_point(v: &Vertex) -> Point {
    (
        v.transformed_position.x as f64,
        v.transformed_position.y as f64,
    )
}

pub fn pixel_center(index: i32) -> f64 {
    index as f64 + 0.5
}

/// `E(A, B, P)`: twice the signed area of `A, B, P`.
pub fn edge(a: Point, b: Point, p: Point) -> f64 {
    (p.0 - a.0) * (b.1 - a.1) - (p.1 - a.1) * (b.0 - a.0)
}

/// Normalises winding so `E(A, B, C) > 0`. Returns the oriented triangle and
/// its doubled area, or `None` for degenerate (zero area) or non-finite input.
pub fn orient(a: Point, b: Point, c: Point) -> Option<([Point; 3], f64)> {
    if ![a, b, c].iter().all(|p| p.0.is_finite() && p.1.is_finite()) {
        return None;
    }
    let area2 = edge(a, b, c);
    if area2 > 0.0 {
        Some(([a, b, c], area2))
    } else if area2 < 0.0 {
        Some(([a, c, b], -area2))
    } else {
        None
    }
}

/// Top-left rule for edge `a -> b` of a positively oriented triangle.
pub fn edge_accepts(a: Point, b: Point, e: f64) -> bool {
    e > 0.0 || (e == 0.0 && (b.1 > a.1 || (b.1 == a.1 && b.0 < a.0)))
}

/// Whether sample point `p` is covered by an oriented triangle.
pub fn covers(tri: &[Point; 3], p: Point) -> bool {
    let [a, b, c] = *tri;
    edge_accepts(a, b, edge(a, b, p))
        && edge_accepts(b, c, edge(b, c, p))
        && edge_accepts(c, a, edge(c, a, p))
}

/// Inclusive range of pixel indices whose centers fall inside `[min, max]`,
/// clamped to `0..limit`. `None` when the range is empty. Casts are guarded
/// so huge coordinates cannot overflow.
pub fn center_range(min: f64, max: f64, limit: u32) -> Option<(i32, i32)> {
    if limit == 0 || !(min.is_finite() && max.is_finite()) || max < min {
        return None;
    }
    let hi = limit as f64;
    let first = (min - 0.5).ceil().clamp(-1.0, hi) as i32;
    let last = (max - 0.5).floor().clamp(-1.0, hi) as i32;
    let first = first.max(0);
    let last = last.min(limit as i32 - 1);
    (first <= last).then_some((first, last))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_sign_follows_orientation() {
        let a = (0.0, 0.0);
        let b = (10.0, 0.0);
        // y grows downward: a point below the edge a->b is on its left/positive side.
        assert!(edge(a, b, (5.0, 5.0)) < 0.0);
        assert!(edge(a, b, (5.0, -5.0)) > 0.0);
        assert_eq!(edge(a, b, (5.0, 0.0)), 0.0);
    }

    #[test]
    fn orient_normalises_both_windings_and_rejects_degenerate() {
        let (t1, area1) = orient((0.0, 0.0), (10.0, 0.0), (0.0, 10.0)).unwrap();
        let (t2, area2) = orient((0.0, 0.0), (0.0, 10.0), (10.0, 0.0)).unwrap();
        assert_eq!(t1, t2);
        assert_eq!(area1, 100.0);
        assert_eq!(area2, 100.0);
        assert!(orient((0.0, 0.0), (5.0, 5.0), (10.0, 10.0)).is_none());
        assert!(orient((1.0, 1.0), (1.0, 1.0), (4.0, 7.0)).is_none());
        assert!(orient((f64::NAN, 0.0), (1.0, 0.0), (0.0, 1.0)).is_none());
        assert!(orient((f64::INFINITY, 0.0), (1.0, 0.0), (0.0, 1.0)).is_none());
    }

    #[test]
    fn top_left_rule_owns_each_shared_edge_once() {
        // Two triangles sharing the diagonal (0,0)-(10,10) of a square.
        let (t1, _) = orient((0.0, 0.0), (10.0, 0.0), (10.0, 10.0)).unwrap();
        let (t2, _) = orient((0.0, 0.0), (10.0, 10.0), (0.0, 10.0)).unwrap();
        for p in [(2.5, 2.5), (5.5, 5.5), (7.5, 7.5)] {
            assert_ne!(covers(&t1, p), covers(&t2, p), "{p:?} owned by exactly one");
        }
        // Shared vertical edge and shared horizontal edge.
        let (l, _) = orient((0.0, 0.0), (5.0, 0.0), (5.0, 10.0)).unwrap();
        let (r, _) = orient((5.0, 0.0), (10.0, 0.0), (5.0, 10.0)).unwrap();
        assert_ne!(covers(&l, (5.0, 4.5)), covers(&r, (5.0, 4.5)));
        let (u, _) = orient((0.0, 0.0), (10.0, 0.0), (0.0, 5.0)).unwrap();
        let (d, _) = orient((0.0, 5.0), (10.0, 0.0), (0.0, 10.0)).unwrap();
        assert_ne!(covers(&u, (2.5, 5.0)), covers(&d, (2.5, 5.0)));
    }

    #[test]
    fn covers_interior_and_rejects_exterior() {
        let (t, _) = orient((0.0, 0.0), (10.0, 0.0), (0.0, 10.0)).unwrap();
        assert!(covers(&t, (2.5, 2.5)));
        assert!(!covers(&t, (7.5, 7.5)));
        assert!(!covers(&t, (-0.5, 0.5)));
        // Top-left corner is owned (down edge + leftward horizontal edge);
        // the other two vertices are excluded by the rule.
        assert!(covers(&t, (0.0, 0.0)));
        assert!(!covers(&t, (10.0, 0.0)));
        assert!(!covers(&t, (0.0, 10.0)));
    }

    #[test]
    fn center_range_maps_coordinates_to_pixel_indices() {
        assert_eq!(center_range(0.0, 10.0, 100), Some((0, 9)));
        assert_eq!(center_range(0.5, 10.5, 100), Some((0, 10)));
        assert_eq!(center_range(-15.0, 3.0, 100), Some((0, 2)));
        assert_eq!(center_range(95.0, 300.0, 100), Some((95, 99)));
        assert_eq!(center_range(-30.0, -1.0, 100), None);
        assert_eq!(center_range(200.0, 300.0, 100), None);
        assert_eq!(center_range(5.0, 5.0, 100), None);
        assert_eq!(center_range(4.4, 5.6, 100), Some((4, 5)));
        assert_eq!(center_range(4.6, 5.4, 100), None);
        assert_eq!(center_range(0.0, 10.0, 0), None);
        assert_eq!(center_range(-1e30, 1e30, 50), Some((0, 49)));
        assert_eq!(center_range(f64::NAN, 1.0, 50), None);
    }
}
