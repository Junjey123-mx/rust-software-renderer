//! Project 3 mesh level of detail: relief planets drop triangles with
//! distance, the live viewer's hysteresis does not flicker at a bound,
//! recording and benchmarks take the stateless level, and the sphere
//! bodies are drawn as before.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::planet::geometry::{LodState, lod_for, lod_step};
use rust_software_renderer::solar::planet::{HeightField, TestField};
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(320, 240, 0.1),
    )
    .unwrap();
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap()
}

fn field() -> HeightField {
    HeightField::Test(TestField {
        amplitude: 0.04,
        frequency: 6.0,
        sea_level: Some(0.0),
    })
}

fn render(s: &SolarScene) -> Framebuffer {
    render_at(s, 320, 240)
}

fn render_at(s: &SolarScene, w: u32, h: u32) -> Framebuffer {
    let mut fb = Framebuffer::new(w, h);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    fb
}

/// The ship `d` planet radii from the center of body 1, nose towards it.
fn look_at_planet(s: &mut SolarScene, d: f32) {
    let p = s.bodies[1].transform.position;
    s.ship.position = p - Vector3::new(0.0, 0.0, d * s.radius_of(1));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
}

#[test]
fn triangles_drop_with_distance() {
    // Large enough a frame for the finest level (360 px of radius).
    let mut s = scene();
    s.set_relief(1, field()).unwrap();
    let mut counts = Vec::new();
    for d in [1.2, 2.5, 6.0, 25.0, 60.0] {
        look_at_planet(&mut s, d);
        render_at(&s, 1200, 900);
        let body = s.last_stats().per_body[1];
        assert!(body.visible, "d {d}");
        counts.push(body.triangles);
    }
    // Coarser with every step back, and every level really used.
    assert!(counts.windows(2).all(|w| w[0] >= w[1]), "{counts:?}");
    assert_eq!(counts.first(), Some(&20480), "{counts:?}");
    assert_eq!(counts.last(), Some(&320), "{counts:?}");
    let mut distinct = counts.clone();
    distinct.dedup();
    assert_eq!(distinct, [20480, 5120, 1280, 320], "{counts:?}");
}

#[test]
fn hysteresis_no_flicker_at_boundary() {
    // Wavering ±2 px around the 140 px bound.
    let sizes: Vec<f32> = (0..30)
        .map(|k| if k % 2 == 0 { 138.0 } else { 142.0 })
        .collect();
    let mut state = LodState::default();
    lod_step(&mut state, 130.0);
    let levels: Vec<u8> = sizes.iter().map(|&px| lod_step(&mut state, px)).collect();
    let changes = levels.windows(2).filter(|w| w[0] != w[1]).count();
    assert!(changes <= 1, "{levels:?}");
    // Without history the same sizes would flip every frame.
    let stateless: Vec<u8> = sizes.iter().map(|&px| lod_for(px)).collect();
    assert_eq!(stateless.windows(2).filter(|w| w[0] != w[1]).count(), 29);
    // The same in the live scene.
    let mut s = scene();
    s.set_relief(1, field()).unwrap();
    s.lod_live = true;
    s.lod_level(1, 130.0);
    let live: Vec<_> = sizes.iter().map(|&px| s.lod_level(1, px)).collect();
    assert!(live.windows(2).filter(|w| w[0] != w[1]).count() <= 1);
    // Bounds of every level: up at 48/140/360, down below 40/120/300.
    let mut state = LodState::default();
    let path: Vec<u8> = [
        47.0, 48.0, 41.0, 39.0, 360.0, 301.0, 299.0, 121.0, 119.0, 0.0,
    ]
    .iter()
    .map(|&px| lod_step(&mut state, px))
    .collect();
    assert_eq!(path, [2, 3, 3, 2, 5, 5, 4, 4, 3, 2]);
    assert_eq!(lod_step(&mut LodState { level: 9 }, f32::NAN), 2);
}

#[test]
fn record_uses_stateless_lod() {
    // Recording and benchmarks keep `lod_live` off: the level depends on
    // the size of this frame only, whatever came before.
    let mut s = scene();
    s.set_relief(1, field()).unwrap();
    assert!(!s.lod_live);
    s.lod_level(1, 400.0);
    assert_eq!(s.lod_level(1, 320.0), Some(lod_for(320.0)));
    assert_eq!(lod_for(320.0), 4);
    // Two renders of the same instant are the same frame.
    look_at_planet(&mut s, 2.5);
    let a = render(&s).to_rgba_bytes();
    look_at_planet(&mut s, 1.6);
    render(&s);
    look_at_planet(&mut s, 2.5);
    assert_eq!(render(&s).to_rgba_bytes(), a);
}

#[test]
fn lab6_spheres_unchanged() {
    // Bodies without relief (and `Flat`) keep the course sphere.
    let mut plain = scene();
    look_at_planet(&mut plain, 2.5);
    let mut flat = plain.clone();
    for i in 0..flat.bodies.len() {
        flat.set_relief(i, HeightField::Flat).unwrap();
        assert_eq!(flat.lod_level(i, 500.0), None);
    }
    assert_eq!(
        render(&plain).to_rgba_bytes(),
        render(&flat).to_rgba_bytes()
    );
    let stats = flat.last_stats();
    assert!(
        stats
            .per_body
            .iter()
            .all(|b| !b.visible || b.triangles == 192),
        "{:?}",
        stats.per_body
    );
    assert_eq!(flat.meshes.relief.builds(), 0);
    assert!(plain.set_relief(99, field()).is_err());
}
