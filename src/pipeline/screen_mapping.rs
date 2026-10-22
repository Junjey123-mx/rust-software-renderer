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
