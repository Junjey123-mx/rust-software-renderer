//! One transformed celestial body and the shader inputs it needs.

use crate::pipeline::{ShaderUniforms, Transform};
use raylib::math::Vector3;

/// Shader uniforms of one instance: the laboratory six shaders expect the
/// light and eye directions in object space, so both world directions are
/// rotated back through the instance transform. Non-unit inputs are
/// normalised; a zero vector is kept as zero.
pub fn instance_uniforms(
    transform: &Transform,
    time: f32,
    light_world: Vector3,
    eye_world: Vector3,
) -> ShaderUniforms {
    ShaderUniforms {
        time,
        light_direction: unit(transform.to_object_dir(light_world)),
        eye_direction: unit(transform.to_object_dir(eye_world)),
    }
}

fn unit(v: Vector3) -> Vector3 {
    let len = v.length();
    if len.is_finite() && len > 1e-6 {
        v / len
    } else {
        Vector3::zero()
    }
}
