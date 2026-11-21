//! Planet atmospheres of the Grand scale: a pass after the frame resolved
//! that follows each pixel's ray through the air shell of every planet,
//! from the ground to the atmosphere top (`SolarScene::atmosphere_top`),
//! with a density that falls off with height. The air lit by the star adds
//! its color and dims what lies behind it, so the same sum gives:
//! - a halo around the disk, brightest on the day side, because rays that
//!   graze the limb cross the most air;
//! - a thin tint on the disk that grows towards its edge (a Fresnel-like
//!   rim);
//! - from inside, a haze over the distant ground and the horizon.
//!
//! The depth buffer ends each ray at the first surface, so planets, the
//! ship and props in front of the air hide it. No planet shader changes.

use raylib::math::Vector3;

use crate::pipeline::PerspectiveCamera;
use crate::rendering::Framebuffer;
use crate::shading::math::smoothstep;

/// The color of a planet's air: `low` near the ground, `high` near the top,
/// brightened by `gain`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Air {
    pub low: Vector3,
    pub high: Vector3,
    pub gain: f32,
}

/// The air of the planet called `name`, or `None` (the star) for no air.
pub fn air_of(name: &str) -> Option<Air> {
    let rgb = Vector3::new;
    let air = |low, high, gain| Some(Air { low, high, gain });
    match name {
        "Crimson Entrance" => air(rgb(1.0, 0.45, 0.25), rgb(0.95, 0.22, 0.18), 0.9),
        // Humid: cyan low, deep blue high.
        "Clover Haven" => air(rgb(0.35, 0.85, 1.0), rgb(0.22, 0.45, 1.0), 1.0),
        "Neon Hearts" => air(rgb(1.0, 0.30, 0.58), rgb(1.0, 0.12, 0.32), 1.15),
        "House of Cards" => air(rgb(0.88, 0.35, 1.0), rgb(0.58, 0.18, 0.95), 1.0),
        // Strange: emerald low, purple high.
        "Joker" => air(rgb(0.30, 1.0, 0.45), rgb(0.62, 0.22, 1.0), 1.0),
        "Ace Crystal" => air(rgb(0.62, 1.0, 1.0), rgb(0.45, 0.80, 1.0), 1.3),
        _ => None,
    }
}

/// The air around one planet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shell {
    pub center: Vector3,
    /// Radius where the air starts (the planet's base sphere).
    pub ground: f32,
    /// Radius where it ends.
    pub top: f32,
    /// Unit direction from the center towards the star.
    pub sun: Vector3,
    pub air: Air,
}

/// Density falls by e every this share of the shell's thickness.
const SCALE_HEIGHT: f32 = 0.35;
/// Optical depth of one thickness of air at the ground's density.
const DENSITY: f32 = 0.45;
/// Share of the optical depth that dims what lies behind the air.
const EXTINCTION: f32 = 0.5;
/// Light the night side of the air keeps (its own faint glow).
const NIGHT_GLOW: f32 = 0.1;
/// Samples along each ray's path through the air.
const SAMPLES: usize = 6;

/// Adds the air of `shells` to the resolved frame in `fb`, seen from
/// `camera`, farthest shell first. Rows are shared among the cores; each
/// pixel only reads its own depth, so the result does not depend on the
/// split. Returns the pixels changed.
pub fn draw_atmospheres(
    fb: &mut Framebuffer,
    camera: &PerspectiveCamera,
    shells: &[Shell],
) -> usize {
    let (w, h) = (fb.width() as i32, fb.height() as i32);
    if w == 0 || h == 0 {
        return 0;
    }
    let focal = camera.focal_px(fb.height());
    let mut order: Vec<&Shell> = shells
        .iter()
        .filter(|s| s.top > s.ground && s.ground > 0.0)
        .collect();
    order.sort_by(|a, c| {
        let d = |s: &Shell| (s.center - camera.eye).length();
        d(c).total_cmp(&d(a))
    });
    let boxes: Vec<_> = order
        .into_iter()
        .filter_map(|s| screen_box(camera, focal, (w, h), s).map(|b| (s, b)))
        .collect();
    let (Some(top), Some(bottom)) = (
        boxes.iter().map(|(_, b)| b.1).min(),
        boxes.iter().map(|(_, b)| b.3).max(),
    ) else {
        return 0;
    };
    // Per pixel of rows `top..=bottom`: the factor on what lies behind the
    // air and the light it adds, over every shell (`c * dim + add`).
    let width = w as usize;
    let rows = (bottom - top + 1) as usize;
    let mut air = vec![(1.0f32, Vector3::zero()); rows * width];
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    let band = rows.div_ceil(cores.clamp(1, rows));
    let frame: &Framebuffer = fb;
    std::thread::scope(|scope| {
        for (k, chunk) in air.chunks_mut(band * width).enumerate() {
            let boxes = &boxes;
            scope.spawn(move || {
                for (j, px) in chunk.iter_mut().enumerate() {
                    let (x, y) = ((j % width) as i32, top + (k * band + j / width) as i32);
                    for (s, b) in boxes {
                        if x < b.0 || x > b.2 || y < b.1 || y > b.3 {
                            continue;
                        }
                        if let Some((dim, add)) = through(frame, camera, focal, s, x, y) {
                            *px = (px.0 * dim, px.1 * dim + add);
                        }
                    }
                }
            });
        }
    });
    let mut changed = 0;
    for (j, row_air) in air.chunks(width).enumerate() {
        let Some(row) = fb.row_mut(top as u32 + j as u32) else {
            continue;
        };
        for (c, &(dim, add)) in row.iter_mut().zip(row_air) {
            if dim < 1.0 {
                *c = *c * dim + add;
                changed += 1;
            }
        }
    }
    changed
}

/// The air of `s` along the ray of pixel (`x`, `y`), up to the depth the
/// frame holds there: the factor on what lies behind it and the light it
/// adds, or `None` where the ray misses it.
fn through(
    frame: &Framebuffer,
    camera: &PerspectiveCamera,
    focal: f32,
    s: &Shell,
    x: i32,
    y: i32,
) -> Option<(f32, Vector3)> {
    let b = camera.basis();
    let (cx, cy) = (frame.width() as f32 / 2.0, frame.height() as f32 / 2.0);
    let ray = b.right * ((x as f32 + 0.5 - cx) / focal)
        + b.up * ((cy - y as f32 - 0.5) / focal)
        + b.forward;
    let len = ray.length();
    let d = ray / len;
    let oc = s.center - camera.eye;
    let tc = oc.dot(d);
    let b2 = oc.dot(oc) - tc * tc;
    let top2 = s.top * s.top;
    if b2 >= top2 {
        return None;
    }
    let half = (top2 - b2).sqrt();
    let t0 = (tc - half).max(0.0);
    let mut t1 = tc + half;
    // The depth buffer holds view z; the ray reaches it at z * len.
    if let Some(z) = frame.get_depth(x, y).filter(|z| z.is_finite()) {
        t1 = t1.min(z * len);
    }
    if t1 <= t0 {
        return None;
    }
    let thick = s.top - s.ground;
    let dt = (t1 - t0) / SAMPLES as f32;
    let (mut depth, mut glow) = (0.0, Vector3::zero());
    for k in 0..SAMPLES {
        let p = d * (t0 + (k as f32 + 0.5) * dt) - oc;
        let r = p.length().max(1e-6);
        let u = ((r - s.ground) / thick).clamp(0.0, 1.0);
        let rho = (-u / SCALE_HEIGHT).exp() * (1.0 - u);
        let day = smoothstep(-0.25, 0.35, p.dot(s.sun) / r);
        let lit = NIGHT_GLOW + (1.0 - NIGHT_GLOW) * day;
        let tint = s.air.low + (s.air.high - s.air.low) * u;
        depth += rho * dt;
        glow += tint * (rho * lit * dt);
    }
    let optical = DENSITY * depth / thick;
    if optical < 1e-4 {
        return None;
    }
    let amount = s.air.gain * (1.0 - (-optical).exp()) / depth;
    Some(((-EXTINCTION * optical).exp(), glow * amount))
}

/// The pixels the air of `s` can cover: the projection of its bounding
/// cube, or the whole screen with the eye inside or near it.
fn screen_box(
    camera: &PerspectiveCamera,
    focal: f32,
    (w, h): (i32, i32),
    s: &Shell,
) -> Option<(i32, i32, i32, i32)> {
    let b = camera.basis();
    let rel = s.center - camera.eye;
    let v = Vector3::new(rel.dot(b.right), rel.dot(b.up), rel.dot(b.forward));
    if v.z + s.top <= 0.0 {
        return None;
    }
    if rel.length() <= 1.05 * s.top || v.z - s.top <= camera.near.max(1e-3) {
        return Some((0, 0, w - 1, h - 1));
    }
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for i in 0..8 {
        let sign = |bit: i32| if i & bit == 0 { -s.top } else { s.top };
        let c = v + Vector3::new(sign(1), sign(2), sign(4));
        let (sx, sy) = (cx + c.x / c.z * focal, cy - c.y / c.z * focal);
        x0 = x0.min(sx);
        x1 = x1.max(sx);
        y0 = y0.min(sy);
        y1 = y1.max(sy);
    }
    let clamp = |v: f32, hi: i32| (v as i32).clamp(0, hi);
    let (x0, x1) = (clamp(x0 - 1.0, w - 1), clamp(x1 + 1.0, w - 1));
    let (y0, y1) = (clamp(y0 - 1.0, h - 1), clamp(y1 + 1.0, h - 1));
    (x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
}
