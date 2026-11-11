//! Nebula backdrop of the solar sky.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::PerspectiveCamera;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::nebula::{Nebula, draw_nebula};
use rust_software_renderer::solar::skybox::random_direction;

const W: u32 = 160;
const H: u32 = 120;

fn cam(eye: Vector3, dir: Vector3) -> PerspectiveCamera {
    PerspectiveCamera::new(eye, eye + dir, Vector3::new(0.0, 1.0, 0.0), 1.0, 0.05).unwrap()
}

fn painted(nebula: &Nebula, camera: &PerspectiveCamera) -> Framebuffer {
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    draw_nebula(&mut fb, nebula, camera);
    fb
}

fn directions(n: usize) -> Vec<Vector3> {
    let mut state = 5u32;
    (0..n).map(|_| random_direction(&mut state)).collect()
}

#[test]
fn the_palette_is_dark_red_black_and_never_outshines_the_bodies() {
    let nebula = Nebula::new(42, 1.0);
    let mut sum = 0.0;
    for d in directions(4000) {
        let c = nebula.color_at(d);
        assert!(c.x >= 0.0 && c.y >= 0.0 && c.z >= 0.0, "{c:?}");
        assert!(c.x.max(c.y).max(c.z) <= 0.5, "too bright: {c:?}");
        // Wine, corinth, purple and magenta: green stays the weakest.
        assert!(c.y < c.x && c.y < c.z, "{c:?}");
        sum += 0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
    }
    let mean = sum / 4000.0;
    assert!((0.01..0.08).contains(&mean), "mean luminance {mean}");
}

#[test]
fn the_sky_varies_and_the_map_follows_the_designed_colors() {
    let nebula = Nebula::new(42, 1.0);
    let colors: Vec<f32> = directions(2000)
        .iter()
        .map(|&d| nebula.color_at(d).x)
        .collect();
    let (lo, hi) = colors
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &c| (lo.min(c), hi.max(c)));
    assert!(hi - lo > 0.15, "clouds and gaps: red {lo}..{hi}");
    for d in directions(500) {
        let (direct, mapped) = (nebula.color_at(d), nebula.sample(d));
        assert!(
            (direct - mapped).length() < 0.03,
            "{direct:?} vs {mapped:?}"
        );
    }
}

#[test]
fn one_seed_fixes_the_sky_and_another_changes_it() {
    let (a, b, c) = (
        Nebula::new(42, 1.0),
        Nebula::new(42, 1.0),
        Nebula::new(7, 1.0),
    );
    let d = Vector3::new(0.3, 0.2, -0.93).normalized();
    assert_eq!(a.sample(d), b.sample(d));
    assert!(
        directions(50)
            .iter()
            .any(|&d| (a.color_at(d) - c.color_at(d)).length() > 0.01)
    );
}

#[test]
fn translating_the_camera_leaves_the_backdrop_unchanged_and_rotating_changes_it() {
    let nebula = Nebula::new(42, 1.0);
    let dir = Vector3::new(0.3, 0.1, -1.0);
    let a = painted(&nebula, &cam(Vector3::zero(), dir));
    let b = painted(&nebula, &cam(Vector3::new(40.0, -2.0, 7.0), dir));
    // Equal up to the rounding of `target - eye` far from the origin.
    let max_step = a
        .to_rgba_bytes()
        .iter()
        .zip(b.to_rgba_bytes())
        .map(|(x, y)| x.abs_diff(y))
        .max();
    assert!(max_step <= Some(1), "{max_step:?}");
    let c = painted(&nebula, &cam(Vector3::zero(), Vector3::new(-1.0, 0.4, 0.2)));
    assert_ne!(a.to_rgba_bytes(), c.to_rgba_bytes());
}

#[test]
fn the_backdrop_writes_color_only_over_the_whole_frame() {
    let fb = painted(
        &Nebula::new(42, 1.0),
        &cam(Vector3::zero(), Vector3::new(0.0, 0.0, -1.0)),
    );
    assert_eq!(fb.count_non_background(), (W * H) as usize);
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            assert_eq!(fb.get_depth(x, y), Some(f32::INFINITY));
        }
    }
}

#[test]
fn intensity_scales_the_color_and_zero_keeps_the_black_sky() {
    let camera = cam(Vector3::zero(), Vector3::new(0.2, -0.3, -1.0));
    assert_eq!(
        painted(&Nebula::new(42, 0.0), &camera).count_non_background(),
        0
    );
    let d = Vector3::new(0.2, -0.3, -1.0).normalized();
    let (full, half) = (Nebula::new(42, 1.0), Nebula::new(42, 0.5));
    assert!((full.sample(d) * 0.5 - half.sample(d)).length() < 1e-6);
    assert_eq!(Nebula::new(42, 3.0).intensity, 1.0, "clamped");
}
