//! Quick travel: warp next to a body at its current position. The clock and
//! the orbits are untouched; only the spacecraft moves.

use crate::solar::SolarScene;
use crate::solar::flight::FlightMode;
use raylib::math::Vector3;

/// Height of the arrival point above the destination's ideal surface:
/// outside the local-flight clearance and within its entry range, so `L`
/// works right after arriving.
pub const WARP_MARGIN: f32 = 1.2;

fn unit(v: Vector3) -> Option<Vector3> {
    let len = v.length();
    (len.is_finite() && len > 1e-5).then(|| v / len)
}

/// Arrival point next to body `index`: on the side the ship comes from when
/// that point is clear of every other body (and their clearance), otherwise
/// the first clear one of a few fixed directions.
pub fn arrival_point(scene: &SolarScene, index: usize) -> Option<Vector3> {
    let target = scene.bodies.get(index)?.transform.position;
    let distance = scene.radius_of(index) + scene.flight_scale(index).warp_margin;
    let candidates = [
        unit(scene.ship.position - target),
        Some(Vector3::new(0.0, 1.0, 0.0)),
        unit(target - scene.center),
        Some(Vector3::new(0.0, -1.0, 0.0)),
        Some(Vector3::new(1.0, 0.0, 0.0)),
        Some(Vector3::new(0.0, 0.0, 1.0)),
    ];
    let clear = |p: Vector3| {
        scene.bodies.iter().enumerate().all(|(i, b)| {
            i == index
                || (p - b.transform.position).length()
                    > scene.radius_of(i) + scene.flight_scale(i).clearance + scene.ship.radius()
        })
    };
    candidates
        .into_iter()
        .flatten()
        .map(|dir| target + dir * distance)
        .find(|&p| clear(p))
}

/// Warps the spacecraft next to body `index`, nose towards it, leaving
/// local flight. Returns `false` (nothing changes) when the index does not
/// exist or no clear arrival point was found.
pub fn warp_to(scene: &mut SolarScene, index: usize) -> bool {
    let Some(arrival) = arrival_point(scene, index) else {
        return false;
    };
    let target = scene.bodies[index].transform.position;
    let Some(forward) = unit(target - arrival) else {
        return false;
    };
    scene.ship.position = arrival;
    scene.ship.forward = forward;
    scene.ship.up = Vector3::new(0.0, 1.0, 0.0);
    scene.ship.renormalize();
    scene.ship.velocity = Vector3::zero();
    scene.flight = FlightMode::Space;
    true
}
