use raylib::math::{Matrix, Vector3};
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{Uniforms, vertex_shader};

fn close(a: Vector3, b: Vector3) -> bool {
    (a.x - b.x).abs() < 1e-5 && (a.y - b.y).abs() < 1e-5 && (a.z - b.z).abs() < 1e-5
}

#[test]
fn identity_preserves_position() {
    let v = Vertex::new(Vector3::new(1.0, -2.0, 3.5));
    let out = vertex_shader(&v, &Uniforms::default());
    assert_eq!(out.transformed_position, v.position);
    assert_eq!(out.position, v.position);
}

#[test]
fn translation_moves_only_transformed_position() {
    let red = Vector3::new(1.0, 0.0, 0.0);
    let v = Vertex::new_with_color(Vector3::new(1.0, 2.0, 3.0), red);
    let u = Uniforms {
        model_matrix: Matrix::translate(10.0, -5.0, 0.5),
    };
    let out = vertex_shader(&v, &u);
    assert!(close(
        out.transformed_position,
        Vector3::new(11.0, -3.0, 3.5)
    ));
    assert_eq!(out.position, Vector3::new(1.0, 2.0, 3.0), "original kept");
    assert_eq!(out.color, red, "color kept");
}

#[test]
fn scale_does_not_swap_axes() {
    let v = Vertex::new(Vector3::new(1.0, 2.0, 3.0));
    let u = Uniforms {
        model_matrix: Matrix::scale(2.0, 3.0, 4.0),
    };
    let out = vertex_shader(&v, &u);
    assert!(close(
        out.transformed_position,
        Vector3::new(2.0, 6.0, 12.0)
    ));
}

#[test]
fn rotation_about_z_by_90_degrees() {
    let v = Vertex::new(Vector3::new(1.0, 0.0, 0.0));
    let u = Uniforms {
        model_matrix: Matrix::rotate_z(std::f32::consts::FRAC_PI_2),
    };
    let out = vertex_shader(&v, &u);
    assert!(
        close(out.transformed_position, Vector3::new(0.0, 1.0, 0.0)),
        "{:?}",
        out.transformed_position
    );
}

#[test]
fn shader_ignores_previous_transformed_position() {
    let mut v = Vertex::new(Vector3::new(1.0, 1.0, 1.0));
    v.transformed_position = Vector3::new(99.0, 99.0, 99.0);
    let out = vertex_shader(&v, &Uniforms::default());
    assert_eq!(out.transformed_position, Vector3::new(1.0, 1.0, 1.0));
}
