//! Filled triangle rasterization by horizontal scanlines.
//!
//! For every pixel row the sample line `y + 0.5` is intersected with the
//! non-horizontal edges using the half-open interval `[min_y, max_y)` so a
//! vertex shared by two edges is counted once. The two crossings bound a
//! horizontal span; pixels strictly inside the span are emitted directly and
//! only the one or two pixels at each end are classified with the shared
//! top-left edge rule. A horizontal edge lying exactly on the sample line is
//! owned or rejected as a whole according to the same rule. This never walks
//! the full bounding box and never calls the barycentric routine.

use crate::geometry::Vertex;
use crate::rasterization::coverage::{
    Point, RasterBounds, center_range, covers, edge_accepts, orient, pixel_center, vertex_point,
};
use crate::rendering::Fragment;

/// Crossings of the sample line `sy` with the triangle edges, plus whether a
/// horizontal edge lies exactly on `sy` and is owned (`Some(bool)`).
fn row_crossings(tri: &[Point; 3], sy: f64) -> ([f64; 3], usize, Option<bool>) {
    let mut xs = [0.0; 3];
    let mut n = 0;
    let mut horizontal = None;
    for i in 0..3 {
        let s = tri[i];
        let e = tri[(i + 1) % 3];
        if s.1 == e.1 {
            if s.1 == sy {
                horizontal = Some(edge_accepts(s, e, 0.0));
            }
            continue;
        }
        let (lo, hi) = if s.1 < e.1 { (s, e) } else { (e, s) };
        if sy >= lo.1 && sy < hi.1 {
            let t = (sy - lo.1) / (hi.1 - lo.1);
            xs[n] = lo.0 + t * (hi.0 - lo.0);
            n += 1;
        }
    }
    (xs, n, horizontal)
}

/// Interior fragments of the triangle, clipped to `bounds`. Uniform color
/// taken from `a`; depth 0 as in the wireframe path.
pub fn fill_scanline(a: &Vertex, b: &Vertex, c: &Vertex, bounds: RasterBounds) -> Vec<Fragment> {
    if bounds.is_empty() {
        return Vec::new();
    }
    let Some((tri, _area2)) = orient(vertex_point(a), vertex_point(b), vertex_point(c)) else {
        return Vec::new();
    };
    let min_y = tri.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let max_y = tri.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let Some((row0, row1)) = center_range(min_y, max_y, bounds.height) else {
        return Vec::new();
    };

    let mut fragments = Vec::new();
    for y in row0..=row1 {
        let sy = pixel_center(y);
        let (xs, n, horizontal) = row_crossings(&tri, sy);
        if horizontal == Some(false) || n < 2 {
            continue;
        }
        let xl = xs[..n].iter().cloned().fold(f64::INFINITY, f64::min);
        let xr = xs[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        // Widen by one pixel so rounding of the crossings cannot drop a
        // center that sits exactly on an edge; the ends are classified by E.
        let Some((c0, c1)) = center_range(xl - 1.0, xr + 1.0, bounds.width) else {
            continue;
        };
        for x in c0..=c1 {
            let at_end = x < c0 + 2 || x > c1 - 2;
            if !at_end || covers(&tri, (pixel_center(x), sy)) {
                fragments.push(Fragment::new(x as f32, y as f32, a.color, 0.0));
            }
        }
    }
    fragments
}
