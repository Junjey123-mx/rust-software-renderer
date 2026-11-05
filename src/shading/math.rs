//! Scalar and color helpers.

use raylib::math::Vector3;

/// `0xRRGGBB` to an RGB color in `0..=1` (exact palette constants).
pub const fn hex(rgb: u32) -> Vector3 {
    Vector3 {
        x: ((rgb >> 16) & 0xff) as f32 / 255.0,
        y: ((rgb >> 8) & 0xff) as f32 / 255.0,
        z: (rgb & 0xff) as f32 / 255.0,
    }
}

pub fn saturate(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// Hermite step from `e0` to `e1`; also valid when `e0 > e1` (falling step).
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    if e0 == e1 {
        return if x < e0 { 0.0 } else { 1.0 };
    }
    let t = saturate((x - e0) / (e1 - e0));
    t * t * (3.0 - 2.0 * t)
}

pub fn mixf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn mix3(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    a + (b - a) * t
}

/// Piecewise-linear color ramp over stops sorted by position; values outside
/// the first and last stop are clamped to those colors.
pub fn ramp(stops: &[(f32, Vector3)], t: f32) -> Vector3 {
    let Some(&(first_t, first_c)) = stops.first() else {
        return Vector3::zero();
    };
    if t <= first_t {
        return first_c;
    }
    for pair in stops.windows(2) {
        let ((t0, c0), (t1, c1)) = (pair[0], pair[1]);
        if t <= t1 {
            let span = t1 - t0;
            let f = if span > 0.0 { (t - t0) / span } else { 1.0 };
            return mix3(c0, c1, f);
        }
    }
    stops[stops.len() - 1].1
}
