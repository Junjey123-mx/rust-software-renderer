//! Shared lighting, bump, parallax and cloud-shell kernels.

use raylib::math::Vector3;
use rust_software_renderer::shading::lighting::{
    blinn_phong, bump_normal, fresnel, lambert, parallax_shift, shell_exit, wrap_lambert,
};

fn unit(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z).normalized()
}

#[test]
fn diffuse_terms_ignore_the_view_and_clamp_backfaces() {
    let n = unit(0.0, 0.0, 1.0);
    let l = unit(0.3, 0.2, 1.0);
    assert!((lambert(n, l) - n.dot(l)).abs() < 1e-6);
    assert_eq!(lambert(n, -l), 0.0);
    assert!(wrap_lambert(n, unit(1.0, 0.0, -0.1), 0.3) > 0.0);
    assert_eq!(wrap_lambert(n, -n, 0.3), 0.0);
    assert!((0.0..=1.0).contains(&wrap_lambert(n, l, 0.35)));
}

#[test]
fn specular_depends_on_the_view() {
    let n = unit(0.0, 0.0, 1.0);
    let l = unit(0.5, 0.0, 1.0);
    let mirror = n * (2.0 * n.dot(l)) - l;
    let far = unit(-0.9, 0.3, 0.3);
    let peak = blinn_phong(n, l, mirror, 48.0);
    assert!((peak - 1.0).abs() < 1e-4);
    assert!(blinn_phong(n, l, far, 48.0) < 0.05);
    assert_eq!(blinn_phong(n, -l, mirror, 48.0), 0.0);
}

#[test]
fn fresnel_is_zero_facing_and_one_grazing() {
    let n = unit(0.0, 0.0, 1.0);
    assert_eq!(fresnel(n, n, 3.0), 0.0);
    assert!((fresnel(n, unit(1.0, 0.0, 0.0), 3.0) - 1.0).abs() < 1e-6);
}

#[test]
fn bump_with_zero_strength_or_flat_height_is_identity() {
    let n = unit(0.2, 0.5, 0.8);
    let rough = |q: Vector3| (q.x * 13.0).sin();
    assert_eq!(bump_normal(n, rough, 0.004, 0.0), n);
    let flat = bump_normal(n, |_| 0.3, 0.004, 2.0);
    assert!((flat - n).length() < 1e-5);
    let bumped = bump_normal(n, rough, 0.004, 1.0);
    assert!((bumped.length() - 1.0).abs() < 1e-5);
}

#[test]
fn bump_gradient_is_frame_invariant() {
    let k = 0.7;
    let x = Vector3::new(1.0, 0.0, 0.0);
    for n in [
        unit(0.2, 0.5, 0.8),
        unit(-0.7, 0.1, 0.3),
        unit(0.01, 0.9995, 0.02),
        unit(-0.02, -0.9997, 0.01),
        unit(0.0, 1.0, 0.0),
    ] {
        let numeric = bump_normal(n, |q| q.x, 0.004, k);
        let analytic = (n - (x - n * n.x) * k).normalized();
        assert!((numeric - analytic).length() < 1e-3, "{n:?}");
    }
}

#[test]
fn parallax_is_zero_at_normal_incidence_or_zero_scale() {
    let n = unit(0.1, 0.3, 0.9);
    let h = |q: Vector3| (q.y * 7.0).sin();
    let p = parallax_shift(n, n, h, 0.06, 8, 3);
    assert!((p - n).length() < 1e-5);
    assert_eq!(parallax_shift(n, unit(0.8, 0.0, 0.6), h, 0.0, 8, 3), n);
}

#[test]
fn parallax_moves_toward_the_view_for_raised_height() {
    let n = unit(0.0, 0.0, 1.0);
    let v = unit(1.0, 0.0, 1.0); // 45 degrees
    let vt = v - n * n.dot(v);
    let raised = parallax_shift(n, v, |_| 0.5, 0.06, 8, 3);
    let sunk = parallax_shift(n, v, |_| -0.5, 0.06, 8, 3);
    assert!((raised - n).dot(vt) > 0.0);
    assert!((sunk - n).dot(vt) < 0.0);
}

#[test]
fn parallax_fades_at_grazing_angles() {
    let n = unit(0.0, 0.0, 1.0);
    let grazing = unit(1.0, 0.0, 0.05);
    assert_eq!(parallax_shift(n, grazing, |_| 0.8, 0.1, 8, 3), n);
    assert_eq!(parallax_shift(n, -n, |_| 0.8, 0.1, 8, 3), n);
    let mid = parallax_shift(n, unit(1.0, 0.0, 0.25), |_| 0.8, 0.1, 8, 3);
    let full = parallax_shift(n, unit(1.0, 0.0, 1.0), |_| 0.8, 0.1, 8, 3);
    assert!((mid - n).length() < (full - n).length() * 4.0 + 1e-6);
}

#[test]
fn shell_exit_lies_on_the_shell_along_the_light() {
    let h = 0.07;
    let n = unit(0.3, 0.2, 0.9);
    for l in [
        unit(0.9, 0.1, 0.3),
        unit(-0.2, 0.8, 0.5),
        unit(0.3, 0.2, 0.9),
    ] {
        let q = shell_exit(n, l, h);
        let on_shell = q * (1.0 + h);
        let along = on_shell - n;
        assert!(
            along.cross(l).length() < 1e-4,
            "point lies on the light ray"
        );
        assert!(along.dot(l) >= -1e-6);
    }
    assert!((shell_exit(n, n, h) - n).length() < 1e-6);
}
