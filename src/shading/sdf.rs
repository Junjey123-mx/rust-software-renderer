//! 2D signed distance fields (negative inside) evaluated in local tangent
//! planes of the sphere. The spade is reserved for Crimson Entrance and the
//! L1 diamond for House of Cards; neither is ever drawn as a decal.

pub type P2 = (f32, f32);

fn sub(a: P2, b: P2) -> P2 {
    (a.0 - b.0, a.1 - b.1)
}

fn dot(a: P2, b: P2) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

fn cross(a: P2, b: P2) -> f32 {
    a.0 * b.1 - a.1 * b.0
}

pub fn circle(p: P2, center: P2, radius: f32) -> f32 {
    let d = sub(p, center);
    dot(d, d).sqrt() - radius
}

/// Exact distance to the triangle `a, b, c` (either winding).
pub fn triangle(p: P2, a: P2, b: P2, c: P2) -> f32 {
    let edges = [(a, sub(b, a)), (b, sub(c, b)), (c, sub(a, c))];
    let s = cross(sub(b, a), sub(c, a)).signum();
    let mut best_d2 = f32::INFINITY;
    let mut inside = true;
    for (origin, e) in edges {
        let v = sub(p, origin);
        let t = (dot(v, e) / dot(e, e)).clamp(0.0, 1.0);
        let q = (v.0 - e.0 * t, v.1 - e.1 * t);
        best_d2 = best_d2.min(dot(q, q));
        if s * cross(e, v) < 0.0 {
            inside = false;
        }
    }
    let d = best_d2.sqrt();
    if inside { -d } else { d }
}

/// Polynomial smooth minimum with blend width `k`.
pub fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// ♠ silhouette (Crimson Entrance only): pointed top at `y = 0.58`, two
/// round lobes and a widening stem down to `y = -0.58`. Mirror symmetric.
pub fn spade(p: P2) -> f32 {
    let lobe_l = circle(p, (-0.24, -0.08), 0.26);
    let lobe_r = circle(p, (0.24, -0.08), 0.26);
    let blade = triangle(p, (0.0, 0.58), (-0.47, -0.08), (0.47, -0.08));
    let stem = triangle(p, (0.0, -0.12), (-0.16, -0.58), (0.16, -0.58));
    lobe_l.min(lobe_r).min(blade).min(stem)
}

/// ♦ metric (House of Cards only): `|x|/a + |y|/b - 1`; its level sets are
/// diamonds with half-diagonals `a` and `b`.
pub fn diamond_l1(p: P2, a: f32, b: f32) -> f32 {
    p.0.abs() / a + p.1.abs() / b - 1.0
}
