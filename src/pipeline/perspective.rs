//! Perspective camera and projection, independent of the orbit projection.
//!
//! View convention: `(x right, y up, z forward)` with `z > 0` in front of the
//! eye; the view depth written for the depth buffer is that `z`, so smaller
//! depth is nearer, as in the orbit path. Screen mapping divides by `z`, so
//! callers clip against the near plane first (`clip_near`, added later).

use crate::geometry::Vertex;
use crate::pipeline::CameraBasis;
use raylib::math::Vector3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerspectiveCamera {
    pub eye: Vector3,
    pub target: Vector3,
    pub up: Vector3,
    /// Vertical field of view, radians, in `(0, PI)`.
    pub fov_y: f32,
    /// Near plane distance, positive.
    pub near: f32,
}

fn finite(v: Vector3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

impl PerspectiveCamera {
    pub fn new(
        eye: Vector3,
        target: Vector3,
        up: Vector3,
        fov_y: f32,
        near: f32,
    ) -> Result<Self, String> {
        if !(finite(eye) && finite(target) && finite(up)) {
            return Err("camera vectors must be finite".into());
        }
        if (target - eye).length() <= 1e-6 {
            return Err("camera eye and target must differ".into());
        }
        if up.length() <= 1e-6 {
            return Err("camera up must not be zero".into());
        }
        if !(fov_y.is_finite() && fov_y > 0.0 && fov_y < std::f32::consts::PI) {
            return Err(format!("field of view {fov_y} must be in (0, PI)"));
        }
        if !(near.is_finite() && near > 0.0) {
            return Err(format!("near plane {near} must be positive"));
        }
        Ok(Self {
            eye,
            target,
            up,
            fov_y,
            near,
        })
    }

    /// Orthonormal `right`, `up`, `forward`. When `up` is parallel to the
    /// view direction a world axis not parallel to it is used instead.
    pub fn basis(&self) -> CameraBasis {
        let forward = (self.target - self.eye).normalized();
        let mut right = forward.cross(self.up);
        if right.length() <= 1e-5 {
            let alt = if forward.y.abs() < 0.9 {
                Vector3::new(0.0, 1.0, 0.0)
            } else {
                Vector3::new(1.0, 0.0, 0.0)
            };
            right = forward.cross(alt);
        }
        let right = right.normalized();
        let up = right.cross(forward).normalized();
        CameraBasis { right, up, forward }
    }

    /// `(x_right, y_up, z_forward)` of a world point relative to the eye.
    pub fn world_to_view(&self, p: Vector3) -> Vector3 {
        let b = self.basis();
        let d = p - self.eye;
        Vector3::new(d.dot(b.right), d.dot(b.up), d.dot(b.forward))
    }

    /// Pixels per unit of `x/z` or `y/z` for a framebuffer `height` pixels
    /// tall (square pixels, so the horizontal factor is the same).
    pub fn focal_px(&self, height: u32) -> f32 {
        (height as f32 / 2.0) / (self.fov_y / 2.0).tan()
    }

    /// Screen position of a view-space point: `(x_px, y_px, z_view)`, `y`
    /// growing downwards. `None` when the point is in front of the near
    /// plane or non-finite.
    pub fn project_view(&self, v: Vector3, width: u32, height: u32) -> Option<Vector3> {
        if !finite(v) || v.z < self.near {
            return None;
        }
        let f = self.focal_px(height);
        let x = width as f32 / 2.0 + f * v.x / v.z;
        let y = height as f32 / 2.0 - f * v.y / v.z;
        (x.is_finite() && y.is_finite()).then_some(Vector3::new(x, y, v.z))
    }

    /// Unit direction from a world point towards the eye.
    pub fn view_direction(&self, world: Vector3) -> Vector3 {
        let d = self.eye - world;
        let len = d.length();
        if len.is_finite() && len > 1e-6 {
            d / len
        } else {
            -self.basis().forward
        }
    }
}

/// A triangle corner carrying every attribute the shaded stage needs after
/// clipping: nothing is looked up by index in the original mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipVertex {
    pub object_position: Vector3,
    pub world_position: Vector3,
    pub view_position: Vector3,
    pub color: Vector3,
}

impl ClipVertex {
    /// From a vertex whose `transformed_position` is already in world space
    /// (after the vertex shader).
    pub fn from_world(v: &Vertex, camera: &PerspectiveCamera) -> Self {
        Self {
            object_position: v.position,
            world_position: v.transformed_position,
            view_position: camera.world_to_view(v.transformed_position),
            color: v.color,
        }
    }

    /// Linear blend of every attribute, `t` in `[0, 1]` from `self` to `other`.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let mix = |a: Vector3, b: Vector3| a + (b - a) * t;
        Self {
            object_position: mix(self.object_position, other.object_position),
            world_position: mix(self.world_position, other.world_position),
            view_position: mix(self.view_position, other.view_position),
            color: mix(self.color, other.color),
        }
    }

    /// Screen vertex for the rasterizers: `position` keeps the object
    /// position and `transformed_position` is `(x_px, y_px, z_view)`.
    pub fn to_screen(&self, camera: &PerspectiveCamera, width: u32, height: u32) -> Option<Vertex> {
        let screen = camera.project_view(self.view_position, width, height)?;
        Some(Vertex {
            position: self.object_position,
            transformed_position: screen,
            color: self.color,
        })
    }
}
