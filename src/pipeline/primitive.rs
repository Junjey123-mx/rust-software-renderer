//! Stage 2: primitive assembly.

use crate::geometry::Vertex;
use std::fmt;

/// The vertex count is not a multiple of three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncompletePrimitive {
    pub vertex_count: usize,
}

impl fmt::Display for IncompletePrimitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "vertex count {} is not a multiple of three",
            self.vertex_count
        )
    }
}

impl std::error::Error for IncompletePrimitive {}

/// Groups every three consecutive vertices into an independent triangle,
/// preserving order. Rejects leftover vertices instead of dropping them.
pub fn primitive_assembly(
    transformed_vertices: &[Vertex],
) -> Result<Vec<[Vertex; 3]>, IncompletePrimitive> {
    if !transformed_vertices.len().is_multiple_of(3) {
        return Err(IncompletePrimitive {
            vertex_count: transformed_vertices.len(),
        });
    }
    Ok(transformed_vertices
        .chunks_exact(3)
        .map(|g| [g[0], g[1], g[2]])
        .collect())
}
