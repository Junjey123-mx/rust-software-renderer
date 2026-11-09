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

impl FollowRig {
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
