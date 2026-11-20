//! Project 3 gravity toggle (`G`): coasting near a planet, the ship goes
//! straight with gravity off and curves with it on; local flight ignores
//! it and the toggle changes nothing else.

use raylib::consts::KeyboardKey;
use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::flight::FlightMode;
use rust_software_renderer::solar::input::{Look, PRESSED, SolarAction};
use rust_software_renderer::solar::viewer::{apply_toggles, status_line, step};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const DT: f32 = 1.0 / 60.0;

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

/// Paused scene (static field) with the ship coasting past Crimson
/// Entrance at three radii, 1 u/s across, dampers off.
fn coasting(gravity: bool) -> SolarScene {
    let mut s = scene();
    apply_toggles(&mut s, &[SolarAction::TogglePause]);
    s.gravity = gravity;
    let planet = s.bodies[1].transform.position;
    s.ship.position = planet + Vector3::new(0.0, 3.0 * s.radius_of(1), 0.0);
    s.ship.velocity = Vector3::new(1.0, 0.0, 0.0);
    s.ship.dampers = false;
    s
}

fn coast(s: &mut SolarScene, seconds: f32) {
    for _ in 0..(seconds / DT).round() as usize {
        step(s, &[], &[], Look::default(), DT);
    }
}

#[test]
fn coast_straight_when_off() {
    let mut s = coasting(false);
    let start = s.ship.position;
    coast(&mut s, 5.0);
    assert_eq!(s.ship.velocity, Vector3::new(1.0, 0.0, 0.0));
    let moved = s.ship.position - start;
    assert!(
        (moved - Vector3::new(5.0, 0.0, 0.0)).length() < 1e-3,
        "{moved:?}"
    );
}

#[test]
fn coast_curves_when_on() {
    let mut s = coasting(true);
    let start = s.ship.position;
    let planet = s.bodies[1].transform.position;
    coast(&mut s, 5.0);
    // Pulled towards Crimson Entrance (below): about a metre in 5 s.
    let drop = start.y - s.ship.position.y;
    assert!(drop > 0.5, "{drop}");
    assert!(s.ship.velocity.y < 0.0);
    assert!((s.ship.position - planet).length() > s.radius_of(1));
}

#[test]
fn toggle_keeps_scene_and_camera() {
    assert!(PRESSED.contains(&(KeyboardKey::KEY_G, SolarAction::ToggleGravity)));
    let mut s = scene();
    assert!(s.gravity, "on in the Project 3 system");
    let (ship, bodies, rig, view, time) = (s.ship, s.bodies.clone(), s.rig, s.view, s.clock.time);
    let messages = apply_toggles(&mut s, &[SolarAction::ToggleGravity]);
    assert_eq!(messages, ["Gravity off"]);
    assert!(!s.gravity && s.gravity_sources().is_empty());
    assert!(status_line(&s, 0.0, 60.0).ends_with("grav off"));
    assert_eq!(
        (s.ship, s.rig, s.view, s.clock.time),
        (ship, rig, view, time)
    );
    assert_eq!(s.bodies, bodies);
    apply_toggles(&mut s, &[SolarAction::ToggleGravity]);
    assert!(status_line(&s, 0.0, 60.0).ends_with("grav on"));
    // The sources are the planets with mu, where they are now.
    let sources = s.gravity_sources();
    // Crimson Entrance, the clover planet (C49) and House of Cards.
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0].center, s.bodies[1].transform.position);
    assert_eq!(sources[0].mu, s.bodies[1].mu);
}

#[test]
fn local_mode_ignores_gravity() {
    let path = |gravity: bool| {
        let mut s = scene();
        let planet = s.bodies[1].transform.position;
        s.ship.position = planet + Vector3::new(0.0, s.radius_of(1) + 1.0, 0.0);
        apply_toggles(&mut s, &[SolarAction::ToggleLocal]);
        assert!(matches!(s.flight, FlightMode::Local { .. }));
        if !gravity {
            apply_toggles(&mut s, &[SolarAction::ToggleGravity]);
        }
        let mut points = vec![];
        for _ in 0..120 {
            step(&mut s, &[SolarAction::Forward], &[], Look::default(), DT);
            points.push(s.ship.position);
        }
        points
    };
    assert_eq!(path(true), path(false));
}

#[test]
fn pause_static_field() {
    let mut s = coasting(true);
    let before = s.gravity_sources();
    coast(&mut s, 2.0);
    assert_eq!(s.gravity_sources(), before, "paused: the planets hold");
    // The pre-project system has no pull at all.
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut legacy = SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    assert!(!legacy.gravity);
    legacy.gravity = true;
    assert!(
        legacy.gravity_sources().is_empty(),
        "no mu in the pre-project spec"
    );
}
