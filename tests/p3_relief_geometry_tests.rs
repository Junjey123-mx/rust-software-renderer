//! Project 3 relief geometry: the height field moves the icosphere's
//! vertices off the base radius, keeps its topology and winding, and the
//! levels of detail sample the same directions.

use raylib::math::Vector3;
use rust_software_renderer::solar::planet::geometry::relief_mesh;
use rust_software_renderer::solar::planet::icosphere::icosphere;
use rust_software_renderer::solar::planet::{HeightField, TestField};

const R: f32 = 0.866;

fn field() -> HeightField {
    HeightField::Test(TestField {
        amplitude: 0.1,
        frequency: 7.0,
        sea_level: None,
    })
}

#[test]
fn vertices_leave_base_radius() {
    let mesh = relief_mesh(3, R, &field()).unwrap();
    let off = mesh
        .iter()
        .filter(|v| (v.position.length() - R).abs() > 1e-3)
        .count();
    assert!(off > mesh.len() / 2, "{off} of {}", mesh.len());
    for v in &mesh {
        let n = v.position.normalized();
        assert!((v.position.length() - R * field().surface_radius(n)).abs() < 1e-5);
    }
}

#[test]
fn flat_field_equals_sphere() {
    for v in relief_mesh(4, R, &HeightField::Flat).unwrap() {
        assert!((v.position.length() - R).abs() < 1e-6);
    }
}

#[test]
fn same_topology_as_icosphere() {
    for level in 2..=4 {
        let ico = icosphere(level).unwrap();
        let mesh = relief_mesh(level, R, &field()).unwrap();
        assert_eq!(mesh.len(), 3 * ico.indices.len());
        for (tri, idx) in mesh.chunks(3).zip(&ico.indices) {
            for (v, &i) in tri.iter().zip(idx) {
                let dir = ico.positions[i as usize];
                assert!(v.position.normalized().dot(dir) > 0.99999);
            }
            // Outward winding kept for a gentle relief.
            let (a, b, c) = (tri[0].position, tri[1].position, tri[2].position);
            assert!((b - a).cross(c - a).dot(a + b + c) > 0.0);
        }
    }
    assert!(relief_mesh(6, R, &field()).is_err());
}

#[test]
fn lod_levels_share_directions() {
    // Every vertex of a level is a vertex of the next one, at the same
    // height: coarse and fine levels agree where they meet.
    for level in 2..5 {
        let coarse = icosphere(level).unwrap();
        let fine = icosphere(level + 1).unwrap();
        assert_eq!(
            &fine.positions[..coarse.positions.len()],
            &coarse.positions[..]
        );
    }
    let f = field();
    let coarse = relief_mesh(2, R, &f).unwrap();
    let fine = relief_mesh(3, R, &f).unwrap();
    let fine_points: Vec<Vector3> = fine.iter().map(|v| v.position).collect();
    for v in &coarse {
        assert!(
            fine_points
                .iter()
                .any(|p| (*p - v.position).length() < 1e-6),
            "{:?}",
            v.position
        );
    }
}
