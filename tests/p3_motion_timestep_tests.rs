//! Project 3 fixed-step motion: equal substeps no longer than 1/240 s, so
//! a flight at 30, 60 or 144 frames per second follows the same path, a
//! long frame is capped and a bad state is undone.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::flight::{FlightInput, space_step};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::physics::{MAX_SUBSTEPS, PHYSICS_STEP, advance};
use rust_software_renderer::solar::viewer::{apply_toggles, step};
use rust_software_renderer::solar::{ShipState, SolarMeshes, SolarScene};

fn inertial() -> ShipState {
    let mut ship = ShipState::new(Vector3::new(0.0, 0.0, 50.0), 0.08);
    ship.move_speed = 3.0;
    ship.inertia = true;
    ship
}

/// Position after 4 s of a scripted flight (thrust, a boost burst, a
/// mouse turn and a coast) at `fps`.
fn flight(fps: f32) -> Vector3 {
    let mut ship = inertial();
    let dt = 1.0 / fps;
    let frames = (4.0 * fps) as usize;
    for f in 0..frames {
        let t = f as f32 * dt;
        let mut actions = vec![];
        if t < 3.0 {
            actions.push(SolarAction::Forward);
        }
        if (1.0..1.5).contains(&t) {
            actions.push(SolarAction::Boost);
        }
        let mut input = FlightInput::from_actions(&actions);
        if t < 1.0 {
            input.look = Look {
                yaw: 0.4 * dt,
                pitch: 0.1 * dt,
            };
        }
        space_step(&mut ship, &input, dt);
    }
    ship.position
}

#[test]
fn trajectory_30_60_144_within_1pct() {
    let start = inertial().position;
    let reference = flight(60.0);
    let travelled = (reference - start).length();
    assert!(travelled > 10.0, "{travelled}");
    for fps in [30.0, 144.0] {
        let gap = (flight(fps) - reference).length();
        assert!(gap < 0.01 * travelled, "{fps}: {gap} of {travelled}");
    }
}

#[test]
fn substeps_bounded() {
    let mut ship = inertial();
    let mut longest: f32 = 0.0;
    for (dt, expected) in [
        (1.0 / 30.0, 8),
        (1.0 / 60.0, 4),
        (1.0 / 144.0, 2),
        (0.1, 24),
    ] {
        let n = advance(&mut ship, dt, |_, h| longest = longest.max(h));
        assert_eq!(n, expected, "{dt}");
        assert!(n <= MAX_SUBSTEPS);
    }
    assert!(longest <= PHYSICS_STEP * 1.0001, "{longest}");
}

#[test]
fn large_dt_capped_no_tunnel() {
    // Five seconds of frame integrate 0.1 s only.
    let mut ship = inertial();
    ship.velocity = Vector3::new(0.0, 0.0, -24.0);
    let mut total = 0.0;
    let n = advance(&mut ship, 5.0, |s, h| {
        total += h;
        s.position += s.velocity * h;
    });
    assert_eq!(n, MAX_SUBSTEPS);
    assert!((total - 0.1).abs() < 1e-5);
    assert!((ship.position.z - (50.0 - 2.4)).abs() < 1e-4);
    // Each substep moves at most 24 / 240 = 0.1: no jump past a body.
    // A negative or non-finite delta is ignored.
    let before = ship;
    assert_eq!(advance(&mut ship, -0.5, |s, _| s.position.x += 1.0), 0);
    assert_eq!(advance(&mut ship, f32::NAN, |s, _| s.position.x += 1.0), 0);
    assert_eq!(ship, before);
}

#[test]
fn non_finite_state_recovers() {
    let mut ship = inertial();
    ship.velocity = Vector3::new(1.0, 0.0, 0.0);
    let mut calls = 0;
    advance(&mut ship, 1.0 / 60.0, |s, h| {
        calls += 1;
        if calls == 3 {
            s.position.x = f32::NAN;
        } else {
            s.position += s.velocity * h;
        }
    });
    assert!(ship.position.x.is_finite());
    assert_eq!(ship.velocity, Vector3::zero(), "stopped after the bad step");
    // Two good substeps before the bad one, none after (it is at rest).
    assert!((ship.position.x - 2.0 / 240.0).abs() < 1e-6);
}

#[test]
fn pause_does_not_freeze_ship() {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::project3_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    apply_toggles(&mut s, &[SolarAction::TogglePause]);
    let time = s.clock.time;
    let p = s.ship.position;
    for _ in 0..30 {
        step(
            &mut s,
            &[SolarAction::Forward],
            &[],
            Look::default(),
            1.0 / 30.0,
        );
    }
    assert_eq!(s.clock.time, time);
    assert!((s.ship.position - p).length() > 0.5);
    assert!(s.ship.velocity.length() > 0.0);
}
