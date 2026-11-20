//! Project 3 Hearts material: an emissive neon grid over the terraces that
//! glows on the night side, fades out while the planet is small on screen,
//! carries pulses that move with the clock and a heart at each crossing.

use raylib::math::Vector3;
use rust_software_renderer::solar::instance::Quality;
use rust_software_renderer::solar::planet::PlanetMaterial;
use rust_software_renderer::solar::planet::hearts::{
    NeonGridParams, PreparedHearts, grid_fade, grid_pattern, heart_sdf,
};

fn params() -> NeonGridParams {
    NeonGridParams::default()
}

fn prepared(p: NeonGridParams, light: Vector3, time: f32) -> PreparedHearts {
    PreparedHearts {
        field: p.field(),
        params: p,
        light,
        time,
    }
}

/// Unit direction at latitude `lat` and longitude `lon` (radians).
fn dir(lat: f32, lon: f32) -> Vector3 {
    Vector3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin())
}

fn brightness(c: Vector3) -> f32 {
    c.x + c.y + c.z
}

#[test]
fn grid_lines_emissive() {
    let p = params();
    let cell = std::f32::consts::TAU / p.cells;
    // On the equator (a parallel line) and at a cell's center, both with
    // the star behind the planet.
    let line = dir(0.0, 0.5 * cell);
    let middle = dir(0.5 * cell, 0.5 * cell);
    for (n, on) in [(line, true), (middle, false)] {
        let night = prepared(p, -n, 1.0).shade(n * 1.02);
        let (l, _, h) = grid_pattern(n, 1.0, &p);
        if on {
            assert!(l > 0.99 && brightness(night) > 1.0, "{night:?}");
        } else {
            assert!(l == 0.0 && h == 0.0 && brightness(night) < 0.3, "{night:?}");
        }
    }
}

#[test]
fn fades_with_distance() {
    let adapted = |px| match PlanetMaterial::Hearts(params()).adapted(Quality::Adaptive, px) {
        PlanetMaterial::Hearts(p) => p,
        other => panic!("{other:?}"),
    };
    let n = dir(0.0, 0.1);
    assert_eq!(grid_pattern(n, 0.0, &adapted(20.0)), (0.0, 0.0, 0.0));
    assert_eq!(adapted(400.0).detail, 1.0);
    let mid = adapted(100.0).detail;
    assert!(mid > 0.0 && mid < 1.0, "{mid}");
    // Monotonic between the bounds; never NaN.
    let mut last = 0.0;
    for px in (0..=200).map(|i| i as f32) {
        let f = grid_fade(px);
        assert!(f >= last);
        last = f;
    }
    assert_eq!(grid_fade(f32::NAN), 0.0);
}

#[test]
fn pulses_move_with_time() {
    let p = params();
    // Along the equator line.
    let points: Vec<Vector3> = (0..200).map(|i| dir(0.0, i as f32 * 0.03)).collect();
    let pulses =
        |t: f32| -> Vec<f32> { points.iter().map(|&n| grid_pattern(n, t, &p).1).collect() };
    let (a, b) = (pulses(1.0), pulses(1.7));
    assert!(a.iter().any(|&x| x > 0.5), "there are pulses");
    let moved = a
        .iter()
        .zip(&b)
        .filter(|(x, y)| (*x - *y).abs() > 0.1)
        .count();
    assert!(moved > 5, "{moved}");
    // One whole period later (4 / pulse_speed s) they are back.
    let back = pulses(1.0 + 4.0 / p.pulse_speed);
    assert!(a.iter().zip(&back).all(|(x, y)| (x - y).abs() < 1e-3));
}

#[test]
fn heart_sdf_inside_outside() {
    let s = 0.3;
    assert!(heart_sdf((0.0, 0.0), s) < 0.0, "middle");
    assert!(heart_sdf((0.0, -0.9 * s), s) < 0.0, "near the tip");
    assert!(heart_sdf((-0.5 * s, 0.4 * s), s) < 0.0, "left lobe");
    assert!(
        heart_sdf((0.0, 0.7 * s), s) > 0.0,
        "the notch between the lobes"
    );
    assert!(heart_sdf((1.2 * s, 0.0), s) > 0.0, "beside");
    assert!(heart_sdf((0.0, -1.1 * s), s) > 0.0, "below the tip");
    for (x, y) in [(0.1, 0.2), (0.25, -0.1), (0.4, 0.3)] {
        let (a, b) = (heart_sdf((x, y), s), heart_sdf((-x, y), s));
        assert!((a - b).abs() < 1e-3, "symmetric: {a} {b}");
    }
}

#[test]
fn zero_width_no_lines_no_nan() {
    let p = NeonGridParams {
        line_width: 0.0,
        ..params()
    };
    for i in 0..500 {
        let n = dir((i as f32 * 0.37).sin() * 1.5, i as f32 * 0.11);
        let (l, pulse, _) = grid_pattern(n, i as f32, &p);
        assert_eq!((l, pulse), (0.0, 0.0));
        let c = prepared(p, Vector3::new(0.0, 1.0, 0.0), i as f32).shade(n);
        assert!(c.x.is_finite() && c.y.is_finite() && c.z.is_finite());
    }
    // The poles too.
    let c = prepared(params(), Vector3::new(1.0, 0.0, 0.0), 0.0).shade(Vector3::new(0.0, 1.0, 0.0));
    assert!(c.x.is_finite() && c.y.is_finite() && c.z.is_finite());
}
