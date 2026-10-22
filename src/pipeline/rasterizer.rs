//! Stage 3: rasterization (wireframe only in laboratory 4).

use crate::geometry::Vertex;
use crate::rasterization::triangle;
use crate::rendering::Fragment;

/// Produces the edge fragments of every triangle. Writes nothing.
pub fn rasterize(triangles: &[[Vertex; 3]]) -> Vec<Fragment> {
    let mut fragments = Vec::new();
    for tri in triangles {
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2]));
    }
    fragments
}
