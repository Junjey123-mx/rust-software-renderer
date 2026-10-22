//! Headless end-to-end render of the real ship through the CPU pipeline.

use raylib::math::Vector3;
use rust_software_renderer::config::AppConfig;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{Projection, Uniforms, render};
use rust_software_renderer::rendering::Framebuffer;

fn render_ship() -> (
    Framebuffer,
    rust_software_renderer::pipeline::RenderStats,
    AppConfig,
) {
    let config = AppConfig::default();
    let mesh = load_obj(&config.model_path).expect("default model path must load");
    let vertex_array = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    let mut fb = Framebuffer::new(config.width, config.height);
    fb.set_background_color(Vector3::new(0.05, 0.05, 0.08));
    fb.clear();
    let stats = render(
        &mut fb,
        &Uniforms::default(),
        &vertex_array,
        Projection::Fit {
            plane: config.view_plane,
            margin_ratio: config.margin_ratio,
        },
    )
    .unwrap();
    (fb, stats, config)
}

fn lit_bounds(fb: &Framebuffer) -> (i32, i32, i32, i32, usize) {
    let (mut x0, mut y0, mut x1, mut y1, mut n) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN, 0);
    for y in 0..fb.height() as i32 {
        for x in 0..fb.width() as i32 {
            if fb.get_pixel(x, y) != Some(fb.background_color()) {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
                n += 1;
            }
        }
    }
    (x0, y0, x1, y1, n)
}

#[test]
fn all_708_triangles_are_processed() {
    let (_, stats, _) = render_ship();
    assert_eq!(stats.vertices_processed, 2124);
    assert_eq!(stats.triangles_processed, 708);
    assert!(stats.fragments_generated > 708 * 3, "{stats:?}");
}

#[test]
fn ship_is_visible_centered_and_inside_margins() {
    let (fb, stats, config) = render_ship();
    let (x0, y0, x1, y1, lit) = lit_bounds(&fb);

    assert!(lit > 1000, "only {lit} pixels differ from the background");
    assert!(lit <= stats.fragments_generated);

    let (w, h) = (config.width as i32, config.height as i32);
    let (mx, my) = (
        (w as f32 * config.margin_ratio) as i32,
        (h as f32 * config.margin_ratio) as i32,
    );
    assert!(
        x0 >= mx && x1 <= w - mx,
        "x range {x0}..{x1} leaves the margin"
    );
    assert!(
        y0 >= my && y1 <= h - my,
        "y range {y0}..{y1} leaves the margin"
    );

    // Fills most of the available height (XZ fit is height-limited).
    assert!(
        y1 - y0 >= (h - 2 * my) * 9 / 10,
        "model too small: {}",
        y1 - y0
    );
    assert!(x1 - x0 >= 300, "model too narrow: {}", x1 - x0);

    let (cx, cy) = ((x0 + x1) / 2, (y0 + y1) / 2);
    assert!((cx - w / 2).abs() <= 2, "center x = {cx}");
    assert!((cy - h / 2).abs() <= 2, "center y = {cy}");
}

#[test]
fn ship_pixels_use_the_vertex_color() {
    let (fb, _, _) = render_ship();
    let mut white = 0;
    for y in 0..fb.height() as i32 {
        for x in 0..fb.width() as i32 {
            if fb.get_pixel(x, y) == Some(Vertex::DEFAULT_COLOR) {
                white += 1;
            }
        }
    }
    let (_, _, _, _, lit) = lit_bounds(&fb);
    assert_eq!(white, lit, "every lit pixel is the wireframe color");
}
