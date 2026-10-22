//! Deterministic headless sweeps of the interactive view across angles,
//! zoom levels and all three modes, plus laboratory 4/5 baselines.

use rust_software_renderer::app::{CameraAction, apply_input};
use rust_software_renderer::geometry::{Vertex, triangle_scene};
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, Projection, Uniforms, ViewPreset, Viewport, model_bounds,
    primitive_assembly, project_orbit, rasterize_with_mode, render, render_interactive,
    render_with_mode,
};
use rust_software_renderer::rasterization::{RasterBounds, TriangleMode};
use rust_software_renderer::rendering::Framebuffer;
use std::collections::HashSet;
use std::path::Path;

fn ship() -> Vec<Vertex> {
    load_obj(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/models/Laboratorio3.obj"
    )))
    .unwrap()
    .expand_vertices(Vertex::DEFAULT_COLOR)
}

fn pixels(frags: &[rust_software_renderer::rendering::Fragment]) -> HashSet<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

#[test]
fn sweeping_angles_and_zoom_keeps_both_fills_identical_and_inside_the_window() {
    let verts = ship();
    let b = model_bounds(&verts).unwrap();
    let mut view = OrbitView::new(
        OrbitCamera::new(b.center, ViewPreset::Oblique),
        &b,
        Viewport::new(800, 600, 0.1),
    )
    .unwrap();
    let mut previous: Option<Vec<u8>> = None;
    for step in 0..12 {
        apply_input(&mut view.camera, CameraAction::OrbitRight, 0.05);
        apply_input(
            &mut view.camera,
            if step % 2 == 0 {
                CameraAction::TiltUp
            } else {
                CameraAction::TiltDown
            },
            0.04,
        );
        apply_input(
            &mut view.camera,
            if step < 6 {
                CameraAction::ZoomIn
            } else {
                CameraAction::ZoomOut
            },
            0.03,
        );

        let mut p: Vec<Vertex> = verts.clone();
        project_orbit(&mut p, &view, 800, 600).unwrap();
        let tris = primitive_assembly(&p).unwrap();
        let bary = rasterize_with_mode(
            &tris,
            TriangleMode::FilledBarycentric,
            RasterBounds::new(800, 600),
        );
        let scan = rasterize_with_mode(
            &tris,
            TriangleMode::FilledScanline,
            RasterBounds::new(800, 600),
        );
        assert_eq!(pixels(&bary), pixels(&scan), "step {step}");
        assert!(bary.iter().all(|f| f.position.x >= 0.0
            && f.position.x < 800.0
            && f.position.y >= 0.0
            && f.position.y < 600.0));

        let mut fb_b = Framebuffer::new(800, 600);
        let mut fb_s = Framebuffer::new(800, 600);
        let sb = render_interactive(
            &mut fb_b,
            &Uniforms::default(),
            &verts,
            view,
            TriangleMode::FilledBarycentric,
        )
        .unwrap();
        let ss = render_interactive(
            &mut fb_s,
            &Uniforms::default(),
            &verts,
            view,
            TriangleMode::FilledScanline,
        )
        .unwrap();
        assert_eq!((sb.triangles_processed, ss.triangles_processed), (708, 708));
        assert_eq!(sb.fragments_generated, ss.fragments_generated);
        let image = fb_b.to_rgba_bytes();
        assert_eq!(
            image,
            fb_s.to_rgba_bytes(),
            "step {step}: same shaded, depth-tested picture"
        );
        if let Some(prev) = &previous {
            assert_ne!(prev, &image, "step {step}: the camera moved");
        }
        previous = Some(image);
    }
}

#[test]
fn zoom_changes_apparent_size_monotonically_without_pulsing() {
    let verts = ship();
    let b = model_bounds(&verts).unwrap();
    let mut view = OrbitView::new(
        OrbitCamera::new(b.center, ViewPreset::Top),
        &b,
        Viewport::new(800, 600, 0.1),
    )
    .unwrap();
    let lit = |view: OrbitView| {
        let mut fb = Framebuffer::new(800, 600);
        render_interactive(
            &mut fb,
            &Uniforms::default(),
            &verts,
            view,
            TriangleMode::FilledScanline,
        )
        .unwrap();
        fb.count_non_background()
    };
    let base = lit(view);
    // Rotating at the same zoom keeps the pixel footprint in the same range
    // (no re-fit per frame): the top view area must not jump when orbiting.
    let mut rotated = view;
    apply_input(&mut rotated.camera, CameraAction::OrbitRight, 0.05);
    let after_rotation = lit(rotated);
    assert!(
        (after_rotation as f32 / base as f32 - 1.0).abs() < 0.15,
        "{base} vs {after_rotation}"
    );
    let mut sizes = vec![base];
    for _ in 0..5 {
        apply_input(&mut view.camera, CameraAction::ZoomIn, 0.05);
        sizes.push(lit(view));
    }
    assert!(sizes.windows(2).all(|w| w[1] > w[0]), "{sizes:?}");
}

#[test]
fn wireframe_outline_matches_the_filled_silhouette_edge() {
    let verts = ship();
    let b = model_bounds(&verts).unwrap();
    let view = OrbitView::new(
        OrbitCamera::new(b.center, ViewPreset::Oblique),
        &b,
        Viewport::new(800, 600, 0.1),
    )
    .unwrap();
    let mut wire = Framebuffer::new(800, 600);
    let mut fill = Framebuffer::new(800, 600);
    render_interactive(
        &mut wire,
        &Uniforms::default(),
        &verts,
        view,
        TriangleMode::Wireframe,
    )
    .unwrap();
    render_interactive(
        &mut fill,
        &Uniforms::default(),
        &verts,
        view,
        TriangleMode::FilledBarycentric,
    )
    .unwrap();
    let bg = wire.background_color();
    let (mut inside, mut total) = (0, 0);
    for y in 0..600 {
        for x in 0..800 {
            if wire.get_pixel(x, y) != Some(bg) {
                total += 1;
                // Every outline pixel is on or next to a filled pixel.
                let near = (-1..=1).any(|dx| {
                    (-1..=1).any(|dy| fill.get_pixel(x + dx, y + dy).is_some_and(|c| c != bg))
                });
                if near {
                    inside += 1;
                }
            }
        }
    }
    assert!(total > 1000);
    assert!(
        inside * 100 >= total * 99,
        "{inside}/{total} outline pixels touch the filled ship"
    );
}

#[test]
fn laboratory_four_and_five_entry_points_are_unchanged() {
    let verts = ship();
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
    assert_eq!(
        (stats.triangles_processed, stats.fragments_generated),
        (708, 79934)
    );
    let mut fb = Framebuffer::new(800, 600);
    let stats = render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &verts,
        Projection::Fit {
            plane: Default::default(),
            margin_ratio: 0.1,
        },
        TriangleMode::FilledScanline,
    )
    .unwrap();
    assert_eq!(stats.fragments_generated, 173500);
    let scene = triangle_scene();
    let mut fb = Framebuffer::new(800, 600);
    let stats = render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &scene,
        Projection::Screen,
        TriangleMode::FilledBarycentric,
    )
    .unwrap();
    assert_eq!(stats.fragments_generated, 192132);
    assert_eq!(
        fb.get_pixel(330, 220),
        Some(raylib::math::Vector3::new(1.0, 0.75, 0.2))
    );
}
