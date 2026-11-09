//! Shared meshes of the solar scene.

use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes};
use std::sync::Arc;

#[test]
fn sphere_and_ship_expand_to_the_known_corner_counts() {
    let m = SolarMeshes::load().unwrap();
    assert_eq!(m.sphere.len(), 576, "192 triangles of the course sphere");
    assert_eq!(m.ship.len(), 2124, "708 triangles of the spacecraft");
    assert!(m.sphere.len().is_multiple_of(3) && m.ship.len().is_multiple_of(3));
}

#[test]
fn clones_share_the_same_geometry_instead_of_copying_it() {
    let m = SolarMeshes::load().unwrap();
    let a = m.clone();
    let b = m.clone();
    assert!(Arc::ptr_eq(&m.sphere, &a.sphere) && Arc::ptr_eq(&a.sphere, &b.sphere));
    assert!(Arc::ptr_eq(&m.ship, &b.ship));
    assert_eq!(Arc::strong_count(&m.sphere), 3);
}

#[test]
fn shared_corners_keep_their_original_positions() {
    let m = SolarMeshes::load().unwrap();
    for v in m.sphere.iter() {
        assert_eq!(v.position, v.transformed_position);
        let r = v.position.length();
        assert!((0.83..=SPHERE_RADIUS + 1e-4).contains(&r), "radius {r}");
    }
}
