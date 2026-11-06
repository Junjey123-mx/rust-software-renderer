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

// ---- The optimized noise is bit-identical to the original formulation ----

use rust_software_renderer::shading::noise::hash3;

/// The original single-pass lattice hash.
fn reference_hash3(i: i32, j: i32, k: i32, seed: u32) -> u32 {
    let mut h = seed ^ 0x9e37_79b9;
    h = (h ^ i as u32).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = (h ^ j as u32).wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h = (h ^ k as u32).wrapping_mul(0x27d4_eb2f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

/// The original gradient noise with one branch per cube-edge gradient.
fn reference_gradient3(p: Vector3, seed: u32) -> f32 {
    let grad = |h: u32, x: f32, y: f32, z: f32| match h % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    };
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let (fx, fy, fz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (i, j, k) = (fx as i32, fy as i32, fz as i32);
    let (x, y, z) = (p.x - fx, p.y - fy, p.z - fz);
    let (u, v, w) = (fade(x), fade(y), fade(z));
    let g = |di: i32, dj: i32, dk: i32| {
        grad(
            reference_hash3(i + di, j + dj, k + dk, seed),
            x - di as f32,
            y - dj as f32,
            z - dk as f32,
        )
    };
    let x00 = lerp(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = lerp(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = lerp(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = lerp(g(0, 1, 1), g(1, 1, 1), u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w) * 0.95
}

#[test]
fn optimized_noise_matches_the_original_bit_for_bit() {
    for (n, p) in points(20_000, 40.0).into_iter().enumerate() {
        let seed = (n as u32).wrapping_mul(2_654_435_761);
        assert_eq!(
            gradient3(p, seed).to_bits(),
            reference_gradient3(p, seed).to_bits(),
            "{p:?} seed {seed}"
        );
        let (i, j, k) = (p.x as i32, p.y as i32 * 7, -(p.z as i32));
        assert_eq!(hash3(i, j, k, seed), reference_hash3(i, j, k, seed));
    }
}
