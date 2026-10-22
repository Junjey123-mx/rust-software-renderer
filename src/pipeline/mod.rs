//! Graphics pipeline.
//!
//! Vertex processing, primitive assembly,
//! screen mapping, rasterization coordination,
//! and fragment processing.

pub mod fragment_shader;
pub mod primitive;
pub mod rasterizer;
pub mod render;
pub mod screen_mapping;
pub mod uniforms;
pub mod vertex_shader;

pub use fragment_shader::fragment_shader;
pub use primitive::{IncompletePrimitive, primitive_assembly};
pub use rasterizer::{rasterize, rasterize_with_mode};
pub use render::{Projection, RenderError, RenderStats, render, render_with_mode};
pub use screen_mapping::{MappingError, ScreenFit, ViewPlane, Viewport, fit_to_viewport};
pub use uniforms::Uniforms;
pub use vertex_shader::vertex_shader;
