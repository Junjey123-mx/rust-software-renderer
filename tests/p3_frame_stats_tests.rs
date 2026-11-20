//! Project 3 frame counters: `SolarScene::last_stats` reports the real
//! objects, triangles and fragments of the last `render_frame`, checked on
//! controlled scenes and against the instances drawn one by one.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::{CameraMode, FrameStats, SolarMeshes, SolarScene};

const W: u32 = 160;
const H: u32 = 120;

fn view() -> OrbitView {
    OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(W, H, 0.1),
    )
    .unwrap()
}

fn system() -> SolarScene {
    let mut s = SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view(),
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    s.clock.time = 6.0;
    s.update();
    s
}

/// A scene with no bodies (so no loops, crown or cards) and the thruster
/// flames off: only the ship mesh.
fn ship_only() -> SolarScene {
    let mut s = SolarScene::new(
        SolarMeshes::load().unwrap(),
        view(),
        Vector3::new(0.0, 1.0, 0.0),
    );
    s.thrusters = None;
    s
}

fn render(scene: &SolarScene, mode: TriangleMode) -> FrameStats {
    let mut fb = Framebuffer::new(W, H);
    let options = ShadedOptions {
        threads: 2,
        ..ShadedOptions::default()
    };
    scene.render_frame(&mut fb, mode, options).unwrap();
    scene.last_stats().clone()
}

fn ship_triangles(scene: &SolarScene) -> usize {
    scene.meshes.ship.len() / 3
}

#[test]
fn empty_scene_counts_zero() {
    // The ship far outside the orthographic orbit view: nothing is drawn.
    let mut s = ship_only();
    s.camera_mode = CameraMode::Orbit;
    s.ship.position = Vector3::new(500.0, 0.0, 0.0);
    let stats = render(&s, TriangleMode::FilledScanline);
    assert_eq!(
        stats,
        FrameStats {
            objects_tested: 1,
            objects_culled: 1,
            ..FrameStats::default()
        }
    );
}

#[test]
fn ship_alone_counts_708_triangles() {
    let s = ship_only();
    assert_eq!(ship_triangles(&s), 708, "the Laboratorio3.obj ship changed");
    for mode in [TriangleMode::FilledScanline, TriangleMode::Wireframe] {
        let stats = render(&s, mode);
        assert_eq!(
            (stats.objects_tested, stats.objects_culled, stats.draws),
            (1, 0, 1)
        );
        assert_eq!(stats.triangles_submitted, 708, "{mode:?}");
        assert!(stats.per_body.is_empty());
        assert!(stats.fragments > 0, "{mode:?}");
    }
}

#[test]
fn ship_facing_the_void_draws_no_body() {
    // Follow camera behind a ship that turned its back on the system.
    // The thruster flames off, so the ship mesh is the only draw.
    let mut s = system();
    s.thrusters = None;
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    let stats = render(&s, TriangleMode::FilledScanline);
    assert_eq!(stats.per_body.len(), s.bodies.len());
    assert!(
        stats
            .per_body
            .iter()
            .all(|b| !b.visible && b.triangles == 0)
    );
    assert_eq!(stats.draws, 1, "only the ship");
    assert_eq!(stats.triangles_submitted, ship_triangles(&s));
    // Bodies and the plasma loops were tested and culled; the ship drawn.
    assert_eq!(stats.objects_culled, stats.objects_tested - 1);
}

#[test]
fn totals_equal_sum_of_parts() {
    // The batch of `render_frame` against the same instances drawn one by
    // one: same draws and triangles.
    let mut s = system();
    s.camera_mode = CameraMode::Orbit;
    let stats = render(&s, TriangleMode::FilledScanline);
    let mut fb = Framebuffer::new(W, H);
    let (mode, options) = (TriangleMode::FilledScanline, ShadedOptions::default());
    let bodies = s
        .render_objects_without_clear(&mut fb, mode, options)
        .unwrap();
    let plasma = s
        .render_plasma_without_clear(&mut fb, mode, options)
        .unwrap();
    let effects = s
        .render_effects_without_clear(&mut fb, mode, options)
        .unwrap();
    let ship = s.render_ship_without_clear(&mut fb, mode, options).unwrap();
    let parts: Vec<_> = bodies
        .iter()
        .filter(|b| b.triangles_processed > 0)
        .chain(&plasma)
        .chain(&effects)
        .chain([&ship])
        .collect();
    assert_eq!(stats.draws, parts.len());
    let sum = |f: fn(&&rust_software_renderer::pipeline::ShadedStats) -> usize| {
        parts.iter().map(f).sum::<usize>()
    };
    assert_eq!(stats.triangles_submitted, sum(|p| p.triangles_processed));
    assert_eq!(stats.triangles_culled, sum(|p| p.triangles_culled));
    let per_body: usize = stats.per_body.iter().map(|b| b.triangles).sum();
    let sphere = s.meshes.sphere.len() / 3;
    let visible = stats.per_body.iter().filter(|b| b.visible).count();
    assert_eq!(per_body, visible * sphere);
    assert!(visible > 0 && stats.fragments > 0);
}

#[test]
fn counters_are_consistent_and_reset_each_frame() {
    let s = system();
    let first = render(&s, TriangleMode::FilledScanline);
    assert!(first.objects_culled <= first.objects_tested);
    assert!(first.triangles_culled <= first.triangles_submitted);
    assert_eq!(first.per_body.len(), s.bodies.len());
    // Rendering the same frame again gives the same counters, not twice.
    assert_eq!(render(&s, TriangleMode::FilledScanline), first);
}
