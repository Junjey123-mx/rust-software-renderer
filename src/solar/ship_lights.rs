//! Automatic lights of the Project 3 ship: they switch on by themselves
//! when a planet hides the star from the ship (its shadow, with a soft
//! penumbra) and off again in sunlight. Red and green navigation lights on
//! the wingtips, a white strobe on the tail and a warm headlight beam ahead
//! of the nose, drawn after the frame resolves as glows behind the depth
//! buffer (like the comets), so planets in front hide them. The headlight
//! also lights the surfaces near the ship (`light_surfaces`): the ground,
//! the city and the props it reaches brighten with their relief, rebuilt
//! from the depth buffer.

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

/// Reach of the headlight on surfaces, ship radii.
pub const LIGHT_REACH: f32 = 16.0;
/// Cosines of the beam's half angles: full inside the inner one (about
/// 22°), none outside the outer one (about 48°).
const CONE_INNER: f32 = 0.93;
const CONE_OUTER: f32 = 0.67;
/// Share of the beam that lights all around the ship (the hull lights),
/// within `AURA_REACH` ship radii, so the ground below reads too.
const AURA: f32 = 0.45;
const AURA_REACH: f32 = 7.0;
/// Strength of the light on a white surface square to it.
const GAIN: f32 = 2.4;
/// Surfaces closer than this many ship radii are the hull itself.
const HULL: f32 = 1.05;

/// Albedo guessed from a resolved color in the dark (where only the dim
/// ambient lit it): the color stretched back towards its full brightness,
/// over a grey floor so black ground still shows the light.
fn albedo_guess(c: Vector3) -> Vector3 {
    let stretch = |v: f32| (v / 0.06).clamp(0.0, 1.0);
    Vector3::new(stretch(c.x), stretch(c.y), stretch(c.z)) * 0.75 + Vector3::one() * 0.12
}

/// Lights what the depth buffer holds near the ship with its headlight at
/// `level` (0 off, 1 full): a warm cone along the nose plus a short aura,
/// Lambert-shaded with normals rebuilt from the depth of neighbouring
/// pixels, fading to nothing at `LIGHT_REACH` ship radii. Works on every
/// surface (terrain, city, props) after the frame resolved, so no planet
/// shader changes. Returns the pixels lit.
pub fn light_surfaces(
    fb: &mut Framebuffer,
    camera: &PerspectiveCamera,
    ship: &ShipState,
    level: f32,
) -> usize {
    if level.is_nan() || level <= 0.0 {
        return 0;
    }
    let (w, h) = (fb.width(), fb.height());
    let focal = camera.focal_px(h);
    let b = camera.basis();
    let r = ship.radius();
    let reach = LIGHT_REACH * r;
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let world = |x: i32, y: i32, z: f32| {
        camera.eye
            + b.right * ((x as f32 + 0.5 - cx) * z / focal)
            + b.up * ((cy - y as f32 - 0.5) * z / focal)
            + b.forward * z
    };
    let depth = |fb: &Framebuffer, x: i32, y: i32| fb.get_depth(x, y).filter(|z| z.is_finite());
    // No surface farther than this from the eye can be within reach.
    let z_max = (ship.position - camera.eye).length() + reach;
    let mut lit = 0;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let Some(z) = depth(fb, x, y).filter(|&z| z <= z_max) else {
                continue;
            };
            let p = world(x, y, z);
            let to_ship = ship.position - p;
            let dist = to_ship.length();
            if dist >= reach || dist < HULL * r {
                continue;
            }
            // Normal from the nearer-depth neighbour on each axis (either
            // side), facing the camera; the view direction where both sides
            // jump (a silhouette).
            let near = |dx: i32, dy: i32| {
                [1, -1].into_iter().find_map(|s| {
                    let (nx, ny) = (x + s * dx, y + s * dy);
                    depth(fb, nx, ny)
                        .filter(|&q| (q - z).abs() < 0.08 * z)
                        .map(|q| (world(nx, ny, q) - p) * s as f32)
                })
            };
            let view = (camera.eye - p).normalized();
            let normal = match (near(1, 0), near(0, 1)) {
                (Some(du), Some(dv)) => {
                    let n = du.cross(dv).normalized();
                    if n.dot(view) < 0.0 { -n } else { n }
                }
                _ => view,
            };
            let l = to_ship / dist;
            let lambert = normal.dot(l).max(0.0);
            if lambert <= 0.0 {
                continue;
            }
            let spot = smoothstep(CONE_OUTER, CONE_INNER, (-l).dot(ship.forward));
            let aura = AURA * (1.0 - smoothstep(0.0, AURA_REACH * r, dist));
            let fade = 1.0 - (dist / reach) * (dist / reach);
            let strength = GAIN * level * lambert * (spot + aura) * fade * fade;
            if strength <= 1e-3 {
                continue;
            }
            if let Some(c) = fb.get_pixel(x, y) {
                let a = albedo_guess(c);
                let add = Vector3::new(BEAM.x * a.x, BEAM.y * a.y, BEAM.z * a.z) * strength;
                if let Some(row) = fb.row_mut(y as u32) {
                    row[x as usize] = c + add;
                    lit += 1;
                }
            }
        }
    }
    lit
}
