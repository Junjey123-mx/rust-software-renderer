//! End-to-end contracts of the laboratory 4 pipeline, headless.

use raylib::math::{Vector2, Vector3};
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    Projection, Uniforms, ViewPlane, Viewport, fit_to_viewport, fragment_shader,
    primitive_assembly, rasterize, render, vertex_shader,
};
use rust_software_renderer::rasterization::{line, triangle};
use rust_software_renderer::rendering::Framebuffer;
use std::path::Path;

const SHIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/models/Laboratorio3.obj"
);

#[test]
fn every_face_of_the_ship_reaches_rasterization_as_three_edges() {
    let mesh = load_obj(Path::new(SHIP)).unwrap();
    let mut verts: Vec<Vertex> = mesh
        .expand_vertices(Vertex::DEFAULT_COLOR)
        .iter()
        .map(|v| vertex_shader(v, &Uniforms::default()))
        .collect();
    fit_to_viewport(&mut verts, ViewPlane::XZ, Viewport::new(800, 600, 0.1)).unwrap();
    let triangles = primitive_assembly(&verts).unwrap();
    assert_eq!(triangles.len(), mesh.triangle_count());
    assert_eq!(triangles.len(), 708);

    // rasterize == concatenation of triangle() per face, and each triangle
    // == exactly its three line() edges in order.
    let fragments = rasterize(&triangles);
    let mut expected = 0;
    for tri in &triangles {
        let p = |v: &Vertex| Vector2::new(v.transformed_position.x, v.transformed_position.y);
        let edges = line(p(&tri[0]), p(&tri[1]), tri[0].color).len()
            + line(p(&tri[1]), p(&tri[2]), tri[0].color).len()
            + line(p(&tri[2]), p(&tri[0]), tri[0].color).len();
        assert_eq!(triangle(&tri[0], &tri[1], &tri[2]).len(), edges);
        expected += edges;
    }
    assert_eq!(fragments.len(), expected);
}

#[test]
fn render_stats_match_the_manual_pipeline_and_pixels_land_in_bounds() {
    let mesh = load_obj(Path::new(SHIP)).unwrap();
    let verts = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    let mut fb = Framebuffer::new(800, 600);
    let stats = render(
        &mut fb,
        &Uniforms::default(),
        &verts,
        Projection::Fit {
            plane: ViewPlane::XZ,
            margin_ratio: 0.1,
        },
    )
    .unwrap();
    assert_eq!(stats.vertices_processed, 2124);
    assert_eq!(stats.triangles_processed, 708);

    let mut fitted = verts.clone();
    fit_to_viewport(&mut fitted, ViewPlane::XZ, Viewport::new(800, 600, 0.1)).unwrap();
    let fragments = rasterize(&primitive_assembly(&fitted).unwrap());
    assert_eq!(stats.fragments_generated, fragments.len());

    // Every fragment is inside the framebuffer and was written with its
    // (passthrough) color.
    for f in &fragments {
        let (x, y) = (f.position.x as i32, f.position.y as i32);
        assert!((0..800).contains(&x) && (0..600).contains(&y), "{f:?}");
        assert_eq!(fb.get_pixel(x, y), Some(fragment_shader(f)));
    }
}

#[test]
fn rendering_is_deterministic() {
    let mesh = load_obj(Path::new(SHIP)).unwrap();
    let verts = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    let projection = Projection::Fit {
        plane: ViewPlane::XZ,
        margin_ratio: 0.1,
    };
    let mut a = Framebuffer::new(400, 300);
    let mut b = Framebuffer::new(400, 300);
    let sa = render(&mut a, &Uniforms::default(), &verts, projection).unwrap();
    let sb = render(&mut b, &Uniforms::default(), &verts, projection).unwrap();
    assert_eq!(sa, sb);
    assert_eq!(a.to_rgba_bytes(), b.to_rgba_bytes());
}

#[test]
fn xy_view_also_renders_the_whole_model() {
    let mesh = load_obj(Path::new(SHIP)).unwrap();
    let verts = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    let mut fb = Framebuffer::new(800, 600);
    let stats = render(
        &mut fb,
        &Uniforms::default(),
        &verts,
        Projection::Fit {
            plane: ViewPlane::XY,
            margin_ratio: 0.1,
        },
    )
    .unwrap();
    assert_eq!(stats.triangles_processed, 708);
    assert!(fb.count_non_background() > 1000);
}

#[test]
fn fragment_color_is_the_vertex_color_end_to_end() {
    let cyan = Vector3::new(0.0, 1.0, 1.0);
    let mesh = load_obj(Path::new("tests/fixtures/cube_triangulated.obj")).unwrap();
    let verts = mesh.expand_vertices(cyan);
    let mut fb = Framebuffer::new(200, 200);
    render(
        &mut fb,
        &Uniforms::default(),
        &verts,
        Projection::Fit {
            plane: ViewPlane::XY,
            margin_ratio: 0.1,
        },
    )
    .unwrap();
    let lit = fb.count_non_background();
    assert!(lit > 0);
    let mut cyan_pixels = 0;
    for y in 0..200 {
        for x in 0..200 {
            if fb.get_pixel(x, y) == Some(cyan) {
                cyan_pixels += 1;
            }
        }
    }
    assert_eq!(cyan_pixels, lit);
}
