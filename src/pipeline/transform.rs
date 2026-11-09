//! Placement of one instance in the world: translation, Euler rotation and
//! uniform scale, composed as `T * R * S` into the column-major model matrix
//! that `vertex_shader` multiplies.
//!
//! Rotation order (applied to an object-space vector): roll around `Z`, then
//! pitch around `X`, then yaw around `Y`, matching the course convention where
//! positive yaw turns the `-Z` nose towards `-X` and positive pitch lifts it.

use raylib::math::{Matrix, Vector3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vector3,
    /// Radians around world `Y`.
    pub yaw: f32,
    /// Radians around the object `X` axis.
    pub pitch: f32,
    /// Radians around the object `Z` axis.
    pub roll: f32,
    /// Uniform, positive and finite.
    pub scale: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vector3::zero(),
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            scale: 1.0,
        }
    }
}

impl Transform {
    /// Translation and uniform scale only.
    pub fn new(position: Vector3, scale: f32) -> Self {
        Self {
            position,
            scale,
            ..Self::default()
        }
    }

    /// `Err` when the scale is not positive and finite or any angle or
    /// coordinate is not finite.
    pub fn validate(&self) -> Result<(), String> {
        let p = self.position;
        let finite = [p.x, p.y, p.z, self.yaw, self.pitch, self.roll]
            .iter()
            .all(|v| v.is_finite());
        if !finite {
            return Err("transform has a non-finite component".into());
        }
        if !(self.scale.is_finite() && self.scale > 0.0) {
            return Err(format!("transform scale {} must be positive", self.scale));
        }
        Ok(())
    }

    /// Rotation basis `(right, up, back)` = the columns of `R` (unit vectors).
    fn rotation_columns(&self) -> [Vector3; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let (sr, cr) = self.roll.sin_cos();
        // R = Ry(yaw) * Rx(pitch) * Rz(roll)
        let c0 = Vector3::new(cy * cr + sy * sp * sr, cp * sr, -sy * cr + cy * sp * sr);
        let c1 = Vector3::new(-cy * sr + sy * sp * cr, cp * cr, sy * sr + cy * sp * cr);
        let c2 = Vector3::new(sy * cp, -sp, cy * cp);
        [c0, c1, c2]
    }

    /// Column-major `T * R * S`, the layout `vertex_shader` multiplies.
    pub fn matrix(&self) -> Matrix {
        let [c0, c1, c2] = self.rotation_columns();
        let s = self.scale;
        let mut m = Matrix::identity();
        m.m0 = c0.x * s;
        m.m1 = c0.y * s;
        m.m2 = c0.z * s;
        m.m4 = c1.x * s;
        m.m5 = c1.y * s;
        m.m6 = c1.z * s;
        m.m8 = c2.x * s;
        m.m9 = c2.y * s;
        m.m10 = c2.z * s;
        m.m12 = self.position.x;
        m.m13 = self.position.y;
        m.m14 = self.position.z;
        m
    }

    /// Rotates an object-space direction into the world (no translation, no
    /// scale).
    pub fn to_world_dir(&self, d: Vector3) -> Vector3 {
        let [c0, c1, c2] = self.rotation_columns();
        c0 * d.x + c1 * d.y + c2 * d.z
    }

    /// Rotates a world direction into object space with the transposed
    /// rotation (no translation, no scale). Unit input stays unit.
    pub fn to_object_dir(&self, d: Vector3) -> Vector3 {
        let [c0, c1, c2] = self.rotation_columns();
        Vector3::new(d.dot(c0), d.dot(c1), d.dot(c2))
    }

    /// Full inverse for a world point: subtract the translation, undo the
    /// rotation and divide by the scale.
    pub fn to_object_point(&self, p: Vector3) -> Vector3 {
        self.to_object_dir(p - self.position) / self.scale
    }
}
