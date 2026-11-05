//! Data handed to laboratory six fragment shaders.
//!
//! `ShaderUniforms` holds the values shared by every fragment of a frame and
//! `SurfaceSample` the attributes interpolated at one pixel center.

use raylib::math::Vector3;

/// Values shared by every fragment of one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaderUniforms {
    /// Simulated seconds (independent of the frame rate).
    pub time: f32,
    /// Unit vector towards the light, object space.
    pub light_direction: Vector3,
    /// Unit vector towards the eye (constant for the orthographic camera).
    pub eye_direction: Vector3,
}

/// Attributes of one covered pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSample {
    pub x: i32,
    pub y: i32,
    /// View depth (smaller is nearer).
    pub depth: f32,
    /// Object-space position interpolated across the triangle.
    pub object_position: Vector3,
    /// Unit shading normal (radial or flat, see `NormalSource`).
    pub normal: Vector3,
    /// Flat normal of the source triangle, object space.
    pub face_normal: Vector3,
}

/// Where `SurfaceSample::normal` comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NormalSource {
    /// `normalize(object_position)`: smooth normal of a sphere centered at
    /// the origin, such as the course sphere.
    #[default]
    Radial,
    /// The flat triangle normal (any mesh, e.g. the spacecraft).
    Face,
}

/// Unit direction from azimuth and elevation in radians, with the same
/// convention as `OrbitCamera::eye_direction`.
pub fn direction_from_angles(azimuth: f32, elevation: f32) -> Vector3 {
    let (sa, ca) = azimuth.sin_cos();
    let (se, ce) = elevation.sin_cos();
    Vector3::new(ce * sa, se, ce * ca)
}
