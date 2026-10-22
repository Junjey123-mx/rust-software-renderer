use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{MappingError, ViewPlane, Viewport, fit_to_viewport};
use std::path::Path;

fn verts(points: &[(f32, f32, f32)]) -> Vec<Vertex> {
    points
        .iter()
        .map(|&(x, y, z)| Vertex::new(Vector3::new(x, y, z)))
        .collect()
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn bounds(v: &[Vertex]) -> (f32, f32, f32, f32) {
    let xs = v.iter().map(|v| v.transformed_position.x);
    let ys = v.iter().map(|v| v.transformed_position.y);
    (
        xs.clone().fold(f32::INFINITY, f32::min),
        xs.fold(f32::NEG_INFINITY, f32::max),
        ys.clone().fold(f32::INFINITY, f32::min),
        ys.fold(f32::NEG_INFINITY, f32::max),
    )
}

#[test]
fn wide_box_is_limited_by_width_and_centered() {
    // 4 wide, 1 tall in XY, viewport 800x600 with 10% margin -> 640 px wide.
    let mut v = verts(&[
        (-2.0, -0.5, 0.0),
        (2.0, -0.5, 0.0),
        (2.0, 0.5, 0.0),
        (-2.0, 0.5, 0.0),
    ]);
    let fit = fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(800, 600, 0.1)).unwrap();
    assert!(approx(fit.scale, 160.0), "scale {}", fit.scale);
    let (x0, x1, y0, y1) = bounds(&v);
    assert!(approx(x0, 80.0) && approx(x1, 720.0));
    assert!(
        approx(y0, 220.0) && approx(y1, 380.0),
        "aspect ratio kept: {y0}..{y1}"
    );
    assert!(approx((x0 + x1) / 2.0, 400.0) && approx((y0 + y1) / 2.0, 300.0));
}

#[test]
fn tall_box_is_limited_by_height() {
    let mut v = verts(&[(0.0, -3.0, 0.0), (1.0, 3.0, 0.0)]);
    let fit = fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(800, 600, 0.1)).unwrap();
    assert!(approx(fit.scale, 80.0));
    let (_, _, y0, y1) = bounds(&v);
    assert!(approx(y0, 60.0) && approx(y1, 540.0));
}

#[test]
fn model_up_axis_points_to_the_top_of_the_screen() {
    let mut v = verts(&[(0.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 0.0, 0.0)]);
    fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(100, 100, 0.0)).unwrap();
    assert!(
        v[1].transformed_position.y < v[0].transformed_position.y,
        "y=1 is higher on screen"
    );
    assert!(v[2].transformed_position.x > v[0].transformed_position.x);
}

#[test]
fn xz_plane_uses_z_as_screen_up() {
    let mut v = verts(&[(0.0, 5.0, 0.0), (0.0, -5.0, 1.0), (1.0, 7.0, 0.0)]);
    fit_to_viewport(&mut v, ViewPlane::XZ, Viewport::new(100, 100, 0.0)).unwrap();
    // y is ignored: vertices 0 and 2 share z=0 so they share screen y.
    assert!(approx(
        v[0].transformed_position.y,
        v[2].transformed_position.y
    ));
    assert!(v[1].transformed_position.y < v[0].transformed_position.y);
}

#[test]
fn single_point_lands_at_the_center() {
    let mut v = verts(&[(7.0, -3.0, 2.0)]);
    let fit = fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(640, 480, 0.1)).unwrap();
    assert_eq!(fit.scale, 1.0);
    assert!(approx(v[0].transformed_position.x, 320.0));
    assert!(approx(v[0].transformed_position.y, 240.0));
}

#[test]
fn vertical_line_uses_the_non_degenerate_axis() {
    let mut v = verts(&[(1.0, 0.0, 0.0), (1.0, 2.0, 0.0)]);
    let fit = fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(800, 600, 0.1)).unwrap();
    assert!(fit.scale.is_finite());
    assert!(approx(fit.scale, 240.0));
    let (x0, x1, y0, y1) = bounds(&v);
    assert!(approx(x0, 400.0) && approx(x1, 400.0));
    assert!(approx(y0, 60.0) && approx(y1, 540.0));
}

#[test]
fn margin_is_respected_in_different_viewports() {
    for (w, h, m) in [
        (800, 600, 0.1),
        (1024, 768, 0.05),
        (320, 240, 0.2),
        (300, 900, 0.0),
    ] {
        let mut v = verts(&[(-1.0, -1.0, 0.0), (1.0, 1.0, 0.0), (3.0, -2.0, 0.0)]);
        fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(w, h, m)).unwrap();
        let (x0, x1, y0, y1) = bounds(&v);
        let (mw, mh) = (w as f32 * m, h as f32 * m);
        assert!(
            x0 >= mw - 1e-3 && x1 <= w as f32 - mw + 1e-3,
            "{w}x{h} m={m}"
        );
        assert!(
            y0 >= mh - 1e-3 && y1 <= h as f32 - mh + 1e-3,
            "{w}x{h} m={m}"
        );
        // At least one axis fills the available area exactly.
        let fills_w = approx(x1 - x0, w as f32 - 2.0 * mw);
        let fills_h = approx(y1 - y0, h as f32 - 2.0 * mh);
        assert!(fills_w || fills_h);
    }
}

#[test]
fn model_far_from_origin_is_still_centered() {
    let mut v = verts(&[(100.0, 200.0, 0.0), (104.0, 203.0, 0.0)]);
    fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(200, 200, 0.1)).unwrap();
    let (x0, x1, y0, y1) = bounds(&v);
    assert!(approx((x0 + x1) / 2.0, 100.0) && approx((y0 + y1) / 2.0, 100.0));
}

#[test]
fn original_position_is_untouched_and_z_is_kept() {
    let mut v = verts(&[(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)]);
    fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(100, 100, 0.1)).unwrap();
    assert_eq!(v[0].position, Vector3::new(1.0, 2.0, 3.0));
    assert_eq!(v[0].transformed_position.z, 3.0);
}

#[test]
fn invalid_inputs_are_reported() {
    let mut empty: Vec<Vertex> = vec![];
    assert_eq!(
        fit_to_viewport(&mut empty, ViewPlane::XY, Viewport::new(10, 10, 0.1)).unwrap_err(),
        MappingError::NoVertices
    );
    let mut v = verts(&[(0.0, 0.0, 0.0)]);
    assert_eq!(
        fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(0, 10, 0.1)).unwrap_err(),
        MappingError::InvalidViewport
    );
    assert_eq!(
        fit_to_viewport(&mut v, ViewPlane::XY, Viewport::new(10, 10, 0.5)).unwrap_err(),
        MappingError::InvalidViewport
    );
    let mut nan = verts(&[(f32::NAN, 0.0, 0.0)]);
    assert_eq!(
        fit_to_viewport(&mut nan, ViewPlane::XY, Viewport::new(10, 10, 0.1)).unwrap_err(),
        MappingError::NonFinitePosition
    );
}

#[test]
fn ship_fits_inside_800x600_in_xz_view() {
    let mesh = load_obj(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/models/Laboratorio3.obj"
    )))
    .unwrap();
    let mut v = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    let fit = fit_to_viewport(&mut v, ViewPlane::XZ, Viewport::new(800, 600, 0.1)).unwrap();
    assert!(
        approx(fit.span_u, 5.66) && approx(fit.span_v, 6.2),
        "{fit:?}"
    );
    let (x0, x1, y0, y1) = bounds(&v);
    assert!(
        x0 >= 80.0 && x1 <= 720.0 && y0 >= 60.0 && y1 <= 540.0,
        "{x0} {x1} {y0} {y1}"
    );
    assert!(approx(y1 - y0, 480.0), "height-limited fit");
    assert!(approx((x0 + x1) / 2.0, 400.0) && approx((y0 + y1) / 2.0, 300.0));
}
