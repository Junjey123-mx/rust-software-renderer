//! Grand scale of the live viewer (second pass): planets five times their
//! Project 3 size against the same ship, the star seven times, orbits that
//! keep the Grand gap, quicker cruise and thrust, the comets outside the
//! star and the ship starting beyond every shell. The Project 3 scale of
//! the tests, the oracles and the recorded tour does not change.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::capture::OrbitState;
use rust_software_renderer::solar::flight::{FlightMode, THRUST_ACCEL};
use rust_software_renderer::solar::input::Look;
use rust_software_renderer::solar::system::layout::WORLD_MARGIN;
use rust_software_renderer::solar::system::{ScaleProfile, SystemSpec};
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};

fn grand() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    SolarScene::grand_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

#[test]
fn planets_dwarf_the_ship_and_the_star_dominates() {
    let (grand, project) = (SystemSpec::grand(), SystemSpec::project3());
    let ship = ScaleProfile::grand().ship_radius;
    assert_eq!(ship, ScaleProfile::project3().ship_radius);
    let mut largest = 0.0f32;
    for (g, p) in grand.planets.iter().zip(&project.planets) {
        assert!((g.radius() / p.radius() - 5.0).abs() < 1e-3, "{}", g.name);
        let ratio = g.radius() / ship;
        assert!((15.0..=30.0).contains(&ratio), "{} {ratio}", g.name);
        largest = largest.max(g.radius());
    }
    let star = grand.star.scale * SPHERE_RADIUS;
    assert!((star / (project.star.scale * SPHERE_RADIUS) - 7.0).abs() < 1e-3);
    assert!(
        (4.0..=6.0).contains(&(star / largest)),
        "{}",
        star / largest
    );
    ScaleProfile::grand().check(&grand).unwrap();
    // The Project 3 scale is untouched.
    let radii: Vec<f32> = project.planets.iter().map(|p| p.radius()).collect();
    for (r, want) in radii.iter().zip([1.10, 1.45, 1.30, 1.75, 1.60]) {
        assert!((r - want).abs() < 1e-4);
    }
}

#[test]
fn orbits_are_the_tightest_that_keep_the_gap() {
    let s = grand();
    let layout = s.layout.as_ref().unwrap();
    let gap = ScaleProfile::grand().min_gap;
    let mut edge = s.shell_of(0);
    for (i, &r) in layout.radii.iter().enumerate() {
        let shell = s.shell_of(i + 1);
        let min = edge + gap + shell;
        assert!(r >= min && r - min < 0.5 + 1e-3, "planet {i}: {r} vs {min}");
        edge = r + shell;
    }
    assert!((layout.world_radius - edge - WORLD_MARGIN).abs() < 1e-3);
    // About five times the Project 3 system.
    assert!(layout.world_radius > 200.0, "{}", layout.world_radius);
}

#[test]
fn ship_starts_beyond_every_shell_with_grand_speeds() {
    let s = grand();
    s.check_spawn().unwrap();
    let layout = s.layout.as_ref().unwrap();
    let from_star = (s.ship.position - s.center).length();
    assert!(
        from_star > layout.world_radius - WORLD_MARGIN,
        "{from_star}"
    );
    assert_eq!(s.ship.move_speed, 6.0);
    assert_eq!(s.ship.accel, 2.0 * THRUST_ACCEL);
    assert!(s.gravity && s.ship.inertia);
}

#[test]
fn gravity_keeps_its_design_at_the_grand_scale() {
    let s = grand();
    let profile = ScaleProfile::grand();
    for i in 1..s.bodies.len() {
        let (mu, r) = (s.bodies[i].mu, s.radius_of(i));
        let surface = mu / (r * r);
        assert!(
            surface <= 0.25 * profile.thrust_accel * 1.1,
            "{i}: {surface}"
        );
        let escape = (2.0 * mu / r).sqrt();
        assert!(escape <= 0.5 * profile.cruise_speed + 1e-3, "{i}: {escape}");
    }
}

#[test]
fn comets_stay_outside_the_star() {
    let s = grand();
    let comets = s.comets.as_ref().unwrap();
    let star = s.shell_of(0);
    for c in &comets.comets {
        let nearest = (0..720)
            .map(|k| (c.position(k as f32 * 0.5) - s.center).length())
            .fold(f32::INFINITY, f32::min);
        assert!(nearest > star, "{nearest} vs {star}");
    }
}

#[test]
fn every_grand_planet_captures_a_slow_ship() {
    for i in 1..grand().bodies.len() {
        let mut s = grand();
        s.capture.enabled = true;
        let z = s.capture_zone(i).unwrap();
        let p = s.bodies[i].transform.position;
        let n = (p - s.center).normalized();
        s.flight = FlightMode::Space;
        s.ship.position = p + n * (z.capture + 0.1 * s.radius_of(i));
        s.ship.forward = Vector3::new(0.0, 1.0, 0.0).cross(n).normalized();
        s.ship.up = n;
        s.ship.renormalize();
        s.ship.velocity = s.body_velocity(i) - n * 1.6;
        s.ship.dampers = false;
        let mut captured = false;
        for _ in 0..300 {
            step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
            if matches!(s.capture.state, OrbitState::Captured { planet, .. } if planet == i) {
                captured = true;
                break;
            }
        }
        assert!(captured, "planet {i}");
    }
}
