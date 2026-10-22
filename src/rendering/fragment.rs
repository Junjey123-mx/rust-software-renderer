//! Fragment: a candidate pixel produced by rasterization.

use raylib::math::{Vector2, Vector3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fragment {
    /// Screen position; origin at the top-left, `y` grows downwards.
    pub position: Vector2,
    /// RGB in `0.0..=1.0`.
    pub color: Vector3,
    /// Kept as an attribute; no depth test is performed in laboratory 4.
    pub depth: f32,
}

impl Fragment {
    pub fn new(x: f32, y: f32, color: Vector3, depth: f32) -> Self {
        Self {
            position: Vector2::new(x, y),
            color,
            depth,
        }
    }
}
