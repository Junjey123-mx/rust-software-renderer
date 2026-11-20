//! Project 3 terrain-aware flight: local flight keeps its clearance above
//! the ground right below the ship (a synthetic relief here; the planets'
//! relief arrives in C34), collisions and the follow camera respect it,
//! and quick travel arrives moving with the planet.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::flight::{
    FlightInput, FlightMode, FlightScale, local_step_over,
};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::physics::{
    COLLISION_MARGIN, SurfaceSampler, collide_with, lift_above,
};
use rust_software_renderer::solar::scene::NEAR;
use rust_software_renderer::solar::system::ScaleProfile;
use rust_software_renderer::solar::travel::warp_to;
use rust_software_renderer::solar::viewer::{apply_toggles, step};
use rust_software_renderer::solar::{ShipState, SolarMeshes, SolarScene};

const R: f32 = 1.1;

/// A mountain of 0.3 R around +Y over rolling bumps of 0.05 R.
fn relief(dir: Vector3) -> f32 {
    let peak = dir.dot(Vector3::new(0.0, 1.0, 0.0)).max(0.0).powi(8) * 0.3;
    let bumps = 0.05 * (7.0 * dir.x).sin() * (5.0 * dir.z).cos();
    R * (1.0 + peak + bumps)
}
const MAX_RELIEF: f32 = R * 1.35;

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

#[test]
fn local_altitude_over_bumps() {
    let scale = FlightScale::for_planet(&ScaleProfile::project3(), R);
    let planet = Vector3::zero();
    // Start low on the far side and fly over the mountain, diving (Q).
    let mut ship = ShipState::new(Vector3::new(0.0, 0.0, R + 0.6), 0.08);
    ship.inertia = true;
    ship.forward = Vector3::new(0.0, 1.0, 0.0);
    ship.up = Vector3::new(0.0, 0.0, 1.0);
    let mut mode = FlightMode::Local {
        planet: 1,
        previous_planet_position: planet,
    };
    let input = FlightInput::from_actions(&[SolarAction::Forward, SolarAction::Descend]);
    let mut lowest = f32::INFINITY;
    for _ in 0..600 {
        local_step_over(
            &mut ship,
            &mut mode,
            planet,
            R,
            &input,
            1.0 / 60.0,
            &scale,
            relief,
        );
        let n = ship.position.normalized();
        lowest = lowest.min(ship.position.length() - relief(n));
    }
    // Never below the clearance over the relief, and it did fly low.
    assert!(lowest >= scale.clearance - 1e-4, "{lowest}");
    assert!(lowest < scale.clearance + 0.05, "{lowest}");
}

#[test]
fn ship_collision_respects_the_mountain() {
    let mut ship = ShipState::new(Vector3::new(0.0, R * 1.2, 0.0), 0.08);
    ship.velocity = Vector3::new(0.3, -2.0, 0.0);
    collide_with(
        &mut ship,
        Vector3::zero(),
        Vector3::zero(),
        MAX_RELIEF,
        relief,
    );
    let floor = relief(Vector3::new(0.0, 1.0, 0.0)) + ship.radius() + COLLISION_MARGIN;
    assert!((ship.position.length() - floor).abs() < 1e-4);
    // Only the velocity into the ground goes; the tangential part stays.
    assert!(ship.velocity.y.abs() < 1e-6 && (ship.velocity.x - 0.3).abs() < 1e-6);
    // Above the broad-phase bound nothing changes.
    let far = ShipState::new(Vector3::new(0.0, 5.0, 0.0), 0.08);
    let mut moved = far;
    collide_with(
        &mut moved,
        Vector3::zero(),
        Vector3::zero(),
        MAX_RELIEF,
        relief,
    );
    assert_eq!(moved, far);
}

#[test]
fn follow_camera_above_terrain() {
    // The ship skims the mountain top; its follow eye, lifted like the
    // ship, never ends below the ground plus the near plane.
    let mut s = scene();
    s.ship.position = Vector3::new(0.0, relief(Vector3::new(0.0, 1.0, 0.0)) + 0.5, 0.0);
    s.ship.forward = Vector3::new(1.0, 0.0, 0.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    for pitch in [-0.6f32, 0.0, 0.6] {
        let mut ship = s.ship;
        ship.pitch(pitch);
        let eye = s.follow.camera(&ship).eye;
        let lifted = lift_above(eye, Vector3::zero(), MAX_RELIEF, 2.0 * NEAR, relief);
        let n = lifted.normalized();
        assert!(lifted.length() >= relief(n) + 2.0 * NEAR - 1e-4, "{pitch}");
    }
    // On the plain sphere the sampler is the radius everywhere.
    let sphere = SurfaceSampler::Sphere { radius: R };
    assert_eq!(sphere.radius_at(Vector3::new(0.0, 0.0, 1.0)), R);
    assert_eq!(sphere.max_radius(), R);
}

#[test]
fn warp_arrival_safe_velocity() {
    let mut s = scene();
    assert!(warp_to(&mut s, 1));
    assert_eq!(s.ship.velocity, s.body_velocity(1));
    let c = s.bodies[1].transform.position;
    let height = (s.ship.position - c).length() - s.radius_of(1);
    assert!((height - s.flight_scale(1).warp_margin).abs() < 1e-3);
    // Left alone for 3 s it neither drifts into the planet nor away from it
    // (it moves with it; gravity pulls a little).
    let mut lowest = f32::INFINITY;
    for _ in 0..180 {
        step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
        let d = (s.ship.position - s.bodies[1].transform.position).length();
        lowest = lowest.min(d - s.radius_of(1));
    }
    assert!(lowest > s.ship.radius(), "{lowest}");
}

#[test]
fn no_double_gravity_in_local() {
    let exit_velocity = |gravity: bool| {
        let mut s = scene();
        s.gravity = gravity;
        let planet = s.bodies[1].transform.position;
        s.ship.position = planet + Vector3::new(0.0, s.radius_of(1) + 1.0, 0.0);
        apply_toggles(&mut s, &[SolarAction::ToggleLocal]);
        for _ in 0..60 {
            step(
                &mut s,
                &[SolarAction::Forward],
                &[],
                Look::default(),
                1.0 / 60.0,
            );
        }
        (s.ship.position, s.ship.velocity)
    };
    assert_eq!(exit_velocity(true), exit_velocity(false));
}
