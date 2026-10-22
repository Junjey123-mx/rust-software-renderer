//! Geometry domain.
//!
//! Responsible for vertices, indexed meshes,
//! and geometric representations.

pub mod mesh;
pub mod vertex;

pub use mesh::{Mesh, MeshError};
pub use vertex::Vertex;
