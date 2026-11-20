//! Procedural height fields of the planets: one source for the relief
//! mesh, the shading normals, the collisions, the follow camera and the
//! effective radius. Heights are relative to the planet radius and read on
//! the unit direction from the center.

use crate::solar::planet::clover::CloverField;
use raylib::math::Vector3;

/// Angular step of the central differences of `HeightField::normal` for
/// shading: about 0.06 degrees, below the finest level's edge (~1 degree).
pub const NORMAL_EPS: f32 = 1e-3;

/// A synthetic relief for tests: `amplitude * sin(fx) sin(fy) sin(fz)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TestField {
    pub amplitude: f32,
    pub frequency: f32,
    /// Water level, relative to the radius, if any.
    pub sea_level: Option<f32>,
}

/// Height field of a planet's surface (Hearts adds its own in C54).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeightField {
    /// The ideal sphere.
    Flat,
    Test(TestField),
    /// Four clover mountains over a low seabed (`clover.rs`).
    Clover(CloverField),
}

/// A unit vector perpendicular to `n`, built from the axis along which `n`
/// is smallest so it never degenerates, poles included.
pub(crate) fn tangent(n: Vector3) -> Vector3 {
    let (ax, ay, az) = (n.x.abs(), n.y.abs(), n.z.abs());
    let aux = if ax <= ay && ax <= az {
        Vector3::new(1.0, 0.0, 0.0)
    } else if ay <= az {
        Vector3::new(0.0, 1.0, 0.0)
    } else {
        Vector3::new(0.0, 0.0, 1.0)
    };
    n.cross(aux).normalized()
}

impl HeightField {
    /// Height above the radius along the unit `n`, in radii; a value that
    /// is not finite counts as 0.
    pub fn height(&self, n: Vector3) -> f32 {
        let h = match *self {
            Self::Flat => 0.0,
            Self::Test(t) => {
                let f = t.frequency;
                t.amplitude * (f * n.x).sin() * (f * n.y).sin() * (f * n.z).sin()
            }
            Self::Clover(c) => c.height(n),
        };
        if h.is_finite() { h } else { 0.0 }
    }

    /// Water level in radii: the ground below it shows as water.
    pub fn sea_level(&self) -> Option<f32> {
        match *self {
            Self::Flat => None,
            Self::Test(t) => t.sea_level,
            Self::Clover(c) => Some(c.sea),
        }
    }

    /// Distance from the center to the surface along `n`, in radii: the
    /// ground, or the sea where the ground is below it.
    pub fn surface_radius(&self, n: Vector3) -> f32 {
        1.0 + self.height(n).max(self.sea_level().unwrap_or(f32::MIN))
    }

    /// Bound of `height` (and the sea), in radii: for culling, collisions
    /// and the effective radius.
    pub fn max_height(&self) -> f32 {
        let ground = match *self {
            Self::Flat => 0.0,
            Self::Test(t) if t.amplitude.is_finite() => t.amplitude.abs(),
            Self::Test(_) => 0.0,
            Self::Clover(c) => c.max_height(),
        };
        ground.max(self.sea_level().unwrap_or(0.0))
    }

    /// `normal`, refusing a step that is not a positive finite number (a
    /// zero step would difference a point with itself).
    pub fn try_normal(&self, n: Vector3, eps: f32) -> Result<Vector3, String> {
        if !(eps.is_finite() && eps > 0.0) {
            return Err(format!(
                "normal step must be positive and finite, got {eps}"
            ));
        }
        Ok(self.normal(n, eps))
    }

    /// Unit outward normal of the surface at `n`: central differences of
    /// the surface over a tangent basis `eps` radians wide.
    pub fn normal(&self, n: Vector3, eps: f32) -> Vector3 {
        let t1 = tangent(n);
        let t2 = n.cross(t1);
        let at = |d: Vector3| {
            let d = d.normalized();
            d * self.surface_radius(d)
        };
        let du = at(n + t1 * eps) - at(n - t1 * eps);
        let dv = at(n + t2 * eps) - at(n - t2 * eps);
        let normal = du.cross(dv);
        let len = normal.length();
        if !(len.is_finite() && len > 1e-12) {
            return n;
        }
        let normal = normal / len;
        if normal.dot(n) < 0.0 { -normal } else { normal }
    }
}
