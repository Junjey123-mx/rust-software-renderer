//! Laboratory 5 behaviour frozen before adding interactive views.

use raylib::math::Vector3;
use rust_software_renderer::app::{Scene, parse_args};
use rust_software_renderer::benchmark::{build_workload, verify_parity};
use rust_software_renderer::geometry::{SCENE_HEIGHT, SCENE_WIDTH, Vertex, triangle_scene};
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    Projection, Uniforms, primitive_assembly, rasterize_with_mode, render, render_with_mode,
};
use rust_software_renderer::rasterization::{RasterBounds, TriangleMode};
use rust_software_renderer::rendering::Framebuffer;
use std::collections::HashSet;
use std::path::Path;

const SHIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/models/Laboratorio3.obj"
);

fn pixels(frags: &[rust_software_renderer::rendering::Fragment]) -> HashSet<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

#[test]
fn synthetic_scene_keeps_its_fragment_counts_and_images() {
    let verts = triangle_scene();
    let tris = primitive_assembly(&verts).unwrap();
    let b = RasterBounds::new(SCENE_WIDTH, SCENE_HEIGHT);
    assert_eq!(
        rasterize_with_mode(&tris, TriangleMode::Wireframe, b).len(),
        4831
    );
    let bary = rasterize_with_mode(&tris, TriangleMode::FilledBarycentric, b);
    let scan = rasterize_with_mode(&tris, TriangleMode::FilledScanline, b);
    assert_eq!(bary.len(), 192132);
    assert_eq!(pixels(&bary), pixels(&scan));

    let mut fb_b = Framebuffer::new(SCENE_WIDTH, SCENE_HEIGHT);
    let mut fb_s = Framebuffer::new(SCENE_WIDTH, SCENE_HEIGHT);
    render_with_mode(
        &mut fb_b,
        &Uniforms::default(),
        &verts,
        Projection::Screen,
        TriangleMode::FilledBarycentric,
    )
    .unwrap();
    render_with_mode(
        &mut fb_s,
        &Uniforms::default(),
        &verts,
        Projection::Screen,
        TriangleMode::FilledScanline,
    )
    .unwrap();
    assert_eq!(fb_b.to_rgba_bytes(), fb_s.to_rgba_bytes());
    assert_eq!(
        fb_b.get_pixel(330, 220),
        Some(Vector3::new(1.0, 0.75, 0.2)),
        "scene colors untouched"
    );
}

#[test]
fn ship_processes_708_triangles_in_every_mode_with_equal_coverage() {
    let mesh = load_obj(Path::new(SHIP)).unwrap();
    assert_eq!(
        (
            mesh.vertices.len(),
            mesh.indices.len(),
            mesh.triangle_count()
        ),
        (356, 2124, 708)
    );
    let verts = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    let fit = Projection::Fit {
        plane: Default::default(),
        margin_ratio: 0.1,
    };
    let mut images = Vec::new();
    for mode in TriangleMode::ALL {
        let mut fb = Framebuffer::new(800, 600);
        let stats = render_with_mode(&mut fb, &Uniforms::default(), &verts, fit, mode).unwrap();
        assert_eq!(stats.triangles_processed, 708, "{mode}");
        images.push((mode, stats.fragments_generated, fb.to_rgba_bytes()));
    }
    assert_eq!(images[0].1, 79934);
    assert_eq!(images[1].1, 173500);
    assert_eq!(images[1].1, images[2].1);
    assert_eq!(images[1].2, images[2].2, "both fills paint the same ship");
    let mut legacy = Framebuffer::new(800, 600);
    render(&mut legacy, &Uniforms::default(), &verts, fit).unwrap();
    assert_eq!(
        legacy.to_rgba_bytes(),
        images[0].2,
        "render() is still wireframe"
    );
}

#[test]
fn benchmark_workload_is_headless_and_parity_checked() {
    let w = build_workload();
    assert_eq!(w.triangles.len(), 207);
    assert!(verify_parity(&w).unwrap() > 100_000);
}

#[test]
fn command_line_defaults_are_unchanged() {
    let a = parse_args(Vec::<String>::new()).unwrap();
    assert_eq!(a.scene, Scene::Ship);
    assert_eq!(a.mode, TriangleMode::Wireframe);
    assert!(!a.benchmark && !a.profile && a.obj.is_none() && a.screenshot.is_none());
    for (flags, mode) in [
        (
            ["--scene", "triangles", "--mode", "scanline"],
            TriangleMode::FilledScanline,
        ),
        (
            ["--scene", "ship", "--mode", "barycentric"],
            TriangleMode::FilledBarycentric,
        ),
    ] {
        let a = parse_args(flags.map(String::from)).unwrap();
        assert_eq!(a.mode, mode);
    }
    assert!(
        parse_args(["--benchmark-triangles", "--profile"].map(String::from))
            .unwrap()
            .profile
    );
}
