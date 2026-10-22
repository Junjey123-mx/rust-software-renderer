//! Flat (orthographic, no perspective) mapping of model space to pixels.
//!
//! Two model axes are selected as the screen plane, the projected bounding
//! box is centered in the viewport and scaled uniformly to fit inside a
//! margin. This is the "scale and translate x/y" adaptation allowed by the
//! laboratory; it is not a camera.

use crate::geometry::Vertex;
use raylib::math::Vector3;
use std::fmt;

/// Which pair of model axes becomes screen (u, v).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewPlane {
    /// Literal lesson example: screen x = model x, screen up = model y.
    XY,
    /// Top view: screen x = model x, screen up = model z. Default for the ship.
    #[default]
    XZ,
}

impl ViewPlane {
    fn project(self, p: Vector3) -> (f32, f32) {
        match self {
            Self::XY => (p.x, p.y),
            Self::XZ => (p.x, p.z),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    /// Fraction of each dimension left empty on each side (0.1 = 10%).
    pub margin_ratio: f32,
}

impl Viewport {
    pub fn new(width: u32, height: u32, margin_ratio: f32) -> Self {
        Self {
            width,
            height,
            margin_ratio,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingError {
    NoVertices,
    InvalidViewport,
    NonFinitePosition,
}

impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoVertices => write!(f, "nothing to map: no vertices"),
            Self::InvalidViewport => {
                write!(f, "viewport must be non-empty with margin in [0, 0.5)")
            }
            Self::NonFinitePosition => write!(f, "a vertex has a non-finite transformed position"),
        }
    }
}

impl std::error::Error for MappingError {}

/// Diagnostics of the fit that was applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenFit {
    pub scale: f32,
    pub center_u: f32,
    pub center_v: f32,
    pub span_u: f32,
    pub span_v: f32,
}

/// Rewrites every `transformed_position` into screen pixels (z is kept).
///
/// Call once, after the vertex shader and before primitive assembly.
pub fn fit_to_viewport(
    vertices: &mut [Vertex],
    plane: ViewPlane,
    viewport: Viewport,
) -> Result<ScreenFit, MappingError> {
    if vertices.is_empty() {
        return Err(MappingError::NoVertices);
    }
    if viewport.width == 0 || viewport.height == 0 || !(0.0..0.5).contains(&viewport.margin_ratio) {
        return Err(MappingError::InvalidViewport);
    }

    let (mut min_u, mut min_v) = (f32::INFINITY, f32::INFINITY);
    let (mut max_u, mut max_v) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for v in vertices.iter() {
        let p = v.transformed_position;
        if !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()) {
            return Err(MappingError::NonFinitePosition);
        }
        let (u, w) = plane.project(p);
        min_u = min_u.min(u);
        max_u = max_u.max(u);
        min_v = min_v.min(w);
        max_v = max_v.max(w);
    }

    let span_u = max_u - min_u;
    let span_v = max_v - min_v;
    let available_w = viewport.width as f32 * (1.0 - 2.0 * viewport.margin_ratio);
    let available_h = viewport.height as f32 * (1.0 - 2.0 * viewport.margin_ratio);

    // Uniform scale; a zero span on one axis defers to the other axis, and a
    // single point maps to the center with scale 1.
    let scale = match (span_u > 0.0, span_v > 0.0) {
        (true, true) => (available_w / span_u).min(available_h / span_v),
        (true, false) => available_w / span_u,
        (false, true) => available_h / span_v,
        (false, false) => 1.0,
    };

    let center_u = (min_u + max_u) / 2.0;
    let center_v = (min_v + max_v) / 2.0;
    let screen_cx = viewport.width as f32 / 2.0;
    let screen_cy = viewport.height as f32 / 2.0;

    for v in vertices.iter_mut() {
        let (u, w) = plane.project(v.transformed_position);
        // Framebuffer y grows downwards, so the model's "up" axis is flipped.
        v.transformed_position = Vector3::new(
            screen_cx + (u - center_u) * scale,
            screen_cy - (w - center_v) * scale,
            v.transformed_position.z,
        );
    }

    Ok(ScreenFit {
        scale,
        center_u,
        center_v,
        span_u,
        span_v,
    })
}

// ---------------------------------------------------------------------------
// Orbit projection (interactive view). Independent of `fit_to_viewport`.
// ---------------------------------------------------------------------------

use crate::pipeline::OrbitCamera;

/// Bounding sphere of a vertex list in model space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelBounds {
    pub center: Vector3,
    pub radius: f32,
}

/// Center of the axis-aligned bounding box and the radius of the sphere
/// around it that contains every `position`. `None` for empty or non-finite
/// input.
pub fn model_bounds(vertices: &[Vertex]) -> Option<ModelBounds> {
    let first = vertices.first()?.position;
    let (mut min, mut max) = (first, first);
    for v in vertices {
        let p = v.position;
        if !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()) {
            return None;
        }
        min = Vector3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
        max = Vector3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
    }
    let center = (min + max) * 0.5;
    let radius = vertices
        .iter()
        .map(|v| (v.position - center).length())
        .fold(0.0_f32, f32::max);
    Some(ModelBounds { center, radius })
}

/// Camera plus a fixed pixels-per-world-unit scale derived once from the
/// model radius, so rotating never re-fits (no zoom pulsing) and the zoom
/// factor is the only thing that changes apparent size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitView {
    pub camera: OrbitCamera,
    pub base_scale: f32,
}

impl OrbitView {
    /// Scale such that the bounding sphere fits the viewport minus margin at
    /// zoom 1 from any orientation.
    pub fn new(
        camera: OrbitCamera,
        bounds: &ModelBounds,
        viewport: Viewport,
    ) -> Result<Self, MappingError> {
        if viewport.width == 0
            || viewport.height == 0
            || !(0.0..0.5).contains(&viewport.margin_ratio)
        {
            return Err(MappingError::InvalidViewport);
        }
        if !bounds.radius.is_finite() {
            return Err(MappingError::NonFinitePosition);
        }
        let available =
            (viewport.width.min(viewport.height) as f32) * (1.0 - 2.0 * viewport.margin_ratio);
        let radius = if bounds.radius > 0.0 {
            bounds.radius
        } else {
            1.0
        };
        Ok(Self {
            camera,
            base_scale: available / (2.0 * radius),
        })
    }

    pub fn scale(&self) -> f32 {
        self.base_scale * self.camera.zoom
    }
}

/// Rewrites every `transformed_position` into `(screen_x, screen_y, depth)`:
/// the point is expressed in the camera basis, scaled by `base_scale * zoom`
/// and centered in the viewport; `depth` is the view-space distance along
/// `forward` (smaller is nearer). Call once per frame after the vertex
/// shader and before primitive assembly.
pub fn project_orbit(
    vertices: &mut [Vertex],
    view: &OrbitView,
    width: u32,
    height: u32,
) -> Result<(), MappingError> {
    if width == 0 || height == 0 {
        return Err(MappingError::InvalidViewport);
    }
    let s = view.scale();
    if !s.is_finite() || s <= 0.0 {
        return Err(MappingError::NonFinitePosition);
    }
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    for v in vertices.iter_mut() {
        let p = v.transformed_position;
        if !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()) {
            return Err(MappingError::NonFinitePosition);
        }
        let e = view.camera.world_to_view(p);
        v.transformed_position = Vector3::new(cx + e.x * s, cy - e.y * s, e.z);
    }
    Ok(())
}
