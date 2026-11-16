//! External plasma loops of The King's Crown in the solar scene: magnetic
//! arcs that leave the star at one foot point, rise outside its silhouette
//! and fall back into it at another. The laboratory six shader
//! (`celestial::crown`) paints its loops inside the disk; these are
//! geometry, described here in the star's object space so they can be
//! built once and drawn with the star's own transform.

use crate::pipeline::direction_from_angles;
use crate::shading::sphere::east_north;
use raylib::math::Vector3;
use std::f32::consts::PI;

/// Quads along each arc and around its tube: few enough that the crown's
/// arcs together cost about as much to transform as one small body.
pub const SEGMENTS: usize = 16;
pub const SIDES: usize = 6;

/// One magnetic arc. Directions are on the unit sphere of the star's object
/// space; heights and widths are in body radii.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlasmaArc {
    /// Midpoint between the feet on the surface, radians, with the
    /// convention of `direction_from_angles` (longitude 0 faces +Z).
    pub longitude: f32,
    pub latitude: f32,
    /// Direction of the line between the feet, radians from east to north.
    pub tilt: f32,
    /// Angle between the feet seen from the center, radians.
    pub span: f32,
    /// Apex height above the surface.
    pub height: f32,
    /// Tube radius at the apex.
    pub width: f32,
    /// Shift along the plasma ramp: below zero redder, above whiter.
    pub heat: f32,
    /// Emission scale.
    pub intensity: f32,
    /// Phase of the pulse and the flow, radians.
    pub phase: f32,
}

impl PlasmaArc {
    /// Unit direction of the midpoint between the feet.
    pub fn center(&self) -> Vector3 {
        direction_from_angles(self.longitude, self.latitude)
    }

    /// Unit directions of the two feet, half the span on each side of the
    /// center along the tilt.
    pub fn feet(&self) -> [Vector3; 2] {
        let c = self.center();
        let (east, north) = east_north(c);
        let chord = east * self.tilt.cos() + north * self.tilt.sin();
        let (s, k) = (self.span / 2.0).sin_cos();
        [c * k - chord * s, c * k + chord * s]
    }
}

const fn deg(d: f32) -> f32 {
    d * PI / 180.0
}

/// The crown's arcs: six around the star, one every 60 degrees of longitude
/// on alternating active belts, so whatever side faces the camera about two
/// of them stand near the limb and show their whole arch against the sky.
pub const CROWN_ARCS: [PlasmaArc; 6] = [
    PlasmaArc {
        longitude: deg(20.0),
        latitude: deg(22.0),
        tilt: deg(10.0),
        span: deg(34.0),
        height: 0.42,
        width: 0.065,
        heat: 0.10,
        intensity: 1.15,
        phase: 0.0,
    },
    PlasmaArc {
        longitude: deg(80.0),
        latitude: deg(-28.0),
        tilt: deg(70.0),
        span: deg(28.0),
        height: 0.32,
        width: 0.055,
        heat: -0.05,
        intensity: 1.0,
        phase: 1.7,
    },
    PlasmaArc {
        longitude: deg(145.0),
        latitude: deg(12.0),
        tilt: deg(-35.0),
        span: deg(40.0),
        height: 0.55,
        width: 0.075,
        heat: 0.05,
        intensity: 1.1,
        phase: 3.1,
    },
    PlasmaArc {
        longitude: deg(200.0),
        latitude: deg(34.0),
        tilt: deg(55.0),
        span: deg(24.0),
        height: 0.28,
        width: 0.05,
        heat: -0.10,
        intensity: 0.95,
        phase: 4.4,
    },
    PlasmaArc {
        longitude: deg(260.0),
        latitude: deg(-14.0),
        tilt: deg(-10.0),
        span: deg(36.0),
        height: 0.48,
        width: 0.065,
        heat: 0.15,
        intensity: 1.2,
        phase: 5.6,
    },
    PlasmaArc {
        longitude: deg(320.0),
        latitude: deg(-36.0),
        tilt: deg(30.0),
        span: deg(30.0),
        height: 0.36,
        width: 0.06,
        heat: 0.0,
        intensity: 1.05,
        phase: 2.4,
    },
];
