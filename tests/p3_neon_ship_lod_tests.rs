//! Project 3 neon ships (C64): the player's spacecraft mesh shared through
//! the same `Arc`, the original OBJ files untouched, and a procedural dart
//! under 30 px that cuts the triangles of a far ship.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::solar::SolarMeshes;
use rust_software_renderer::solar::attach::traffic::{DART_PX, NeonShader, NeonShip, dart_mesh};
use rust_software_renderer::solar::ship::{MESH_CENTER, MESH_RADIUS};
use std::sync::Arc;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn asset(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/assets/models/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn shares_player_mesh_arc() {
    let meshes = SolarMeshes::load().unwrap();
    let before = Arc::strong_count(&meshes.ship);
    let ships: Vec<NeonShip> = (0..6).map(|_| NeonShip::new(&meshes, 0.2)).collect();
    for s in &ships {
        assert!(Arc::ptr_eq(&s.mesh, &meshes.ship), "the player's mesh");
        assert!(Arc::ptr_eq(&s.dart, &meshes.dart), "one dart for all");
    }
    // Six more owners of the same vertices, no copies.
    assert_eq!(Arc::strong_count(&meshes.ship), before + 6);
    assert!((ships[0].radius() - 0.2).abs() < 1e-6);
    assert!((ships[0].scale - 0.2 / MESH_RADIUS).abs() < 1e-9);
}

#[test]
fn obj_file_unchanged() {
    // The same fingerprints as before Project 3 (`solar_contract_tests`).
    let ship = asset("Laboratorio3.obj");
    assert_eq!((ship.len(), fnv1a64(&ship)), (42865, 0x89f91b441c62212e));
    let sphere = asset("sphere.obj");
    assert_eq!(
        (sphere.len(), fnv1a64(&sphere)),
        (16423, 0xf05526000e2b7420)
    );
    // Negative case: one more byte in the file changes the fingerprint.
    let mut edited = ship.clone();
    edited.push(b'\n');
    assert_ne!(fnv1a64(&edited), 0x89f91b441c62212e);
}

#[test]
fn far_uses_dart() {
    let meshes = SolarMeshes::load().unwrap();
    let ship = NeonShip::new(&meshes, 0.2);
    assert!(Arc::ptr_eq(ship.lod(DART_PX - 0.1), &meshes.dart));
    assert!(Arc::ptr_eq(ship.lod(DART_PX), &meshes.ship));
    assert!(Arc::ptr_eq(ship.lod(400.0), &meshes.ship));
    // In the spacecraft's frame: nose to -Z, inside its bounding sphere,
    // so one model matrix places either mesh.
    let dart = dart_mesh();
    let nose = dart
        .vertices
        .iter()
        .min_by(|a, b| a.z.total_cmp(&b.z))
        .unwrap();
    assert!(nose.z < MESH_CENTER.z - 3.0 && (nose.x - MESH_CENTER.x).abs() < 1e-6);
    for v in &dart.vertices {
        assert!((*v - MESH_CENTER).length() <= MESH_RADIUS);
    }
    // Closed and wound outwards: positive signed volume.
    let volume: f32 = dart
        .indices
        .chunks(3)
        .map(|t| {
            let [a, b, c] = [0, 1, 2].map(|k| dart.vertices[t[k]] - MESH_CENTER);
            a.dot(b.cross(c)) / 6.0
        })
        .sum();
    assert!(volume > 1.0, "{volume}");
}

#[test]
fn triangles_drop_far() {
    let meshes = SolarMeshes::load().unwrap();
    let triangles = |m: &Arc<Vec<Vertex>>| m.len() / 3;
    let (near, far) = (triangles(&meshes.ship), triangles(&meshes.dart));
    assert_eq!(far, 34);
    assert!(far * 5 < near, "dart {far} vs ship {near}");
    // The neon rim: brighter where the hull turns away from the eye.
    let shader = NeonShader {
        body: Vector3::new(0.1, 0.05, 0.2),
        edge: Vector3::new(1.0, 0.2, 0.6),
        light: Vector3::new(0.0, 1.0, 0.0),
        view: Vector3::new(0.0, 0.0, 1.0),
        fresnel: 2.0,
    };
    let facing = shader.shade(Vector3::new(0.0, 0.0, 1.0));
    let grazing = shader.shade(Vector3::new(1.0, 0.0, 0.0));
    assert!(grazing.x > facing.x + 0.5);
    assert!(facing.x.is_finite() && grazing.x.is_finite());
}
