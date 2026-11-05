//! The instructor's sphere is versioned byte for byte and shared by the three
//! celestial bodies: 98 unique vertices, 192 triangles, 288 unique edges and
//! 576 triangle corners.

use raylib::math::Vector3;
use rust_software_renderer::config::SPHERE_MODEL_PATH;
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::face_normal;
use std::collections::HashMap;
use std::path::Path;

fn sphere() -> Mesh {
    load_obj(Path::new(SPHERE_MODEL_PATH)).expect("course sphere loads")
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn faces(mesh: &Mesh) -> impl Iterator<Item = [usize; 3]> + '_ {
    mesh.indices.chunks_exact(3).map(|f| [f[0], f[1], f[2]])
}

#[test]
fn sphere_file_is_the_unmodified_course_asset() {
    let bytes = std::fs::read(SPHERE_MODEL_PATH).unwrap();
    assert_eq!(bytes.len(), 16_423);
    assert_eq!(fnv1a64(&bytes), 0xf05526000e2b7420);
}

#[test]
fn sphere_loads_98_positions_and_192_triangles() {
    let mesh = sphere();
    assert_eq!(mesh.vertices.len(), 98, "unique vertices");
    assert_eq!(mesh.indices.len(), 576, "triangle corners");
    assert_eq!(mesh.triangle_count(), 192);
}

#[test]
fn sphere_is_centered_with_expected_radii() {
    let mesh = sphere();
    for axis in 0..3 {
        let get = |p: &Vector3| [p.x, p.y, p.z][axis];
        let min = mesh.vertices.iter().map(get).fold(f32::INFINITY, f32::min);
        let max = mesh
            .vertices
            .iter()
            .map(get)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min + 0.839506).abs() < 1e-6, "axis {axis} min {min}");
        assert!((max - 0.839506).abs() < 1e-6, "axis {axis} max {max}");
    }
    for p in &mesh.vertices {
        let r = p.length();
        assert!((0.8395..=0.8661).contains(&r), "radius {r}");
    }
}

#[test]
fn every_sphere_face_points_outward() {
    let mesh = sphere();
    for [a, b, c] in faces(&mesh) {
        let (pa, pb, pc) = (mesh.vertices[a], mesh.vertices[b], mesh.vertices[c]);
        let n = face_normal(pa, pb, pc).expect("non-degenerate face");
        let centroid = (pa + pb + pc) / 3.0;
        assert!(n.dot(centroid) > 0.0);
    }
}

#[test]
fn sphere_is_a_closed_manifold() {
    let mesh = sphere();
    let mut edges: HashMap<(usize, usize), usize> = HashMap::new();
    for [a, b, c] in faces(&mesh) {
        for (u, v) in [(a, b), (b, c), (c, a)] {
            *edges.entry((u.min(v), u.max(v))).or_default() += 1;
        }
    }
    assert_eq!(edges.len(), 288, "unique edges");
    assert!(edges.values().all(|&n| n == 2), "each edge shared twice");
    let euler = mesh.vertices.len() as i64 - edges.len() as i64 + mesh.triangle_count() as i64;
    assert_eq!(euler, 2);
}
