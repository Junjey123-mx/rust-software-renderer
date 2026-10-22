//! Integration checks for the real spaceship asset (NaveBimotor).
//!
//! The counts below describe this specific Blender export; they are not
//! assumptions of the generic loader.

use rust_software_renderer::io::load_obj;
use std::path::Path;

const SHIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/models/Laboratorio3.obj"
);

#[test]
fn ship_has_expected_vertex_and_face_counts() {
    let mesh = load_obj(Path::new(SHIP)).unwrap();
    assert_eq!(mesh.vertices.len(), 356);
    assert_eq!(mesh.indices.len(), 2124);
    assert_eq!(mesh.triangle_count(), 708);
    assert_eq!(mesh.indices.len() % 3, 0);
}

#[test]
fn ship_indices_are_all_in_range() {
    let mesh = load_obj(Path::new(SHIP)).unwrap();
    assert!(mesh.indices.iter().all(|&i| i < 356));
    assert_eq!(*mesh.indices.iter().min().unwrap(), 0);
    assert_eq!(*mesh.indices.iter().max().unwrap(), 355);
}

#[test]
fn ship_bounding_box_matches_export() {
    let mesh = load_obj(Path::new(SHIP)).unwrap();
    let (mut min, mut max) = (mesh.vertices[0], mesh.vertices[0]);
    for v in &mesh.vertices {
        min.x = min.x.min(v.x);
        min.y = min.y.min(v.y);
        min.z = min.z.min(v.z);
        max.x = max.x.max(v.x);
        max.y = max.y.max(v.y);
        max.z = max.z.max(v.z);
    }
    let eps = 1e-4;
    assert!((min.x - -2.83).abs() < eps, "min.x = {}", min.x);
    assert!((max.x - 2.83).abs() < eps, "max.x = {}", max.x);
    assert!((min.y - -0.420022).abs() < eps, "min.y = {}", min.y);
    assert!((max.y - 0.93).abs() < eps, "max.y = {}", max.y);
    assert!((min.z - -4.4).abs() < eps, "min.z = {}", min.z);
    assert!((max.z - 1.8).abs() < eps, "max.z = {}", max.z);
}

#[test]
fn ship_expands_to_2124_vertices_and_708_triangles() {
    use rust_software_renderer::geometry::Vertex;
    use rust_software_renderer::pipeline::primitive_assembly;

    let mesh = load_obj(Path::new(SHIP)).unwrap();
    let expanded = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    assert_eq!(expanded.len(), 2124);

    let triangles = primitive_assembly(&expanded).unwrap();
    assert_eq!(triangles.len(), 708);

    // Every assembled corner must equal the position its index points to.
    for (t, tri) in triangles.iter().enumerate() {
        for (k, corner) in tri.iter().enumerate() {
            let index = mesh.indices[t * 3 + k];
            assert_eq!(
                corner.position, mesh.vertices[index],
                "triangle {t} corner {k}"
            );
        }
    }
}
