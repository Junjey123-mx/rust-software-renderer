//! Post-coverage interpolation weights and the shader data types.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, ViewPreset, direction_from_angles};
use rust_software_renderer::rasterization::coverage::interpolation_weights;

const A: (f64, f64) = (10.0, 20.0);
const B: (f64, f64) = (310.0, 60.0);
const C: (f64, f64) = (120.0, 400.0);

fn samples() -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for i in 0..20 {
        for j in 0..20 {
            out.push((5.0 + i as f64 * 17.3, 3.0 + j as f64 * 21.7));
        }
    }
    out
}

#[test]
fn weights_sum_to_one() {
    for p in samples() {
        let w = interpolation_weights(A, B, C, p).unwrap();
        assert!((w[0] + w[1] + w[2] - 1.0).abs() < 1e-12);
    }
}

#[test]
fn weights_are_one_hot_at_vertices() {
    for (k, v) in [A, B, C].into_iter().enumerate() {
        let w = interpolation_weights(A, B, C, v).unwrap();
        for (i, wi) in w.iter().enumerate() {
            let expected = if i == k { 1.0 } else { 0.0 };
            assert!(
                (wi - expected).abs() < 1e-12,
                "vertex {k} weight {i} = {wi}"
            );
        }
    }
}

#[test]
fn centroid_has_equal_weights() {
    let g = ((A.0 + B.0 + C.0) / 3.0, (A.1 + B.1 + C.1) / 3.0);
    let w = interpolation_weights(A, B, C, g).unwrap();
    for wi in w {
        assert!((wi - 1.0 / 3.0).abs() < 1e-12);
    }
}

#[test]
fn winding_does_not_change_weights() {
    for p in samples() {
        let w = interpolation_weights(A, B, C, p).unwrap();
        let r = interpolation_weights(A, C, B, p).unwrap();
        assert!((w[0] - r[0]).abs() < 1e-12);
        assert!((w[1] - r[2]).abs() < 1e-12);
        assert!((w[2] - r[1]).abs() < 1e-12);
    }
}

#[test]
fn linear_attributes_are_reproduced_exactly() {
    let f = |p: (f64, f64)| 2.0 * p.0 + 3.0 * p.1 + 1.0;
    for p in samples() {
        let w = interpolation_weights(A, B, C, p).unwrap();
        let interpolated = w[0] * f(A) + w[1] * f(B) + w[2] * f(C);
        assert!((interpolated - f(p)).abs() < 1e-9);
    }
}

#[test]
fn degenerate_or_non_finite_input_is_rejected() {
    assert!(interpolation_weights((0.0, 0.0), (5.0, 5.0), (10.0, 10.0), (1.0, 2.0)).is_none());
    assert!(interpolation_weights(A, A, C, (1.0, 2.0)).is_none());
    assert!(interpolation_weights((f64::NAN, 0.0), B, C, (1.0, 2.0)).is_none());
    assert!(interpolation_weights(A, B, C, (f64::INFINITY, 2.0)).is_none());
}

#[test]
fn light_angles_follow_the_camera_convention() {
    for (az, el) in [(-2.45_f32, 0.55_f32), (0.0, 0.0), (1.3, -0.7), (3.0, 1.2)] {
        let mut cam = OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique);
        cam.azimuth = az;
        cam.elevation = el;
        let d = direction_from_angles(az, el);
        assert!((d - cam.eye_direction()).length() < 1e-6);
        assert!((d.length() - 1.0).abs() < 1e-6);
    }
}
