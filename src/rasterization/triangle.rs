//! Wireframe triangle: exactly three edges drawn with the CPU line rasterizer.

use crate::geometry::Vertex;
use crate::rasterization::line;
use crate::rendering::Fragment;
use raylib::math::Vector2;

/// Emits the fragments of edges A→B, B→C and C→A.
///
/// Reads `transformed_position` (already in screen space) and uses the color
/// of `a` for all three edges. The interior is not filled.
pub fn triangle(a: &Vertex, b: &Vertex, c: &Vertex) -> Vec<Fragment> {
    let pa = screen(a);
    let pb = screen(b);
    let pc = screen(c);
    let mut fragments = line(pa, pb, a.color);
    fragments.extend(line(pb, pc, a.color));
    fragments.extend(line(pc, pa, a.color));
    fragments
}

fn screen(v: &Vertex) -> Vector2 {
    Vector2::new(v.transformed_position.x, v.transformed_position.y)
}
