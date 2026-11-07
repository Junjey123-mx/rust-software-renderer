//! House of Cards: violet gas giant with diamond storms (gas giant).
//!
//! Latitude bands warped by fBm slide past each other under a latitude-
//! dependent wind (a rotation of the sampling direction, never an angle
//! with a seam), mapped through violet, magenta and pearl ramps with an
//! atmospheric rim. Up to three migrating storms (large, medium and small,
//! a third of a period apart) advect the band sampling domain along the
//! contours of an L1 diamond metric in their tangent planes, so the bands
//! wind into rhombic spirals. Brief neon lightning crackles inside each
//! storm diamond on its own rhythm and now and then between the bands. Its
//! only symbol is the diamond.
//!
//! Storm `m` is born at `m * P / 3` in slot `m mod 3` and lives one period.
//! Its center is the first seeded candidate whose diamond stays clear of
//! the two storms it shares the sky with and of the one its slot just
//! left; the sequence is replayed from a fixed first storm, so a center
//! never moves while its storm is visible.

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::pipeline::{ViewPreset, direction_from_angles};
use crate::shading::cycle::{cycle, envelope};
use crate::shading::lighting::{fresnel, wrap_lambert};
use crate::shading::math::{hex, mix3, ramp, saturate, smoothstep};
use crate::shading::noise::{fbm, hash3};
use crate::shading::sphere::{
    Placement, angular_distance, clearance, east_north, fibonacci_direction, from_local,
    hash_direction, local_plane, rotate_y,
};
use raylib::math::Vector3;
use std::f32::consts::PI;

params! {
    HouseParams {
        seed: 23.0, 0.0, 9999.0, "noise seed";
        band_count: 6.0, 3.0, 14.0, "number of main bands";
        band_warp: 0.25, 0.0, 1.5, "how much the band boundaries ripple";
        band_flow: 0.12, 0.0, 1.0, "mean wind, radians per second";
        differential_wind: 0.06, 0.0, 1.5, "wind difference between latitudes";
        filament_strength: 0.15, 0.0, 0.5, "fine filaments along the bands";
        rim_strength: 0.6, 0.0, 2.0, "violet and magenta atmospheric rim";
        wrap: 0.35, 0.0, 1.0, "wrapped diffuse lighting of the clouds";
        vortex: 1.0, 0.0, 1.0, "switch: diamond storms (0 or 1)";
        vortex_count: 3.0, 1.0, 3.0, "diamond storms at once: large, + medium, + small (1 to 3, rounded)";
        vortex_period: 18.0, 4.0, 90.0, "seconds per storm life cycle (storms a third apart)";
        vortex_radius: 0.45, 0.1, 0.5, "half-diagonal of the storm diamond in its tangent plane";
        vortex_strength: 6.0, 0.0, 8.0, "circulation at the storm core, radians";
        vortex_aspect: 1.35, 0.6, 2.0, "north-south over east-west diagonal of the diamond";
        secondary_strength: 4.5, 0.0, 8.0, "circulation of the medium and small storms, radians";
        vortex_max_latitude: 45.0, 0.0, 60.0, "storm centers stay within this latitude, degrees";
        lightning: 1.0, 0.0, 1.0, "switch: neon lightning in the storms (0 or 1)";
        lightning_rate: 1.0, 0.0, 3.0, "how often lightning strikes";
        lightning_intensity: 1.4, 0.0, 3.0, "brightness of the lightning";
    }
}

const BAND_RAMP: [(f32, Vector3); 7] = [
    (0.0, hex(0x1A0A20)),
    (0.2, hex(0x52106A)),
    (0.45, hex(0x8A2BE2)),
    (0.68, hex(0xC13CFF)),
    (0.86, hex(0xF0408A)),
    (0.96, hex(0xF4C6DA)),
    (1.0, hex(0xD6D0C4)),
];

/// Per-frame state of the gas giant shader.
#[derive(Debug, Clone)]
pub struct Frame {
    pub params: HouseParams,
    pub uniforms: ShaderUniforms,
    pub debug: DebugView,
    seed: u32,
    /// Large, medium and small storms of this frame, by slot.
    pub storms: [Storm; STORM_SLOTS],
    /// Lightning channels of this instant (boxed: built once per frame).
    pub strikes: Box<[Strike; LIGHTNING_CHANNELS]>,
}

/// Lightning channels: 0, 3 and 4 follow the large, medium and small storm
/// diamonds, 1 and 2 strike between the bands.
pub const LIGHTNING_CHANNELS: usize = 5;
/// Length of one possible strike per channel, seconds.
pub const STRIKE_SLOTS: [f32; LIGHTNING_CHANNELS] = [0.11, 0.17, 0.23, 0.13, 0.19];
/// Storm each channel is bound to, if any.
pub const CHANNEL_STORM: [Option<usize>; LIGHTNING_CHANNELS] =
    [Some(0), None, None, Some(1), Some(2)];

/// One lightning channel at the current instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strike {
    pub center: Vector3,
    /// Angular radius of the discharge region.
    pub radius: f32,
    /// Brightness of this instant: a fast-decaying flash, 0 when idle.
    pub flash: f32,
    /// Mix between electric violet (0) and neon pink (1).
    pub hue: f32,
    /// Direction of the main bolt in the tangent plane of `center`.
    pub angle: f32,
    pub east: Vector3,
    pub north: Vector3,
    /// Storm whose diamond confines the strike, if any.
    pub storm: Option<usize>,
    seed: u32,
}

/// Uniform value in `[0, 1)` from integer coordinates.
fn unit_hash(a: i32, b: i32, c: i32, seed: u32) -> f32 {
    hash3(a, b, c, seed) as f32 / (u32::MAX as f32 + 1.0)
}

/// State of lightning channel `c` at time `t`; `storm` is the one the
/// channel is bound to (ignored by the channels between the bands). Time is
/// cut into short slots; a hash decides whether a slot strikes (more often
/// while the storm is strong) and where. The flash decays within its slot,
/// sometimes with a second pulse, so discharges are brief and intermittent.
pub fn strike(p: &HouseParams, storm: &Storm, c: usize, t: f32) -> Strike {
    let seed = (p.seed as u32) ^ 0xa5a5_1f3d;
    let slot = (t / STRIKE_SLOTS[c]).floor();
    let phase = t / STRIKE_SLOTS[c] - slot;
    let (k, ch) = (slot as i64, c as i32);
    let (lo, hi) = (k as i32, (k >> 32) as i32);
    let h = |salt: i32| unit_hash(ch * 8 + salt, lo, hi, seed);
    let storm_bound = CHANNEL_STORM[c].is_some();
    let chance = if p.lightning < 0.5 {
        0.0
    } else if storm_bound {
        if storm.w != 0.0 {
            p.lightning_rate * (0.15 + 0.75 * storm.envelope) * STORM_LIGHTNING[storm.slot]
        } else {
            0.0
        }
    } else {
        p.lightning_rate * 0.22
    };
    let active = h(0) < chance;
    let double = if h(1) < 0.4 {
        0.6 * (-30.0 * (phase - 0.45).abs()).exp()
    } else {
        0.0
    };
    let flash = if active {
        ((-5.0 * phase).exp() * (0.7 + 0.3 * h(2)) + double).min(1.0)
    } else {
        0.0
    };
    let (center, radius) = if storm_bound {
        let (u, v) = ((h(3) - 0.5) * 0.9 * storm.a, (h(4) - 0.5) * 0.9 * storm.b);
        (
            from_local(storm.center, storm.east, storm.north, u, v),
            0.9 * storm.a.max(storm.b),
        )
    } else {
        let (az, el) = ViewPreset::Oblique.angles();
        let focus = direction_from_angles(az, el);
        (
            hash_direction(seed, 50 + c as u32, k, focus, 60.0_f32.to_radians()),
            0.22 + 0.1 * h(5),
        )
    };
    let (east, north) = east_north(center);
    Strike {
        center,
        radius,
        flash,
        hue: h(6),
        angle: PI * h(7),
        east,
        north,
        storm: CHANNEL_STORM[c],
        seed: hash3(ch, lo, hi, seed ^ 0x2545_f491),
    }
}

/// Jagged 1D offset of a bolt: four octaves of gradient noise along `x`.
fn jag(x: f32, seed: u32) -> f32 {
    fbm(Vector3::new(x, 0.31, 0.77), 4, seed)
}

/// Distance from plane point `(x, y)` to a jagged stroke that starts at the
/// origin and runs `length` along the positive x axis, with both ends
/// anchored. `None` beyond the ends.
fn bolt_distance(x: f32, y: f32, length: f32, amplitude: f32, seed: u32) -> Option<f32> {
    if !(0.0..=length).contains(&x) {
        return None;
    }
    let anchor = (PI * x / length).sin();
    Some((y - amplitude * anchor * jag(7.0 * x / length, seed)).abs())
}

/// A diamond storm in its current cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Storm {
    pub center: Vector3,
    pub east: Vector3,
    pub north: Vector3,
    /// East-west and north-south half-diagonals of the diamond.
    pub a: f32,
    pub b: f32,
    /// Circulation of this frame, radians (0 at cycle boundaries; negative
    /// for the anticyclone).
    pub w: f32,
    /// Life-cycle envelope in `[0, 1]`.
    pub envelope: f32,
    /// Cycle of its slot.
    pub cycle: i64,
    /// Large (0), medium (1) or small (2).
    pub slot: usize,
    /// Cosine of the angular radius of the diamond: an exact early-out.
    reach_cos: f32,
}

impl Storm {
    /// Diamond distance `d = |u|/a + |v|/b` and contour parameter of `n`,
    /// or `None` when `n` is outside the storm (`d >= 1`) or far away.
    pub fn coords(&self, n: Vector3) -> Option<(f32, f32)> {
        if n.dot(self.center) < self.reach_cos {
            return None;
        }
        let (u, v) = local_plane(n, self.center, self.east, self.north)?;
        let d = u.abs() / self.a + v.abs() / self.b;
        (d < 1.0).then(|| (d, contour_param(u, v, self.a, self.b, d)))
    }

    /// Sampling direction advected along the diamond contours. Identity
    /// outside the diamond and when the circulation is 0.
    pub fn warp(&self, n: Vector3) -> Vector3 {
        if self.w == 0.0 {
            return n;
        }
        let Some((d, s)) = self.coords(n) else {
            return n;
        };
        if d <= 0.0 {
            return n;
        }
        let falloff = (1.0 - smoothstep(0.6, 1.0, d)) * (-2.5 * d).exp();
        let shifted = (s + self.w * (4.0 / std::f32::consts::TAU) * falloff).rem_euclid(4.0);
        let (u, v) = contour_point(shifted, d, self.a, self.b);
        from_local(self.center, self.east, self.north, u, v)
    }
}

/// Position `s` in `[0, 4)` of plane point `(u, v)` along the diamond
/// contour of level `d` (`|u|/a + |v|/b = d`), one unit per quadrant,
/// counter-clockwise from the positive `u` vertex.
pub fn contour_param(u: f32, v: f32, a: f32, b: f32, d: f32) -> f32 {
    if d <= 0.0 {
        return 0.0;
    }
    if u > 0.0 && v >= 0.0 {
        v / (b * d)
    } else if u <= 0.0 && v > 0.0 {
        1.0 + (-u) / (a * d)
    } else if u < 0.0 && v <= 0.0 {
        2.0 + (-v) / (b * d)
    } else {
        3.0 + u / (a * d)
    }
}

/// Inverse of `contour_param`: the plane point at `s` on contour `d`.
pub fn contour_point(s: f32, d: f32, a: f32, b: f32) -> (f32, f32) {
    let s = s.rem_euclid(4.0);
    let k = s.floor();
    let f = s - k;
    match k as i32 {
        0 => (a * d * (1.0 - f), b * d * f),
        1 => (-a * d * f, b * d * (1.0 - f)),
        2 => (-a * d * (1.0 - f), -b * d * f),
        _ => (a * d * f, -b * d * (1.0 - f)),
    }
}

/// Storm slots: large, medium and small.
pub const STORM_SLOTS: usize = 3;
/// Size of each storm relative to `vortex_radius`.
pub const STORM_SIZE: [f32; STORM_SLOTS] = [1.0, 0.72, 0.5];
/// Turning sense of each storm: the medium one is an anticyclone.
pub const STORM_SENSE: [f32; STORM_SLOTS] = [1.0, -1.0, 1.0];
/// Angular speed of the glowing spiral in each core, radians per second.
const STORM_SPIN: [f32; STORM_SLOTS] = [2.0, 2.6, 3.3];
/// Core glow of each storm relative to the large one.
const STORM_GLOW: [f32; STORM_SLOTS] = [1.0, 0.8, 0.65];
/// Strike chance of each storm's lightning relative to the large one.
const STORM_LIGHTNING: [f32; STORM_SLOTS] = [1.0, 0.75, 0.55];
/// Storm centers are seeded within these angles of the default view: the
/// large storm forms in view, the others roam the latitude band.
pub const STORM_CONE_DEG: [f32; STORM_SLOTS] = [35.0, 120.0, 180.0];
/// Extra angular gap between storm diamonds, radians.
pub const STORM_MARGIN: f32 = 0.08;
/// Earlier storms a new one keeps clear of: the two still in the sky.
pub const STORM_HISTORY: usize = 2;
/// Seeded candidates tried before the fixed fallback directions.
const STORM_ATTEMPTS: u32 = 16;
const STORM_FALLBACKS: usize = 64;
/// First storm replayed: the oldest one in the sky at `t = 0`.
pub const STORM_ORIGIN: i64 = -2;

/// Moves `c` to the nearest direction within `max_lat` of the equator.
fn limit_latitude(c: Vector3, max_lat: f32) -> Vector3 {
    let lat = c.y.clamp(-1.0, 1.0).asin();
    if lat.abs() <= max_lat {
        return c;
    }
    let y = max_lat.sin() * lat.signum();
    let h = (c.x * c.x + c.z * c.z).sqrt();
    if h < 1e-6 {
        return Vector3::new(max_lat.cos(), y, 0.0);
    }
    let r = max_lat.cos() / h;
    Vector3::new(c.x * r, y, c.z * r)
}

/// Storm life: calm, perturbation, formation, apogee, dissolution and
/// stability, as an envelope over `tau` in `[0.1, 0.9]`.
fn storm_envelope(tau: f32) -> f32 {
    envelope((tau - 0.1) / 0.8, 0.4375, 0.25, 0.3125)
}

/// East-west and north-south half-diagonals of a storm of `slot`.
fn storm_axes(p: &HouseParams, slot: usize) -> (f32, f32) {
    let a = p.vortex_radius * STORM_SIZE[slot];
    (a, a * p.vortex_aspect)
}

/// Angular radius of the diamond of a storm of `slot`.
pub fn storm_reach(p: &HouseParams, slot: usize) -> f32 {
    let (a, b) = storm_axes(p, slot);
    a.max(b).atan()
}

/// Centers of storms `STORM_ORIGIN..=last` in birth order: each takes the
/// first seeded candidate of its slot cone, inside the latitude limit,
/// that keeps `STORM_MARGIN` clear of the previous `STORM_HISTORY` storms
/// and one radius away from where its slot last formed; otherwise the
/// clearest fixed direction. The medium and small storms prefer candidates
/// that leave the middle of the default view to the large one.
// ponytail: O(last) replay per frame (600 storms per hour of simulated
// time); checkpoint the sequence if days-long runs matter.
pub fn place_storms(p: &HouseParams, focus: Vector3, last: i64) -> Vec<Placement> {
    let max_lat = p.vortex_max_latitude.to_radians();
    let seed = p.seed as u32;
    let mut placed: Vec<Placement> = Vec::new();
    for index in STORM_ORIGIN..=last.max(STORM_ORIGIN) {
        let slot = index.rem_euclid(STORM_SLOTS as i64) as usize;
        let r = storm_reach(p, slot);
        let cone = STORM_CONE_DEG[slot].to_radians();
        let recent = &placed[placed.len().saturating_sub(STORM_HISTORY)..];
        // The storm its slot just left never shares the sky with this one:
        // moving one diamond radius away is enough to form elsewhere.
        let left = placed
            .len()
            .checked_sub(STORM_SLOTS)
            .map(|i| placed[i].center);
        let fits = |c: Vector3| {
            let gap = clearance(c, r, recent, STORM_MARGIN);
            left.map_or(gap, |l| gap.min(angular_distance(c, l) - r))
        };
        // The medium and small storms first try to leave the middle of the
        // view free for the next large storm.
        let aside = |c: Vector3| {
            slot == 0 || angular_distance(c, focus) >= storm_reach(p, 0) + r + STORM_MARGIN
        };
        let candidates = (0..STORM_ATTEMPTS)
            .map(|a| limit_latitude(hash_direction(seed, 60 + a, index, focus, cone), max_lat));
        let center = candidates
            .clone()
            .find(|&c| fits(c) >= 0.0 && aside(c))
            .or_else(|| candidates.clone().find(|&c| fits(c) >= 0.0))
            .unwrap_or_else(|| {
                (0..STORM_FALLBACKS)
                    .map(|i| limit_latitude(fibonacci_direction(i, STORM_FALLBACKS), max_lat))
                    .max_by(|a, b| fits(*a).total_cmp(&fits(*b)))
                    .expect("fallback directions")
            });
        placed.push(Placement {
            index,
            center,
            reach: r,
        });
    }
    placed
}

/// Storm `index` at `center` and phase `tau` of its cycle `k`.
fn storm(p: &HouseParams, index: i64, k: i64, center: Vector3, tau: f32) -> Storm {
    let slot = index.rem_euclid(STORM_SLOTS as i64) as usize;
    let (east, north) = east_north(center);
    let on = p.vortex >= 0.5 && slot < p.vortex_count.round() as usize;
    let env = if on { storm_envelope(tau) } else { 0.0 };
    let strength = if slot == 0 {
        p.vortex_strength
    } else {
        p.secondary_strength
    };
    let (a, b) = storm_axes(p, slot);
    Storm {
        center,
        east,
        north,
        a,
        b,
        w: STORM_SENSE[slot] * strength * env,
        envelope: env,
        cycle: k,
        slot,
        reach_cos: storm_reach(p, slot).cos(),
    }
}

/// The three storms at time `t` (clamped to `t >= 0`), by slot. A slot
/// beyond `vortex_count` keeps its place with no circulation.
pub fn active_storms(p: &HouseParams, t: f32) -> [Storm; STORM_SLOTS] {
    let (az, el) = ViewPreset::Oblique.angles();
    let focus = direction_from_angles(az, el);
    let third = p.vortex_period / STORM_SLOTS as f32;
    let cycles: [(i64, f32); STORM_SLOTS] =
        std::array::from_fn(|slot| cycle(t.max(0.0), p.vortex_period, -(slot as f32) * third));
    let index = |slot: usize| STORM_SLOTS as i64 * cycles[slot].0 + slot as i64;
    let newest = (0..STORM_SLOTS).map(index).max().unwrap_or(0);
    let placed = place_storms(p, focus, newest);
    std::array::from_fn(|slot| {
        let center = placed[(index(slot) - STORM_ORIGIN) as usize].center;
        storm(p, index(slot), cycles[slot].0, center, cycles[slot].1)
    })
}

pub fn prepare(params: &HouseParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    let seed = params.seed as u32;
    let t = uniforms.time;
    let storms = active_storms(params, t);
    let strikes = Box::new(std::array::from_fn(|c| {
        strike(params, &storms[CHANNEL_STORM[c].unwrap_or(0)], c, t)
    }));
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
        seed,
        storms,
        strikes,
    }
}

impl Frame {
    /// Sampling direction after the differential wind of this frame.
    fn winds(&self, n: Vector3) -> Vector3 {
        let p = &self.params;
        let omega = p.band_flow + p.differential_wind * (3.0 * PI * n.y).sin();
        rotate_y(n, omega * self.uniforms.time)
    }

    /// Warped band coordinate `y'` of a sampling direction.
    pub fn band_coordinate(&self, n: Vector3) -> f32 {
        let w = self.winds(n);
        w.y + self.params.band_warp * fbm(w * 1.6, 4, self.seed)
    }

    /// Band value in `[0, 1]` (the ramp position before tinting).
    pub fn band_value(&self, n: Vector3) -> f32 {
        let y = self.band_coordinate(n);
        0.5 + 0.5 * (y * self.params.band_count * PI / 2.0).sin()
    }

    /// Sampling direction after every storm, in slot order. The diamonds
    /// never overlap, so at most one of them moves a direction.
    pub fn vortex_warp(&self, n: Vector3) -> Vector3 {
        self.storms.iter().fold(n, |q, st| st.warp(q))
    }

    /// Cloud color at a surface direction, storms included, before lighting.
    pub fn albedo(&self, n: Vector3) -> Vector3 {
        self.band_albedo(self.vortex_warp(n))
    }

    /// Cloud color of a sampling direction (no storm), before lighting.
    pub fn band_albedo(&self, n: Vector3) -> Vector3 {
        let p = &self.params;
        let w = self.winds(n);
        let y = w.y + p.band_warp * fbm(w * 1.6, 4, self.seed);
        let v = 0.5 + 0.5 * (y * p.band_count * PI / 2.0).sin();
        let mut color = ramp(&BAND_RAMP, v);
        let tint = saturate(0.5 + fbm(w * 0.9, 2, self.seed.wrapping_add(1)));
        color = mix3(color, hex(0x4E0E27), 0.35 * tint * (1.0 - v));
        let filaments = fbm(
            Vector3::new(2.0 * w.x, 8.0 * y, 2.0 * w.z),
            3,
            self.seed.wrapping_add(2),
        );
        color * (1.0 + p.filament_strength * filaments)
    }

    /// Glowing spiral cores of the storms, each turning at its own speed
    /// and sense.
    fn core_glow(&self, n: Vector3) -> Vector3 {
        let mut total = Vector3::zero();
        for storm in self.storms.iter().filter(|st| st.w != 0.0) {
            let Some((d, _)) = storm.coords(n) else {
                continue;
            };
            let s = storm.coords(storm.warp(n)).map_or(0.0, |(_, s)| s);
            let spin = STORM_SENSE[storm.slot] * STORM_SPIN[storm.slot];
            let swirl = 0.55 + 0.45 * (6.0 * s + 10.0 * d - spin * self.uniforms.time).sin();
            let glow =
                storm.envelope * STORM_GLOW[storm.slot] * (1.0 - smoothstep(0.0, 0.45, d)) * swirl;
            total += mix3(hex(0xFF4FB8), hex(0xF4C6DA), 1.0 - d) * glow;
        }
        total
    }

    /// Neon lightning emitted at unit direction `n`: a jagged main bolt
    /// with two forks per active strike, a pale core inside a violet or
    /// pink halo, and a faint glow of the clouds it lights up. Storm bolts
    /// are drawn in the domain the vortex deforms, so they curl with it.
    pub fn lightning(&self, n: Vector3) -> Vector3 {
        let mut total = Vector3::zero();
        for st in self.strikes.iter() {
            if st.flash <= 0.0 || n.dot(st.center) < (1.3 * st.radius).min(1.5).cos() {
                continue;
            }
            let (reach, sample) = if let Some(i) = st.storm {
                // Confined to its storm diamond, fading before its edge.
                match self.storms[i].coords(n) {
                    Some((d, _)) => (1.0 - smoothstep(0.7, 1.0, d), self.vortex_warp(n)),
                    None => continue,
                }
            } else {
                let dist = angular_distance(n, st.center);
                (1.0 - smoothstep(0.7 * st.radius, 1.2 * st.radius, dist), n)
            };
            let Some((u, v)) = local_plane(sample, st.center, st.east, st.north) else {
                continue;
            };
            if reach <= 0.0 {
                continue;
            }
            let half = 0.9 * st.radius.tan();
            let (sa, ca) = st.angle.sin_cos();
            let (x, y) = (u * ca + v * sa + half, -u * sa + v * ca);
            let amplitude = 0.16 * half;
            let width = 0.012;
            // Main bolt across the region, then two forks leaving it.
            let mut best = bolt_distance(x, y, 2.0 * half, amplitude, st.seed)
                .map(|d| d / width)
                .unwrap_or(f32::INFINITY);
            for (i, turn) in [(0_u32, 0.45_f32), (1, -0.6)] {
                let start = 2.0 * half * (0.3 + 0.35 * i as f32);
                let anchor = (PI * start / (2.0 * half)).sin();
                let y0 = amplitude * anchor * jag(7.0 * start / (2.0 * half), st.seed);
                let (bs, bc) = turn.sin_cos();
                let (dx, dy) = (x - start, y - y0);
                let (bx, by) = (dx * bc + dy * bs, -dx * bs + dy * bc);
                let length = 0.8 * half;
                if let Some(d) = bolt_distance(bx, by, length, 0.5 * amplitude, st.seed ^ (i + 1)) {
                    best = best.min(d / (0.7 * width * (1.0 - 0.6 * bx / length)));
                }
            }
            let core = (-best * best).exp();
            let glow = (-(best / 4.0).powi(2)).exp();
            let tint = mix3(hex(0xC13CFF), hex(0xFF4FB8), st.hue);
            let clouds = (-(angular_distance(n, st.center) / st.radius).powi(2)).exp();
            total += (tint * (0.9 * glow)
                + mix3(hex(0xFF4FB8), hex(0xF8E8FF), core) * (1.4 * core)
                + hex(0xC13CFF) * (0.2 * clouds))
                * (st.flash * reach);
        }
        total * self.params.lightning_intensity
    }

    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        let n = s.normal;
        match self.debug {
            DebugView::Height => return Vector3::one() * self.band_value(self.vortex_warp(n)),
            DebugView::Materials => {
                let inside = self
                    .storms
                    .iter()
                    .any(|st| st.w != 0.0 && st.coords(n).is_some());
                return if inside { hex(0xFF00FF) } else { hex(0x52106A) };
            }
            _ => {}
        }
        let p = &self.params;
        let u = &self.uniforms;
        let lit = (self.albedo(n) + self.core_glow(n))
            * (0.18 + 0.82 * wrap_lambert(n, u.light_direction, p.wrap));
        let rim = fresnel(n, u.eye_direction, 3.0);
        lit + mix3(hex(0xC13CFF), hex(0xF0408A), rim) * (p.rim_strength * rim) + self.lightning(n)
    }
}
