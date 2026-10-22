//! Stage 3: rasterization.
//!
//! Laboratory 4 used only the wireframe path. Laboratory 5 selects one of
//! three fragment-generation algorithms per triangle; assembly and fragment
//! processing are unchanged.

use crate::geometry::Vertex;
use crate::rasterization::{RasterBounds, TriangleMode, fill_barycentric, fill_scanline, triangle};
use crate::rendering::Fragment;

/// Laboratory 4 entry point: outline fragments only (three `line` calls per
/// triangle). Writes nothing.
pub fn rasterize(triangles: &[[Vertex; 3]]) -> Vec<Fragment> {
    let mut fragments = Vec::new();
    for tri in triangles {
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2]));
    }
    fragments
}

/// Fragments of every triangle under `mode`. Filled modes clip to `bounds`;
/// `Wireframe` is exactly `rasterize` and ignores `bounds` (the framebuffer
/// discards out-of-range points, as in laboratory 4).
pub fn rasterize_with_mode(
    triangles: &[[Vertex; 3]],
    mode: TriangleMode,
    bounds: RasterBounds,
) -> Vec<Fragment> {
    match mode {
        TriangleMode::Wireframe => rasterize(triangles),
        TriangleMode::FilledBarycentric => {
            let mut fragments = Vec::new();
            for tri in triangles {
                fragments.extend(fill_barycentric(&tri[0], &tri[1], &tri[2], bounds));
            }
            fragments
        }
        TriangleMode::FilledScanline => {
            let mut fragments = Vec::new();
            for tri in triangles {
                fragments.extend(fill_scanline(&tri[0], &tri[1], &tri[2], bounds));
            }
            fragments
        }
    }
}
