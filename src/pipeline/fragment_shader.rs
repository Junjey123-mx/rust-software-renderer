//! Stage 4 (color decision): passthrough fragment shader.

use crate::rendering::Fragment;
use raylib::math::Vector3;

/// Returns the fragment's own color unchanged.
pub fn fragment_shader(fragment: &Fragment) -> Vector3 {
    fragment.color
}
