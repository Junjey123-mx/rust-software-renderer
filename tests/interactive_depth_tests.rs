//! View depth carried by both fills, with unchanged coverage.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::rasterization::{RasterBounds, fill_barycentric, fill_scanline};
use rust_software_renderer::rendering::Fragment;
use std::collections::HashMap;

fn v(x: f32, y: f32, z: f32) -> Vertex {
    Vertex::new(Vector3::new(x, y, z))
}

const B: RasterBounds = RasterBounds {
    width: 100,
    height: 100,
};

fn by_pixel(frags: &[Fragment]) -> HashMap<(i32, i32), f32> {
    frags
        .iter()
        .map(|f| ((f.position.x as i32, f.position.y as i32), f.depth))
        .collect()
}

fn both(a: Vertex, b: Vertex, c: Vertex) -> (Vec<Fragment>, Vec<Fragment>) {
    (
        fill_barycentric(&a, &b, &c, B),
        fill_scanline(&a, &b, &c, B),
    )
}

#[test]
fn constant_depth_plane_gives_the_same_depth_everywhere() {
    let (bary, scan) = both(v(10.0, 10.0, 7.5), v(90.0, 20.0, 7.5), v(40.0, 85.0, 7.5));
    assert!(!bary.is_empty());
    assert!(bary.iter().all(|f| (f.depth - 7.5).abs() < 1e-4));
    assert!(scan.iter().all(|f| (f.depth - 7.5).abs() < 1e-4));
}

#[test]
fn inclined_plane_depth_equals_the_analytic_value_at_pixel_centers() {
    // z = x / 10 + y / 20 on a large triangle.
    let plane = |x: f32, y: f32| x / 10.0 + y / 20.0;
    let a = (5.0, 5.0);
    let b = (95.0, 10.0);
    let c = (20.0, 95.0);
    let (bary, scan) = both(
        v(a.0, a.1, plane(a.0, a.1)),
        v(b.0, b.1, plane(b.0, b.1)),
        v(c.0, c.1, plane(c.0, c.1)),
    );
    for f in bary.iter().chain(scan.iter()) {
        let expected = plane(f.position.x + 0.5, f.position.y + 0.5);
        assert!(
            (f.depth - expected).abs() < 1e-3,
            "({}, {}) depth {} expected {expected}",
            f.position.x,
            f.position.y,
            f.depth
        );
    }
    let db = by_pixel(&bary);
    let ds = by_pixel(&scan);
    assert_eq!(db.len(), ds.len());
    for (px, z) in &db {
        assert!((ds[px] - z).abs() < 1e-3, "{px:?}");
    }
}

#[test]
fn coverage_is_unchanged_by_depth_and_winding_does_not_change_depths() {
    let t09 = (v(10.5, 10.5, 1.0), v(60.5, 10.5, 2.0), v(10.5, 60.5, 3.0));
    let (bary, scan) = both(t09.0, t09.1, t09.2);
    assert_eq!(bary.len(), 1275);
    assert_eq!(scan.len(), 1275);
    let (rb, rs) = both(t09.0, t09.2, t09.1);
    let (db, ds, drb, drs) = (
        by_pixel(&bary),
        by_pixel(&scan),
        by_pixel(&rb),
        by_pixel(&rs),
    );
    assert_eq!(
        db.keys().collect::<std::collections::HashSet<_>>(),
        drb.keys().collect()
    );
    for (px, z) in &db {
        assert!(
            (drb[px] - z).abs() < 1e-4 && (ds[px] - z).abs() < 1e-4 && (drs[px] - z).abs() < 1e-4
        );
    }
    // Vertex (10,10) is covered: its depth is exactly the vertex depth.
    assert!((db[&(10, 10)] - 1.0).abs() < 1e-4);
}

#[test]
fn degenerate_and_non_finite_depth_inputs_emit_nothing() {
    let (b, s) = both(v(10.0, 10.0, 1.0), v(30.0, 30.0, 2.0), v(50.0, 50.0, 3.0));
    assert!(b.is_empty() && s.is_empty());
    let (b, s) = both(
        v(10.0, 10.0, f32::NAN),
        v(90.0, 20.0, 0.0),
        v(40.0, 85.0, 0.0),
    );
    // Positions are finite so coverage exists; depth is NaN and must be
    // rejected later by the depth buffer, never by changing coverage.
    assert_eq!(b.len(), s.len());
    assert!(b.iter().all(|f| f.depth.is_nan()));
}

#[test]
fn lab5_zero_depth_scene_still_reports_depth_zero() {
    let (b, s) = both(v(10.0, 10.0, 0.0), v(90.0, 20.0, 0.0), v(40.0, 85.0, 0.0));
    assert!(b.iter().chain(s.iter()).all(|f| f.depth == 0.0));
}
