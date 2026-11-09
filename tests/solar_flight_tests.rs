//! Spacecraft flight controllers and the solar input map.

use raylib::math::Vector3;
use rust_software_renderer::solar::ShipState;
use rust_software_renderer::solar::flight::{FlightInput, space_step};
use rust_software_renderer::solar::input::{HELD, PRESSED, SolarAction};

fn close(a: Vector3, b: Vector3, tol: f32) -> bool {
    (a - b).length() < tol
}

fn ship() -> ShipState {
    ShipState::new(Vector3::zero(), 0.1)
}

fn input(actions: &[SolarAction]) -> FlightInput {
    FlightInput::from_actions(actions)
}

#[test]
fn forward_motion_does_not_depend_on_the_frame_rate() {
    let fwd = input(&[SolarAction::Forward]);
    let (mut a, mut b) = (ship(), ship());
    for _ in 0..30 {
        space_step(&mut a, &fwd, 1.0 / 30.0);
    }
    for _ in 0..60 {
        space_step(&mut b, &fwd, 1.0 / 60.0);
    }
    assert!(close(a.position, b.position, 1e-4));
    // One second at move_speed 1.5 along -Z.
    assert!(
        close(a.position, Vector3::new(0.0, 0.0, -1.5), 1e-4),
        "{:?}",
        a.position
    );
}

#[test]
fn turning_does_not_depend_on_the_frame_rate() {
    // Rotations about one fixed axis compose exactly, so each axis alone is
    // frame-rate independent (mixing both is order-dependent by nature).
    for action in [SolarAction::YawLeft, SolarAction::PitchUp] {
        let turn = input(&[action]);
        let (mut a, mut b) = (ship(), ship());
        for _ in 0..30 {
            space_step(&mut a, &turn, 1.0 / 30.0);
        }
        for _ in 0..60 {
            space_step(&mut b, &turn, 1.0 / 60.0);
        }
        assert!(
            close(a.forward, b.forward, 1e-4),
            "{action:?}: {:?} vs {:?}",
            a.forward,
            b.forward
        );
        assert!(close(a.up, b.up, 1e-4));
        assert_eq!(a.position, Vector3::zero(), "turning in place");
        // One second at turn_speed 1 rad/s.
        let angle = a
            .forward
            .dot(Vector3::new(0.0, 0.0, -1.0))
            .clamp(-1.0, 1.0)
            .acos();
        assert!((angle - 1.0).abs() < 1e-3, "{action:?}: {angle}");
    }
}

#[test]
fn opposite_keys_cancel_and_zero_or_bad_deltas_do_nothing() {
    let both = input(&[SolarAction::Forward, SolarAction::Backward]);
    assert_eq!(both.throttle, 0.0);
    let mut s = ship();
    space_step(&mut s, &both, 0.05);
    assert_eq!(s, ship());
    let fwd = input(&[SolarAction::Forward]);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut s = ship();
        space_step(&mut s, &fwd, dt);
        assert_eq!(s.position, Vector3::zero(), "dt {dt}");
    }
    // A stalled frame moves at most one clamped step.
    let mut s = ship();
    space_step(&mut s, &fwd, 5.0);
    assert!((s.position.length() - 0.15).abs() < 1e-5);
}

#[test]
fn a_lifted_nose_travels_the_same_distance_in_its_own_direction() {
    let mut level = ship();
    let mut lifted = ship();
    lifted.pitch(0.6);
    let fwd = input(&[SolarAction::Forward]);
    for _ in 0..10 {
        space_step(&mut level, &fwd, 0.05);
        space_step(&mut lifted, &fwd, 0.05);
    }
    assert!((level.position.length() - lifted.position.length()).abs() < 1e-5);
    assert!(lifted.position.y > 0.4 && level.position.y.abs() < 1e-6);
    assert!(close(lifted.position.normalized(), lifted.forward, 1e-5));
}

#[test]
fn backward_retreats_along_the_nose_and_yaw_left_turns_towards_minus_x() {
    let mut s = ship();
    space_step(&mut s, &input(&[SolarAction::Backward]), 0.1);
    assert!(s.position.z > 0.0);
    let mut s = ship();
    space_step(&mut s, &input(&[SolarAction::YawLeft]), 0.1);
    assert!(s.forward.x < 0.0);
}

#[test]
fn the_input_map_uses_distinct_keys() {
    let mut keys: Vec<_> = HELD.iter().map(|(k, _)| *k as i32).collect();
    keys.extend(PRESSED.iter().map(|(k, _)| *k as i32));
    let n = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), n, "no key bound twice");
}
