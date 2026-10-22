//! Indexed triangle mesh.

use raylib::math::Vector3;
use std::fmt;

/// Errors raised when an indexed mesh violates its invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    /// The mesh has no faces to render.
    Empty,
    /// The index list cannot be split into triangles.
    IndexCountNotMultipleOfThree(usize),
    /// A face references a position that does not exist.
    IndexOutOfRange { index: usize, vertex_count: usize },
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "mesh has no triangles"),
            Self::IndexCountNotMultipleOfThree(n) => {
                write!(f, "index count {n} is not a multiple of three")
            }
            Self::IndexOutOfRange {
                index,
                vertex_count,
            } => write!(
                f,
                "index {index} is out of range for {vertex_count} vertices"
            ),
        }
    }
}

impl std::error::Error for MeshError {}

/// Positions stored once, faces stored as zero-based index triples.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vector3>,
    pub indices: Vec<usize>,
}

impl Mesh {
    /// Builds a mesh, validating that every consecutive index triple is a
    /// face whose references exist.
    pub fn new(vertices: Vec<Vector3>, indices: Vec<usize>) -> Result<Self, MeshError> {
        if indices.is_empty() {
            return Err(MeshError::Empty);
        }
        if !indices.len().is_multiple_of(3) {
            return Err(MeshError::IndexCountNotMultipleOfThree(indices.len()));
        }
        if let Some(&index) = indices.iter().find(|&&i| i >= vertices.len()) {
            return Err(MeshError::IndexOutOfRange {
                index,
                vertex_count: vertices.len(),
            });
        }
        Ok(Self { vertices, indices })
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}
