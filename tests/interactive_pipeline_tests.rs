//! The orbit view feeds the same four-stage pipeline for all three modes.

use rust_software_renderer::app::{CameraAction, apply_input};
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, Projection, Uniforms, ViewPreset, Viewport, model_bounds,
    primitive_assembly, project_orbit, rasterize_with_mode, render_with_mode, vertex_shader,
};
use rust_software_renderer::rasterization::{RasterBounds, TriangleMode};
use rust_software_renderer::rendering::Framebuffer;
use std::collections::HashSet;
use std::path::Path;

const SHIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/models/Laboratorio3.obj"
);

fn ship() -> Vec<Vertex> {
    load_obj(Path::new(SHIP))
        .unwrap()
        .expand_vertices(Vertex::DEFAULT_COLOR)
}

fn view(verts: &[Vertex], preset: ViewPreset) -> OrbitView {
    let b = model_bounds(verts).unwrap();
    OrbitView::new(
        OrbitCamera::new(b.center, preset),
        &b,
        Viewport::new(800, 600, 0.1),
    )
    .unwrap()
}

fn projected(verts: &[Vertex], view: &OrbitView) -> Vec<Vertex> {
    let mut p: Vec<Vertex> = verts
        .iter()
        .map(|v| vertex_shader(v, &Uniforms::default()))
        .collect();
    project_orbit(&mut p, view, 800, 600).unwrap();
    p
}

fn pixels(frags: &[rust_software_renderer::rendering::Fragment]) -> HashSet<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

#[test]
fn all_708_faces_reach_every_mode_under_the_orbit_view() {
    let verts = ship();
    let vw = view(&verts, ViewPreset::Oblique);
    let tris = primitive_assembly(&projected(&verts, &vw)).unwrap();
    assert_eq!(tris.len(), 708);
    let b = RasterBounds::new(800, 600);
    let wire = rasterize_with_mode(&tris, TriangleMode::Wireframe, b);
    let bary = rasterize_with_mode(&tris, TriangleMode::FilledBarycentric, b);
    let scan = rasterize_with_mode(&tris, TriangleMode::FilledScanline, b);
    assert_eq!(
        pixels(&bary),
        pixels(&scan),
        "same coverage from the same view"
    );
    assert_eq!(bary.len(), scan.len());
    assert!(bary.len() > wire.len());
    for mode in TriangleMode::ALL {
        let mut fb = Framebuffer::new(800, 600);
        let stats = render_with_mode(
            &mut fb,
            &Uniforms::default(),
            &verts,
            Projection::Orbit(vw),
            mode,
        )
        .unwrap();
        assert_eq!(stats.triangles_processed, 708, "{mode}");
        assert_eq!(stats.vertices_processed, 2124);
        let expected = match mode {
            TriangleMode::Wireframe => wire.len(),
            TriangleMode::FilledBarycentric => bary.len(),
            TriangleMode::FilledScanline => scan.len(),
        };
        assert_eq!(stats.fragments_generated, expected, "{mode}");
    }
}

#[test]
fn changing_the_camera_changes_the_projection() {
    let verts = ship();
    let a = projected(&verts, &view(&verts, ViewPreset::Top));
    let b = projected(&verts, &view(&verts, ViewPreset::Side));
    let moved = a
        .iter()
        .zip(&b)
        .filter(|(p, q)| (p.transformed_position - q.transformed_position).length() > 1.0)
        .count();
    assert!(
        moved > 2000,
        "only {moved} of 2124 vertices moved between top and side"
    );
    let mut fa = Framebuffer::new(800, 600);
    let mut fb = Framebuffer::new(800, 600);
    render_with_mode(
        &mut fa,
        &Uniforms::default(),
        &verts,
        Projection::Orbit(view(&verts, ViewPreset::Top)),
        TriangleMode::FilledScanline,
    )
    .unwrap();
    render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &verts,
        Projection::Orbit(view(&verts, ViewPreset::Side)),
        TriangleMode::FilledScanline,
    )
    .unwrap();
    assert_ne!(fa.to_rgba_bytes(), fb.to_rgba_bytes());
}

#[test]
fn switching_modes_keeps_the_camera_and_zoom() {
    let verts = ship();
    let mut vw = view(&verts, ViewPreset::Oblique);
    for _ in 0..20 {
        apply_input(&mut vw.camera, CameraAction::OrbitRight, 0.03);
        apply_input(&mut vw.camera, CameraAction::ZoomIn, 0.03);
    }
    let before = vw;
    // A mode switch is just a different `mode` argument with the same view:
    // the projected geometry is identical for all three modes.
    let p = projected(&verts, &vw);
    for mode in TriangleMode::ALL {
        let mut fb = Framebuffer::new(800, 600);
        render_with_mode(
            &mut fb,
            &Uniforms::default(),
            &verts,
            Projection::Orbit(vw),
            mode,
        )
        .unwrap();
        assert_eq!(vw, before, "{mode} must not touch the camera");
        assert_eq!(projected(&verts, &vw), p);
    }
}

#[test]
fn synthetic_scene_and_fit_paths_are_untouched_by_the_orbit_code() {
    let verts = rust_software_renderer::geometry::triangle_scene();
    let mut fb = Framebuffer::new(800, 600);
    let stats = render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &verts,
        Projection::Screen,
        TriangleMode::FilledBarycentric,
    )
    .unwrap();
    assert_eq!(stats.fragments_generated, 192132);
    let ship = ship();
    let mut fb = Framebuffer::new(800, 600);
    let stats = render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &ship,
        Projection::Fit {
            plane: Default::default(),
            margin_ratio: 0.1,
        },
        TriangleMode::Wireframe,
    )
    .unwrap();
    assert_eq!(stats.fragments_generated, 79934);
}

#[test]
fn no_raylib_model_or_camera_apis_in_the_interactive_sources() {
    for file in [
        "src/app.rs",
        "src/pipeline/orbit_camera.rs",
        "src/pipeline/screen_mapping.rs",
        "src/pipeline/render.rs",
    ] {
        let text =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(file)).unwrap();
        for needle in [
            "draw_model",
            "draw_mesh",
            "draw_triangle",
            "draw_line",
            "load_model",
            "begin_mode3D",
            "Camera3D",
            "get_camera_matrix",
        ] {
            assert!(!text.contains(needle), "{file} uses {needle}");
        }
    }
}
