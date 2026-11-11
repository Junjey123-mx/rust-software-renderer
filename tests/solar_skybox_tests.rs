//! Procedural star field.

use rust_software_renderer::solar::skybox::{MAX_STARS, StarField, random01};

#[test]
fn the_generator_is_reproducible_and_in_unit_range() {
    let (mut a, mut b) = (42u32, 42u32);
    for _ in 0..1000 {
        let x = random01(&mut a);
        assert_eq!(x, random01(&mut b));
        assert!((0.0..1.0).contains(&x));
    }
}

#[test]
fn the_same_seed_gives_the_same_sky_and_another_seed_a_different_one() {
    let a = StarField::generate(42, 2000);
    assert_eq!(a, StarField::generate(42, 2000));
    let b = StarField::generate(7, 2000);
    assert_ne!(a.stars[0].direction, b.stars[0].direction);
    assert_eq!(a.stars.len(), 2000);
}

#[test]
fn stars_are_unit_directions_with_bounded_brightness_and_tint() {
    let f = StarField::generate(42, 2000);
    for s in &f.stars {
        assert!((s.direction.length() - 1.0).abs() < 1e-4);
        assert!((0.3..=1.0).contains(&s.brightness));
        let c = s.color();
        assert!(c.x <= 1.0 && c.y <= 1.0 && c.z <= 1.0 && c.x > 0.0);
    }
    let bright = f.stars.iter().filter(|s| s.brightness > 0.8).count();
    assert!(bright * 4 < f.stars.len(), "most stars are faint: {bright}");
}

#[test]
fn directions_are_spread_evenly_over_the_sphere() {
    let f = StarField::generate(42, 4000);
    let n = f.stars.len() as f32;
    let mean = f
        .stars
        .iter()
        .fold(raylib::math::Vector3::zero(), |acc, s| acc + s.direction)
        / n;
    assert!(mean.length() < 0.05, "{mean:?}");
    // Equal-area bands in y hold about the same number of stars.
    let upper = f.stars.iter().filter(|s| s.direction.y > 0.5).count() as f32;
    let middle = f
        .stars
        .iter()
        .filter(|s| s.direction.y.abs() <= 0.5)
        .count() as f32;
    assert!((upper / n - 0.25).abs() < 0.03, "{upper}");
    assert!((middle / n - 0.5).abs() < 0.03, "{middle}");
}

#[test]
fn the_count_is_clamped() {
    assert_eq!(
        StarField::generate(1, MAX_STARS + 10).stars.len(),
        MAX_STARS
    );
    assert!(StarField::generate(1, 0).stars.is_empty());
}

// ---------------------------------------------------------------------------
// Drawing the star field
// ---------------------------------------------------------------------------

use raylib::math::Vector3;
use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, PerspectiveCamera, ShadedOptions, Transform, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::camera::CameraRig;
use rust_software_renderer::solar::skybox::{
    GLINT_STAR, MAX_FOOTPRINT, SPECTRAL, StarSize, draw_stars, spectral_tint,
};
use rust_software_renderer::solar::{BodyInstance, CameraMode, SolarMeshes, SolarScene};

const W: u32 = 320;
const H: u32 = 240;

fn cam(eye: Vector3, dir: Vector3) -> PerspectiveCamera {
    PerspectiveCamera::new(eye, eye + dir, Vector3::new(0.0, 1.0, 0.0), 1.0, 0.05).unwrap()
}

fn sky(c: &PerspectiveCamera) -> (Framebuffer, usize) {
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    let n = draw_stars(&mut fb, &StarField::generate(42, 2000), c);
    (fb, n)
}

fn bytes(fb: &Framebuffer) -> Vec<u8> {
    fb.to_rgba_bytes()
}

#[test]
fn translating_the_camera_leaves_the_sky_unchanged_and_rotating_changes_it() {
    let dir = Vector3::new(0.3, 0.1, -1.0);
    let (a, n) = sky(&cam(Vector3::zero(), dir));
    let (b, _) = sky(&cam(Vector3::new(50.0, -3.0, 9.0), dir));
    assert!(n > 50, "{n} stars in view");
    assert_eq!(bytes(&a), bytes(&b));
    let (c, _) = sky(&cam(Vector3::zero(), Vector3::new(1.0, 0.0, 0.0)));
    assert_ne!(bytes(&a), bytes(&c));
    // Turning back to the first orientation shows the same stars again.
    let (d, _) = sky(&cam(Vector3::new(1.0, 1.0, 1.0), dir));
    assert_eq!(bytes(&a), bytes(&d));
}

#[test]
fn stars_write_color_only_and_stay_inside_the_framebuffer() {
    let (fb, n) = sky(&cam(Vector3::zero(), Vector3::new(0.0, 0.0, -1.0)));
    assert!(n > 0);
    // Bright stars light a small footprint around their pixel.
    assert!(fb.count_non_background() > 0 && fb.count_non_background() <= n * MAX_FOOTPRINT);
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            assert_eq!(fb.get_depth(x, y), Some(f32::INFINITY));
        }
    }
    // A very wide view puts stars at the borders without any panic.
    let wide = PerspectiveCamera::new(
        Vector3::zero(),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, 1.0, 0.0),
        3.1,
        0.05,
    )
    .unwrap();
    let (_, m) = sky(&wide);
    assert!(m > n, "{m} vs {n}");
}

#[test]
fn bodies_hide_the_stars_behind_them() {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Front),
        2.0,
        Viewport::new(W, H, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::new(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 0.0, -1.0),
    );
    s.camera_mode = CameraMode::Perspective;
    s.rig = CameraRig::Orbit;
    s.stars = StarField::generate(3, 20_000);
    s.ship.position = Vector3::new(0.0, 0.0, 500.0);
    let opts = ShadedOptions {
        threads: 1,
        ..ShadedOptions::default()
    };
    let mut sky_only = Framebuffer::new(W, H);
    s.render_frame(&mut sky_only, TriangleMode::FilledScanline, opts)
        .unwrap();
    s.bodies = vec![BodyInstance::new(
        Body::HouseOfCards,
        Transform::new(Vector3::zero(), 1.5),
        0.0,
    )];
    let mut with_body = Framebuffer::new(W, H);
    s.render_frame(&mut with_body, TriangleMode::FilledScanline, opts)
        .unwrap();
    let (cx, cy) = (W as i32 / 2, H as i32 / 2);
    let mut covered = 0;
    for y in cy - 15..cy + 15 {
        for x in cx - 15..cx + 15 {
            let star = sky_only.get_pixel(x, y).unwrap();
            if star != sky_only.background_color() {
                covered += 1;
                assert_ne!(
                    with_body.get_pixel(x, y),
                    Some(star),
                    "star over the body at ({x}, {y})"
                );
            }
        }
    }
    assert!(covered > 0, "some stars lay behind the body");
}

// ---------------------------------------------------------------------------
// Density by noise
// ---------------------------------------------------------------------------

use rust_software_renderer::solar::skybox::{MIN_DENSITY, StarDensity, density_at};

#[test]
fn clustered_fields_are_reproducible_and_keep_fewer_stars_than_attempts() {
    let a = StarField::generate_with(42, 8000, StarDensity::Clustered);
    assert_eq!(
        a,
        StarField::generate_with(42, 8000, StarDensity::Clustered)
    );
    assert!(a.stars.len() < 8000);
    assert!(
        a.stars.len() as f32 > 8000.0 * MIN_DENSITY,
        "{}",
        a.stars.len()
    );
    assert_ne!(
        a,
        StarField::generate_with(43, 8000, StarDensity::Clustered)
    );
    assert_eq!(
        StarField::generate_with(42, 2000, StarDensity::Uniform),
        StarField::generate(42, 2000)
    );
    assert!(
        StarField::generate_with(42, 0, StarDensity::Clustered)
            .stars
            .is_empty()
    );
}

#[test]
fn noise_density_creates_denser_and_sparser_regions_than_uniform() {
    // The kept stars must sit where the density field is high: their mean
    // keep probability is clearly above the mean over uniform candidates.
    let noise_seed = 42u32 ^ 0x5bd1_e995;
    let mean = |f: &StarField| {
        f.stars
            .iter()
            .map(|s| density_at(s.direction, noise_seed))
            .sum::<f32>()
            / f.stars.len() as f32
    };
    let uniform = mean(&StarField::generate_with(42, 20_000, StarDensity::Uniform));
    let clustered = mean(&StarField::generate_with(
        42,
        20_000,
        StarDensity::Clustered,
    ));
    assert!(
        clustered > uniform + 0.03,
        "clustered {clustered} vs uniform {uniform}"
    );
    // And the field really varies: some candidates are near the floor, some
    // well above it.
    let mut state = 5u32;
    let (mut lo, mut hi) = (1.0f32, 0.0f32);
    for _ in 0..5000 {
        let d = density_at(
            rust_software_renderer::solar::skybox::random_direction(&mut state),
            noise_seed,
        );
        lo = lo.min(d);
        hi = hi.max(d);
    }
    assert!(hi - lo > 0.4, "density range {lo}..{hi}");
}

#[test]
fn density_stays_within_its_bounds() {
    let mut state = 9u32;
    for _ in 0..2000 {
        let d = rust_software_renderer::solar::skybox::random_direction(&mut state);
        let p = density_at(d, 77);
        assert!((MIN_DENSITY..=1.0).contains(&p), "{p}");
    }
    assert!("clustered".parse::<StarDensity>().is_ok());
    assert!(
        "nebula"
            .parse::<StarDensity>()
            .unwrap_err()
            .contains("nebula")
    );
}

// ---------------------------------------------------------------------------
// Spectral classes and footprints
// ---------------------------------------------------------------------------

#[test]
fn every_spectral_class_appears_in_its_share() {
    let f = StarField::generate(42, 10_000);
    let mut previous = 0.0;
    for (tint, share) in SPECTRAL {
        let n = f.stars.iter().filter(|s| s.tint == tint).count() as f32;
        let expected = share - previous;
        assert!(
            (n / 10_000.0 - expected).abs() < 0.02,
            "{tint:?}: {n} for a share of {expected}"
        );
        previous = share;
    }
    assert_eq!(spectral_tint(0.0), SPECTRAL[0].0);
    assert_eq!(spectral_tint(0.999), SPECTRAL[4].0);
    // Red stars are red and blue-white ones bluish.
    let (blue, red) = (SPECTRAL[0].0, SPECTRAL[4].0);
    assert!(blue.z > blue.x && red.x > 2.0 * red.z);
}

#[test]
fn few_stars_are_bright_and_glints_are_rare() {
    let f = StarField::generate(42, 10_000);
    let share = |size| f.stars.iter().filter(|s| s.size() == size).count() as f32 / 10_000.0;
    let (bright, glint) = (share(StarSize::Bright), share(StarSize::Glint));
    assert!((0.02..0.07).contains(&bright), "bright {bright}");
    assert!((0.001..0.015).contains(&glint), "glint {glint}");
    assert!(share(StarSize::Dot) > 0.9);
    for size in [StarSize::Dot, StarSize::Bright, StarSize::Glint] {
        let fp = size.footprint();
        assert_eq!(fp[0], (0, 0, 1.0), "the center is drawn at full color");
        assert!(fp.len() <= MAX_FOOTPRINT);
        assert!(fp.iter().all(|&(_, _, w)| w > 0.0 && w <= 1.0));
    }
}

#[test]
fn stars_add_light_and_never_darken_the_background() {
    let c = cam(Vector3::zero(), Vector3::new(0.0, 0.0, -1.0));
    let field = StarField {
        seed: 0,
        stars: vec![rust_software_renderer::solar::skybox::Star {
            direction: Vector3::new(0.0, 0.0, -1.0),
            brightness: GLINT_STAR + 0.01,
            tint: SPECTRAL[4].0,
        }],
    };
    let base = Vector3::new(0.2, 0.05, 0.1);
    let mut fb = Framebuffer::new(W, H);
    fb.set_background_color(base);
    fb.clear();
    assert_eq!(draw_stars(&mut fb, &field, &c), 1);
    let (cx, cy) = (W as i32 / 2, H as i32 / 2);
    // The glint's arms reach three pixels from the center and fade.
    let lit = |x, y| fb.get_pixel(x, y).unwrap() != base;
    assert!(lit(cx, cy) && lit(cx + 3, cy) && lit(cx, cy - 3) && !lit(cx + 4, cy));
    let center = fb.get_pixel(cx, cy).unwrap();
    assert_eq!(center.x, 1.0, "clamped at full red");
    assert!(fb.get_pixel(cx + 1, cy).unwrap().y > fb.get_pixel(cx + 3, cy).unwrap().y);
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let p = fb.get_pixel(x, y).unwrap();
            assert!(p.x >= base.x && p.y >= base.y && p.z >= base.z);
        }
    }
}
