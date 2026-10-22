//! Demonstration flat shading: one tone per triangle from its model-space
//! normal and a fixed directional light. These are computed colors, not the
//! Blender materials (the OBJ loader keeps positions only).

use crate::geometry::Vertex;
use raylib::math::Vector3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    /// Direction *towards* the light, world space (normalised on use).
    pub direction: Vector3,
    /// Minimum intensity so faces turned away stay visible, in `[0, 1]`.
    pub ambient: f32,
    /// Base albedo multiplied by the intensity.
    pub base_color: Vector3,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vector3::new(0.45, 1.0, -0.6),
            ambient: 0.28,
            base_color: Vector3::new(0.78, 0.84, 0.95),
        }
    }
}

/// Unit normal of the triangle `a, b, c` in model space; `None` when the
/// triangle is degenerate or non-finite.
pub fn face_normal(a: Vector3, b: Vector3, c: Vector3) -> Option<Vector3> {
    let n = (b - a).cross(c - a);
    let len = n.length();
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some(n / len)
}

/// Lambert intensity in `[ambient, 1]`. Two-sided: faces lit from behind are
/// shaded like front faces, since no back-face culling is performed.
pub fn intensity(normal: Vector3, light: &DirectionalLight) -> f32 {
    let l = light.direction.normalized();
    let ambient = light.ambient.clamp(0.0, 1.0);
    let diffuse = normal.dot(l).abs().clamp(0.0, 1.0);
    ambient + (1.0 - ambient) * diffuse
}

/// Colors every triangle (consecutive triples) of an expanded vertex list.
/// Degenerate faces receive the ambient tone.
pub fn shade_faces(vertices: &mut [Vertex], light: &DirectionalLight) {
    for tri in vertices.chunks_exact_mut(3) {
        let k = match face_normal(tri[0].position, tri[1].position, tri[2].position) {
            Some(n) => intensity(n, light),
            None => light.ambient.clamp(0.0, 1.0),
        };
        let color = light.base_color * k;
        for v in tri {
            v.color = color;
        }
    }
}
