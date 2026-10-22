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
    DirectionalLight, IncompletePrimitive, MappingError, OrbitView, Uniforms, ViewPlane, Viewport,
    fit_to_viewport, fragment_shader, primitive_assembly, project_orbit, rasterize_with_mode,
    shade_faces, vertex_shader,
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
    /// Orbit camera with a fixed model-derived scale; writes view depth.
    Orbit(OrbitView),
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

/// Options of the final stage. Defaults reproduce laboratories 4 and 5.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RenderSettings {
    /// Resolve overlapping filled faces with the depth buffer (nearest wins).
    /// Wireframe always draws every edge regardless of this flag.
    pub depth_test: bool,
    /// Flat-shade filled faces from their normals. Wireframe keeps the
    /// vertex colors so outlines stay bright.
    pub shading: Option<DirectionalLight>,
}

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
/// clip against the framebuffer size. Last write wins (laboratory 5).
pub fn render_with_mode(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    projection: Projection,
    mode: TriangleMode,
) -> Result<RenderStats, RenderError> {
    render_with_settings(
        framebuffer,
        uniforms,
        vertex_array,
        projection,
        mode,
        RenderSettings::default(),
    )
}

/// Interactive view: orbit projection plus depth testing for filled modes.
pub fn render_interactive(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    view: OrbitView,
    mode: TriangleMode,
) -> Result<RenderStats, RenderError> {
    render_with_settings(
        framebuffer,
        uniforms,
        vertex_array,
        Projection::Orbit(view),
        mode,
        RenderSettings {
            depth_test: true,
            shading: Some(DirectionalLight::default()),
        },
    )
}

/// The single pipeline body shared by every entry point.
pub fn render_with_settings(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    projection: Projection,
    mode: TriangleMode,
    settings: RenderSettings,
) -> Result<RenderStats, RenderError> {
    // 1. Vertex processing.
    let mut transformed: Vec<Vertex> = vertex_array
        .iter()
        .map(|v| vertex_shader(v, uniforms))
        .collect();
    match projection {
        Projection::Screen => {}
        Projection::Fit {
            plane,
            margin_ratio,
        } => {
            let viewport = Viewport::new(framebuffer.width(), framebuffer.height(), margin_ratio);
            fit_to_viewport(&mut transformed, plane, viewport).map_err(RenderError::Mapping)?;
        }
        Projection::Orbit(view) => {
            project_orbit(
                &mut transformed,
                &view,
                framebuffer.width(),
                framebuffer.height(),
            )
            .map_err(RenderError::Mapping)?;
        }
    }

    // Demonstration shading (interactive filled modes): per-face color from
    // the model-space normal, computed before assembly so every fragment of
    // the face inherits it through the unchanged passthrough shader.
    if let Some(light) = settings.shading
        && mode != TriangleMode::Wireframe
    {
        shade_faces(&mut transformed, &light);
    }

    // 2. Primitive assembly.
    let triangles = primitive_assembly(&transformed).map_err(RenderError::Primitive)?;

    // 3. Rasterization.
    let bounds = RasterBounds::new(framebuffer.width(), framebuffer.height());
    let fragments = rasterize_with_mode(&triangles, mode, bounds);

    // 4. Fragment processing.
    let depth_test = settings.depth_test && mode != TriangleMode::Wireframe;
    for fragment in &fragments {
        let color = fragment_shader(fragment);
        let (x, y) = (fragment.position.x as i32, fragment.position.y as i32);
        if depth_test {
            framebuffer.point_depth(x, y, color, fragment.depth);
        } else {
            framebuffer.point(x, y, color);
        }
    }

    Ok(RenderStats {
        vertices_processed: transformed.len(),
        triangles_processed: triangles.len(),
        fragments_generated: fragments.len(),
    })
}
