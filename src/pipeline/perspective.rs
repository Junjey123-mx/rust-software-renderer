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

/// Result of clipping one triangle: up to four corners in order. Empty when
/// the triangle lies entirely in front of the near plane or is degenerate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipPolygon {
    corners: [ClipVertex; 4],
    len: usize,
}

impl ClipPolygon {
    fn empty() -> Self {
        let blank = ClipVertex {
            object_position: Vector3::zero(),
            world_position: Vector3::zero(),
            view_position: Vector3::zero(),
            color: Vector3::zero(),
        };
        Self {
            corners: [blank; 4],
            len: 0,
        }
    }

    fn push(&mut self, v: ClipVertex) {
        debug_assert!(
            self.len < 4,
            "a triangle clipped by one plane has at most 4 corners"
        );
        self.corners[self.len] = v;
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len < 3
    }

    pub fn corners(&self) -> &[ClipVertex] {
        &self.corners[..self.len]
    }

    /// Fan triangulation: zero, one or two triangles.
    pub fn triangles(&self) -> impl Iterator<Item = [ClipVertex; 3]> + '_ {
        (1..self.len.saturating_sub(1))
            .map(move |i| [self.corners[0], self.corners[i], self.corners[i + 1]])
    }

    /// Perimeter edges in order, without the fan diagonal.
    pub fn edges(&self) -> impl Iterator<Item = (ClipVertex, ClipVertex)> + '_ {
        (0..self.len).map(move |i| (self.corners[i], self.corners[(i + 1) % self.len]))
    }
}

/// Sutherland-Hodgman clipping of a triangle against the plane
/// `view.z >= near`, before any perspective division. Every attribute of a
/// new corner is interpolated at `t = (near - zA) / (zB - zA)`. Non-finite
/// corners empty the result.
pub fn clip_near(tri: &[ClipVertex; 3], near: f32) -> ClipPolygon {
    let mut out = ClipPolygon::empty();
    if !near.is_finite() || tri.iter().any(|v| !finite(v.view_position)) {
        return out;
    }
    let inside = |v: &ClipVertex| v.view_position.z >= near;
    for i in 0..3 {
        let a = &tri[i];
        let b = &tri[(i + 1) % 3];
        let (ina, inb) = (inside(a), inside(b));
        if ina {
            out.push(*a);
        }
        if ina != inb {
            let t = (near - a.view_position.z) / (b.view_position.z - a.view_position.z);
            let mut cut = a.lerp(b, t.clamp(0.0, 1.0));
            cut.view_position.z = near;
            out.push(cut);
        }
    }
    if out.len < 3 {
        out.len = 0;
    }
    out
}
