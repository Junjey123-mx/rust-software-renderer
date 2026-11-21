//! Automatic lights of the Project 3 ship: they switch on by themselves
//! when a planet hides the star from the ship (its shadow, with a soft
//! penumbra) and off again in sunlight. Red and green navigation lights on
//! the wingtips, a white strobe on the tail and a warm headlight beam ahead
//! of the nose, drawn after the frame resolves as glows behind the depth
//! buffer (like the comets), so planets in front hide them.

use raylib::math::Vector3;

use crate::pipeline::PerspectiveCamera;
use crate::rendering::Framebuffer;
use crate::shading::math::smoothstep;
use crate::solar::comets::glow;
use crate::solar::ship::ShipState;

/// How far into a planet's shadow the ship at `ship` is, in [0, 1]: the
/// largest share over `bodies` (centers and radii) standing between the
/// ship and the star at `star`, 0 well outside the shadow cylinder's edge
/// (1.15 R from its axis), 1 well inside it (0.85 R).
pub fn shadow(ship: Vector3, star: Vector3, bodies: &[(Vector3, f32)]) -> f32 {
    let to_star = star - ship;
    let length = to_star.length();
    if !(length.is_finite() && length > 0.0) {
        return 0.0;
    }
    let u = to_star / length;
    bodies
        .iter()
        .filter_map(|&(c, r)| {
            let t = (c - ship).dot(u);
            (t > 0.0 && t < length).then(|| {
                let off = (c - ship - u * t).length();
                1.0 - smoothstep(0.85 * r, 1.15 * r, off)
            })
        })
        .fold(0.0, f32::max)
}

/// Whether the tail strobe is lit at time `t`: a 0.12 s flash every second.
pub fn strobe_on(t: f32) -> bool {
    t.is_finite() && t.rem_euclid(1.0) < 0.12
}

const PORT: Vector3 = Vector3::new(1.0, 0.12, 0.1);
const STARBOARD: Vector3 = Vector3::new(0.1, 1.0, 0.25);
const STROBE: Vector3 = Vector3::new(1.0, 1.0, 1.0);
const BEAM: Vector3 = Vector3::new(1.0, 0.9, 0.65);
/// Samples of the headlight beam and its reach, ship radii.
const BEAM_SAMPLES: usize = 24;
const BEAM_REACH: f32 = 6.0;
/// Depth pulled towards the camera so the lights sit on the hull, not in it.
const BIAS: f32 = 0.03;

/// Draws the lights of `ship` at `level` (0 off, 1 full) and time `t`.
/// Returns the pixels lit.
pub fn draw_ship_lights(
    fb: &mut Framebuffer,
    camera: &PerspectiveCamera,
    ship: &ShipState,
    level: f32,
    t: f32,
) -> usize {
    if level.is_nan() || level <= 0.0 {
        return 0;
    }
    let (w, h) = (fb.width(), fb.height());
    let focal = camera.focal_px(h);
    let r = ship.radius();
    let (f, u, s) = (ship.forward, ship.up, ship.right());
    let mut lit = 0;
    let mut spot = |fb: &mut Framebuffer, p: Vector3, size: f32, color: Vector3| {
        if let Some(q) = camera.project_view(camera.world_to_view(p), w, h) {
            let px = (size * focal / q.z).clamp(1.0, 14.0);
            lit += glow(fb, q.x, q.y, px, q.z - BIAS, color * level);
        }
    };
    let back = -f * (0.15 * r);
    spot(fb, ship.position - s * (0.85 * r) + back, 0.06 * r, PORT);
    spot(
        fb,
        ship.position + s * (0.85 * r) + back,
        0.06 * r,
        STARBOARD,
    );
    if strobe_on(t) {
        spot(
            fb,
            ship.position - f * (0.9 * r) + u * (0.2 * r),
            0.09 * r,
            STROBE,
        );
    }
    // The beam: a cone of fading glows ahead of the nose.
    for k in 0..BEAM_SAMPLES {
        let a = k as f32 / BEAM_SAMPLES as f32;
        let p = ship.position + f * (r * (0.9 + BEAM_REACH * a));
        let fade = (1.0 - a) * (1.0 - a);
        spot(fb, p, r * (0.08 + 0.5 * a), BEAM * (0.35 * fade));
    }
    lit
}
