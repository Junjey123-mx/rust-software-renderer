//! Circular orbits on the XZ plane as closed-form functions of the simulated
//! time, so the position never depends on the frame rate.

use raylib::math::Vector3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
    /// Distance from the center to the body center, positive.
    pub radius: f32,
    /// Radians per simulated second (negative = clockwise seen from +Y).
    pub speed: f32,
    /// Radians at t = 0.
    pub phase: f32,
}

impl Orbit {
    /// `Err` for a non-positive or non-finite radius or non-finite angles.
    pub fn new(radius: f32, speed: f32, phase: f32) -> Result<Self, String> {
        if !(radius.is_finite() && radius > 0.0) {
            return Err(format!("orbit radius {radius} must be positive"));
        }
        if !(speed.is_finite() && phase.is_finite()) {
            return Err("orbit speed and phase must be finite".into());
        }
        Ok(Self {
            radius,
            speed,
            phase,
        })
    }

    /// Orbit of `radius` covered at `linear_speed` world units per second
    /// (signed: positive turns from +X towards +Z): `speed = v / r`, so the
    /// linear speed, not the angle, is what stays bounded on big orbits.
    pub fn from_linear(radius: f32, linear_speed: f32, phase: f32) -> Result<Self, String> {
        if !(radius.is_finite() && radius > 0.0) {
            return Err(format!("orbit radius {radius} must be positive"));
        }
        Self::new(radius, linear_speed / radius, phase)
    }

    pub fn angle_at(&self, time: f32) -> f32 {
        self.phase + self.speed * time
    }

    /// `center + radius * (cos a, 0, sin a)`: angle 0 lies on +X, PI/2 on +Z.
    pub fn position_at(&self, time: f32, center: Vector3) -> Vector3 {
        let (s, c) = self.angle_at(time).sin_cos();
        center + Vector3::new(self.radius * c, 0.0, self.radius * s)
    }

    /// Seconds per revolution; infinite when the speed is zero.
    pub fn period(&self) -> f32 {
        std::f32::consts::TAU / self.speed.abs()
    }
}
