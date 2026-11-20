//! Project 3 culling savings: deterministic cases where the view test
//! saves real work (G faces empty space, G3 turns the camera 90 degrees)
//! and never removes visible geometry (G2 cuts a planet with the screen
//! edge).

use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{CULLING_SCENARIOS, scenario_scene};
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::{FrameStats, SolarScene};

const MODE: TriangleMode = TriangleMode::FilledScanline;

fn options() -> ShadedOptions {
    ShadedOptions {
        threads: 4,
        ..ShadedOptions::default()
    }
}

fn scene(id: &str) -> SolarScene {
    scenario_scene(id, &SolarBenchConfig::default()).unwrap()
}

fn render(s: &SolarScene) -> (Framebuffer, FrameStats) {
    let mut fb = Framebuffer::new(800, 600);
    s.render_frame(&mut fb, MODE, options()).unwrap();
    (fb, s.last_stats().clone())
}

#[test]
fn the_scenarios_exist_beside_a_to_j() {
    let ids: Vec<&str> = CULLING_SCENARIOS.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ["G2", "G3"]);
}

#[test]
fn g_culls_every_body() {
    let s = scene("G");
    let (_, stats) = render(&s);
    assert_eq!(stats.visible_bodies(), 0);
    // Every body and the plasma loops culled; only the ship drawn.
    assert_eq!(stats.objects_culled, s.bodies.len() + 1);
    assert_eq!(stats.objects_tested, stats.objects_culled + 1);
    let ship = s.meshes.ship.len() / 3;
    if ship == 708 {
        assert!(stats.triangles_submitted >= 708);
    }
}

#[test]
fn partially_visible_planet_is_kept() {
    let s = scene("G2");
    let (with, stats) = render(&s);
    assert!(stats.per_body[1].visible, "the cut planet must be drawn");
    // Its pixels reach the first column: it is really cut by the edge.
    let mut without = s.clone();
    without.bodies.remove(1);
    let (without, _) = render(&without);
    let differs = |x: i32| (0..600).any(|y| with.get_pixel(x, y) != without.get_pixel(x, y));
    assert!(differs(0), "the planet touches the left edge");
    assert!(!differs(799));
    // And the view test changes no pixel.
    let mut off = s.clone();
    off.culling = false;
    assert_eq!(render(&off).0.to_rgba_bytes(), with.to_rgba_bytes());
}

#[test]
fn rotated_camera_culls_offscreen() {
    let s = scene("G3");
    let (on, stats) = render(&s);
    assert!(stats.objects_culled >= 2, "{stats:?}");
    let mut off = s.clone();
    off.culling = false;
    let (off_fb, off_stats) = render(&off);
    assert_eq!(on.to_rgba_bytes(), off_fb.to_rgba_bytes());
    assert!(stats.triangles_submitted < off_stats.triangles_submitted);
}

#[test]
fn culled_bodies_would_produce_zero_fragments() {
    // Drawn anyway (culling off), every body the view test rejects
    // generates no fragment: no false positive in any case.
    for id in ["D", "E", "F", "G", "G2", "G3", "J"] {
        let mut s = scene(id);
        let (_, stats) = render(&s);
        s.culling = false;
        let mut fb = Framebuffer::new(800, 600);
        let drawn = s
            .render_objects_without_clear(&mut fb, MODE, options())
            .unwrap();
        for body in stats.per_body.iter().filter(|b| !b.visible) {
            assert_eq!(drawn[body.index].fragments_generated, 0, "{id}: {body:?}");
        }
    }
}

#[test]
fn sweep_across_the_edge_never_removes_pixels() {
    // The planet of G2 crossing the left edge in small steps: the image is
    // the same with and without the view test at every step.
    let config = SolarBenchConfig {
        width: 400,
        height: 300,
        ..SolarBenchConfig::default()
    };
    for k in 0..12 {
        let mut s = scenario_scene("D", &config).unwrap();
        s.ship.yaw(-0.30 - 0.02 * k as f32);
        let mut on = Framebuffer::new(400, 300);
        s.render_frame(&mut on, MODE, options()).unwrap();
        s.culling = false;
        let mut off = Framebuffer::new(400, 300);
        s.render_frame(&mut off, MODE, options()).unwrap();
        assert_eq!(on.to_rgba_bytes(), off.to_rgba_bytes(), "step {k}");
    }
}
