//! Project 3 gravity: softened inverse-square pull of each planet, summed
//! over the sources, finite everywhere, and integrated once per substep.

use raylib::math::Vector3;
use rust_software_renderer::solar::ShipState;
use rust_software_renderer::solar::flight::THRUST_ACCEL;
use rust_software_renderer::solar::physics::{GravitySource, advance, design_mu, gravity_accel};
use rust_software_renderer::solar::system::{SpecError, SystemSpec};

fn source(center: Vector3, mu: f32, radius: f32) -> GravitySource {
    GravitySource::new(center, mu, radius)
}

#[test]
fn single_body_points_to_center() {
    let c = Vector3::new(3.0, -1.0, 2.0);
    let p = Vector3::new(7.0, 1.0, -2.0);
    let a = gravity_accel(p, &[source(c, 1.0, 1.1)]);
    assert!(a.normalized().dot((c - p).normalized()) > 0.99999);
}

#[test]
fn inverse_square_far_field() {
    let s = [source(Vector3::zero(), 2.0, 1.0)];
    let a = |d: f32| gravity_accel(Vector3::new(d, 0.0, 0.0), &s).length();
    // Far away the softening fades: a ~ mu / d^2, and doubling d quarters it.
    assert!((a(40.0) - 2.0 / 1600.0).abs() / (2.0 / 1600.0) < 1e-3);
    assert!((a(20.0) / a(40.0) - 4.0).abs() < 1e-2);
}

#[test]
fn two_bodies_superpose() {
    let a = source(Vector3::new(-5.0, 0.0, 0.0), 1.0, 1.1);
    let b = source(Vector3::new(0.0, 4.0, 1.0), 2.0, 1.75);
    let p = Vector3::new(1.0, 0.5, -2.0);
    let both = gravity_accel(p, &[a, b]);
    let sum = gravity_accel(p, &[a]) + gravity_accel(p, &[b]);
    assert!((both - sum).length() < 1e-7);
}

#[test]
fn finite_at_center() {
    let c = Vector3::new(1.0, 2.0, 3.0);
    let a = gravity_accel(c, &[source(c, 1.0, 1.1)]);
    assert_eq!(a, Vector3::zero());
    // A point source (no softening) at its own center adds nothing.
    let point = GravitySource {
        center: c,
        mu: 1.0,
        softening: 0.0,
    };
    assert_eq!(gravity_accel(c, &[point]), Vector3::zero());
}

#[test]
fn zero_mu_zero_force() {
    let p = Vector3::new(2.0, 0.0, 0.0);
    assert_eq!(
        gravity_accel(p, &[source(Vector3::zero(), 0.0, 1.0)]),
        Vector3::zero()
    );
    assert_eq!(gravity_accel(p, &[]), Vector3::zero());
}

#[test]
fn design_mu_meets_the_bounds() {
    // Document B: 0.99 for Crimson Entrance (R 1.1), 1.97 for House (1.75).
    assert!((design_mu(1.1, THRUST_ACCEL, 3.0) - 0.99).abs() < 0.01);
    assert!((design_mu(1.75, THRUST_ACCEL, 3.0) - 1.97).abs() < 0.01);
    for r in [1.1, 1.3, 1.45, 1.75] {
        let mu = design_mu(r, THRUST_ACCEL, 3.0);
        let surface = gravity_accel(Vector3::new(r, 0.0, 0.0), &[source(Vector3::zero(), mu, r)]);
        assert!(surface.length() <= 0.25 * THRUST_ACCEL + 1e-4, "{r}");
        assert!((2.0 * mu / r).sqrt() <= 0.5 * 3.0 + 1e-4, "{r}: escape");
    }
    let spec = SystemSpec::project3();
    for p in &spec.planets {
        let expected = design_mu(p.radius(), THRUST_ACCEL, 3.0);
        assert!((p.mu - expected).abs() < 1e-5, "{}", p.name);
    }
    assert_eq!(spec.star.mu, 0.0);
    // A negative mu is refused by the spec.
    let mut bad = spec.clone();
    bad.planets[0].mu = -1.0;
    assert_eq!(
        bad.validate(),
        Err(SpecError::OutOfRange("planet.mu", -1.0))
    );
}

#[test]
fn gravity_applied_once_per_substep() {
    // Semi-implicit Euler over one frame: the velocity gains a dt, not n a dt.
    let s = [source(Vector3::zero(), 1.0, 1.1)];
    let mut ship = ShipState::new(Vector3::new(6.0, 0.0, 0.0), 0.08);
    let a0 = gravity_accel(ship.position, &s);
    let n = advance(&mut ship, 1.0 / 60.0, |ship, h| {
        ship.velocity += gravity_accel(ship.position, &s) * h;
        ship.position += ship.velocity * h;
    });
    assert_eq!(n, 4);
    assert!((ship.velocity - a0 / 60.0).length() < 1e-4 * a0.length());
}
