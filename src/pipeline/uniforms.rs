//! Values shared by every vertex of one draw call.

use raylib::math::Matrix;

#[derive(Debug, Clone, Copy)]
pub struct Uniforms {
    /// 4x4 model matrix (translation, rotation, scale). Identity by default.
    pub model_matrix: Matrix,
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            model_matrix: Matrix::identity(),
        }
    }
}
