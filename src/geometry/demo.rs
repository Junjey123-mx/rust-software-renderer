//! Deterministic demonstration scene for comparing rasterization modes.
//!
//! Vertices are already in screen pixels for an 800x600 framebuffer, so the
//! scene is rendered with `Projection::Screen` (no viewport fit). The same
//! triangles, colors and order are used by every mode.

use crate::geometry::Vertex;
use raylib::math::Vector3;

pub const SCENE_WIDTH: u32 = 800;
pub const SCENE_HEIGHT: u32 = 600;

/// Seven triangles: small, large, horizontal edge, vertical edge, two sharing
/// a diagonal edge (different colors reveal edge ownership), and one that
/// extends past the right and bottom borders.
pub fn triangle_scene() -> Vec<Vertex> {
    let tris: [([(f32, f32); 3], Vector3); 7] = [
        (
            [(60.0, 60.0), (110.0, 70.0), (80.0, 120.0)],
            rgb(1.0, 0.3, 0.3),
        ),
        (
            [(150.0, 80.0), (520.0, 140.0), (330.0, 420.0)],
            rgb(1.0, 0.75, 0.2),
        ),
        (
            [(560.0, 80.0), (760.0, 80.0), (660.0, 250.0)],
            rgb(0.3, 0.9, 0.4),
        ),
        (
            [(580.0, 300.0), (580.0, 540.0), (760.0, 420.0)],
            rgb(0.3, 0.5, 1.0),
        ),
        (
            [(60.0, 320.0), (300.0, 320.0), (300.0, 560.0)],
            rgb(0.2, 0.9, 0.9),
        ),
        (
            [(60.0, 320.0), (300.0, 560.0), (60.0, 560.0)],
            rgb(0.9, 0.3, 0.9),
        ),
        (
            [(380.0, 480.0), (900.0, 520.0), (560.0, 700.0)],
            rgb(1.0, 1.0, 0.4),
        ),
    ];
    tris.iter()
        .flat_map(|(corners, color)| {
            corners
                .iter()
                .map(|&(x, y)| Vertex::new_with_color(Vector3::new(x, y, 0.0), *color))
        })
        .collect()
}

fn rgb(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3::new(r, g, b)
}
