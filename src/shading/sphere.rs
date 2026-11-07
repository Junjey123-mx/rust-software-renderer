//! Frames, local planes, rotations and seeded directions on the unit sphere.

use crate::shading::noise::hash3;
use raylib::math::Vector3;

/// Orthonormal tangent `(t, b)` at unit normal `n`: `t` points east and `b`
/// north. Uses world `X` instead of `Y` as reference near the poles.
pub fn tangent_frame(n: Vector3) -> (Vector3, Vector3) {
    let up = if n.y.abs() > 0.999 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    let t = up.cross(n).normalized();
    let b = n.cross(t);
    (t, b)
}

/// East and north directions at an event center `c` (unit).
pub fn east_north(c: Vector3) -> (Vector3, Vector3) {
    tangent_frame(c)
}

/// Gnomonic coordinates of unit direction `n` in the tangent plane at `c`
/// with axes `e`, `nn`. `None` when `n` is far from `c` (`n . c <= 0.05`).
pub fn local_plane(n: Vector3, c: Vector3, e: Vector3, nn: Vector3) -> Option<(f32, f32)> {
    let d = n.dot(c);
    if d <= 0.05 {
        return None;
    }
    let q = n / d;
    Some((q.dot(e), q.dot(nn)))
}

/// Inverse of `local_plane`: unit direction of plane point `(u, v)`.
pub fn from_local(c: Vector3, e: Vector3, nn: Vector3, u: f32, v: f32) -> Vector3 {
    (c + e * u + nn * v).normalized()
}

/// Rotation around the world `Y` axis.
pub fn rotate_y(p: Vector3, angle: f32) -> Vector3 {
    let (s, c) = angle.sin_cos();
    Vector3::new(p.x * c + p.z * s, p.y, -p.x * s + p.z * c)
}

/// Rodrigues rotation of `p` around the unit `axis`.
pub fn rotate_axis(p: Vector3, axis: Vector3, angle: f32) -> Vector3 {
    let (s, c) = angle.sin_cos();
    p * c + axis.cross(p) * s + axis * (axis.dot(p) * (1.0 - c))
}

/// Angle in radians between two directions (robust near 0 and pi).
pub fn angular_distance(a: Vector3, b: Vector3) -> f32 {
    a.cross(b).length().atan2(a.dot(b))
}

/// Seeded direction, uniform over the cap of angular radius `cone` around
/// the unit `focus`. Changes with `cycle`, so each event cycle relocates.
pub fn hash_direction(seed: u32, slot: u32, cycle: i64, focus: Vector3, cone: f32) -> Vector3 {
    let lo = cycle as i32;
    let hi = (cycle >> 32) as i32;
    let h1 = hash3(slot as i32, lo, hi, seed);
    let h2 = hash3(slot as i32, lo, hi, seed ^ 0x5bd1_e995);
    let u1 = h1 as f32 / u32::MAX as f32;
    let u2 = h2 as f32 / u32::MAX as f32;
    let cos_t = 1.0 - u1 * (1.0 - cone.cos());
    let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
    let phi = std::f32::consts::TAU * u2;
    let (t, b) = tangent_frame(focus);
    (focus * cos_t + (t * phi.cos() + b * phi.sin()) * sin_t).normalized()
}

/// Center of one placed event and the angular radius it occupies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub index: i64,
    pub center: Vector3,
    pub reach: f32,
}

/// Smallest angular gap between an event at `center` with reach `r` and the
/// `earlier` ones once both reaches and `margin` are subtracted. Not
/// negative = no overlap.
pub fn clearance(center: Vector3, r: f32, earlier: &[Placement], margin: f32) -> f32 {
    earlier
        .iter()
        .map(|e| angular_distance(center, e.center) - (r + e.reach + margin))
        .fold(f32::INFINITY, f32::min)
}

/// Direction `i` of `count` spread evenly over the sphere (Fibonacci).
pub fn fibonacci_direction(i: usize, count: usize) -> Vector3 {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
    let r = (1.0 - y * y).sqrt();
    let phi = golden * i as f32;
    Vector3::new(r * phi.cos(), y, r * phi.sin())
}
