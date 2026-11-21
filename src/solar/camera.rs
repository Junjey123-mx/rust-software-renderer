//! Camera rigs of the solar scene: the orbit camera of the earlier scenes or
//! a perspective camera following the spacecraft. Both produce the same
//! `PerspectiveCamera`, so the view and projection math is not duplicated;
//! local flight reuses the follow rig (the controller changes, not the
//! camera).

use crate::pipeline::{OrbitCamera, PerspectiveCamera};
use crate::solar::ShipState;
use crate::solar::scene::{FOV_Y, NEAR};
use raylib::math::Vector3;

/// Follow distances, world units; they only frame the ship and never change
/// its trajectory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FollowRig {
    /// Behind the ship along `-forward`.
    pub distance: f32,
    /// Above the ship along `up`.
    pub height: f32,
    /// Look-at point ahead of the ship along `forward`.
    pub look_ahead: f32,
}

impl Default for FollowRig {
    fn default() -> Self {
        Self {
            distance: 0.9,
            height: 0.22,
            look_ahead: 0.3,
        }
    }
}

/// Follow framing of atmospheric flight: as high above the ship as behind
/// it, so the ship is seen against the ground it skims; `tilt_to_horizon`
/// then aims the view down to the planet's horizon.
pub const ATMOSPHERE_RIG: FollowRig = FollowRig {
    distance: 0.9,
    height: 0.9,
    look_ahead: 1.5,
};

/// Where `tilt_to_horizon` puts the horizon: this many radians above the
/// center of the view, so the ground fills a little more than half of it.
pub const HORIZON_ABOVE: f32 = 0.12;
/// The steepest look down of `tilt_to_horizon`, radians below the local
/// horizontal.
const MAX_LOOK_DOWN: f32 = 1.2;

/// `camera` turned about its right axis by `share` (0 to 1) of the way to
/// a view whose center is `HORIZON_ABOVE` below the horizon of the sphere
/// at `center` with `radius`: the curvature of a planet stays in the view
/// at any height. Only ever tilts down; unchanged with `share` 0 (or not a
/// number) or the eye inside the sphere.
pub fn tilt_to_horizon(
    camera: PerspectiveCamera,
    center: Vector3,
    radius: f32,
    share: f32,
) -> PerspectiveCamera {
    let out = camera.eye - center;
    let height = out.length();
    if share.is_nan() || share <= 0.0 || !(height > radius && radius > 0.0) {
        return camera;
    }
    let normal = out / height;
    let b = camera.basis();
    let dip = (radius / height).acos();
    let elevation = b.forward.dot(normal).clamp(-1.0, 1.0).asin();
    let want = -(dip + HORIZON_ABOVE).min(MAX_LOOK_DOWN);
    let tilt = (want - elevation).min(0.0) * share.min(1.0);
    let forward = b.forward * tilt.cos() + b.up * tilt.sin();
    PerspectiveCamera {
        target: camera.eye + forward * (camera.target - camera.eye).length(),
        ..camera
    }
}

impl FollowRig {
    /// This rig moved `t` (0 to 1) of the way to `other`; `t` 0 (or not a
    /// number) returns it unchanged.
    pub fn blended(&self, other: &FollowRig, t: f32) -> FollowRig {
        if t.is_nan() || t <= 0.0 {
            return *self;
        }
        let t = t.min(1.0);
        let mix = |a: f32, b: f32| a + (b - a) * t;
        FollowRig {
            distance: mix(self.distance, other.distance),
            height: mix(self.height, other.height),
            look_ahead: mix(self.look_ahead, other.look_ahead),
        }
    }

    /// Perspective camera behind and above the ship, with the ship's `up`.
    pub fn camera(&self, ship: &ShipState) -> PerspectiveCamera {
        let eye = ship.position - ship.forward * self.distance + ship.up * self.height;
        let target = ship.position + ship.forward * self.look_ahead;
        PerspectiveCamera::new(eye, target, ship.up, FOV_Y, NEAR)
            .expect("follow distances give distinct eye and target")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CameraRig {
    /// The orbit camera rig of the scene (target, azimuth, elevation, zoom).
    Orbit,
    /// Behind the spacecraft.
    #[default]
    Follow,
}

impl CameraRig {
    pub fn toggled(self) -> Self {
        match self {
            Self::Orbit => Self::Follow,
            Self::Follow => Self::Orbit,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Orbit => "orbit",
            Self::Follow => "follow",
        }
    }
}

/// Perspective camera of an orbit rig: the eye sits along the orbit eye
/// direction at `framing / zoom` from the target.
pub fn orbit_perspective(camera: &OrbitCamera, framing: f32) -> PerspectiveCamera {
    let distance = framing.max(1e-3) / camera.zoom;
    PerspectiveCamera::new(
        camera.target + camera.eye_direction() * distance,
        camera.target,
        Vector3::new(0.0, 1.0, 0.0),
        FOV_Y,
        NEAR,
    )
    .expect("orbit rig gives a valid perspective camera")
}
