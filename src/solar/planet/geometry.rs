//! Planet geometry: the icosphere displaced by the planet's height field.
//! Each level of detail is a mesh of its own (no cracks between levels:
//! they are never drawn together), and the sea is part of the surface
//! (`HeightField::surface_radius`), not a second mesh.

use crate::geometry::Vertex;
use crate::solar::planet::HeightField;
use crate::solar::planet::icosphere::{icosphere, to_mesh};

/// The icosphere of `level` with every direction `n` at
/// `n * radius * field.surface_radius(n)`, in the pipeline's triangle list.
pub fn relief_mesh(level: u8, radius: f32, field: &HeightField) -> Result<Vec<Vertex>, String> {
    Ok(to_mesh(&icosphere(level)?, |n| {
        radius * field.surface_radius(n)
    }))
}
