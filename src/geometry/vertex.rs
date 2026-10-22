//! Renderable vertex record.

use raylib::math::Vector3;

/// A vertex carried through the pipeline.
///
/// `position` keeps the original model coordinates. `transformed_position`
/// is the only field the pipeline updates; the rasterizer reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: Vector3,
    pub transformed_position: Vector3,
    pub color: Vector3,
}

impl Vertex {
    pub const DEFAULT_COLOR: Vector3 = Vector3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    pub fn new(position: Vector3) -> Self {
        Self::new_with_color(position, Self::DEFAULT_COLOR)
    }

    pub fn new_with_color(position: Vector3, color: Vector3) -> Self {
        Self {
            position,
            transformed_position: position,
            color,
        }
    }
}
