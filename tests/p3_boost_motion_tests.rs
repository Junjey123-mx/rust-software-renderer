//! Project 3 boost and braking in inertial flight: the boost ramp scales
//! both the thrust and the speed cap, releasing it slows the ship
//! progressively, and the dampers brake without thrust unless turned off.

use raylib::consts::KeyboardKey;
use raylib::math::Vector3;
use rust_software_renderer::solar::ShipState;
use rust_software_renderer::solar::flight::{BOOST, BRAKE_RATE, FlightInput, space_step};
use rust_software_renderer::solar::input::{PRESSED, SolarAction};
use rust_software_renderer::solar::viewer::apply_toggles;

const DT: f32 = 1.0 / 60.0;

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
fn boost_never_exceeds_max() {
    let mut ship = inertial();
    let boosted = input(&[SolarAction::Forward, SolarAction::Boost]);
    let mut top: f32 = 0.0;
    for _ in 0..600 {
        space_step(&mut ship, &boosted, DT);
        let speed = ship.velocity.length();
        let cap = ship.move_speed * (1.0 + (BOOST - 1.0) * ship.boost);
        assert!(speed <= cap + 1e-3, "{speed} > {cap}");
        top = top.max(speed);
    }
    assert!((top - ship.move_speed * BOOST).abs() < 1e-2, "{top}");
    // Pressing and releasing the boost for 1000 frames: never above
    // cruise x BOOST, never NaN.
    let mut ship = inertial();
    for f in 0..1000 {
        let held = (f / 7) % 2 == 0;
        let actions: &[SolarAction] = if held {
            &[SolarAction::Forward, SolarAction::Boost]
        } else if f % 3 == 0 {
            &[]
        } else {
            &[SolarAction::Forward]
        };
        space_step(&mut ship, &input(actions), DT);
        let speed = ship.velocity.length();
        assert!(speed.is_finite() && ship.position.x.is_finite());
        assert!(speed <= ship.move_speed * BOOST + 1e-3);
    }
}

#[test]
fn release_decelerates_smoothly() {
    let mut ship = inertial();
    let boosted = input(&[SolarAction::Forward, SolarAction::Boost]);
    for _ in 0..300 {
        space_step(&mut ship, &boosted, DT);
    }
    let forward = input(&[SolarAction::Forward]);
    let mut previous = ship.velocity.length();
    let mut largest_drop: f32 = 0.0;
    for _ in 0..120 {
        space_step(&mut ship, &forward, DT);
        let speed = ship.velocity.length();
        assert!(
            speed <= previous + 1e-4,
            "never speeds up after the release"
        );
        largest_drop = largest_drop.max(previous - speed);
        previous = speed;
    }
    // Down to the cruise over the boost fall, never in one frame.
    assert!((previous - ship.move_speed).abs() < 1e-3);
    assert!(
        largest_drop < 0.1 * ship.move_speed * BOOST,
        "{largest_drop}"
    );
}

#[test]
fn no_teleport_on_boost() {
    let mut ship = inertial();
    let boosted = input(&[SolarAction::Forward, SolarAction::Boost]);
    let mut last = ship.position;
    for _ in 0..240 {
        space_step(&mut ship, &boosted, DT);
        let step = (ship.position - last).length();
        assert!(step <= ship.move_speed * BOOST * DT + 1e-4, "{step}");
        last = ship.position;
    }
}

#[test]
fn dampers_brake_and_off_keeps_velocity() {
    let mut ship = inertial();
    for _ in 0..90 {
        space_step(&mut ship, &input(&[SolarAction::Forward]), DT);
    }
    let v = ship.velocity;
    // Dampers on: v * exp(-BRAKE_RATE dt) each idle frame.
    let mut braked = ship;
    space_step(&mut braked, &input(&[]), DT);
    assert!((braked.velocity - v * (-BRAKE_RATE * DT).exp()).length() < 1e-5);
    for _ in 0..180 {
        space_step(&mut braked, &input(&[]), DT);
    }
    assert!(braked.velocity.length() < 0.05 * v.length());
    // Dampers off: it coasts.
    let mut coasting = ship;
    coasting.dampers = false;
    for _ in 0..180 {
        space_step(&mut coasting, &input(&[]), DT);
    }
    assert_eq!(coasting.velocity, v);
}

#[test]
fn x_toggles_the_dampers() {
    assert!(PRESSED.contains(&(KeyboardKey::KEY_X, SolarAction::ToggleDampers)));
    assert!(inertial().dampers, "on by default");
    let view = rust_software_renderer::pipeline::OrbitView::for_world(
        rust_software_renderer::pipeline::OrbitCamera::new(
            Vector3::zero(),
            rust_software_renderer::pipeline::ViewPreset::Oblique,
        ),
        8.0,
        rust_software_renderer::pipeline::Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut s = rust_software_renderer::solar::SolarScene::project3_system(
        rust_software_renderer::solar::SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    let messages = apply_toggles(&mut s, &[SolarAction::ToggleDampers]);
    assert_eq!(messages, ["Inertial dampers off"]);
    assert!(!s.ship.dampers);
    apply_toggles(&mut s, &[SolarAction::ToggleDampers]);
    assert!(s.ship.dampers);
}
