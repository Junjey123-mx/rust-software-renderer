//! Deterministic routes over a planet for its vehicles: a pose is a pure
//! function of the time, so a fixed `t` gives the same frame, a paused
//! clock freezes them and no history is kept. The altitude counts from the
//! ground (`HeightField::surface_radius`), so a route rises and falls with
//! the terrain.

use crate::solar::planet::HeightField;
use crate::solar::planet::field::tangent;
use raylib::math::Vector3;
use std::f32::consts::TAU;

/// A closed route in the host's mesh space; angles in radians, speeds in
/// radians per second, altitudes in planet radii over the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Path {
    /// The great circle perpendicular to `axis`.
    GreatCircle {
        axis: Vector3,
        speed: f32,
        phase: f32,
        altitude: f32,
    },
    /// The parallel at latitude `lat` (about +Y).
    Latitude {
        lat: f32,
        speed: f32,
        phase: f32,
        altitude: f32,
    },
    /// The small circle `radius` radians around `axis`: a loop around a
    /// block of the city.
    Ring {
        axis: Vector3,
        radius: f32,
        speed: f32,
        phase: f32,
        altitude: f32,
    },
}

/// Where a vehicle is on its route: position (the host's mesh space),
/// heading along the route and "up" (the radial direction), both unit and
/// orthogonal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathPose {
    pub pos: Vector3,
    pub tangent: Vector3,
    pub up: Vector3,
}

impl Path {
    fn speed(&self) -> f32 {
        match *self {
            Self::GreatCircle { speed, .. }
            | Self::Latitude { speed, .. }
            | Self::Ring { speed, .. } => speed,
        }
    }

    /// Height over the ground, planet radii.
    pub fn altitude(&self) -> f32 {
        match *self {
            Self::GreatCircle { altitude, .. }
            | Self::Latitude { altitude, .. }
            | Self::Ring { altitude, .. } => altitude,
        }
    }

    /// Seconds of one lap (infinite while still).
    pub fn period(&self) -> f32 {
        TAU / self.speed().abs()
    }

    /// Unit direction from the host center at time `t`.
    pub fn direction(&self, t: f32) -> Vector3 {
        self.frame(t).0
    }

    /// Unit direction from the host center and unit heading at time `t`.
    fn frame(&self, t: f32) -> (Vector3, Vector3) {
        let (speed, phase) = match *self {
            Self::GreatCircle { speed, phase, .. }
            | Self::Latitude { speed, phase, .. }
            | Self::Ring { speed, phase, .. } => (speed, phase),
        };
        // Wrapped, so a long run keeps its precision; a still route keeps
        // its phase.
        let a = (speed * t + phase).rem_euclid(TAU);
        let a = if a.is_finite() { a } else { phase };
        let (s, c) = a.sin_cos();
        let sign = if speed < 0.0 { -1.0 } else { 1.0 };
        match *self {
            Self::GreatCircle { axis, .. } => {
                let axis = axis.normalized();
                let u = tangent(axis);
                let v = axis.cross(u);
                (u * c + v * s, (v * c - u * s) * sign)
            }
            Self::Latitude { lat, .. } => {
                let (sl, cl) = lat.sin_cos();
                (
                    Vector3::new(cl * c, sl, cl * s),
                    Vector3::new(-s, 0.0, c) * sign,
                )
            }
            Self::Ring { axis, radius, .. } => {
                let axis = axis.normalized();
                let u = tangent(axis);
                let v = axis.cross(u);
                let (sr, cr) = radius.sin_cos();
                (axis * cr + (u * c + v * s) * sr, (v * c - u * s) * sign)
            }
        }
    }

    /// The pose at time `t` on a host mesh of radius `radius` whose ground
    /// is `field`.
    pub fn pose(&self, t: f32, field: &HeightField, radius: f32) -> PathPose {
        let (dir, heading) = self.frame(t);
        PathPose {
            pos: dir * (radius * (field.surface_radius(dir) + self.altitude())),
            tangent: heading,
            up: dir,
        }
    }

    /// `n` poses evenly from `t0` to `t1` (both included; one pose at `t0`
    /// when `n` is 1) into `out`, which is cleared first and keeps its
    /// capacity.
    pub fn sample(
        &self,
        t0: f32,
        t1: f32,
        n: usize,
        field: &HeightField,
        radius: f32,
        out: &mut Vec<PathPose>,
    ) {
        out.clear();
        let step = if n > 1 {
            (t1 - t0) / (n - 1) as f32
        } else {
            0.0
        };
        out.extend((0..n).map(|i| self.pose(t0 + step * i as f32, field, radius)));
    }
}
