//! Project 3 height fields: finite heights everywhere, unit normals also at
//! the poles and seams, a bound that holds, and one field for the mesh and
//! the collisions.

use raylib::math::Vector3;
use rust_software_renderer::solar::physics::SurfaceSampler;
use rust_software_renderer::solar::planet::{HeightField, TestField};

/// `count` directions spread over the sphere (Fibonacci lattice).
fn fibonacci(count: usize) -> Vec<Vector3> {
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    (0..count)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
            let r = (1.0 - y * y).sqrt();
            let a = golden * i as f32;
            Vector3::new(r * a.cos(), y, r * a.sin())
        })
        .collect()
}

fn test_field() -> HeightField {
    HeightField::Test(TestField {
        amplitude: 0.12,
        frequency: 9.0,
        sea_level: Some(-0.02),
    })
}

#[test]
fn finite_everywhere_on_fibonacci_10k() {
    for field in [HeightField::Flat, test_field()] {
        for n in fibonacci(10_000) {
            assert!(field.height(n).is_finite());
            let r = field.surface_radius(n);
            assert!(r.is_finite() && r > 0.5);
        }
    }
    // A field that gives NaN is sanitized to 0.
    let nan = HeightField::Test(TestField {
        amplitude: f32::NAN,
        frequency: 3.0,
        sea_level: None,
    });
    assert_eq!(nan.height(Vector3::new(0.3, 0.4, 0.866)), 0.0);
    assert_eq!(nan.max_height(), 0.0);
}

#[test]
fn unit_normals_at_poles_and_seams() {
    let field = test_field();
    let special = [
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(1.0, 1.0, 0.0).normalized(),
        Vector3::new(1e-7, 1.0, -1e-7).normalized(),
    ];
    for n in special.into_iter().chain(fibonacci(2000)) {
        let normal = field.normal(n, 1e-3);
        assert!((normal.length() - 1.0).abs() < 1e-4, "{n:?}");
        assert!(normal.dot(n) > 0.0, "outward at {n:?}");
    }
}

#[test]
fn flat_field_normal_is_radial() {
    for n in fibonacci(500) {
        let normal = HeightField::Flat.normal(n, 1e-3);
        assert!(normal.dot(n) > 0.9999, "{n:?}");
        assert_eq!(HeightField::Flat.surface_radius(n), 1.0);
    }
    assert_eq!(HeightField::Flat.max_height(), 0.0);
}

#[test]
fn max_height_bounds_samples() {
    let field = test_field();
    let highest = fibonacci(10_000)
        .into_iter()
        .map(|n| field.surface_radius(n) - 1.0)
        .fold(f32::MIN, f32::max);
    assert!(highest <= field.max_height() + 1e-6, "{highest}");
    // It is a real bound, not a loose one: within 10 % of the samples.
    assert!(highest > 0.9 * field.max_height(), "{highest}");
    // A bound below a real sample would fail.
    assert!(highest > 0.9 * 0.12);
    // The sea flattens the low ground.
    let low = fibonacci(10_000)
        .into_iter()
        .map(|n| field.surface_radius(n))
        .fold(f32::MAX, f32::min);
    assert!((low - (1.0 - 0.02)).abs() < 1e-6);
}

#[test]
fn sampler_uses_same_field() {
    let field = test_field();
    let sampler = SurfaceSampler::Field { base: 1.45, field };
    for n in fibonacci(300) {
        assert_eq!(sampler.radius_at(n), 1.45 * field.surface_radius(n));
        assert!(sampler.radius_at(n) <= sampler.max_radius() + 1e-6);
    }
    assert_eq!(sampler.max_radius(), 1.45 * (1.0 + field.max_height()));
}
