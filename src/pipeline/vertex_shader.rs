//! Stage 1: vertex processing with the model matrix.

use crate::geometry::Vertex;
use crate::pipeline::Uniforms;
use raylib::math::{Matrix, Vector3, Vector4};

/// Column-major `m0..m15` layout used by raylib, as in the course lesson.
fn multiply_matrix_vector4(m: &Matrix, v: &Vector4) -> Vector4 {
    Vector4::new(
        m.m0 * v.x + m.m4 * v.y + m.m8 * v.z + m.m12 * v.w,
        m.m1 * v.x + m.m5 * v.y + m.m9 * v.z + m.m13 * v.w,
        m.m2 * v.x + m.m6 * v.y + m.m10 * v.z + m.m14 * v.w,
        m.m3 * v.x + m.m7 * v.y + m.m11 * v.z + m.m15 * v.w,
    )
}

/// Applies the model matrix to `vertex.position` and stores the result in
/// `transformed_position`. All other attributes are preserved. No perspective
/// division is performed: the model matrix is affine, so `w` stays 1.
pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    let p = Vector4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
    let t = multiply_matrix_vector4(&uniforms.model_matrix, &p);
    Vertex {
        transformed_position: Vector3::new(t.x, t.y, t.z),
        ..*vertex
    }
}
