//! Project 3 relief geometry cache: one set of level-of-detail buffers per
//! radius and field, shared by every instance, built at startup and never
//! while a frame is drawn.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::planet::geometry::{GeometryCache, LOD_LEVELS};
use rust_software_renderer::solar::planet::{HeightField, TestField};
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};
use std::sync::Arc;

fn field(amplitude: f32) -> HeightField {
    HeightField::Test(TestField {
        amplitude,
        frequency: 5.0,
        sea_level: Some(0.0),
    })
}

#[test]
fn same_spec_shares_arc() {
    let mut cache = GeometryCache::default();
    let a = cache.get_or_build(SPHERE_RADIUS, &field(0.05)).unwrap();
    let b = cache.get_or_build(SPHERE_RADIUS, &field(0.05)).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(cache.builds(), 1);
    // Every level is its own mesh, coarse to fine.
    for (mesh, level) in a.iter().zip(LOD_LEVELS) {
        assert_eq!(mesh.len(), 3 * 20 * 4usize.pow(level as u32));
    }
    // A clone of the meshes (one per scene) shares the buffers too.
    let mut copy = cache.clone();
    let c = copy.get_or_build(SPHERE_RADIUS, &field(0.05)).unwrap();
    assert!(Arc::ptr_eq(&a, &c));
    assert_eq!(copy.builds(), 1);
}

#[test]
fn seed_change_rebuilds_only_affected() {
    let mut cache = GeometryCache::default();
    let a = cache.get_or_build(SPHERE_RADIUS, &field(0.05)).unwrap();
    let b = cache.get_or_build(SPHERE_RADIUS, &field(0.08)).unwrap();
    assert_eq!(cache.builds(), 2);
    // Another field parameter (or radius) is another key: only that one is
    // built, the first stays as it was.
    let b2 = cache.get_or_build(SPHERE_RADIUS, &field(0.09)).unwrap();
    let r = cache
        .get_or_build(2.0 * SPHERE_RADIUS, &field(0.05))
        .unwrap();
    assert_eq!(cache.builds(), 4);
    assert!(!Arc::ptr_eq(&b, &b2) && !Arc::ptr_eq(&a, &r));
    assert!(Arc::ptr_eq(
        &a,
        &cache.get_or_build(SPHERE_RADIUS, &field(0.05)).unwrap()
    ));
    assert_eq!(cache.builds(), 4);
}

#[test]
fn no_build_during_render() {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut meshes = SolarMeshes::load().unwrap();
    meshes
        .relief
        .get_or_build(SPHERE_RADIUS, &field(0.05))
        .unwrap();
    let scene = SolarScene::scaled_from_spec(
        meshes,
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap();
    let built = scene.meshes.relief.builds();
    assert_eq!(built, 1, "built at startup");
    let mut fb = Framebuffer::new(80, 60);
    for _ in 0..3 {
        scene
            .render_frame(
                &mut fb,
                TriangleMode::FilledScanline,
                ShadedOptions::default(),
            )
            .unwrap();
    }
    assert_eq!(
        scene.meshes.relief.builds() - built,
        0,
        "no build per frame"
    );
}

#[test]
fn memory_within_budget() {
    // Four relief planets with the same field: one set with the cache,
    // four without it. Either way well under the 64 MB ceiling.
    let mut cache = GeometryCache::default();
    for _ in 0..4 {
        cache.get_or_build(SPHERE_RADIUS, &field(0.05)).unwrap();
    }
    let one = cache.bytes();
    println!(
        "relief LOD 2-5: {one} bytes cached for 4 planets, {} without the cache",
        4 * one
    );
    assert!(4 * one < 64 << 20, "{one}");
}
