//! `Transform`: model matrix composition and inverse direction mapping.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{Transform, Uniforms, vertex_shader};
use std::f32::consts::FRAC_PI_2;

fn close(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-5
}

fn apply(t: &Transform, p: Vector3) -> Vector3 {
    let uniforms = Uniforms {
        model_matrix: t.matrix(),
    };
    vertex_shader(&Vertex::new(p), &uniforms).transformed_position
}

#[test]
fn identity_leaves_points_in_place() {
    let t = Transform::default();
    let p = Vector3::new(1.0, -2.0, 3.0);
    assert!(close(apply(&t, p), p));
    assert!(close(t.to_object_dir(p), p));
}

#[test]
fn translation_moves_the_origin_and_scale_doubles_lengths() {
    let t = Transform::new(Vector3::new(4.0, 5.0, 6.0), 2.0);
    assert!(close(
        apply(&t, Vector3::zero()),
        Vector3::new(4.0, 5.0, 6.0)
    ));
    assert!(close(
        apply(&t, Vector3::new(1.0, 0.0, 0.0)),
        Vector3::new(6.0, 5.0, 6.0)
    ));
    // Directions ignore translation and scale.
    assert!(close(
        t.to_object_dir(Vector3::new(0.0, 1.0, 0.0)),
        Vector3::new(0.0, 1.0, 0.0)
    ));
    assert!(close(
        t.to_object_point(Vector3::new(6.0, 5.0, 6.0)),
        Vector3::new(1.0, 0.0, 0.0)
    ));
}

#[test]
fn positive_yaw_turns_the_nose_from_minus_z_towards_minus_x() {
    let t = Transform {
        yaw: FRAC_PI_2,
        ..Transform::default()
    };
    let nose = Vector3::new(0.0, 0.0, -1.0);
    assert!(close(apply(&t, nose), Vector3::new(-1.0, 0.0, 0.0)));
    assert!(close(t.to_world_dir(nose), Vector3::new(-1.0, 0.0, 0.0)));
}

#[test]
fn positive_pitch_lifts_the_nose() {
    let t = Transform {
        pitch: 0.3,
        ..Transform::default()
    };
    let nose = apply(&t, Vector3::new(0.0, 0.0, -1.0));
    assert!(nose.y > 0.0 && nose.z < 0.0, "{nose:?}");
    assert!((nose.length() - 1.0).abs() < 1e-5);
}

#[test]
fn object_direction_is_the_inverse_of_world_direction() {
    let t = Transform {
        position: Vector3::new(1.0, 2.0, 3.0),
        yaw: 0.7,
        pitch: -0.4,
        roll: 1.1,
        scale: 3.0,
    };
    for d in [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.3, -0.5, 0.8).normalized(),
    ] {
        let world = t.to_world_dir(d);
        assert!((world.length() - 1.0).abs() < 1e-5, "rotation keeps length");
        assert!(close(t.to_object_dir(world), d), "{d:?}");
        // The matrix applied to a point agrees with the direction map plus
        // translation and scale.
        assert!(close(apply(&t, d), world * 3.0 + t.position));
        assert!(close(t.to_object_point(apply(&t, d)), d));
    }
}

#[test]
fn validation_rejects_bad_scale_and_non_finite_values() {
    assert!(Transform::default().validate().is_ok());
    assert!(Transform::new(Vector3::zero(), 0.0).validate().is_err());
    assert!(Transform::new(Vector3::zero(), -1.0).validate().is_err());
    assert!(
        Transform::new(Vector3::zero(), f32::NAN)
            .validate()
            .is_err()
    );
    let bad = Transform {
        yaw: f32::INFINITY,
        ..Transform::default()
    };
    assert!(bad.validate().is_err());
}
