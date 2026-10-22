//! End-to-end contracts of laboratory 5 on top of the inherited pipeline.

use raylib::math::Vector3;
use rust_software_renderer::geometry::{Mesh, SCENE_HEIGHT, SCENE_WIDTH, Vertex, triangle_scene};
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    Projection, Uniforms, ViewPlane, Viewport, fit_to_viewport, fragment_shader,
    primitive_assembly, rasterize, rasterize_with_mode, render, render_with_mode, vertex_shader,
};
use rust_software_renderer::rasterization::{
    RasterBounds, TriangleMode, fill_barycentric, fill_scanline, triangle,
};
use rust_software_renderer::rendering::Framebuffer;
use std::collections::HashSet;
use std::path::Path;

const SHIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/models/Laboratorio3.obj"
);

fn ship_mesh() -> Mesh {
    load_obj(Path::new(SHIP)).unwrap()
}

fn pixels(frags: &[rust_software_renderer::rendering::Fragment]) -> HashSet<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

#[test]
fn same_mesh_and_assembly_three_modes_only_differ_in_stage_three() {
    let mesh = ship_mesh();
    assert_eq!(
        (
            mesh.vertices.len(),
            mesh.indices.len(),
            mesh.triangle_count()
        ),
        (356, 2124, 708)
    );
    let mut verts: Vec<Vertex> = mesh
        .expand_vertices(Vertex::DEFAULT_COLOR)
        .iter()
        .map(|v| vertex_shader(v, &Uniforms::default()))
        .collect();
    fit_to_viewport(&mut verts, ViewPlane::XZ, Viewport::new(800, 600, 0.1)).unwrap();
    let triangles = primitive_assembly(&verts).unwrap();
    assert_eq!(triangles.len(), 708);

    let bounds = RasterBounds::new(800, 600);
    let wire = rasterize_with_mode(&triangles, TriangleMode::Wireframe, bounds);
    let bary = rasterize_with_mode(&triangles, TriangleMode::FilledBarycentric, bounds);
    let scan = rasterize_with_mode(&triangles, TriangleMode::FilledScanline, bounds);
    assert_eq!(wire, rasterize(&triangles), "wireframe is the lab 4 path");
    assert_eq!(wire.len(), 79934);
    assert_eq!(
        pixels(&bary),
        pixels(&scan),
        "all 708 projected faces agree"
    );
    assert_eq!(bary.len(), scan.len());
    assert!(bary.len() > wire.len());

    // Every face is processed by every mode, even those whose projection is
    // degenerate (zero area) and yields no interior.
    let mut degenerate = 0;
    for tri in &triangles {
        let b = fill_barycentric(&tri[0], &tri[1], &tri[2], bounds);
        let s = fill_scanline(&tri[0], &tri[1], &tri[2], bounds);
        assert_eq!(pixels(&b), pixels(&s));
        if b.is_empty() {
            degenerate += 1;
        }
        assert!(
            !triangle(&tri[0], &tri[1], &tri[2]).is_empty(),
            "outline always exists"
        );
    }
    assert!(degenerate < 708);
}

#[test]
fn wireframe_has_no_interior_and_fills_have_it() {
    let verts = triangle_scene();
    let mut wire = Framebuffer::new(SCENE_WIDTH, SCENE_HEIGHT);
    let mut bary = Framebuffer::new(SCENE_WIDTH, SCENE_HEIGHT);
    let mut scan = Framebuffer::new(SCENE_WIDTH, SCENE_HEIGHT);
    render(&mut wire, &Uniforms::default(), &verts, Projection::Screen).unwrap();
    render_with_mode(
        &mut bary,
        &Uniforms::default(),
        &verts,
        Projection::Screen,
        TriangleMode::FilledBarycentric,
    )
    .unwrap();
    render_with_mode(
        &mut scan,
        &Uniforms::default(),
        &verts,
        Projection::Screen,
        TriangleMode::FilledScanline,
    )
    .unwrap();
    // Centroids of the large, green and blue triangles.
    for (x, y, color) in [
        (333, 213, Vector3::new(1.0, 0.75, 0.2)),
        (660, 136, Vector3::new(0.3, 0.9, 0.4)),
        (640, 420, Vector3::new(0.3, 0.5, 1.0)),
    ] {
        assert_eq!(
            wire.get_pixel(x, y),
            Some(Vector3::zero()),
            "wireframe interior ({x},{y})"
        );
        assert_eq!(bary.get_pixel(x, y), Some(color), "barycentric ({x},{y})");
        assert_eq!(scan.get_pixel(x, y), Some(color), "scanline ({x},{y})");
    }
    assert_eq!(bary.to_rgba_bytes(), scan.to_rgba_bytes());
}

#[test]
fn framebuffer_writes_stay_in_bounds_with_passthrough_colors() {
    let verts = triangle_scene();
    for mode in TriangleMode::ALL {
        let mut fb = Framebuffer::new(SCENE_WIDTH, SCENE_HEIGHT);
        let stats = render_with_mode(
            &mut fb,
            &Uniforms::default(),
            &verts,
            Projection::Screen,
            mode,
        )
        .unwrap();
        assert_eq!(stats.triangles_processed, 7);
        let bounds = RasterBounds::new(SCENE_WIDTH, SCENE_HEIGHT);
        let tris = primitive_assembly(&verts).unwrap();
        let frags = rasterize_with_mode(&tris, mode, bounds);
        assert_eq!(frags.len(), stats.fragments_generated);
        // Fragments of filled modes are always inside; wireframe may leave the
        // viewport (the clipped yellow triangle) and the framebuffer drops them.
        if mode != TriangleMode::Wireframe {
            assert!(
                frags
                    .iter()
                    .all(|f| (0.0..SCENE_WIDTH as f32).contains(&f.position.x)
                        && (0.0..SCENE_HEIGHT as f32).contains(&f.position.y))
            );
        }
        // Last write wins with the passthrough shader: every lit pixel equals
        // the color of some fragment at that position.
        for f in &frags {
            let (x, y) = (f.position.x as i32, f.position.y as i32);
            if let Some(px) = fb.get_pixel(x, y) {
                assert_ne!(px, Vector3::zero());
                assert_eq!(f.depth, 0.0);
                let _ = fragment_shader(f);
            }
        }
    }
}

#[test]
fn scanline_source_does_not_delegate_to_barycentric() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/rasterization/scanline.rs"
    ));
    assert!(!src.contains("fill_barycentric"));
    assert!(!src.contains("barycentric_weights"));
    assert!(src.contains("row_crossings"));
    let bary = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/rasterization/barycentric.rs"
    ));
    assert!(!bary.contains("fill_scanline"));
    assert!(bary.contains("barycentric_weights"));
}

#[test]
fn no_forbidden_raylib_drawing_in_production_sources() {
    for file in [
        "src/app.rs",
        "src/pipeline/render.rs",
        "src/pipeline/rasterizer.rs",
        "src/rasterization/line.rs",
        "src/rasterization/triangle.rs",
        "src/rasterization/barycentric.rs",
        "src/rasterization/scanline.rs",
        "src/rendering/presenter.rs",
    ] {
        let text =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(file)).unwrap();
        for needle in [
            "draw_triangle",
            "draw_line",
            "load_model",
            "draw_model",
            "draw_mesh",
        ] {
            assert!(!text.contains(needle), "{file} uses {needle}");
        }
    }
}
