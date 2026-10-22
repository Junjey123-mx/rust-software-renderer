use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    MappingError, OrbitCamera, OrbitView, Projection, RenderError, Uniforms, ViewPreset, Viewport,
    model_bounds, project_orbit, render_with_mode,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use std::path::Path;

const SHIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/models/Laboratorio3.obj"
);

fn v(x: f32, y: f32, z: f32) -> Vertex {
    Vertex::new(Vector3::new(x, y, z))
}

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

fn bounds2d(verts: &[Vertex]) -> (f32, f32, f32, f32) {
    let xs = verts.iter().map(|v| v.transformed_position.x);
    let ys = verts.iter().map(|v| v.transformed_position.y);
    (
        xs.clone().fold(f32::INFINITY, f32::min),
        xs.fold(f32::NEG_INFINITY, f32::max),
        ys.clone().fold(f32::INFINITY, f32::min),
        ys.fold(f32::NEG_INFINITY, f32::max),
    )
}

#[test]
fn model_bounds_of_the_ship_match_its_export() {
    let b = model_bounds(&ship()).unwrap();
    assert!(
        (b.center.x).abs() < 1e-3
            && (b.center.y - 0.255).abs() < 1e-3
            && (b.center.z + 1.3).abs() < 1e-3,
        "{b:?}"
    );
    assert!(b.radius > 3.0 && b.radius < 4.5, "{}", b.radius);
    assert!(model_bounds(&[]).is_none());
    assert!(model_bounds(&[v(f32::NAN, 0.0, 0.0)]).is_none());
}

#[test]
fn ship_is_centered_and_inside_the_viewport_from_every_preset() {
    let verts = ship();
    for preset in [
        ViewPreset::Oblique,
        ViewPreset::Top,
        ViewPreset::Front,
        ViewPreset::Side,
    ] {
        let mut p = verts.clone();
        project_orbit(&mut p, &view(&verts, preset), 800, 600).unwrap();
        let (x0, x1, y0, y1) = bounds2d(&p);
        assert!(
            x0 >= 60.0 && x1 <= 740.0 && y0 >= 60.0 && y1 <= 540.0,
            "{preset:?}: {x0} {x1} {y0} {y1}"
        );
        assert!(x1 - x0 > 150.0 || y1 - y0 > 150.0, "{preset:?} too small");
        assert!(p.iter().all(|v| v.transformed_position.z.is_finite()));
        // Model-space positions are preserved.
        assert_eq!(p[0].position, verts[0].position);
    }
}

#[test]
fn elevation_reveals_height_and_azimuth_changes_the_silhouette() {
    let verts = ship();
    let top = view(&verts, ViewPreset::Top);
    let mut p_top = verts.clone();
    project_orbit(&mut p_top, &top, 800, 600).unwrap();
    let (_, _, ty0, ty1) = bounds2d(&p_top);

    let front = view(&verts, ViewPreset::Front);
    let mut p_front = verts.clone();
    project_orbit(&mut p_front, &front, 800, 600).unwrap();
    let (fx0, fx1, fy0, fy1) = bounds2d(&p_front);
    // From the top, the long axis (Z, 6.2 units) is vertical; from the front
    // only the height (1.35 units) is vertical, so the extent shrinks a lot.
    assert!(
        (fy1 - fy0) < (ty1 - ty0) * 0.5,
        "front {} vs top {}",
        fy1 - fy0,
        ty1 - ty0
    );
    assert!(
        (fx1 - fx0) > (fy1 - fy0) * 2.0,
        "front view is wide (wings)"
    );

    let side = view(&verts, ViewPreset::Side);
    let mut p_side = verts.clone();
    project_orbit(&mut p_side, &side, 800, 600).unwrap();
    let (sx0, sx1, _, _) = bounds2d(&p_side);
    assert!(
        (sx1 - sx0) > (fx1 - fx0),
        "side shows the longer Z axis horizontally"
    );
}

#[test]
fn zoom_scales_around_the_center_and_rotation_keeps_the_scale() {
    let verts = ship();
    let mut vw = view(&verts, ViewPreset::Oblique);
    let base = vw.scale();
    let mut p1 = verts.clone();
    project_orbit(&mut p1, &vw, 800, 600).unwrap();
    let (x0, x1, _, _) = bounds2d(&p1);
    vw.camera.zoom_by(2.0);
    assert!((vw.scale() - 2.0 * base).abs() < 1e-5);
    let mut p2 = verts.clone();
    project_orbit(&mut p2, &vw, 800, 600).unwrap();
    let (zx0, zx1, _, _) = bounds2d(&p2);
    assert!(
        (zx1 - zx0 - 2.0 * (x1 - x0)).abs() < 1.0,
        "zoom doubles the width"
    );
    // The target projects to the viewport center regardless of zoom/angle.
    let mut t = vec![Vertex::new(vw.camera.target)];
    project_orbit(&mut t, &vw, 800, 600).unwrap();
    assert!(
        (t[0].transformed_position.x - 400.0).abs() < 1e-3
            && (t[0].transformed_position.y - 300.0).abs() < 1e-3
    );
    vw.camera.orbit(1.0);
    vw.camera.tilt(-0.4);
    assert!(
        (vw.scale() - 2.0 * base).abs() < 1e-5,
        "orientation never changes the scale"
    );
}

#[test]
fn depth_comes_from_the_camera_not_from_model_z() {
    let verts = vec![v(0.0, 0.0, -2.0), v(0.0, 0.0, 2.0)];
    let b = model_bounds(&verts).unwrap();
    let cam = OrbitCamera::new(b.center, ViewPreset::Front);
    let vw = OrbitView::new(cam, &b, Viewport::new(100, 100, 0.1)).unwrap();
    let mut p = verts.clone();
    project_orbit(&mut p, &vw, 100, 100).unwrap();
    // Front camera sits on the -Z side: z=-2 is nearer (smaller depth).
    assert!(p[0].transformed_position.z < p[1].transformed_position.z);
    let cam = OrbitCamera::new(b.center, ViewPreset::Top);
    let vw = OrbitView::new(cam, &b, Viewport::new(100, 100, 0.1)).unwrap();
    let mut p = verts.clone();
    project_orbit(&mut p, &vw, 100, 100).unwrap();
    assert!(
        (p[0].transformed_position.z - p[1].transformed_position.z).abs() < 0.2,
        "same height, similar depth from above"
    );
}

#[test]
fn invalid_inputs_are_reported() {
    let verts = ship();
    let b = model_bounds(&verts).unwrap();
    let cam = OrbitCamera::new(b.center, ViewPreset::Oblique);
    assert_eq!(
        OrbitView::new(cam, &b, Viewport::new(0, 10, 0.1)).unwrap_err(),
        MappingError::InvalidViewport
    );
    assert_eq!(
        OrbitView::new(cam, &b, Viewport::new(10, 10, 0.5)).unwrap_err(),
        MappingError::InvalidViewport
    );
    let vw = view(&verts, ViewPreset::Oblique);
    let mut bad = vec![v(f32::NAN, 0.0, 0.0)];
    assert_eq!(
        project_orbit(&mut bad, &vw, 10, 10).unwrap_err(),
        MappingError::NonFinitePosition
    );
    let mut ok = vec![v(0.0, 0.0, 0.0)];
    assert_eq!(
        project_orbit(&mut ok, &vw, 0, 10).unwrap_err(),
        MappingError::InvalidViewport
    );
    let mut fb = Framebuffer::new(10, 10);
    assert!(matches!(
        render_with_mode(
            &mut fb,
            &Uniforms::default(),
            &bad,
            Projection::Orbit(vw),
            TriangleMode::Wireframe
        )
        .unwrap_err(),
        RenderError::Mapping(_)
    ));
}

#[test]
fn render_with_orbit_projection_draws_all_faces() {
    let verts = ship();
    let vw = view(&verts, ViewPreset::Oblique);
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
        assert_eq!(stats.triangles_processed, 708);
        assert!(fb.count_non_background() > 1000, "{mode}");
    }
}
