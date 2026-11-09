//! Spacecraft flight controllers: pure functions of the ship state, the
//! input of one frame and the real frame delta (flight keeps working while
//! the simulation is paused).

use crate::solar::ShipState;
use crate::solar::clock::MAX_STEP;
use crate::solar::input::SolarAction;

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
