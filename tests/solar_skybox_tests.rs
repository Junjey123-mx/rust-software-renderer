//! Procedural star field.

use rust_software_renderer::solar::skybox::{MAX_STARS, StarField, random01};

#[test]
fn the_generator_is_reproducible_and_in_unit_range() {
    let (mut a, mut b) = (42u32, 42u32);
    for _ in 0..1000 {
        let x = random01(&mut a);
        assert_eq!(x, random01(&mut b));
        assert!((0.0..1.0).contains(&x));
    }
}

#[test]
fn the_same_seed_gives_the_same_sky_and_another_seed_a_different_one() {
    let a = StarField::generate(42, 2000);
    assert_eq!(a, StarField::generate(42, 2000));
    let b = StarField::generate(7, 2000);
    assert_ne!(a.stars[0].direction, b.stars[0].direction);
    assert_eq!(a.stars.len(), 2000);
}

#[test]
fn stars_are_unit_directions_with_bounded_brightness_and_tint() {
    let f = StarField::generate(42, 2000);
    for s in &f.stars {
        assert!((s.direction.length() - 1.0).abs() < 1e-4);
        assert!((0.3..=1.0).contains(&s.brightness));
        let c = s.color();
        assert!(c.x <= 1.0 && c.y <= 1.0 && c.z <= 1.0 && c.x > 0.0);
    }
    let bright = f.stars.iter().filter(|s| s.brightness > 0.8).count();
    assert!(bright * 4 < f.stars.len(), "most stars are faint: {bright}");
}

#[test]
fn directions_are_spread_evenly_over_the_sphere() {
    let f = StarField::generate(42, 4000);
    let n = f.stars.len() as f32;
    let mean = f
        .stars
        .iter()
        .fold(raylib::math::Vector3::zero(), |acc, s| acc + s.direction)
        / n;
    assert!(mean.length() < 0.05, "{mean:?}");
    // Equal-area bands in y hold about the same number of stars.
    let upper = f.stars.iter().filter(|s| s.direction.y > 0.5).count() as f32;
    let middle = f
        .stars
        .iter()
        .filter(|s| s.direction.y.abs() <= 0.5)
        .count() as f32;
    assert!((upper / n - 0.25).abs() < 0.03, "{upper}");
    assert!((middle / n - 0.5).abs() < 0.03, "{middle}");
}

#[test]
fn the_count_is_clamped() {
    assert_eq!(
        StarField::generate(1, MAX_STARS + 10).stars.len(),
        MAX_STARS
    );
    assert!(StarField::generate(1, 0).stars.is_empty());
}
