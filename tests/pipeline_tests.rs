use raylib::math::{Matrix, Vector3};
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{
    Projection, RenderError, Uniforms, ViewPlane, fragment_shader, rasterize, render,
};
use rust_software_renderer::rendering::{Fragment, Framebuffer};

const WHITE: Vector3 = Vertex::DEFAULT_COLOR;

fn screen(x: f32, y: f32) -> Vertex {
    Vertex::new(Vector3::new(x, y, 0.0))
}

fn lesson_triangle() -> Vec<Vertex> {
    vec![
        screen(300.0, 200.0),
        screen(400.0, 400.0),
        screen(500.0, 200.0),
    ]
}

#[test]
fn single_triangle_outline_reaches_the_framebuffer() {
    let mut fb = Framebuffer::new(800, 600);
    let stats = render(
        &mut fb,
        &Uniforms::default(),
        &lesson_triangle(),
        Projection::Screen,
    )
    .unwrap();

    assert_eq!(stats.vertices_processed, 3);
    assert_eq!(stats.triangles_processed, 1);
    // Each edge spans 200 px on its longest axis, both endpoints inclusive.
    assert_eq!(stats.fragments_generated, 3 * 201);

    // Vertices and the horizontal edge are white.
    assert_eq!(fb.get_pixel(300, 200), Some(WHITE));
    assert_eq!(fb.get_pixel(400, 400), Some(WHITE));
    assert_eq!(fb.get_pixel(500, 200), Some(WHITE));
    assert_eq!(fb.get_pixel(400, 200), Some(WHITE));
    // Interior stays background.
    assert_eq!(fb.get_pixel(400, 300), Some(Vector3::zero()));
    assert_eq!(fb.get_pixel(400, 250), Some(Vector3::zero()));
    // (400,400) is lower on screen than (300,200): y grows downwards.
    assert!(fb.count_non_background() > 0);
}

#[test]
fn two_triangles_are_processed_in_order_with_their_colors() {
    let red = Vector3::new(1.0, 0.0, 0.0);
    let mut verts = lesson_triangle();
    verts.extend([
        Vertex::new_with_color(Vector3::new(10.0, 10.0, 0.0), red),
        Vertex::new_with_color(Vector3::new(30.0, 10.0, 0.0), red),
        Vertex::new_with_color(Vector3::new(10.0, 30.0, 0.0), red),
    ]);
    let mut fb = Framebuffer::new(800, 600);
    let stats = render(&mut fb, &Uniforms::default(), &verts, Projection::Screen).unwrap();
    assert_eq!(stats.triangles_processed, 2);
    assert_eq!(fb.get_pixel(20, 10), Some(red));
    assert_eq!(fb.get_pixel(10, 20), Some(red));
    assert_eq!(fb.get_pixel(400, 200), Some(WHITE));
}

#[test]
fn model_matrix_is_applied_before_rasterization() {
    let mut fb = Framebuffer::new(100, 100);
    let verts = vec![screen(0.0, 0.0), screen(10.0, 0.0), screen(0.0, 10.0)];
    let uniforms = Uniforms {
        model_matrix: Matrix::translate(50.0, 50.0, 0.0),
    };
    render(&mut fb, &uniforms, &verts, Projection::Screen).unwrap();
    assert_eq!(fb.get_pixel(50, 50), Some(WHITE));
    assert_eq!(fb.get_pixel(60, 50), Some(WHITE));
    assert_eq!(fb.get_pixel(0, 0), Some(Vector3::zero()));
}

#[test]
fn fit_projection_centers_model_coordinates() {
    let mut fb = Framebuffer::new(200, 100);
    let verts = vec![
        Vertex::new(Vector3::new(-1.0, -1.0, 0.0)),
        Vertex::new(Vector3::new(1.0, -1.0, 0.0)),
        Vertex::new(Vector3::new(0.0, 1.0, 0.0)),
    ];
    let projection = Projection::Fit {
        plane: ViewPlane::XY,
        margin_ratio: 0.1,
    };
    render(&mut fb, &Uniforms::default(), &verts, projection).unwrap();
    // Apex (0,1) maps to the top margin, centered horizontally.
    assert_eq!(fb.get_pixel(100, 10), Some(WHITE));
    // Base corners at the bottom margin.
    assert_eq!(fb.get_pixel(60, 90), Some(WHITE));
    assert_eq!(fb.get_pixel(140, 90), Some(WHITE));
    assert_eq!(fb.get_pixel(100, 50), Some(Vector3::zero()));
}

#[test]
fn off_screen_fragments_are_clipped_safely() {
    let mut fb = Framebuffer::new(50, 50);
    let verts = vec![screen(-20.0, -20.0), screen(80.0, 25.0), screen(25.0, 80.0)];
    let stats = render(&mut fb, &Uniforms::default(), &verts, Projection::Screen).unwrap();
    assert!(stats.fragments_generated > 0);
    assert!(fb.count_non_background() > 0);
    assert!(fb.count_non_background() < stats.fragments_generated);
}

#[test]
fn incomplete_vertex_list_is_an_error() {
    let mut fb = Framebuffer::new(10, 10);
    let err = render(
        &mut fb,
        &Uniforms::default(),
        &lesson_triangle()[..2],
        Projection::Screen,
    )
    .unwrap_err();
    assert!(matches!(err, RenderError::Primitive(_)));
    assert_eq!(fb.count_non_background(), 0, "nothing drawn on error");
}

#[test]
fn empty_fit_is_a_mapping_error() {
    let mut fb = Framebuffer::new(10, 10);
    let projection = Projection::Fit {
        plane: ViewPlane::XZ,
        margin_ratio: 0.1,
    };
    assert!(matches!(
        render(&mut fb, &Uniforms::default(), &[], projection).unwrap_err(),
        RenderError::Mapping(_)
    ));
}

#[test]
fn rasterize_does_not_touch_any_framebuffer_and_counts_edges() {
    let tris = [[screen(0.0, 0.0), screen(4.0, 0.0), screen(0.0, 4.0)]];
    let frags = rasterize(&tris);
    // Three edges, both endpoints inclusive: 5 + 5 + 5.
    assert_eq!(frags.len(), 15);
    assert!(frags.iter().all(|f| f.color == WHITE));
}

#[test]
fn fragment_shader_is_passthrough() {
    let green = Vector3::new(0.0, 1.0, 0.0);
    let f = Fragment::new(3.0, 4.0, green, 0.0);
    assert_eq!(fragment_shader(&f), green);
}
