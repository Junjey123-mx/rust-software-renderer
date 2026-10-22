//! Orthographic orbit camera (CPU only, no raylib camera or matrices).
//!
//! World convention: `Y` up, the ship is wide along `X` and long along `Z`.
//! The eye sits on a sphere around `target`, parametrised by `azimuth`
//! (rotation around `Y`) and `elevation` (angle above the XZ plane). The view
//! basis is `right`, `up`, `forward` (forward points from the eye to the
//! target). View depth of a point is its coordinate along `forward`:
//! **smaller depth is nearer** to the eye.

use raylib::math::Vector3;

pub const MIN_ELEVATION: f32 = -89.0_f32.to_radians();
pub const MAX_ELEVATION: f32 = 89.0_f32.to_radians();
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 8.0;

/// Named starting orientations (reproducible screenshots).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewPreset {
    /// Above and to the front-left: shows wings, fuselage and nose at once.
    #[default]
    Oblique,
    /// Straight down, nose towards the bottom of the screen.
    Top,
    /// From the nose (negative Z side), level with the ship.
    Front,
    /// From the right wing tip (positive X side), level with the ship.
    Side,
}

impl ViewPreset {
    /// `(azimuth, elevation)` in radians.
    pub fn angles(self) -> (f32, f32) {
        match self {
            Self::Oblique => (-2.45, 0.55),
            Self::Top => (0.0, MAX_ELEVATION),
            Self::Front => (std::f32::consts::PI, 0.0),
            Self::Side => (std::f32::consts::FRAC_PI_2, 0.0),
        }
    }
}

impl std::str::FromStr for ViewPreset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "oblique" => Ok(Self::Oblique),
            "top" => Ok(Self::Top),
            "front" => Ok(Self::Front),
            "side" => Ok(Self::Side),
            other => Err(format!(
                "unknown view '{other}' (expected oblique, top, front or side)"
            )),
        }
    }
}

/// Orthonormal view basis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraBasis {
    pub right: Vector3,
    pub up: Vector3,
    pub forward: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    pub target: Vector3,
    pub azimuth: f32,
    pub elevation: f32,
    pub zoom: f32,
    preset: ViewPreset,
}

impl OrbitCamera {
    pub fn new(target: Vector3, preset: ViewPreset) -> Self {
        let (azimuth, elevation) = preset.angles();
        Self {
            target,
            azimuth,
            elevation,
            zoom: 1.0,
            preset,
        }
    }

    /// Restores the construction preset and zoom.
    pub fn reset(&mut self) {
        *self = Self::new(self.target, self.preset);
    }

    pub fn set_preset(&mut self, preset: ViewPreset) {
        let (azimuth, elevation) = preset.angles();
        self.azimuth = azimuth;
        self.elevation = elevation;
    }

    pub fn preset(&self) -> ViewPreset {
        self.preset
    }

    /// Rotates around the target; the angle wraps into `(-pi, pi]`.
    pub fn orbit(&mut self, delta: f32) {
        let a = self.azimuth + delta;
        self.azimuth = a.rem_euclid(std::f32::consts::TAU);
        if self.azimuth > std::f32::consts::PI {
            self.azimuth -= std::f32::consts::TAU;
        }
    }

    pub fn tilt(&mut self, delta: f32) {
        self.elevation = (self.elevation + delta).clamp(MIN_ELEVATION, MAX_ELEVATION);
    }

    /// Multiplies the zoom, clamped to `[MIN_ZOOM, MAX_ZOOM]`.
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Unit vector from the target towards the eye.
    pub fn eye_direction(&self) -> Vector3 {
        let (sa, ca) = self.azimuth.sin_cos();
        let (se, ce) = self.elevation.sin_cos();
        Vector3::new(ce * sa, se, ce * ca)
    }

    pub fn basis(&self) -> CameraBasis {
        let forward = -self.eye_direction();
        // Elevation is clamped below 90 degrees, so forward is never parallel
        // to the world up vector and the cross product is well defined.
        let right = forward.cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
        let up = right.cross(forward).normalized();
        CameraBasis { right, up, forward }
    }

    /// `(x_right, y_up, depth)` of a world point relative to the target.
    pub fn world_to_view(&self, p: Vector3) -> Vector3 {
        let b = self.basis();
        let d = p - self.target;
        Vector3::new(d.dot(b.right), d.dot(b.up), d.dot(b.forward))
    }
}
