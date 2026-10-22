//! Filled triangle rasterization with barycentric coordinates.
//!
//! Scan the clipped bounding box of the triangle; for each pixel center
//! compute the three barycentric weights `w_i = E(opposite edge, P) / area2`
//! and emit a fragment when the point is inside under the shared top-left
//! rule. Winding is normalised locally; zero-area triangles emit nothing.

use crate::geometry::Vertex;
use crate::rasterization::coverage::{
    Point, RasterBounds, center_range, edge, edge_accepts, orient, pixel_center, vertex_point,
};
use crate::rendering::Fragment;

/// Barycentric weights of `p` relative to an oriented triangle, or `None`
/// when `p` is not covered. Weights are relative to `(a, b, c)` in that order.
pub fn barycentric_weights(tri: &[Point; 3], area2: f64, p: Point) -> Option<[f64; 3]> {
    let [a, b, c] = *tri;
    // Numerators: E of the edge opposite to each vertex.
    let e_bc = edge(b, c, p); // weight of a
    let e_ca = edge(c, a, p); // weight of b
    let e_ab = edge(a, b, p); // weight of c
    let inside = edge_accepts(a, b, e_ab) && edge_accepts(b, c, e_bc) && edge_accepts(c, a, e_ca);
    inside.then(|| [e_bc / area2, e_ca / area2, e_ab / area2])
}

/// Interior fragments of the triangle, clipped to `bounds`. Uniform color
/// taken from `a`; depth 0 as in the wireframe path.
pub fn fill_barycentric(a: &Vertex, b: &Vertex, c: &Vertex, bounds: RasterBounds) -> Vec<Fragment> {
    if bounds.is_empty() {
        return Vec::new();
    }
    let Some((tri, area2)) = orient(vertex_point(a), vertex_point(b), vertex_point(c)) else {
        return Vec::new();
    };
    let xs = tri.map(|p| p.0);
    let ys = tri.map(|p| p.1);
    let (min_x, max_x) = (
        xs.iter().cloned().fold(f64::INFINITY, f64::min),
        xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );
    let (min_y, max_y) = (
        ys.iter().cloned().fold(f64::INFINITY, f64::min),
        ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );
    let Some((x0, x1)) = center_range(min_x, max_x, bounds.width) else {
        return Vec::new();
    };
    let Some((y0, y1)) = center_range(min_y, max_y, bounds.height) else {
        return Vec::new();
    };

    let mut fragments = Vec::new();
    for y in y0..=y1 {
        let py = pixel_center(y);
        for x in x0..=x1 {
            let p = (pixel_center(x), py);
            if barycentric_weights(&tri, area2, p).is_some() {
                fragments.push(Fragment::new(x as f32, y as f32, a.color, 0.0));
            }
        }
    }
    fragments
}
