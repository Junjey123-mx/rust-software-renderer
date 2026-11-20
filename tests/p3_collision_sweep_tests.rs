//! Project 3 collisions: every physics substep is swept against each body
//! (radius plus ship plus margin) in the body's moving frame, so the ship
//! never enters a body, at light speed, after a warp or when a planet runs
//! into it.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::physics::{COLLISION_MARGIN, sweep};
use rust_software_renderer::solar::travel::warp_to;
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

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

/// Smallest gap between the ship's bounding sphere and body `i`'s surface
/// over `frames` frames of `actions` at `dt`.
fn closest(s: &mut SolarScene, i: usize, actions: &[SolarAction], frames: usize, dt: f32) -> f32 {
    let mut gap = f32::INFINITY;
    for _ in 0..frames {
        step(s, actions, &[], Look::default(), dt);
        let d = (s.ship.position - s.bodies[i].transform.position).length();
        gap = gap.min(d - s.radius_of(i) - s.ship.radius());
    }
    gap
}

/// Ship `distance` from body `i`'s center, nose and velocity towards it.
fn aim_at(s: &mut SolarScene, i: usize, distance: f32) {
    let c = s.bodies[i].transform.position;
    s.ship.position = c + Vector3::new(0.0, 0.0, distance);
    s.ship.forward = Vector3::new(0.0, 0.0, -1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
}

#[test]
fn boost_never_penetrates() {
    for dt in [1.0 / 60.0, 0.1] {
        let mut s = scene();
        aim_at(&mut s, 1, 20.0);
        let boosted = [SolarAction::Forward, SolarAction::Boost];
        let gap = closest(&mut s, 1, &boosted, (4.0 / dt) as usize, dt);
        assert!(gap > -1e-3, "dt {dt}: {gap}");
        // It got there and rests on the surface: the margin, plus up to one
        // frame of the planet's motion (1 u/s), since the ship is placed
        // against the planet at the end of the frame and measured against
        // where it was at the start.
        assert!(
            gap < COLLISION_MARGIN + dt + 0.05,
            "dt {dt}: never reached ({gap})"
        );
    }
}

#[test]
fn grazing_pass_no_false_contact() {
    let r = 1.5;
    let y = r + 1e-3;
    assert_eq!(
        sweep(Vector3::new(-5.0, y, 0.0), Vector3::new(5.0, y, 0.0), r),
        None
    );
    let hit = sweep(
        Vector3::new(-5.0, y - 2e-3, 0.0),
        Vector3::new(5.0, y - 2e-3, 0.0),
        r,
    );
    assert!(hit.is_some());
    // A degenerate segment outside: nothing; inside: a finite normal.
    let p = Vector3::new(3.0, 0.0, 0.0);
    assert_eq!(sweep(p, p, r), None);
    let c = sweep(Vector3::zero(), Vector3::zero(), r).unwrap();
    assert!(c.normal.length().is_finite() && (c.normal.length() - 1.0).abs() < 1e-6);
}

#[test]
fn start_inside_is_expelled() {
    let mut s = scene();
    let c = s.bodies[1].transform.position;
    s.ship.position = c + Vector3::new(0.2, 0.1, 0.0);
    step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
    let d = (s.ship.position - s.bodies[1].transform.position).length();
    assert!(d >= s.radius_of(1) + s.ship.radius(), "{d}");
    assert!(s.ship.position.x.is_finite());
}

#[test]
fn after_warp_no_penetration() {
    let mut s = scene();
    assert!(warp_to(&mut s, 2));
    let gap = closest(
        &mut s,
        2,
        &[SolarAction::Forward, SolarAction::Boost],
        180,
        1.0 / 60.0,
    );
    assert!(gap > -1e-3, "{gap}");
}

#[test]
fn moving_planet_relative_velocity() {
    // A parked ship in Crimson Entrance's path: the planet pushes it along
    // instead of swallowing it.
    let mut s = scene();
    let c = s.bodies[1].transform.position;
    let v = s.body_velocity(1);
    assert!((v.length() - 1.0).abs() < 1e-3, "1 u/s along the orbit");
    let ahead = v.normalized() * (s.radius_of(1) + s.ship.radius() + 0.3);
    s.ship.position = c + ahead;
    let gap = closest(&mut s, 1, &[], 240, 1.0 / 60.0);
    assert!(gap > -1e-3, "{gap}");
    assert!(s.ship.velocity.dot(v.normalized()) > 0.5, "carried along");
}

#[test]
fn variable_radius() {
    // The star (5.40) and House of Cards (1.75, index 3 since C49 put the
    // clover planet at index 2): each stops the ship at its own radius.
    for i in [0, 3] {
        let mut s = scene();
        s.gravity = false;
        let distance = s.radius_of(i) + 6.0;
        aim_at(&mut s, i, distance);
        let gap = closest(&mut s, i, &[SolarAction::Forward], 240, 1.0 / 60.0);
        assert!(
            gap > -1e-3 && gap < COLLISION_MARGIN + 0.05,
            "body {i}: {gap}"
        );
    }
}
