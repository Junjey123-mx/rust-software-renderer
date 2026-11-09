//! Pre-project 3 oracle: frozen images of the three laboratory six bodies.
//!
//! Each body is rendered exactly as the laboratory six viewer does at t = 6 s
//! (oblique preset, 800x600, scanline fill, one shader thread) and compared
//! against an FNV-1a fingerprint of the RGBA bytes captured on the final
//! laboratory six code. The spacecraft is covered by
//! `lab6_regression_guard_tests`; this suite guards the shaded stage, the
//! orbit projection and the three fragment shaders before the solar scene
//! touches the pipeline. The hashes are never recomputed by the suite.

use rust_software_renderer::celestial::{Body, CelestialScene};
use rust_software_renderer::pipeline::{ShadedOptions, ViewPreset};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;

const TIME: f32 = 6.0;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn render(body: Body) -> (usize, u64) {
    let mut scene = CelestialScene::new(body, ViewPreset::Oblique, 800, 600).unwrap();
    scene.time = TIME;
    let mut fb = Framebuffer::new(800, 600);
    fb.clear();
    let options = ShadedOptions {
        threads: 1,
        ..ShadedOptions::default()
    };
    let stats = scene
        .render(&mut fb, TriangleMode::FilledScanline, options)
        .unwrap();
    (stats.fragments_shaded, fnv1a64(&fb.to_rgba_bytes()))
}

/// Captured on the final laboratory six code: (fragments shaded, fingerprint).
const EXPECTED: [(Body, usize, u64); 3] = [
    (Body::KingsCrown, 172152, 0x4549ddc1a00edaed),
    (Body::CrimsonEntrance, 172152, 0x46e1556e23b26759),
    (Body::HouseOfCards, 172152, 0x7e0bcb5480743c31),
];

#[test]
fn laboratory_six_body_images_are_frozen() {
    let actual: Vec<(Body, usize, u64)> = EXPECTED
        .iter()
        .map(|&(body, _, _)| {
            let (n, h) = render(body);
            (body, n, h)
        })
        .collect();
    let printed: Vec<String> = actual
        .iter()
        .map(|(b, n, h)| format!("({b}, {n}, {h:#018x})"))
        .collect();
    for &(body, _, hash) in &EXPECTED {
        assert_ne!(
            hash, 0,
            "{body}: hash not captured yet; actual = {printed:?}"
        );
    }
    assert_eq!(actual, EXPECTED, "actual = {printed:?}");
}

#[test]
fn body_rendering_is_deterministic_across_runs() {
    for body in Body::ALL {
        assert_eq!(render(body), render(body), "{body}");
    }
}
