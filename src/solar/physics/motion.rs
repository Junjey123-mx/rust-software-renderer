//! Fixed-step integration: a frame's time is split into equal substeps no
//! longer than `PHYSICS_STEP`, so the motion does not depend on the frame
//! rate and a long frame cannot jump through a body.

use crate::solar::ShipState;
use crate::solar::clock::MAX_STEP;
use raylib::math::Vector3;

/// Longest physics substep, seconds.
pub const PHYSICS_STEP: f32 = 1.0 / 240.0;
/// Substeps of the longest frame the flight integrates (`MAX_STEP`).
pub const MAX_SUBSTEPS: u32 = 24;

fn finite(v: Vector3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

/// Runs `step(ship, h)` over `real_dt` (at most `MAX_STEP`; zero, negative
/// or non-finite does nothing) in equal substeps `h <= PHYSICS_STEP`, at
/// most `MAX_SUBSTEPS`. A substep that leaves the position or the velocity
/// non-finite is undone and the ship stopped. Returns the substeps run.
pub fn advance<F: FnMut(&mut ShipState, f32)>(
    ship: &mut ShipState,
    real_dt: f32,
    mut step: F,
) -> u32 {
    if !(real_dt.is_finite() && real_dt > 0.0) {
        return 0;
    }
    let dt = real_dt.min(MAX_STEP);
    let n = ((dt / PHYSICS_STEP).ceil() as u32).clamp(1, MAX_SUBSTEPS);
    let h = dt / n as f32;
    for _ in 0..n {
        let valid = *ship;
        step(ship, h);
        if !(finite(ship.position) && finite(ship.velocity)) {
            *ship = valid;
            ship.velocity = Vector3::zero();
        }
    }
    n
}
