//! Geometry domain.
//!
//! Responsible for vertices, indexed meshes,
//! and geometric representations.

pub mod demo;
pub mod mesh;
pub mod vertex;

pub use demo::{SCENE_HEIGHT, SCENE_WIDTH, triangle_scene};
pub use mesh::{Mesh, MeshError};
pub use vertex::Vertex;
