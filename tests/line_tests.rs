use raylib::math::{Vector2, Vector3};
use rust_software_renderer::rasterization::line;

const WHITE: Vector3 = Vector3 {
    x: 1.0,
    y: 1.0,
    z: 1.0,
};

fn points(a: (f32, f32), b: (f32, f32)) -> Vec<(i32, i32)> {
    line(Vector2::new(a.0, a.1), Vector2::new(b.0, b.1), WHITE)
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

#[test]
fn horizontal_line_covers_every_x() {
    assert_eq!(
        points((0.0, 0.0), (5.0, 0.0)),
        vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]
    );
}

#[test]
fn vertical_line_covers_every_y() {
    assert_eq!(
        points((0.0, 0.0), (0.0, 5.0)),
        vec![(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]
    );
}

#[test]
fn diagonal_line_steps_both_axes() {
    assert_eq!(
        points((0.0, 0.0), (5.0, 5.0)),
        vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]
    );
}

#[test]
fn negative_slope_line() {
    assert_eq!(
        points((5.0, 0.0), (0.0, 5.0)),
        vec![(5, 0), (4, 1), (3, 2), (2, 3), (1, 4), (0, 5)]
    );
}

#[test]
fn steep_line_has_one_pixel_per_row_and_is_connected() {
    let p = points((2.0, 2.0), (3.0, 10.0));
    assert_eq!(p.len(), 9);
    assert_eq!(p.first(), Some(&(2, 2)));
    assert_eq!(p.last(), Some(&(3, 10)));
    for w in p.windows(2) {
        assert!((w[1].0 - w[0].0).abs() <= 1 && (w[1].1 - w[0].1).abs() <= 1);
    }
}

#[test]
fn reversed_endpoints_cover_the_same_pixels() {
    let mut forward = points((5.0, 0.0), (0.0, 0.0));
    let mut backward = points((0.0, 0.0), (5.0, 0.0));
    assert_eq!(forward.first(), Some(&(5, 0)));
    forward.sort();
    backward.sort();
    assert_eq!(forward, backward);

    let mut a = points((-3.0, 7.0), (12.0, -4.0));
    let mut b = points((12.0, -4.0), (-3.0, 7.0));
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn degenerate_segment_is_a_single_pixel() {
    assert_eq!(points((7.0, 7.0), (7.0, 7.0)), vec![(7, 7)]);
}

#[test]
fn negative_coordinates_are_rasterized() {
    assert_eq!(
        points((-2.0, -2.0), (0.0, 0.0)),
        vec![(-2, -2), (-1, -1), (0, 0)]
    );
}

#[test]
fn fractional_endpoints_are_rounded() {
    assert_eq!(
        points((0.4, 0.6), (2.5, 0.6)),
        vec![(0, 1), (1, 1), (2, 1), (3, 1)]
    );
}

#[test]
fn fragments_keep_color_and_zero_depth() {
    let red = Vector3::new(1.0, 0.0, 0.0);
    let frags = line(Vector2::new(0.0, 0.0), Vector2::new(3.0, 1.0), red);
    assert!(frags.iter().all(|f| f.color == red && f.depth == 0.0));
}

#[test]
fn non_finite_input_produces_nothing() {
    assert!(line(Vector2::new(f32::NAN, 0.0), Vector2::new(1.0, 1.0), WHITE).is_empty());
    assert!(
        line(
            Vector2::new(0.0, 0.0),
            Vector2::new(f32::INFINITY, 1.0),
            WHITE
        )
        .is_empty()
    );
}
