//! Project 3 inertial flight: the spacecraft starts at rest, thrust builds
//! a velocity up to the cruise (times the boost) and the ship coasts
//! without input; the pre-project direct flight is left as it was.

use raylib::math::Vector3;
use rust_software_renderer::solar::ShipState;
use rust_software_renderer::solar::flight::{
    FlightInput, FlightMode, THRUST_ACCEL, local_step, space_step, thrust_accel,
};
use rust_software_renderer::solar::input::SolarAction;

fn inertial() -> ShipState {
    let mut ship = ShipState::new(Vector3::new(0.0, 0.0, 50.0), 0.08);
    ship.move_speed = 3.0;
    ship.inertia = true;
    ship
}

fn input(actions: &[SolarAction]) -> FlightInput {
    FlightInput::from_actions(actions)
}

#[test]
fn starts_at_rest() {
    let mut ship = inertial();
    assert_eq!(ship.velocity, Vector3::zero());
    let p = ship.position;
    for _ in 0..30 {
        space_step(&mut ship, &input(&[]), 1.0 / 30.0);
    }
    assert_eq!(ship.position, p);
}

#[test]
fn accelerates_toward_max() {
    let mut ship = inertial();
    let forward = input(&[SolarAction::Forward]);
    space_step(&mut ship, &forward, 0.1);
    // One step of full thrust: a = 3 u/s^2 along the nose.
    assert!((ship.velocity.length() - THRUST_ACCEL * 0.1).abs() < 1e-5);
    assert!(ship.velocity.normalized().dot(ship.forward) > 0.9999);
    let mut previous = ship.velocity.length();
    for _ in 0..60 {
        space_step(&mut ship, &forward, 1.0 / 30.0);
        let speed = ship.velocity.length();
        assert!(speed >= previous - 1e-6 && speed <= ship.move_speed + 1e-4);
        previous = speed;
    }
    // After 2 s it cruises.
    assert!((previous - ship.move_speed).abs() < 1e-4);
}

#[test]
fn diagonal_not_faster() {
    let ship = inertial();
    let straight = thrust_accel(&input(&[SolarAction::Forward]), &ship);
    let diagonal = thrust_accel(
        &input(&[SolarAction::Forward, SolarAction::StrafeRight]),
        &ship,
    );
    assert!((straight.length() - diagonal.length()).abs() < 1e-5);
    let mut a = inertial();
    let mut b = inertial();
    for _ in 0..90 {
        space_step(&mut a, &input(&[SolarAction::Forward]), 1.0 / 30.0);
        space_step(
            &mut b,
            &input(&[SolarAction::Forward, SolarAction::StrafeLeft]),
            1.0 / 30.0,
        );
    }
    assert!(b.velocity.length() <= a.velocity.length() + 1e-5);
}

#[test]
fn zero_input_keeps_velocity() {
    let mut ship = inertial();
    for _ in 0..15 {
        space_step(&mut ship, &input(&[SolarAction::Forward]), 1.0 / 30.0);
    }
    let v = ship.velocity;
    let p = ship.position;
    space_step(&mut ship, &input(&[]), 0.1);
    assert_eq!(ship.velocity, v);
    assert!(((ship.position - p) - v * 0.1).length() < 1e-5);
}

#[test]
fn zero_or_nan_dt_leaves_the_ship() {
    let mut ship = inertial();
    space_step(&mut ship, &input(&[SolarAction::Forward]), 0.2);
    let before = ship;
    space_step(&mut ship, &input(&[SolarAction::Forward]), 0.0);
    assert_eq!(ship, before);
    space_step(&mut ship, &input(&[SolarAction::Forward]), f32::NAN);
    assert_eq!(ship, before);
}

#[test]
fn pre_project_flight_is_direct() {
    let mut ship = ShipState::new(Vector3::zero(), 0.08);
    assert!(!ship.inertia);
    space_step(&mut ship, &input(&[SolarAction::Forward]), 0.1);
    // 1.5 u/s straight away, no velocity kept.
    assert!((ship.position.length() - 0.15).abs() < 1e-5);
    assert_eq!(ship.velocity, Vector3::zero());
    let p = ship.position;
    space_step(&mut ship, &input(&[]), 0.1);
    assert_eq!(ship.position, p);
}

#[test]
fn local_flight_hands_over_its_world_velocity() {
    let mut ship = inertial();
    let planet = Vector3::new(0.0, 0.0, 47.0);
    ship.position = planet + Vector3::new(0.0, 2.0, 0.0);
    let mut mode = FlightMode::Local {
        planet: 1,
        previous_planet_position: planet,
    };
    // The planet moves 0.1 along +X in this 0.1 s step: 1 u/s.
    let moved = planet + Vector3::new(0.1, 0.0, 0.0);
    local_step(&mut ship, &mut mode, moved, 1.0, &input(&[]), 0.1);
    assert!((ship.velocity - Vector3::new(1.0, 0.0, 0.0)).length() < 1e-4);
}
