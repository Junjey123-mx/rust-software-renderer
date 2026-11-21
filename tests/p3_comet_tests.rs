//! Comets of the Project 3 sky: objects that really travel (closed-form
//! ellipses of the simulated time), a tail of fixed length away from the
//! star, drawn behind the depth buffer and skipped when off the frame.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::PerspectiveCamera;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::comets::{CometField, draw_comets};

const STAR: Vector3 = Vector3::new(0.0, 0.0, 0.0);

fn field() -> CometField {
    CometField::project3(STAR)
}

#[test]
fn three_comets_that_move() {
    let f = field();
    assert_eq!(f.comets.len(), 3);
    for c in &f.comets {
        // Continuous and deterministic: the velocity is the derivative.
        for k in 0..50 {
            let t = k as f32 * 3.7;
            let h = 1e-2;
            let fd = (c.position(t + h) - c.position(t - h)) / (2.0 * h);
            assert!((fd - c.velocity(t)).length() < 0.05, "{fd:?}");
            assert_eq!(c.position(t), c.position(t));
            let speed = c.velocity(t).length();
            assert!((0.5..=6.0).contains(&speed), "{speed}");
        }
        // Clear of the star's shell (radius about 9.5) at the near end.
        let closest = (0..2000)
            .map(|k| c.position(k as f32 * 0.25).length())
            .fold(f32::INFINITY, f32::min);
        assert!(closest > 15.0, "{closest}");
    }
}

#[test]
fn tails_point_away_from_the_star_and_stay_bounded() {
    for c in &field().comets {
        for k in 0..40 {
            let t = k as f32 * 5.3;
            let p = c.position(t);
            let d = c.tail_dir(t, STAR);
            assert!((d.length() - 1.0).abs() < 1e-4);
            assert!(d.dot((p - STAR).normalized()) > 0.5);
            let end = c.tail_point(t, STAR, 1.0);
            let reach = c.tail + 0.6 * c.velocity(t).length();
            assert!((end - p).length() <= reach + 1e-3);
            assert_eq!(c.tail_point(t, STAR, 0.0), p);
        }
    }
}

/// A camera 30 units from comet 0 at time `t`, looking at it.
fn camera_on_comet(t: f32) -> PerspectiveCamera {
    let c = field().comets[0];
    let p = c.position(t);
    let side = c.velocity(t).cross(p - STAR).normalized();
    PerspectiveCamera::new(
        p + side * 30.0,
        p,
        Vector3::new(0.0, 1.0, 0.0),
        std::f32::consts::FRAC_PI_3,
        0.1,
    )
    .unwrap()
}

fn lit(fb: &Framebuffer) -> usize {
    fb.to_rgba_bytes()
        .chunks(4)
        .filter(|p| p[0] > 20 || p[1] > 20 || p[2] > 20)
        .count()
}

/// Screen centroid of the lit pixels.
fn centroid(fb: &Framebuffer) -> (f32, f32) {
    let (w, mut sx, mut sy, mut n) = (fb.width() as usize, 0.0, 0.0, 0.0);
    for (i, p) in fb.to_rgba_bytes().chunks(4).enumerate() {
        if p[0] > 200 && p[1] > 200 && p[2] > 200 {
            sx += (i % w) as f32;
            sy += (i / w) as f32;
            n += 1.0;
        }
    }
    (sx / n, sy / n)
}

#[test]
fn drawn_where_it_is_and_moving() {
    let camera = camera_on_comet(10.0);
    let mut a = Framebuffer::new(320, 240);
    a.clear();
    assert!(draw_comets(&mut a, &field(), &camera, STAR, 10.0) >= 1);
    assert!(lit(&a) > 40, "{}", lit(&a));
    // Two seconds later, seen by the same camera, it has moved on screen.
    let mut b = Framebuffer::new(320, 240);
    b.clear();
    draw_comets(&mut b, &field(), &camera, STAR, 12.0);
    let (ca, cb) = (centroid(&a), centroid(&b));
    let moved = (ca.0 - cb.0).hypot(ca.1 - cb.1);
    assert!(moved > 5.0, "{ca:?} -> {cb:?}");
}

#[test]
fn hidden_behind_nearer_geometry() {
    let camera = camera_on_comet(10.0);
    let mut fb = Framebuffer::new(320, 240);
    fb.clear();
    // Something drawn one unit from the camera everywhere.
    for y in 0..240 {
        for x in 0..320 {
            fb.set_depth(x, y, 1.0);
        }
    }
    draw_comets(&mut fb, &field(), &camera, STAR, 10.0);
    assert_eq!(lit(&fb), 0);
}

#[test]
fn skipped_when_off_the_frame() {
    // The same camera turned around: no comet is drawn.
    let c = field().comets[0];
    let p = c.position(10.0);
    let side = c.velocity(10.0).cross(p - STAR).normalized();
    let eye = p + side * 30.0;
    let away = PerspectiveCamera::new(
        eye,
        eye + side * 10.0,
        Vector3::new(0.0, 1.0, 0.0),
        0.3,
        0.1,
    )
    .unwrap();
    let mut fb = Framebuffer::new(320, 240);
    fb.clear();
    let drawn = draw_comets(&mut fb, &field(), &away, STAR, 10.0);
    assert!(drawn < 3);
    let comet0_only = CometField { comets: vec![c] };
    let mut fb = Framebuffer::new(320, 240);
    fb.clear();
    assert_eq!(draw_comets(&mut fb, &comet0_only, &away, STAR, 10.0), 0);
    assert_eq!(lit(&fb), 0);
}
