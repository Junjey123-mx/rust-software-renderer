//! Per-instance shader uniforms in object space.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::Transform;
use rust_software_renderer::solar::instance_uniforms;
use std::f32::consts::FRAC_PI_2;

fn close(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-5
}

#[test]
fn identity_instance_keeps_world_directions() {
    let light = Vector3::new(0.0, 0.0, 1.0);
    let eye = Vector3::new(1.0, 0.0, 0.0);
    let u = instance_uniforms(&Transform::default(), 4.5, light, eye);
    assert_eq!(u.time, 4.5);
    assert!(close(u.light_direction, light));
    assert!(close(u.eye_direction, eye));
}

#[test]
fn rotated_instance_receives_directions_in_its_own_space() {
    // Yaw +90 degrees turns object -Z to world -X, so world -X is object -Z.
    let t = Transform {
        yaw: FRAC_PI_2,
        ..Transform::default()
    };
    let u = instance_uniforms(
        &t,
        0.0,
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    assert!(
        close(u.light_direction, Vector3::new(0.0, 0.0, -1.0)),
        "{:?}",
        u.light_direction
    );
    assert!(close(u.eye_direction, Vector3::new(0.0, 1.0, 0.0)));
}

#[test]
fn light_towards_the_star_is_unit_and_translation_independent() {
    let planet = Transform::new(Vector3::new(3.0, 0.0, 4.0), 0.6);
    let light_world = Vector3::zero() - planet.position;
    let u = instance_uniforms(&planet, 1.0, light_world, Vector3::new(0.0, 0.0, 1.0));
    assert!((u.light_direction.length() - 1.0).abs() < 1e-5);
    assert!(close(u.light_direction, Vector3::new(-0.6, 0.0, -0.8)));
}

#[test]
fn zero_direction_stays_zero_instead_of_nan() {
    let u = instance_uniforms(&Transform::default(), 0.0, Vector3::zero(), Vector3::zero());
    assert_eq!(u.light_direction, Vector3::zero());
    assert_eq!(u.eye_direction, Vector3::zero());
}
