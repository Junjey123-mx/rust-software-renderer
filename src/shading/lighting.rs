//! Lighting, relief and cloud-shell models shared by the celestial bodies.
//!
//! All directions are unit vectors in object space; `l` points towards the
//! light and `v` towards the eye. Height functions are any `Fn(Vector3) ->
//! f32` evaluated on unit directions, so these kernels know nothing about
//! the bodies that use them.

use crate::shading::math::{saturate, smoothstep};
use crate::shading::sphere::tangent_frame;
use raylib::math::Vector3;

pub fn lambert(n: Vector3, l: Vector3) -> f32 {
    n.dot(l).max(0.0)
}

/// Diffuse term that wraps `w` past the terminator (soft, dusty surfaces).
pub fn wrap_lambert(n: Vector3, l: Vector3, w: f32) -> f32 {
    ((n.dot(l) + w) / (1.0 + w)).max(0.0)
}

/// Blinn-Phong highlight; 0 when the surface faces away from the light.
pub fn blinn_phong(n: Vector3, l: Vector3, v: Vector3, shininess: f32) -> f32 {
    if n.dot(l) <= 0.0 {
        return 0.0;
    }
    let h = l + v;
    let len = h.length();
    if len <= 1e-6 {
        return 0.0;
    }
    n.dot(h / len).max(0.0).powf(shininess)
}

/// Schlick-style rim factor: 0 facing the eye, 1 at grazing angles.
pub fn fresnel(n: Vector3, v: Vector3, power: f32) -> f32 {
    (1.0 - saturate(n.dot(v))).powf(power)
}

/// Normal perturbed by the tangent-plane gradient of `height`, estimated
/// with forward differences of step `eps` from the height `h0` the caller
/// already knows at `n` (two lookups instead of four central ones):
/// `N' = normalize(N - k (h_t t + h_b b))`. The gradient does not depend on
/// the chosen frame.
pub fn bump_normal(
    n: Vector3,
    h0: f32,
    height: impl Fn(Vector3) -> f32,
    eps: f32,
    strength: f32,
) -> Vector3 {
    if strength == 0.0 || eps <= 0.0 {
        return n;
    }
    let (t, b) = tangent_frame(n);
    let h_t = (height((n + t * eps).normalized()) - h0) / eps;
    let h_b = (height((n + b * eps).normalized()) - h0) / eps;
    (n - (t * h_t + b * h_b) * strength).normalized()
}

/// View-dependent parallax: searches along the tangential view direction
/// for the first point where the view ray meets the height field
/// `scale * height`, with `steps` uniform steps plus `refine` bisections,
/// and returns the shifted lookup direction. Identity for zero scale or a
/// back-facing view; fades out at grazing angles where the approximation
/// breaks down.
pub fn parallax_shift(
    n: Vector3,
    v: Vector3,
    height: impl Fn(Vector3) -> f32,
    scale: f32,
    steps: u32,
    refine: u32,
) -> Vector3 {
    let vn = n.dot(v);
    if scale == 0.0 || vn <= 0.0 {
        return n;
    }
    let fade = smoothstep(0.1, 0.35, vn);
    if fade == 0.0 {
        return n;
    }
    let vt = v - n * vn;
    let f = |t: f32| t * vn - scale * height((n + vt * t).normalized());
    let steps = steps.max(1);
    let (t_start, t_end) = (scale.abs() / vn, -scale.abs() / vn);
    let mut prev = t_start;
    let mut hit = t_end;
    if f(t_start) <= 0.0 {
        hit = t_start;
    } else {
        for i in 1..=steps {
            let t = t_start + (t_end - t_start) * i as f32 / steps as f32;
            if f(t) <= 0.0 {
                let (mut above, mut below) = (prev, t);
                for _ in 0..refine {
                    let mid = 0.5 * (above + below);
                    if f(mid) <= 0.0 {
                        below = mid;
                    } else {
                        above = mid;
                    }
                }
                hit = 0.5 * (above + below);
                break;
            }
            prev = t;
        }
    }
    (n + vt * (fade * hit)).normalized()
}

/// Direction of the point where the ray from surface point `n` towards the
/// light leaves a cloud shell of radius `1 + h`: `n + s l` with
/// `s = -mu + sqrt(mu^2 + 2h + h^2)` and `mu = n . l`.
pub fn shell_exit(n: Vector3, l: Vector3, h: f32) -> Vector3 {
    let mu = n.dot(l);
    let s = -mu + (mu * mu + 2.0 * h + h * h).max(0.0).sqrt();
    (n + l * s).normalized()
}
