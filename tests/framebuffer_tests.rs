use raylib::math::Vector3;
use rust_software_renderer::rendering::{Fragment, Framebuffer};

const RED: Vector3 = Vector3 {
    x: 1.0,
    y: 0.0,
    z: 0.0,
};
const GREEN: Vector3 = Vector3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};
const BLUE: Vector3 = Vector3 {
    x: 0.0,
    y: 0.0,
    z: 1.0,
};
const YELLOW: Vector3 = Vector3 {
    x: 1.0,
    y: 1.0,
    z: 0.0,
};

#[test]
fn fragment_constructor_stores_fields() {
    let f = Fragment::new(100.0, 80.0, RED, 0.5);
    assert_eq!(f.position.x, 100.0);
    assert_eq!(f.position.y, 80.0);
    assert_eq!(f.color, RED);
    assert_eq!(f.depth, 0.5);
}

#[test]
fn dimensions_and_initial_background() {
    let fb = Framebuffer::new(800, 600);
    assert_eq!(fb.width(), 800);
    assert_eq!(fb.height(), 600);
    assert_eq!(fb.get_pixel(0, 0), Some(Vector3::zero()));
    assert_eq!(fb.get_pixel(799, 599), Some(Vector3::zero()));
    assert_eq!(fb.count_non_background(), 0);
}

#[test]
fn lesson_pixels_red_green_blue_and_overwrite() {
    let mut fb = Framebuffer::new(200, 100);
    fb.point(100, 80, RED);
    fb.point(101, 80, GREEN);
    fb.point(102, 80, BLUE);
    assert_eq!(fb.get_pixel(100, 80), Some(RED));
    assert_eq!(fb.get_pixel(101, 80), Some(GREEN));
    assert_eq!(fb.get_pixel(102, 80), Some(BLUE));

    fb.point(100, 80, YELLOW);
    assert_eq!(fb.get_pixel(100, 80), Some(YELLOW), "last write wins");
    assert_eq!(fb.count_non_background(), 3);
}

#[test]
fn out_of_bounds_writes_are_discarded_without_panic() {
    let mut fb = Framebuffer::new(10, 10);
    fb.point(-1, 80, RED);
    fb.point(5, -1, RED);
    fb.point(10, 5, RED);
    fb.point(5, 10, RED);
    fb.point(i32::MIN, i32::MAX, RED);
    assert_eq!(fb.count_non_background(), 0);
    assert_eq!(fb.get_pixel(-1, 0), None);
    assert_eq!(fb.get_pixel(10, 0), None);
}

#[test]
fn row_major_index_distinguishes_x_and_y() {
    let mut fb = Framebuffer::new(4, 3);
    fb.point(1, 2, RED);
    let bytes = fb.to_rgba_bytes();
    let offset = (2 * 4 + 1) * 4;
    assert_eq!(&bytes[offset..offset + 4], &[255, 0, 0, 255]);
    assert_eq!(bytes.len(), 4 * 3 * 4);
    // (2, 1) must stay untouched.
    assert_eq!(fb.get_pixel(2, 1), Some(Vector3::zero()));
}

#[test]
fn clear_restores_background_color() {
    let mut fb = Framebuffer::new(4, 4);
    fb.set_background_color(Vector3::new(0.1, 0.2, 0.3));
    fb.point(1, 1, RED);
    fb.clear();
    assert_eq!(fb.get_pixel(1, 1), Some(Vector3::new(0.1, 0.2, 0.3)));
    assert_eq!(fb.count_non_background(), 0);
}

#[test]
fn rgba_conversion_clamps_and_rounds_once() {
    let mut fb = Framebuffer::new(3, 1);
    fb.point(0, 0, Vector3::new(0.5, 2.0, -1.0));
    fb.point(1, 0, Vector3::new(1.0, 1.0, 1.0));
    let bytes = fb.to_rgba_bytes();
    assert_eq!(&bytes[0..4], &[128, 255, 0, 255]);
    assert_eq!(&bytes[4..8], &[255, 255, 255, 255]);
    assert_eq!(&bytes[8..12], &[0, 0, 0, 255]);
}
