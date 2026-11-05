//! Spade and diamond distance fields, sphere frames and seeded directions.

use raylib::math::Vector3;
use rust_software_renderer::shading::sdf::{diamond_l1, spade, triangle};
use rust_software_renderer::shading::sphere::{
    angular_distance, from_local, hash_direction, local_plane, rotate_axis, rotate_y, tangent_frame,
};

/// Fibonacci-sphere directions (deterministic, roughly uniform).
fn directions(n: usize) -> Vec<Vector3> {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
            let r = (1.0 - y * y).sqrt();
            let phi = golden * i as f32;
            Vector3::new(r * phi.cos(), y, r * phi.sin())
        })
        .collect()
}

#[test]
fn spade_is_negative_inside_and_positive_outside() {
    for p in [(0.0, 0.2), (-0.24, -0.1), (0.0, -0.4)] {
        assert!(spade(p) < 0.0, "{p:?} should be inside");
    }
    for p in [(0.6, 0.5), (0.0, 0.8), (0.45, -0.5)] {
        assert!(spade(p) > 0.0, "{p:?} should be outside");
    }
    // The exact triangle distance matches a simple known case.
    let d = triangle((0.0, 2.0), (-1.0, 0.0), (1.0, 0.0), (0.0, 1.0));
    assert!((d - 1.0).abs() < 1e-6);
}

#[test]
fn spade_is_mirror_symmetric() {
    for i in 0..40 {
        for j in 0..40 {
            let p = (-0.8 + i as f32 * 0.04, -0.8 + j as f32 * 0.04);
            assert!((spade(p) - spade((-p.0, p.1))).abs() < 1e-6);
        }
    }
}

#[test]
fn diamond_metric_is_zero_on_its_vertices() {
    let (a, b) = (0.45, 0.6);
    for p in [(a, 0.0), (-a, 0.0), (0.0, b), (0.0, -b)] {
        assert!(diamond_l1(p, a, b).abs() < 1e-6);
    }
    assert!(diamond_l1((0.0, 0.0), a, b) < 0.0);
    assert!(diamond_l1((a, b), a, b) > 0.0);
}

#[test]
fn tangent_frames_are_orthonormal_including_poles() {
    let mut dirs = directions(1000);
    dirs.extend([Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, -1.0, 0.0)]);
    for n in dirs {
        let (t, b) = tangent_frame(n);
        assert!((t.length() - 1.0).abs() < 1e-5);
        assert!((b.length() - 1.0).abs() < 1e-5);
        assert!(t.dot(b).abs() < 1e-5 && t.dot(n).abs() < 1e-5 && b.dot(n).abs() < 1e-5);
    }
}

#[test]
fn local_plane_round_trips() {
    let c = Vector3::new(0.3, 0.5, 0.81).normalized();
    let (e, nn) = tangent_frame(c);
    for n in directions(500) {
        if let Some((u, v)) = local_plane(n, c, e, nn) {
            let back = from_local(c, e, nn, u, v);
            assert!((back - n).length() < 1e-4, "{n:?} -> {back:?}");
        } else {
            assert!(n.dot(c) <= 0.05);
        }
    }
}

#[test]
fn rotations_preserve_length() {
    let axis = Vector3::new(1.0, 2.0, -0.5).normalized();
    for p in directions(200) {
        for angle in [0.3_f32, 1.7, -2.9] {
            assert!((rotate_y(p, angle).length() - 1.0).abs() < 1e-5);
            assert!((rotate_axis(p, axis, angle).length() - 1.0).abs() < 1e-5);
        }
        assert!((rotate_y(p, 0.0) - p).length() < 1e-6);
        // A rotation by the angle and its inverse returns the point.
        let back = rotate_axis(rotate_axis(p, axis, 0.8), axis, -0.8);
        assert!((back - p).length() < 1e-5);
    }
}

#[test]
fn hashed_directions_stay_in_cone_and_change_per_cycle() {
    let focus = Vector3::new(-0.5, 0.4, -0.7).normalized();
    let cone = 65.0_f32.to_radians();
    let mut previous = hash_direction(3, 0, 0, focus, cone);
    let mut moved = 0;
    for k in 0..200 {
        let d = hash_direction(3, 0, k, focus, cone);
        assert!((d.length() - 1.0).abs() < 1e-5);
        assert!(angular_distance(d, focus) <= cone + 1e-4);
        assert_eq!(d, hash_direction(3, 0, k, focus, cone));
        if angular_distance(d, previous) > 0.05 {
            moved += 1;
        }
        previous = d;
    }
    assert!(moved > 190, "{moved}");
    assert_ne!(
        hash_direction(3, 0, 5, focus, cone),
        hash_direction(3, 1, 5, focus, cone)
    );
}
