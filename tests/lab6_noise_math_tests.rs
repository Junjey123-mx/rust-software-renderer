//! Procedural toolkit: palettes, steps, ramps, gradient noise and cycles.

use raylib::math::Vector3;
use rust_software_renderer::shading::cycle::{cycle, envelope};
use rust_software_renderer::shading::math::{hex, mix3, mixf, ramp, smoothstep};
use rust_software_renderer::shading::noise::{fbm, gradient3, ridged, warp};

/// Deterministic pseudo-random points in a cube of half-size `extent`.
fn points(n: usize, extent: f32) -> Vec<Vector3> {
    let mut s: u32 = 0x1234_5678;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    (0..n)
        .map(|_| Vector3::new(next() * extent, next() * extent, next() * extent))
        .collect()
}

#[test]
fn hex_matches_red_black_palette_bytes() {
    let c = hex(0xD21F2B);
    assert_eq!(c, Vector3::new(210.0 / 255.0, 31.0 / 255.0, 43.0 / 255.0));
    assert_eq!(hex(0x000000), Vector3::zero());
    assert_eq!(hex(0xFFFFFF), Vector3::one());
}

#[test]
fn smoothstep_and_mix_edges() {
    assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
    assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
    assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
    // Reversed edges give a falling step.
    assert_eq!(smoothstep(1.0, 0.0, 0.0), 1.0);
    assert_eq!(smoothstep(1.0, 0.0, 1.0), 0.0);
    assert_eq!(mixf(2.0, 4.0, 0.25), 2.5);
    assert_eq!(
        mix3(Vector3::zero(), Vector3::one(), 0.5),
        Vector3::new(0.5, 0.5, 0.5)
    );
}

#[test]
fn ramp_hits_stops_and_clamps() {
    let stops = [
        (0.0, hex(0x000000)),
        (0.5, hex(0xFF0000)),
        (1.0, hex(0xFFFFFF)),
    ];
    assert_eq!(ramp(&stops, -3.0), hex(0x000000));
    assert_eq!(ramp(&stops, 0.5), hex(0xFF0000));
    assert_eq!(ramp(&stops, 9.0), hex(0xFFFFFF));
    let mid = ramp(&stops, 0.25);
    assert!((mid.x - 0.5).abs() < 1e-6 && mid.y == 0.0);
}

#[test]
fn noise_is_deterministic_and_seeded() {
    for p in points(200, 10.0) {
        assert_eq!(gradient3(p, 7), gradient3(p, 7));
        assert_eq!(fbm(p, 5, 3), fbm(p, 5, 3));
    }
    let differs = points(200, 10.0)
        .into_iter()
        .filter(|&p| (gradient3(p, 1) - gradient3(p, 2)).abs() > 1e-3)
        .count();
    assert!(differs > 150, "seed changes the field ({differs}/200)");
}

#[test]
fn gradient_noise_vanishes_on_integer_lattice() {
    for i in -3..4 {
        for j in -3..4 {
            for k in -3..4 {
                let p = Vector3::new(i as f32, j as f32, k as f32);
                assert_eq!(gradient3(p, 11), 0.0);
            }
        }
    }
}

#[test]
fn noise_and_fbm_stay_in_range() {
    let pts = points(20_000, 50.0);
    let max_noise = pts
        .iter()
        .map(|&p| gradient3(p, 5).abs())
        .fold(0.0, f32::max);
    let max_fbm = pts.iter().map(|&p| fbm(p, 6, 5).abs()).fold(0.0, f32::max);
    assert!(max_noise <= 1.05, "noise max {max_noise}");
    assert!(max_fbm <= 1.05, "fbm max {max_fbm}");
    assert!(
        max_noise > 0.5,
        "noise has a useful amplitude ({max_noise})"
    );
    for &p in pts.iter().take(2_000) {
        let r = ridged(p, 4, 5);
        assert!((0.0..=1.0).contains(&r));
        let w = warp(p, 0.8, 5);
        assert!((w - p).length() <= 0.8 * 3.0_f32.sqrt() * 1.05 + 1e-4);
    }
}

#[test]
fn noise_is_lipschitz_continuous() {
    let eps = 1e-3;
    for p in points(5_000, 20.0) {
        for d in [Vector3::new(eps, 0.0, 0.0), Vector3::new(0.0, eps, eps)] {
            let dist = d.length();
            assert!((gradient3(p + d, 9) - gradient3(p, 9)).abs() <= 8.0 * dist);
            assert!((fbm(p + d, 3, 9) - fbm(p, 3, 9)).abs() <= 8.0 * dist);
        }
    }
}

#[test]
fn no_nan_on_poles_and_lattice() {
    let special = [
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::zero(),
        Vector3::new(1.0, 1.0, 1.0),
        Vector3::new(-1e-7, 0.999_999, 1e-7),
    ];
    for p in special {
        for scale in [1.0, 2.6, 22.0] {
            let q = p * scale;
            assert!(gradient3(q, 1).is_finite());
            assert!(fbm(q, 7, 1).is_finite());
            assert!(ridged(q, 4, 1).is_finite());
            let w = warp(q, 1.0, 1);
            assert!(w.x.is_finite() && w.y.is_finite() && w.z.is_finite());
        }
    }
}

#[test]
fn cycle_and_envelope_are_continuous_at_boundaries() {
    let (k0, tau0) = cycle(11.999, 12.0, 0.0);
    let (k1, tau1) = cycle(12.001, 12.0, 0.0);
    assert_eq!((k0, k1), (0, 1));
    assert!(tau0 > 0.999 && tau1 < 0.001);
    assert_eq!(cycle(5.0, 0.0, 0.0), (0, 0.0));
    assert_eq!(cycle(5.0, 4.0, 2.0), (1, 0.75));
    assert_eq!(envelope(0.0, 0.3, 0.25, 0.3), 0.0);
    assert_eq!(envelope(1.0, 0.3, 0.25, 0.3), 0.0);
    assert!(envelope(1e-4, 0.3, 0.25, 0.3) < 1e-3);
    assert!(envelope(0.999_9, 0.3, 0.25, 0.3) < 1e-3);
    assert_eq!(envelope(0.4, 0.3, 0.25, 0.3), 1.0);
    let mut prev = envelope(0.0, 0.3, 0.25, 0.3);
    for i in 1..=1000 {
        let e = envelope(i as f32 / 1000.0, 0.3, 0.25, 0.3);
        assert!((e - prev).abs() < 0.01, "jump at {i}");
        prev = e;
    }
}
