//! Laboratory 4 contracts frozen before laboratory 5 adds fill modes.
//! These tests must keep passing unchanged: the default pipeline is wireframe.

use raylib::math::{Vector2, Vector3};
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{Projection, Uniforms, rasterize, render};
use rust_software_renderer::rasterization::{line, triangle};
use rust_software_renderer::rendering::Framebuffer;
use std::path::Path;

const SHIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/models/Laboratorio3.obj"
);

fn screen(x: f32, y: f32) -> Vertex {
    Vertex::new(Vector3::new(x, y, 0.0))
}

#[test]
fn default_rasterize_is_exactly_three_lines_per_triangle() {
    let tri = [
        screen(300.0, 200.0),
        screen(400.0, 400.0),
        screen(500.0, 200.0),
    ];
    let frags = rasterize(&[tri]);
    let w = Vertex::DEFAULT_COLOR;
    let mut expected = line(Vector2::new(300.0, 200.0), Vector2::new(400.0, 400.0), w);
    expected.extend(line(
        Vector2::new(400.0, 400.0),
        Vector2::new(500.0, 200.0),
        w,
    ));
    expected.extend(line(
        Vector2::new(500.0, 200.0),
        Vector2::new(300.0, 200.0),
        w,
    ));
    assert_eq!(frags, expected);
    assert_eq!(frags, triangle(&tri[0], &tri[1], &tri[2]));
}

#[test]
fn default_render_leaves_triangle_interior_empty() {
    let mut fb = Framebuffer::new(800, 600);
    let verts = vec![
        screen(300.0, 200.0),
        screen(400.0, 400.0),
        screen(500.0, 200.0),
    ];
    let stats = render(&mut fb, &Uniforms::default(), &verts, Projection::Screen).unwrap();
    assert_eq!(stats.triangles_processed, 1);
    assert_eq!(fb.get_pixel(400, 200), Some(Vertex::DEFAULT_COLOR));
    // A filled implementation would paint these; the outline must not.
    for (x, y) in [(400, 300), (400, 250), (380, 320), (420, 320)] {
        assert_eq!(
            fb.get_pixel(x, y),
            Some(Vector3::zero()),
            "({x},{y}) filled"
        );
    }
    // Only outline pixels: far fewer than the ~20000 px interior.
    assert!(fb.count_non_background() < 1000);
}

#[test]
fn fragment_colors_come_from_the_first_vertex() {
    let red = Vector3::new(1.0, 0.0, 0.0);
    let a = Vertex::new_with_color(Vector3::new(5.0, 5.0, 0.0), red);
    let frags = rasterize(&[[a, screen(20.0, 5.0), screen(5.0, 20.0)]]);
    assert!(!frags.is_empty());
    assert!(frags.iter().all(|f| f.color == red && f.depth == 0.0));
}

#[test]
fn ship_pipeline_counts_are_unchanged() {
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
    let mut fb = Framebuffer::new(800, 600);
    let stats = render(
        &mut fb,
        &Uniforms::default(),
        &verts,
        Projection::Fit {
            plane: Default::default(),
            margin_ratio: 0.1,
        },
    )
    .unwrap();
    assert_eq!(stats.vertices_processed, 2124);
    assert_eq!(stats.triangles_processed, 708);
    assert_eq!(stats.fragments_generated, 79934);
    assert!(fb.count_non_background() > 1000);
}
