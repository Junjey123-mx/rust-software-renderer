//! Spacecraft flight controllers: pure functions of the ship state, the
//! input of one frame and the real frame delta (flight keeps working while
//! the simulation is paused).

use crate::solar::ShipState;
use crate::solar::clock::MAX_STEP;
use crate::solar::input::SolarAction;
use raylib::math::Vector3;

/// Signed axes of one frame, each in `[-1, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FlightInput {
    /// `W` = +1, `S` = -1.
    pub throttle: f32,
    /// Left arrow = +1 (nose towards the ship's left), right arrow = -1.
    pub yaw: f32,
    /// Up arrow = +1 (lift the nose), down arrow = -1.
    pub pitch: f32,
}

impl FlightInput {
    /// Opposite keys cancel out.
    pub fn from_actions(actions: &[SolarAction]) -> Self {
        let axis = |pos: SolarAction, neg: SolarAction| {
            actions.contains(&pos) as i32 as f32 - actions.contains(&neg) as i32 as f32
        };
        Self {
            throttle: axis(SolarAction::Forward, SolarAction::Backward),
            yaw: axis(SolarAction::YawLeft, SolarAction::YawRight),
            pitch: axis(SolarAction::PitchUp, SolarAction::PitchDown),
        }
    }
}

fn clamp_dt(dt: f32) -> f32 {
    if dt.is_finite() {
        dt.clamp(0.0, MAX_STEP)
    } else {
        0.0
    }
}

/// Space flight: turn around the ship's own axes, then move along the nose
/// by `throttle * move_speed * dt`. No inertia, no collisions.
pub fn space_step(ship: &mut ShipState, input: &FlightInput, dt: f32) {
    let dt = clamp_dt(dt);
    if input.yaw != 0.0 {
        ship.yaw(input.yaw * ship.turn_speed * dt);
    }
    if input.pitch != 0.0 {
        ship.pitch(input.pitch * ship.turn_speed * dt);
    }
    let step = input.throttle * ship.move_speed * dt;
    ship.position += ship.forward * step;
}

/// Radial frame of the spacecraft around a planet treated as an ideal
/// sphere of `radius` (no relief, no gravity).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalFrame {
    /// Unit direction from the planet center to the ship: the local up.
    pub normal: Vector3,
    /// Distance from the planet center to the ship center.
    pub distance: f32,
    /// `distance - radius`: height of the ship center above the surface.
    pub altitude: f32,
    /// Unit direction of travel on the tangent plane: the nose projected
    /// onto it, or the ship's up when the nose points radially.
    pub tangent: Vector3,
}

/// Tolerance below which a vector is treated as zero.
pub const EPSILON: f32 = 1e-5;

fn unit_or_none(v: Vector3) -> Option<Vector3> {
    let len = v.length();
    (len.is_finite() && len > EPSILON).then(|| v / len)
}

/// Local frame of `ship` around a planet centered at `planet` with
/// `radius`. `None` when the ship sits (almost) at the center or any value
/// is non-finite.
pub fn local_frame(ship: &ShipState, planet: Vector3, radius: f32) -> Option<LocalFrame> {
    let offset = ship.position - planet;
    let distance = offset.length();
    let normal = unit_or_none(offset)?;
    if !radius.is_finite() {
        return None;
    }
    let project = |v: Vector3| v - normal * v.dot(normal);
    let tangent = unit_or_none(project(ship.forward))
        .or_else(|| unit_or_none(project(ship.up)))
        .or_else(|| unit_or_none(project(ship.right())))?;
    Some(LocalFrame {
        normal,
        distance,
        altitude: distance - radius,
        tangent,
    })
}
