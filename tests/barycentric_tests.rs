//! Independent numeric checks of the barycentric fill. Expected coordinates
//! are derived by hand from the top-left rule, not from the implementation.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::rasterization::{RasterBounds, fill_barycentric};
use rust_software_renderer::rendering::Fragment;
use std::collections::HashSet;

fn v(x: f32, y: f32) -> Vertex {
    Vertex::new(Vector3::new(x, y, 0.0))
}

fn fill(a: (f32, f32), b: (f32, f32), c: (f32, f32), w: u32, h: u32) -> Vec<Fragment> {
    fill_barycentric(
        &v(a.0, a.1),
        &v(b.0, b.1),
        &v(c.0, c.1),
        RasterBounds::new(w, h),
    )
}

fn pixels(frags: &[Fragment]) -> HashSet<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

fn assert_valid(frags: &[Fragment], w: u32, h: u32) {
    let set = pixels(frags);
    assert_eq!(set.len(), frags.len(), "duplicate pixels");
    for (x, y) in &set {
        assert!(
            *x >= 0 && *x < w as i32 && *y >= 0 && *y < h as i32,
            "({x},{y}) outside"
        );
    }
}

#[test]
fn small_right_triangle_has_exact_hand_computed_coverage() {
    // Oriented: (0,0)->(0,4) down edge (owned), (4,0)->(0,0) leftward top (owned),
    // hypotenuse goes up (excluded). Centers with x + y < 3.
    let frags = fill((0.0, 0.0), (4.0, 0.0), (0.0, 4.0), 10, 10);
    let expected: HashSet<(i32, i32)> = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (0, 2)].into();
    assert_eq!(pixels(&frags), expected);
    assert_valid(&frags, 10, 10);
}

#[test]
fn subpixel_edges_on_centers_follow_top_left_rule() {
    // T09: vertices on pixel centers. Left column 10 and top row 10 owned,
    // hypotenuse x + y = 71 excluded -> centers with x + y <= 69.
    let frags = fill((10.5, 10.5), (60.5, 10.5), (10.5, 60.5), 100, 100);
    let set = pixels(&frags);
    assert_eq!(frags.len(), 1275);
    assert!(set.contains(&(10, 10)));
    assert!(set.contains(&(59, 10)) && set.contains(&(10, 59)));
    assert!(!set.contains(&(60, 10)) && !set.contains(&(10, 60)));
    assert!(set.iter().all(|(x, y)| x + y <= 69 && *x >= 10 && *y >= 10));
    assert_valid(&frags, 100, 100);
}

#[test]
fn large_triangle_covers_interior_points_only() {
    let frags = fill((10.0, 10.0), (90.0, 20.0), (40.0, 85.0), 100, 100);
    let set = pixels(&frags);
    assert!(set.contains(&(46, 38)), "centroid");
    assert!(set.contains(&(20, 15)));
    assert!(!set.contains(&(80, 80)));
    assert!(!set.contains(&(5, 5)));
    // Area = 2850; pixel count must be close.
    assert!((2750..=2950).contains(&frags.len()), "{}", frags.len());
    assert_valid(&frags, 100, 100);
}

#[test]
fn horizontal_top_edge_row_is_included() {
    // (10,20),(70,20),(40,75): the horizontal edge y=20 is a top edge; centers
    // at y=20.5 are inside the triangle anyway. Row 20 spans x in [10,69].
    let frags = fill((10.0, 20.0), (70.0, 20.0), (40.0, 75.0), 100, 100);
    let set = pixels(&frags);
    let row20: Vec<i32> = (0..100).filter(|x| set.contains(&(*x, 20))).collect();
    assert_eq!(row20.first(), Some(&10));
    assert_eq!(row20.last(), Some(&69));
    assert_eq!(row20.len(), 60);
    assert!(!set.iter().any(|(_, y)| *y < 20));
    assert_valid(&frags, 100, 100);
}

#[test]
fn vertical_left_edge_column_is_included() {
    let frags = fill((20.0, 10.0), (20.0, 80.0), (75.0, 40.0), 100, 100);
    let set = pixels(&frags);
    assert!(set.contains(&(20, 40)));
    assert!(!set.iter().any(|(x, _)| *x < 20));
    assert!(set.contains(&(70, 40)));
    assert_valid(&frags, 100, 100);
}

#[test]
fn winding_does_not_change_coverage() {
    let a = fill((10.0, 10.0), (90.0, 20.0), (40.0, 85.0), 100, 100);
    let b = fill((10.0, 10.0), (40.0, 85.0), (90.0, 20.0), 100, 100);
    let c = fill((90.0, 20.0), (10.0, 10.0), (40.0, 85.0), 100, 100);
    assert_eq!(pixels(&a), pixels(&b));
    assert_eq!(pixels(&a), pixels(&c));
}

#[test]
fn degenerate_and_invalid_inputs_emit_nothing() {
    assert!(fill((10.0, 10.0), (30.0, 30.0), (50.0, 50.0), 100, 100).is_empty());
    assert!(fill((20.0, 20.0), (20.0, 20.0), (60.0, 50.0), 100, 100).is_empty());
    assert!(fill((f32::NAN, 1.0), (5.0, 1.0), (1.0, 5.0), 100, 100).is_empty());
    assert!(fill((f32::INFINITY, 1.0), (5.0, 1.0), (1.0, 5.0), 100, 100).is_empty());
    assert!(fill((10.0, 10.0), (90.0, 20.0), (40.0, 85.0), 0, 100).is_empty());
    assert!(fill((10.0, 10.0), (90.0, 20.0), (40.0, 85.0), 100, 0).is_empty());
}

#[test]
fn triangles_outside_are_clipped_or_empty() {
    assert!(fill((200.0, 200.0), (300.0, 200.0), (200.0, 300.0), 100, 100).is_empty());
    assert!(fill((-50.0, -50.0), (-10.0, -50.0), (-50.0, -10.0), 100, 100).is_empty());
    // Partially outside: every emitted pixel is inside the framebuffer and
    // the visible interior is present.
    let frags = fill((-15.0, 20.0), (50.0, 15.0), (40.0, 115.0), 100, 100);
    assert_valid(&frags, 100, 100);
    assert!(pixels(&frags).contains(&(20, 40)));
    assert!(pixels(&frags).contains(&(0, 30)), "column 0 visible");
    // Huge but finite coordinates do not blow up the scan.
    let frags = fill((-1e9, -1e9), (1e9, -1e9), (0.0, 1e9), 64, 64);
    assert_eq!(frags.len(), 64 * 64, "framebuffer fully covered");
}

#[test]
fn fragments_keep_color_depth_and_inputs() {
    let red = Vector3::new(1.0, 0.0, 0.0);
    let a = Vertex::new_with_color(Vector3::new(2.0, 2.0, 0.0), red);
    let b = v(12.0, 2.0);
    let c = v(2.0, 12.0);
    let frags = fill_barycentric(&a, &b, &c, RasterBounds::new(20, 20));
    assert!(!frags.is_empty());
    assert!(frags.iter().all(|f| f.color == red && f.depth == 0.0));
    assert_eq!(a.transformed_position, Vector3::new(2.0, 2.0, 0.0));
}
