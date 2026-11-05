//! Seeded 3D gradient noise (improved-Perlin style) and its fractal variants.
//!
//! Sampled on unit directions of the sphere, so patterns have no texture
//! coordinate seams. Deterministic for a given seed; no allocation.

use raylib::math::Vector3;

/// Integer lattice hash with a final avalanche.
pub fn hash3(i: i32, j: i32, k: i32, seed: u32) -> u32 {
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

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Dot product with one of the 12 cube-edge gradients.
fn grad(h: u32, x: f32, y: f32, z: f32) -> f32 {
    match h % 12 {
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
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Scale that keeps the gradient noise within about `[-1, 1]`.
const NOISE_SCALE: f32 = 0.95;

/// Gradient noise in about `[-1, 1]`; exactly 0 on the integer lattice.
pub fn gradient3(p: Vector3, seed: u32) -> f32 {
    let (fx, fy, fz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (i, j, k) = (fx as i32, fy as i32, fz as i32);
    let (x, y, z) = (p.x - fx, p.y - fy, p.z - fz);
    let (u, v, w) = (fade(x), fade(y), fade(z));
    let g = |di: i32, dj: i32, dk: i32| {
        grad(
            hash3(i + di, j + dj, k + dk, seed),
            x - di as f32,
            y - dj as f32,
            z - dk as f32,
        )
    };
    let x00 = lerp(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = lerp(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = lerp(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = lerp(g(0, 1, 1), g(1, 1, 1), u);
    let y0 = lerp(x00, x10, v);
    let y1 = lerp(x01, x11, v);
    lerp(y0, y1, w) * NOISE_SCALE
}

/// Fractal sum: lacunarity 2, gain 0.5, normalised to `[-1, 1]`.
pub fn fbm(p: Vector3, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut freq) = (0.0, 1.0, 0.0, 1.0);
    for o in 0..octaves.max(1) {
        sum += amp * gradient3(p * freq, seed.wrapping_add(o * 0x9e37));
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Ridged fractal in `[0, 1]`: bright thin ridges where the noise crosses 0.
pub fn ridged(p: Vector3, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut freq) = (0.0, 1.0, 0.0, 1.0);
    for o in 0..octaves.max(1) {
        let n = 1.0 - gradient3(p * freq, seed.wrapping_add(o * 0x9e37)).abs();
        sum += amp * n * n;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    (sum / norm).clamp(0.0, 1.0)
}

/// Domain warp: `p` displaced by three fBm fields with fixed offsets.
pub fn warp(p: Vector3, amount: f32, seed: u32) -> Vector3 {
    let a = fbm(p + Vector3::new(1.7, 9.2, 3.1), 3, seed);
    let b = fbm(p + Vector3::new(8.3, 2.8, 5.4), 3, seed.wrapping_add(1));
    let c = fbm(p + Vector3::new(4.6, 7.1, 0.9), 3, seed.wrapping_add(2));
    p + Vector3::new(a, b, c) * amount
}
