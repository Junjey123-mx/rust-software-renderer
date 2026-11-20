//! Project 3 mouse turn easing: the mouse sets where the nose goes and the
//! inertial ship eases into it with `1 - exp(-k dt)`, so the same movement
//! turns the ship alike at 30, 60 and 144 frames per second.

use raylib::math::Vector3;
use rust_software_renderer::solar::ShipState;
use rust_software_renderer::solar::flight::{
    FlightInput, MAX_PITCH, SPACE_UP, TURN_EASE, ease_turn, nose_elevation, space_step,
};
use rust_software_renderer::solar::input::Look;
use rust_software_renderer::solar::ship::TurnState;

fn inertial() -> ShipState {
    let mut ship = ShipState::new(Vector3::zero(), 0.08);
    ship.inertia = true;
    ship
}

fn look(yaw: f32, pitch: f32) -> FlightInput {
    FlightInput {
        look: Look { yaw, pitch },
        ..FlightInput::default()
    }
}

/// Heading (radians around +Y) and elevation after a mouse moving at
/// `rate` rad/s for 0.5 s, then still for 0.5 s, at `fps`.
fn sweep(fps: f32, rate: (f32, f32)) -> (f32, f32) {
    let mut ship = inertial();
    let dt = 1.0 / fps;
    let frames = fps as usize;
    for f in 0..frames {
        let moving = f < frames / 2;
        let input = if moving {
            look(rate.0 * dt, rate.1 * dt)
        } else {
            look(0.0, 0.0)
        };
        space_step(&mut ship, &input, dt);
    }
    (
        (-ship.forward.x).atan2(-ship.forward.z),
        nose_elevation(&ship, SPACE_UP),
    )
}

#[test]
fn same_heading_at_30_60_144_fps() {
    let one_degree = 1f32.to_radians();
    let reference = sweep(60.0, (1.2, 0.5));
    for fps in [30.0, 144.0] {
        let (yaw, pitch) = sweep(fps, (1.2, 0.5));
        assert!(
            (yaw - reference.0).abs() < one_degree,
            "{fps}: {yaw} vs {}",
            reference.0
        );
        assert!((pitch - reference.1).abs() < one_degree, "{fps}");
    }
    // The ship did turn: 0.6 rad of mouse, most of it applied.
    assert!(reference.0 > 0.55 && reference.1 > 0.2);
}

#[test]
fn no_double_dt_on_mouse() {
    // One mouse movement of 0.3 rad, whatever the frame time: the ship
    // ends up turned 0.3 rad (the delta is not multiplied by dt).
    for fps in [30.0, 144.0] {
        let mut ship = inertial();
        let dt = 1.0 / fps;
        space_step(&mut ship, &look(0.3, 0.0), dt);
        for _ in 0..(2.0 * fps) as usize {
            space_step(&mut ship, &look(0.0, 0.0), dt);
        }
        let yaw = (-ship.forward.x).atan2(-ship.forward.z);
        assert!((yaw - 0.3).abs() < 1e-3, "{fps}: {yaw}");
    }
}

#[test]
fn pitch_clamped() {
    let mut ship = inertial();
    space_step(&mut ship, &look(0.0, 3.0), 1.0 / 60.0);
    for _ in 0..120 {
        space_step(&mut ship, &look(0.0, 0.0), 1.0 / 60.0);
    }
    let e = nose_elevation(&ship, SPACE_UP);
    assert!(e <= MAX_PITCH + 1e-4, "{e}");
    // No wind-up: a small movement down lowers the nose right away.
    space_step(&mut ship, &look(0.0, -0.2), 1.0 / 60.0);
    for _ in 0..60 {
        space_step(&mut ship, &look(0.0, 0.0), 1.0 / 60.0);
    }
    assert!(nose_elevation(&ship, SPACE_UP) < e - 0.15);
}

#[test]
fn easing_never_overshoots_and_k_zero_holds() {
    let mut turn = TurnState {
        yaw: 0.5,
        pitch: -0.2,
    };
    assert_eq!(ease_turn(&mut turn, 0.0, 0.1), (0.0, 0.0));
    assert_eq!(turn.yaw, 0.5);
    let (yaw, pitch) = ease_turn(&mut turn, TURN_EASE, 1e6);
    assert_eq!((yaw, pitch), (0.5, -0.2));
    assert_eq!(turn, TurnState::default());
    let mut turn = TurnState {
        yaw: 1.0,
        pitch: 0.0,
    };
    let (yaw, _) = ease_turn(&mut turn, TURN_EASE, 1.0 / 60.0);
    assert!((yaw - (1.0 - (-TURN_EASE / 60.0).exp())).abs() < 1e-6);
    assert!(ease_turn(&mut turn, f32::NAN, 0.1) == (0.0, 0.0));
}

#[test]
fn pre_project_mouse_turns_at_once() {
    let mut ship = ShipState::new(Vector3::zero(), 0.08);
    space_step(&mut ship, &look(0.3, 0.0), 1.0 / 60.0);
    let yaw = (-ship.forward.x).atan2(-ship.forward.z);
    assert!((yaw - 0.3).abs() < 1e-5);
    assert_eq!(ship.turn, TurnState::default());
}
