//! Gravity of the planets on the spacecraft: softened inverse square, so
//! the pull stays finite at the center. The bodies themselves follow their
//! orbits (no n-body motion).

use raylib::math::Vector3;

/// One attracting body this instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GravitySource {
    pub center: Vector3,
    /// Gravitational parameter, world units^3 / s^2.
    pub mu: f32,
    /// Softening length `epsilon` (0.25 R).
    pub softening: f32,
}

impl GravitySource {
    /// Source of a body of `radius` at `center`, softened by 0.25 R.
    pub fn new(center: Vector3, mu: f32, radius: f32) -> Self {
        Self {
            center,
            mu,
            softening: 0.25 * radius,
        }
    }
}

/// Sum of `mu (c - p) / (|c - p|^2 + epsilon^2)^(3/2)` over `sources`; a
/// source whose term is not finite adds nothing.
pub fn gravity_accel(p: Vector3, sources: &[GravitySource]) -> Vector3 {
    sources.iter().fold(Vector3::zero(), |sum, s| {
        let d = s.center - p;
        let r2 = d.dot(d) + s.softening * s.softening;
        let term = d * (s.mu / (r2 * r2.sqrt()));
        if term.x.is_finite() && term.y.is_finite() && term.z.is_finite() {
            sum + term
        } else {
            sum
        }
    })
}

/// `mu` of a planet of `radius` within the bounds of document B (section
/// 7.4): the pull at the surface at most a quarter of the thrust
/// (`thrust_accel`), `0.25 a 1.0954 R^2` with the softening, and the escape
/// speed at most half the `cruise`, `(0.5 v)^2 R / 2`.
pub fn design_mu(radius: f32, thrust_accel: f32, cruise: f32) -> f32 {
    let surface = 0.25 * thrust_accel * 1.0625f32.powf(1.5) * radius * radius;
    let escape = (0.5 * cruise).powi(2) * radius / 2.0;
    surface.min(escape)
}
