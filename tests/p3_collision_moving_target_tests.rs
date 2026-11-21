//! Collision against moving planets at any frame rate (expansion fix of the
//! limitation reported in `docs/results/project3/gravity_evidence.md`).
//!
//! The collision shapes of a frame are built after the clock placed the
//! planets at the frame's end, while `physics::collide` moves each shape
//! from the frame's start. Before the fix the shape started at the end
//! position, so it led the planet by one frame of its orbit and a ship
//! sliding along a planet at 10 FPS ended up to 0.035 u inside it (Clover,
//! Hearts and House of Cards). Now the shape starts where the planet was at
//! the frame's start: the gap, measured against where the planet really is
//! at the end of every frame, keeps the 0.05 margin (0.0496 or more
//! measured at 10-144 FPS on the five planets).

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

/// Smallest gap accepted: most of the 0.05 collision margin.
const KEPT: f32 = 0.045;

/// Frame rates of the regression, frames per second.
const FPS: [f32; 5] = [10.0, 15.0, 30.0, 60.0, 144.0];

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    SolarScene::project3_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

/// Room between the ship's sphere and the planet's highest ground, against
/// where the planet is now (the end of the last frame).
fn gap(s: &SolarScene, i: usize) -> f32 {
    let relief = s.relief_of(i).map_or(0.0, |r| r.field.max_height());
    (s.ship.position - s.bodies[i].transform.position).length()
        - s.radius_of(i) * (1.0 + relief)
        - s.ship.radius()
}

/// Smallest gap over `seconds` of `actions` at `fps`.
fn closest(s: &mut SolarScene, i: usize, actions: &[SolarAction], fps: f32, seconds: f32) -> f32 {
    let dt = 1.0 / fps;
    let mut min = f32::INFINITY;
    for _ in 0..(seconds * fps) as usize {
        step(s, actions, &[], Look::default(), dt);
        assert!(s.ship.position.x.is_finite());
        min = min.min(gap(s, i));
    }
    min
}

#[test]
fn shapes_start_where_the_planets_were() {
    let mut s = scene();
    step(&mut s, &[], &[], Look::default(), 0.1);
    let shapes = s.collision_shapes();
    for (i, shape) in shapes.iter().enumerate() {
        let start = s.bodies[i].transform.position - s.body_velocity(i) * s.clock.real_delta;
        assert!((shape.center - start).length() < 1e-5, "body {i}");
        // Moved by its velocity over the frame, it is where the body is.
        let end = shape.center + shape.velocity * s.clock.real_delta;
        assert!(
            (end - s.bodies[i].transform.position).length() < 2e-3,
            "body {i}"
        );
    }
    // Paused: nothing moves, the shapes are the bodies.
    s.clock.paused = true;
    step(&mut s, &[], &[], Look::default(), 0.1);
    for (i, shape) in s.collision_shapes().iter().enumerate() {
        assert_eq!(shape.center, s.bodies[i].transform.position);
    }
}

#[test]
fn boost_into_every_planet_at_any_fps() {
    let boosted = [SolarAction::Forward, SolarAction::Boost];
    for fps in FPS {
        for i in 1..=5 {
            let mut s = scene();
            let c = s.bodies[i].transform.position;
            s.ship.position = c + Vector3::new(0.0, 0.0, 6.0 * s.radius_of(i));
            s.ship.forward = Vector3::new(0.0, 0.0, -1.0);
            s.ship.up = Vector3::new(0.0, 1.0, 0.0);
            let min = closest(&mut s, i, &boosted, fps, 5.0);
            println!("radial body {i} {fps} fps: closest gap {min:.4}");
            assert!(min > KEPT, "body {i} at {fps} fps: {min}");
            assert!(min < 0.5, "body {i} at {fps} fps: never reached ({min})");
        }
    }
}

#[test]
fn planet_runs_into_a_waiting_ship() {
    // The ship waits on the orbit, ahead of the planet, dampers on: the
    // planet (the moving target) comes to it and pushes it aside.
    for fps in FPS {
        for i in 1..=5 {
            let mut s = scene();
            let c = s.bodies[i].transform.position;
            let v = s.body_velocity(i);
            let ahead = v.normalized() * (1.6 * s.radius_of(i) + 1.0);
            s.ship.position = c + ahead + Vector3::new(0.0, 0.2 * s.radius_of(i), 0.0);
            let seconds = 3.0 * ahead.length() / v.length();
            let min = closest(&mut s, i, &[], fps, seconds);
            println!("tangential body {i} {fps} fps: closest gap {min:.4}");
            assert!(min > KEPT, "body {i} at {fps} fps: {min}");
        }
    }
}

#[test]
fn reported_planets_regression() {
    // Clover (2), Hearts (3) and House of Cards (4) at 10 FPS, the cases of
    // the report (-0.035, -0.007 and -0.025 before the fix).
    let boosted = [SolarAction::Forward, SolarAction::Boost];
    for i in [2, 3, 4] {
        let mut s = scene();
        let c = s.bodies[i].transform.position;
        s.ship.position = c + Vector3::new(0.0, 0.0, 6.0 * s.radius_of(i));
        s.ship.forward = Vector3::new(0.0, 0.0, -1.0);
        s.ship.up = Vector3::new(0.0, 1.0, 0.0);
        let min = closest(&mut s, i, &boosted, 10.0, 5.0);
        assert!(min > KEPT, "body {i}: {min}");
    }
}
