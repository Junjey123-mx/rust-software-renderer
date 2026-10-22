//! Independent checks of the scanline fill using hand-derived row spans.
//! These never consult the barycentric implementation.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::rasterization::{RasterBounds, fill_scanline};
use rust_software_renderer::rendering::Fragment;
use std::collections::{BTreeMap, HashSet};

fn v(x: f32, y: f32) -> Vertex {
    Vertex::new(Vector3::new(x, y, 0.0))
}

fn fill(a: (f32, f32), b: (f32, f32), c: (f32, f32), w: u32, h: u32) -> Vec<Fragment> {
    fill_scanline(
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

/// Per row: (first x, last x, count). Spans must be contiguous.
fn rows(frags: &[Fragment]) -> BTreeMap<i32, (i32, i32, usize)> {
    let mut m: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for f in frags {
        m.entry(f.position.y as i32)
            .or_default()
            .push(f.position.x as i32);
    }
    m.into_iter()
        .map(|(y, mut xs)| {
            xs.sort();
            let (lo, hi) = (xs[0], xs[xs.len() - 1]);
            assert_eq!(xs.len() as i32, hi - lo + 1, "row {y} has a gap: {xs:?}");
            (y, (lo, hi, xs.len()))
        })
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
    rows(frags);
}

#[test]
fn right_triangle_rows_shrink_by_one_pixel_each_row() {
    // (0,0),(4,0),(0,4): row y covers x in [0, 2 - y] (hypotenuse excluded).
    let frags = fill((0.0, 0.0), (4.0, 0.0), (0.0, 4.0), 10, 10);
    let r = rows(&frags);
    assert_eq!(r.get(&0), Some(&(0, 2, 3)));
    assert_eq!(r.get(&1), Some(&(0, 1, 2)));
    assert_eq!(r.get(&2), Some(&(0, 0, 1)));
    assert_eq!(r.len(), 3);
    assert_valid(&frags, 10, 10);
}

#[test]
fn vertex_exactly_on_a_row_center_is_counted_once() {
    // Middle vertex (30, 20.5) lies on the sample line of row 20: the span
    // on that row runs from the long edge to the vertex, no double count.
    let frags = fill((10.0, 0.5), (30.0, 20.5), (10.0, 40.5), 100, 100);
    let r = rows(&frags);
    // Long edge is x = 10 (owned); right boundary at row 20 is the vertex x=30.
    assert_eq!(r.get(&20), Some(&(10, 29, 20)));
    assert_eq!(r.get(&10).map(|s| s.0), Some(10));
    assert_valid(&frags, 100, 100);
}

#[test]
fn horizontal_top_edge_row_spans_the_whole_edge() {
    let frags = fill((10.0, 20.0), (70.0, 20.0), (40.0, 75.0), 100, 100);
    let r = rows(&frags);
    assert_eq!(r.get(&20), Some(&(10, 69, 60)));
    assert_eq!(r.keys().next(), Some(&20));
    // Row 74 (sample 74.5) spans [39.73, 40.27]: no pixel center inside.
    assert_eq!(r.keys().next_back(), Some(&73));
    assert_valid(&frags, 100, 100);
}

#[test]
fn horizontal_bottom_edge_on_sample_line_is_excluded() {
    // Bottom edge at y = 60.5 lies exactly on row 60's sample line and is
    // not owned; top vertex at 10.5 is on row 10's line and is excluded.
    let frags = fill((35.5, 10.5), (10.5, 60.5), (60.5, 60.5), 100, 100);
    let r = rows(&frags);
    assert!(!r.contains_key(&60));
    assert!(!r.contains_key(&10), "apex alone is not covered");
    assert!(r.contains_key(&59) && r.contains_key(&11));
    assert_valid(&frags, 100, 100);
}

#[test]
fn vertical_edge_column_and_pointed_right_side() {
    let frags = fill((20.0, 10.0), (20.0, 80.0), (75.0, 40.0), 100, 100);
    let r = rows(&frags);
    assert!(
        r.values().all(|s| s.0 == 20),
        "every span starts at the owned column"
    );
    assert_eq!(r.get(&40).map(|s| s.1), Some(73));
    assert_valid(&frags, 100, 100);
}

#[test]
fn winding_does_not_change_spans() {
    let a = fill((10.0, 10.0), (90.0, 20.0), (40.0, 85.0), 100, 100);
    let b = fill((10.0, 10.0), (40.0, 85.0), (90.0, 20.0), 100, 100);
    let c = fill((40.0, 85.0), (90.0, 20.0), (10.0, 10.0), 100, 100);
    assert_eq!(pixels(&a), pixels(&b));
    assert_eq!(pixels(&a), pixels(&c));
    assert!((2750..=2950).contains(&a.len()));
}

#[test]
fn small_and_large_triangles_have_contiguous_rows() {
    for (a, b, c) in [
        ((10.0, 10.0), (18.0, 12.0), (12.0, 20.0)),
        ((10.0, 10.0), (90.0, 20.0), (40.0, 85.0)),
    ] {
        let frags = fill(a, b, c, 100, 100);
        assert!(!frags.is_empty());
        assert_valid(&frags, 100, 100);
    }
}

#[test]
fn clipping_at_every_border_keeps_pixels_inside() {
    let cases = [
        ((-15.0, 20.0), (50.0, 15.0), (40.0, 115.0)),
        ((50.0, -20.0), (120.0, 50.0), (50.0, 60.0)),
        ((-30.0, -30.0), (130.0, -30.0), (50.0, 130.0)),
        ((90.0, 10.0), (140.0, 50.0), (90.0, 90.0)),
    ];
    for (a, b, c) in cases {
        let frags = fill(a, b, c, 100, 100);
        assert!(!frags.is_empty(), "{a:?} {b:?} {c:?}");
        assert_valid(&frags, 100, 100);
    }
    let r = rows(&fill((-15.0, 20.0), (50.0, 15.0), (40.0, 115.0), 100, 100));
    assert_eq!(r.get(&30).map(|s| s.0), Some(0));
    assert_eq!(r.keys().next_back(), Some(&99));
}

#[test]
fn degenerate_outside_and_invalid_inputs_emit_nothing() {
    assert!(fill((10.0, 10.0), (30.0, 30.0), (50.0, 50.0), 100, 100).is_empty());
    assert!(fill((20.0, 20.0), (20.0, 20.0), (60.0, 50.0), 100, 100).is_empty());
    assert!(fill((f32::NAN, 1.0), (5.0, 1.0), (1.0, 5.0), 100, 100).is_empty());
    assert!(fill((1.0, f32::NEG_INFINITY), (5.0, 1.0), (1.0, 5.0), 100, 100).is_empty());
    assert!(fill((200.0, 200.0), (300.0, 200.0), (200.0, 300.0), 100, 100).is_empty());
    assert!(fill((10.0, 10.0), (90.0, 20.0), (40.0, 85.0), 0, 100).is_empty());
    assert!(fill((10.0, 10.0), (90.0, 20.0), (40.0, 85.0), 100, 0).is_empty());
    let huge = fill((-1e9, -1e9), (1e9, -1e9), (0.0, 1e9), 64, 64);
    assert_eq!(huge.len(), 64 * 64);
}

#[test]
fn fragments_keep_color_and_depth() {
    let green = Vector3::new(0.0, 1.0, 0.0);
    let a = Vertex::new_with_color(Vector3::new(2.0, 2.0, 0.0), green);
    let frags = fill_scanline(&a, &v(12.0, 2.0), &v(2.0, 12.0), RasterBounds::new(20, 20));
    assert!(!frags.is_empty());
    assert!(frags.iter().all(|f| f.color == green && f.depth == 0.0));
}
