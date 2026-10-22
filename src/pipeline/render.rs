//! Pipeline coordination: the four course stages in order.
//!
//! 1. vertex processing (`vertex_shader`, then viewport fitting)
//! 2. primitive assembly
//! 3. rasterization (wireframe `triangle` -> `line`, or a filled mode)
//! 4. fragment processing (`fragment_shader` -> `framebuffer.point`)
//!
//! Laboratory 5 selects the stage-3 algorithm with `TriangleMode`; the other
//! three stages are shared verbatim.

use crate::geometry::Vertex;
use crate::pipeline::{
    IncompletePrimitive, MappingError, Uniforms, ViewPlane, Viewport, fit_to_viewport,
    fragment_shader, primitive_assembly, rasterize_with_mode, vertex_shader,
};
use crate::rasterization::{RasterBounds, TriangleMode};
use crate::rendering::Framebuffer;
use std::fmt;

/// How transformed positions become pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Positions are already in screen pixels (lesson-style test triangles).
    Screen,
    /// Orthographic fit of the chosen plane into the framebuffer.
    Fit { plane: ViewPlane, margin_ratio: f32 },
}

/// Real counts from the run, for diagnostics and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderStats {
    pub vertices_processed: usize,
    pub triangles_processed: usize,
    pub fragments_generated: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderError {
    Mapping(MappingError),
    Primitive(IncompletePrimitive),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mapping(e) => write!(f, "screen mapping failed: {e}"),
            Self::Primitive(e) => write!(f, "primitive assembly failed: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Laboratory 4 entry point: wireframe rendering.
pub fn render(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    projection: Projection,
) -> Result<RenderStats, RenderError> {
    render_with_mode(
        framebuffer,
        uniforms,
        vertex_array,
        projection,
        TriangleMode::Wireframe,
    )
}

/// Full pipeline with a selectable rasterization algorithm. Filled modes
/// clip against the framebuffer size.
pub fn render_with_mode(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    projection: Projection,
    mode: TriangleMode,
) -> Result<RenderStats, RenderError> {
    // 1. Vertex processing.
    let mut transformed: Vec<Vertex> = vertex_array
        .iter()
        .map(|v| vertex_shader(v, uniforms))
        .collect();
    if let Projection::Fit {
        plane,
        margin_ratio,
    } = projection
    {
        let viewport = Viewport::new(framebuffer.width(), framebuffer.height(), margin_ratio);
        fit_to_viewport(&mut transformed, plane, viewport).map_err(RenderError::Mapping)?;
    }

    // 2. Primitive assembly.
    let triangles = primitive_assembly(&transformed).map_err(RenderError::Primitive)?;

    // 3. Rasterization.
    let bounds = RasterBounds::new(framebuffer.width(), framebuffer.height());
    let fragments = rasterize_with_mode(&triangles, mode, bounds);

    // 4. Fragment processing.
    for fragment in &fragments {
        let color = fragment_shader(fragment);
        framebuffer.point(
            fragment.position.x as i32,
            fragment.position.y as i32,
            color,
        );
    }

    Ok(RenderStats {
        vertices_processed: transformed.len(),
        triangles_processed: triangles.len(),
        fragments_generated: fragments.len(),
    })
}
