//! Project 3 HUD: the on-screen counters are the text of the frame's
//! `FrameStats`, painted only inside their top-left box, and nothing is
//! painted when the HUD is off.

use raylib::consts::KeyboardKey;
use raylib::math::Vector3;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ShadedStats, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::hud::{draw_hud, hud_box, hud_lines};
use rust_software_renderer::solar::input::{PRESSED, SolarAction};
use rust_software_renderer::solar::{FrameStats, SolarMeshes, SolarScene};

const W: u32 = 400;
const H: u32 = 300;

fn rendered(scene: &SolarScene) -> Framebuffer {
    let mut fb = Framebuffer::new(W, H);
    let options = ShadedOptions {
        threads: 2,
        ..ShadedOptions::default()
    };
    scene
        .render_frame(&mut fb, TriangleMode::FilledScanline, options)
        .unwrap();
    fb
}

fn system() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(W, H, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    s.clock.time = 6.0;
    s.update();
    s
}

#[test]
fn lines_match_frame_stats() {
    let mut stats = FrameStats::default();
    let draw = |t, c, f| ShadedStats {
        triangles_processed: t,
        triangles_culled: c,
        fragments_generated: f,
        ..ShadedStats::default()
    };
    stats.body(0, true, &draw(192, 90, 1000));
    stats.body(1, false, &ShadedStats::default());
    stats.test(true);
    stats.add(&draw(708, 300, 2345));
    let lines = hud_lines(&stats, 59.6);
    assert_eq!(
        lines,
        [
            "FPS 60",
            "OBJECTS 3 CULLED 1",
            "BODIES 1/2 TRIS 192",
            "DRAWS 2 TRIS 900 CULLED 390",
            "FRAGMENTS 3345",
        ]
    );
}

#[test]
fn hud_reads_the_last_frame_of_the_scene() {
    // A ship that turned its back on the system: no body triangles.
    let mut s = system();
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    rendered(&s);
    let lines = hud_lines(&s.last_stats(), 30.0);
    assert_eq!(lines[2], format!("BODIES 0/{} TRIS 0", s.bodies.len()));
    let tris = s.last_stats().triangles_submitted;
    assert!(lines[3].contains(&format!("TRIS {tris} ")), "{lines:?}");
}

#[test]
fn hud_off_leaves_framebuffer_identical() {
    let s = system();
    let mut fb = rendered(&s);
    let before = fb.to_rgba_bytes();
    draw_hud(&mut fb, &[]);
    assert_eq!(fb.to_rgba_bytes(), before);
}

#[test]
fn hud_on_changes_only_top_left_box() {
    let s = system();
    let mut fb = rendered(&s);
    let before = fb.clone();
    let lines = hud_lines(&s.last_stats(), 30.0);
    draw_hud(&mut fb, &lines);
    let (bx, by, bw, bh) = hud_box(&lines);
    assert!(bx + bw <= W as i32 && by + bh <= H as i32, "box fits");
    let mut changed = 0;
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if fb.get_pixel(x, y) != before.get_pixel(x, y) {
                assert!(
                    (bx..bx + bw).contains(&x) && (by..by + bh).contains(&y),
                    "({x}, {y}) outside the box"
                );
                changed += 1;
            }
        }
    }
    assert!(changed > 0);
}

#[test]
fn h_toggles_the_hud() {
    assert!(PRESSED.contains(&(KeyboardKey::KEY_H, SolarAction::ToggleHud)));
}
