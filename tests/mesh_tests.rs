use raylib::math::Vector3;
use rust_software_renderer::geometry::{Mesh, MeshError, Vertex};

fn positions(n: usize) -> Vec<Vector3> {
    (0..n).map(|i| Vector3::new(i as f32, 0.0, 0.0)).collect()
}

#[test]
fn single_face_is_one_triangle() {
    let mesh = Mesh::new(positions(3), vec![0, 1, 2]).unwrap();
    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices.len(), 3);
    assert_eq!(mesh.triangle_count(), 1);
}

#[test]
fn two_faces_can_share_positions() {
    // Four positions, two triangles, six references.
    let mesh = Mesh::new(positions(4), vec![0, 1, 2, 0, 2, 3]).unwrap();
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices.len(), 6);
    assert_eq!(mesh.triangle_count(), 2);
}

#[test]
fn index_order_is_preserved() {
    let mesh = Mesh::new(positions(3), vec![2, 0, 1]).unwrap();
    assert_eq!(mesh.indices, vec![2, 0, 1]);
}

#[test]
fn index_out_of_range_is_rejected() {
    let err = Mesh::new(positions(3), vec![0, 1, 3]).unwrap_err();
    assert_eq!(
        err,
        MeshError::IndexOutOfRange {
            index: 3,
            vertex_count: 3
        }
    );
}

#[test]
fn incomplete_face_is_rejected() {
    let err = Mesh::new(positions(3), vec![0, 1, 2, 0]).unwrap_err();
    assert_eq!(err, MeshError::IndexCountNotMultipleOfThree(4));
}

#[test]
fn mesh_without_faces_is_rejected() {
    assert_eq!(
        Mesh::new(positions(3), vec![]).unwrap_err(),
        MeshError::Empty
    );
}

#[test]
fn vertex_keeps_original_and_transformed_positions() {
    let p = Vector3::new(1.0, 2.0, 3.0);
    let v = Vertex::new(p);
    assert_eq!(v.position, p);
    assert_eq!(v.transformed_position, p);
    assert_eq!(v.color, Vector3::new(1.0, 1.0, 1.0));

    let red = Vertex::new_with_color(p, Vector3::new(1.0, 0.0, 0.0));
    assert_eq!(red.color, Vector3::new(1.0, 0.0, 0.0));
}
