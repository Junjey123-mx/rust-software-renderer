//! ShipState: basis conventions, stability and the model matrix.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{Uniforms, vertex_shader};
use rust_software_renderer::solar::SolarMeshes;
use rust_software_renderer::solar::ship::{MESH_CENTER, MESH_RADIUS, ShipState};
use std::f32::consts::FRAC_PI_2;

fn close(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-4
}

fn orthonormal(s: &ShipState) -> bool {
    (s.forward.length() - 1.0).abs() < 1e-4
        && (s.up.length() - 1.0).abs() < 1e-4
        && s.forward.dot(s.up).abs() < 1e-4
}

#[test]
fn the_default_basis_has_the_nose_on_minus_z_and_the_top_on_plus_y() {
    let s = ShipState::new(Vector3::zero(), 1.0);
    assert!(close(s.forward, Vector3::new(0.0, 0.0, -1.0)));
    assert!(close(s.up, Vector3::new(0.0, 1.0, 0.0)));
    assert!(close(s.right(), Vector3::new(1.0, 0.0, 0.0)), "right is +X");
}

#[test]
fn positive_yaw_turns_the_nose_towards_minus_x_and_pitch_lifts_it() {
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    s.yaw(FRAC_PI_2);
    assert!(
        close(s.forward, Vector3::new(-1.0, 0.0, 0.0)),
        "{:?}",
        s.forward
    );
    assert!(close(s.up, Vector3::new(0.0, 1.0, 0.0)));
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    s.pitch(0.4);
    assert!(s.forward.y > 0.3 && s.forward.z < 0.0, "{:?}", s.forward);
    assert!(s.up.z > 0.0, "the top tilts backwards (+Z): {:?}", s.up);
    assert!(orthonormal(&s));
}

#[test]
fn ten_thousand_turns_keep_the_basis_orthonormal() {
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    for i in 0..10_000 {
        s.yaw(0.013 * ((i % 7) as f32 - 3.0));
        s.pitch(0.011 * ((i % 5) as f32 - 2.0));
    }
    assert!(orthonormal(&s));
    assert!((s.right().length() - 1.0).abs() < 1e-4);
}

#[test]
fn renormalize_recovers_from_a_degenerate_basis() {
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    s.up = s.forward;
    s.renormalize();
    assert!(orthonormal(&s));
    s.forward = Vector3::zero();
    s.renormalize();
    assert!(orthonormal(&s));
}

#[test]
fn the_model_matrix_places_the_mesh_center_at_the_position_and_the_nose_ahead() {
    let mut s = ShipState::new(Vector3::new(5.0, 1.0, -2.0), 0.1);
    s.yaw(FRAC_PI_2);
    let apply = |p: Vector3| {
        let u = Uniforms {
            model_matrix: s.model_matrix(),
        };
        vertex_shader(&Vertex::new(p), &u).transformed_position
    };
    assert!(close(apply(MESH_CENTER), s.position));
    // The measured nose tip of the OBJ, 3.1 ahead of the center.
    let nose = apply(Vector3::new(0.0, 0.0, -4.4));
    let expected = s.position + s.forward * 0.31 + s.up * (-0.255 * 0.1);
    assert!(close(nose, expected), "{nose:?} vs {expected:?}");
    // The real mesh: every corner lies within the scaled radius of the position.
    let meshes = SolarMeshes::load().unwrap();
    let far = meshes
        .ship
        .iter()
        .map(|v| (apply(v.position) - s.position).length())
        .fold(0.0, f32::max);
    assert!(far <= MESH_RADIUS * 0.1 + 1e-3, "{far}");
    assert!((s.radius() - 0.399).abs() < 1e-5);
}

#[test]
fn object_direction_mapping_inverts_the_basis() {
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    s.yaw(0.8);
    s.pitch(-0.3);
    assert!(close(
        s.to_object_dir(s.forward),
        Vector3::new(0.0, 0.0, -1.0)
    ));
    assert!(close(s.to_object_dir(s.up), Vector3::new(0.0, 1.0, 0.0)));
    assert!(close(
        s.to_object_dir(s.right()),
        Vector3::new(1.0, 0.0, 0.0)
    ));
}
