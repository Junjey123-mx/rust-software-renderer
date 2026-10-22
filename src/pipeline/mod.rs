//! Graphics pipeline.
//!
//! Vertex processing, primitive assembly,
//! screen mapping, rasterization coordination,
//! and fragment processing.

pub mod uniforms;
pub mod vertex_shader;

pub use uniforms::Uniforms;
pub use vertex_shader::vertex_shader;
