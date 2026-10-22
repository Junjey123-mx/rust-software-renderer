use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, Projection, RenderSettings, Uniforms, ViewPreset, Viewport,
    model_bounds, render_interactive, render_with_mode, render_with_settings,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use std::path::Path;

const RED: Vector3 = Vector3 {
    x: 1.0,
    y: 0.0,
    z: 0.0,
};
const BLUE: Vector3 = Vector3 {
    x: 0.0,
    y: 0.0,
    z: 1.0,
};

fn tri(color: Vector3, pts: [(f32, f32, f32); 3]) -> Vec<Vertex> {
    pts.iter()
        .map(|&(x, y, z)| Vertex::new_with_color(Vector3::new(x, y, z), color))
        .collect()
}

#[test]
fn point_depth_keeps_the_nearest_and_ignores_invalid_writes() {
    let mut fb = Framebuffer::new(10, 10);
    fb.point_depth(3, 3, RED, 5.0);
    fb.point_depth(3, 3, BLUE, 8.0);
    assert_eq!(fb.get_pixel(3, 3), Some(RED), "farther fragment rejected");
    fb.point_depth(3, 3, BLUE, 2.0);
    assert_eq!(fb.get_pixel(3, 3), Some(BLUE), "nearer fragment wins");
    assert_eq!(fb.get_depth(3, 3), Some(2.0));
    fb.point_depth(3, 3, RED, 2.0);
    assert_eq!(
        fb.get_pixel(3, 3),
        Some(BLUE),
        "ties keep the first fragment"
    );
    fb.point_depth(3, 3, RED, f32::NAN);
    fb.point_depth(3, 3, RED, f32::NEG_INFINITY);
    assert_eq!(fb.get_pixel(3, 3), Some(BLUE));
    fb.point_depth(-1, 3, RED, 0.0);
    fb.point_depth(10, 3, RED, 0.0);
    assert_eq!(fb.count_non_background(), 1);
    assert_eq!(fb.get_depth(-1, 0), None);
}

#[test]
fn clear_resets_color_and_depth_and_legacy_point_ignores_depth() {
    let mut fb = Framebuffer::new(4, 4);
    fb.point_depth(1, 1, RED, 1.0);
    fb.clear();
    assert_eq!(fb.get_pixel(1, 1), Some(Vector3::zero()));
    assert_eq!(fb.get_depth(1, 1), Some(f32::INFINITY));
    fb.point(1, 1, RED);
    fb.point(1, 1, BLUE);
    assert_eq!(
        fb.get_pixel(1, 1),
        Some(BLUE),
        "last write wins without depth"
    );
    assert_eq!(
        fb.get_depth(1, 1),
        Some(f32::INFINITY),
        "legacy writes leave depth untouched"
    );
    fb.point_depth(1, 1, RED, 9.0);
    assert_eq!(fb.get_pixel(1, 1), Some(RED));
}

#[test]
fn crossing_triangles_render_the_same_in_both_draw_orders() {
    // Red is nearer (depth 1) on its right half; blue is nearer on its left.
    let red = tri(
        RED,
        [(10.0, 50.0, 3.0), (90.0, 50.0, 1.0), (50.0, 90.0, 2.0)],
    );
    let blue = tri(
        BLUE,
        [(10.0, 10.0, 1.0), (90.0, 10.0, 3.0), (50.0, 80.0, 2.0)],
    );
    let mut ab = red.clone();
    ab.extend(blue.clone());
    let mut ba = blue.clone();
    ba.extend(red.clone());
    for mode in [
        TriangleMode::FilledBarycentric,
        TriangleMode::FilledScanline,
    ] {
        let settings = RenderSettings { depth_test: true };
        let mut f1 = Framebuffer::new(100, 100);
        let mut f2 = Framebuffer::new(100, 100);
        render_with_settings(
            &mut f1,
            &Uniforms::default(),
            &ab,
            Projection::Screen,
            mode,
            settings,
        )
        .unwrap();
        render_with_settings(
            &mut f2,
            &Uniforms::default(),
            &ba,
            Projection::Screen,
            mode,
            settings,
        )
        .unwrap();
        assert_eq!(
            f1.to_rgba_bytes(),
            f2.to_rgba_bytes(),
            "{mode}: order independent"
        );
        // Without depth testing the order matters (legacy behaviour).
        let mut l1 = Framebuffer::new(100, 100);
        let mut l2 = Framebuffer::new(100, 100);
        render_with_mode(&mut l1, &Uniforms::default(), &ab, Projection::Screen, mode).unwrap();
        render_with_mode(&mut l2, &Uniforms::default(), &ba, Projection::Screen, mode).unwrap();
        assert_ne!(l1.to_rgba_bytes(), l2.to_rgba_bytes());
    }
}

#[test]
fn moving_a_face_closer_changes_the_visible_winner() {
    let red = tri(
        RED,
        [(10.0, 10.0, 5.0), (90.0, 10.0, 5.0), (50.0, 90.0, 5.0)],
    );
    let near_blue = tri(
        BLUE,
        [(10.0, 10.0, 1.0), (90.0, 10.0, 1.0), (50.0, 90.0, 1.0)],
    );
    let far_blue = tri(
        BLUE,
        [(10.0, 10.0, 9.0), (90.0, 10.0, 9.0), (50.0, 90.0, 9.0)],
    );
    let settings = RenderSettings { depth_test: true };
    let mut scene = red.clone();
    scene.extend(near_blue);
    let mut fb = Framebuffer::new(100, 100);
    render_with_settings(
        &mut fb,
        &Uniforms::default(),
        &scene,
        Projection::Screen,
        TriangleMode::FilledScanline,
        settings,
    )
    .unwrap();
    assert_eq!(fb.get_pixel(50, 40), Some(BLUE));
    let mut scene = red;
    scene.extend(far_blue);
    let mut fb = Framebuffer::new(100, 100);
    render_with_settings(
        &mut fb,
        &Uniforms::default(),
        &scene,
        Projection::Screen,
        TriangleMode::FilledScanline,
        settings,
    )
    .unwrap();
    assert_eq!(fb.get_pixel(50, 40), Some(RED));
}

#[test]
fn wireframe_ignores_the_depth_flag_and_draws_every_edge() {
    let red = tri(
        RED,
        [(10.0, 10.0, 5.0), (90.0, 10.0, 5.0), (50.0, 90.0, 5.0)],
    );
    let mut with = Framebuffer::new(100, 100);
    let mut without = Framebuffer::new(100, 100);
    render_with_settings(
        &mut with,
        &Uniforms::default(),
        &red,
        Projection::Screen,
        TriangleMode::Wireframe,
        RenderSettings { depth_test: true },
    )
    .unwrap();
    render_with_mode(
        &mut without,
        &Uniforms::default(),
        &red,
        Projection::Screen,
        TriangleMode::Wireframe,
    )
    .unwrap();
    assert_eq!(with.to_rgba_bytes(), without.to_rgba_bytes());
}

#[test]
fn ship_renders_interactively_in_every_mode_without_panics() {
    let verts = load_obj(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/models/Laboratorio3.obj"
    )))
    .unwrap()
    .expand_vertices(Vertex::DEFAULT_COLOR);
    let b = model_bounds(&verts).unwrap();
    let view = OrbitView::new(
        OrbitCamera::new(b.center, ViewPreset::Oblique),
        &b,
        Viewport::new(800, 600, 0.1),
    )
    .unwrap();
    let mut images = Vec::new();
    for mode in TriangleMode::ALL {
        let mut fb = Framebuffer::new(800, 600);
        let stats = render_interactive(&mut fb, &Uniforms::default(), &verts, view, mode).unwrap();
        assert_eq!(stats.triangles_processed, 708);
        assert!(fb.count_non_background() > 1000);
        images.push(fb.to_rgba_bytes());
    }
    assert_eq!(
        images[1], images[2],
        "both fills with depth give the same picture"
    );
    assert_ne!(images[0], images[1]);
}
