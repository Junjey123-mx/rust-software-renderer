//! House of Cards: violet gas giant with diamond storms (gas giant).
//!
//! Latitude bands warped by fBm slide past each other under a latitude-
//! dependent wind (a rotation of the sampling direction, never an angle
//! with a seam), mapped through violet, magenta and pearl ramps with an
//! atmospheric rim. A migrating storm advects the band sampling domain along
//! the contours of an L1 diamond metric in its tangent plane, so the bands
//! wind into rhombic spirals. Brief neon lightning crackles inside the
//! storm diamond and now and then between the bands. Its only symbol is the
//! diamond.

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::pipeline::{ViewPreset, direction_from_angles};
use crate::shading::cycle::{cycle, envelope};
use crate::shading::lighting::{fresnel, wrap_lambert};
use crate::shading::math::{hex, mix3, ramp, saturate, smoothstep};
use crate::shading::noise::{fbm, hash3};
use crate::shading::sphere::{
    angular_distance, east_north, from_local, hash_direction, local_plane, rotate_y,
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
        vortex_period: 18.0, 4.0, 90.0, "seconds per storm life cycle";
        vortex_radius: 0.45, 0.1, 0.5, "half-diagonal of the storm diamond in its tangent plane";
        vortex_strength: 6.0, 0.0, 8.0, "circulation at the storm core, radians";
        vortex_aspect: 1.35, 0.6, 2.0, "north-south over east-west diagonal of the diamond";
        secondary_strength: 0.6, 0.0, 2.0, "circulation of the smaller secondary storm";
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
    /// Principal and secondary storms of this frame.
    pub storms: [Storm; 2],
    /// Lightning channels of this instant (boxed: built once per frame).
    pub strikes: Box<[Strike; LIGHTNING_CHANNELS]>,
}

/// Lightning channels: 0 follows the main storm diamond, 1 and 2 strike
/// between the bands.
pub const LIGHTNING_CHANNELS: usize = 3;
/// Length of one possible strike per channel, seconds.
pub const STRIKE_SLOTS: [f32; LIGHTNING_CHANNELS] = [0.11, 0.17, 0.23];

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
    /// Bound to the main storm: confined to its diamond.
    pub storm_bound: bool,
    seed: u32,
}

/// Uniform value in `[0, 1)` from integer coordinates.
fn unit_hash(a: i32, b: i32, c: i32, seed: u32) -> f32 {
    hash3(a, b, c, seed) as f32 / (u32::MAX as f32 + 1.0)
}

/// State of lightning channel `c` at time `t`. Time is cut into short slots;
/// a hash decides whether a slot strikes (more often while the main storm
/// is strong) and where. The flash decays within its slot, sometimes with a
/// second pulse, so discharges are brief and intermittent.
pub fn strike(p: &HouseParams, storm: &Storm, c: usize, t: f32) -> Strike {
    let seed = (p.seed as u32) ^ 0xa5a5_1f3d;
    let slot = (t / STRIKE_SLOTS[c]).floor();
    let phase = t / STRIKE_SLOTS[c] - slot;
    let (k, ch) = (slot as i64, c as i32);
    let (lo, hi) = (k as i32, (k >> 32) as i32);
    let h = |salt: i32| unit_hash(ch * 8 + salt, lo, hi, seed);
    let storm_bound = c == 0;
    let chance = if p.lightning < 0.5 {
        0.0
    } else if storm_bound {
        if storm.w > 0.0 {
            p.lightning_rate * (0.15 + 0.75 * storm.envelope)
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
        storm_bound,
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
    /// Circulation of this frame, radians (0 at cycle boundaries).
    pub w: f32,
    /// Life-cycle envelope in `[0, 1]`.
    pub envelope: f32,
    pub cycle: i64,
}

impl Storm {
    /// Diamond distance `d = |u|/a + |v|/b` and contour parameter of `n`,
    /// or `None` when `n` is outside the storm (`d >= 1`) or far away.
    pub fn coords(&self, n: Vector3) -> Option<(f32, f32)> {
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

/// Storm centers are seeded within this angle of the default view.
pub const STORM_CONE_DEG: f32 = 35.0;

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

fn storm(
    p: &HouseParams,
    slot: u32,
    t: f32,
    period: f32,
    phase: f32,
    scale: f32,
    strength: f32,
) -> Storm {
    let (az, el) = ViewPreset::Oblique.angles();
    let focus = direction_from_angles(az, el);
    let (k, tau) = cycle(t, period, phase * period);
    let center = limit_latitude(
        hash_direction(p.seed as u32, slot, k, focus, STORM_CONE_DEG.to_radians()),
        p.vortex_max_latitude.to_radians(),
    );
    let (east, north) = east_north(center);
    let env = if p.vortex >= 0.5 {
        storm_envelope(tau)
    } else {
        0.0
    };
    Storm {
        center,
        east,
        north,
        a: p.vortex_radius * scale,
        b: p.vortex_radius * p.vortex_aspect * scale,
        w: strength * env,
        envelope: env,
        cycle: k,
    }
}

pub fn prepare(params: &HouseParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    let seed = params.seed as u32;
    let t = uniforms.time;
    let storms = [
        storm(
            params,
            0,
            t,
            params.vortex_period,
            0.0,
            1.0,
            params.vortex_strength,
        ),
        storm(params, 1, t, 11.0, 0.37, 0.5, params.secondary_strength),
    ];
    let strikes = Box::new(std::array::from_fn(|c| strike(params, &storms[0], c, t)));
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

    /// Sampling direction after the principal and secondary storms.
    pub fn vortex_warp(&self, n: Vector3) -> Vector3 {
        self.storms[1].warp(self.storms[0].warp(n))
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

    /// Glowing spiral core of the principal storm.
    fn core_glow(&self, n: Vector3) -> Vector3 {
        let storm = &self.storms[0];
        if storm.envelope <= 0.0 {
            return Vector3::zero();
        }
        let Some((d, _)) = storm.coords(n) else {
            return Vector3::zero();
        };
        let warped = storm.warp(n);
        let s = storm.coords(warped).map_or(0.0, |(_, s)| s);
        let swirl = 0.55 + 0.45 * (6.0 * s + 10.0 * d - 2.0 * self.uniforms.time).sin();
        let glow = storm.envelope * (1.0 - smoothstep(0.0, 0.45, d)) * swirl;
        mix3(hex(0xFF4FB8), hex(0xF4C6DA), 1.0 - d) * glow
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
            let (reach, sample) = if st.storm_bound {
                // Confined to the storm diamond, fading before its edge.
                match self.storms[0].coords(n) {
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
                    .any(|st| st.w > 0.0 && st.coords(n).is_some());
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
