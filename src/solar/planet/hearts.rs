//! Hearts planet relief: a seeded fbm cut into terraces, so wide flat
//! plateaus (where the city stands) are split by stepped canyons. Each step
//! ends in a short smooth ramp, so the normals stay finite. Heights are in
//! planet radii.

use crate::celestial::{ParamSpec, check_range, find_spec};
use crate::shading::lighting::lambert;
use crate::shading::math::{hex, mix3, smoothstep};
use crate::shading::noise::fbm;
use crate::shading::sdf::{P2, circle, triangle};
use crate::solar::planet::clover::planet_params;
use crate::solar::planet::field::{HeightField, NORMAL_EPS, tangent};
use raylib::math::Vector3;
use std::f32::consts::{PI, TAU};

/// Height of the highest terrace, radii.
pub const HEARTS_HEIGHT: f32 = 0.06;
/// Share of each step taken by the ramp up to the next one.
const EDGE: f32 = 0.18;

/// Height field of the Hearts planet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeartsField {
    /// Terrace levels between the canyon floor and the top (at least 1).
    pub steps: f32,
    /// Height of the highest terrace, radii.
    pub height: f32,
    /// Frequency of the noise on the unit sphere.
    pub freq: f32,
    pub seed: u32,
    /// Largest slope (radii of height per radian) still read as plateau.
    pub plateau_slope: f32,
}

impl HeartsField {
    /// Four terraces up to `HEARTS_HEIGHT` for `seed`.
    pub fn new(seed: u32) -> Self {
        Self {
            steps: 4.0,
            height: HEARTS_HEIGHT,
            freq: 1.6,
            seed,
            plateau_slope: 0.05,
        }
    }

    /// Checks a field built by hand: finite values, at least one step, a
    /// height that is not negative, positive frequency and plateau slope.
    pub fn validate(&self) -> Result<(), String> {
        let values = [self.steps, self.height, self.freq, self.plateau_slope];
        if !values.iter().all(|v| v.is_finite()) {
            return Err("hearts field values must be finite".into());
        }
        if self.steps < 1.0 {
            return Err(format!(
                "hearts steps must be at least 1, got {}",
                self.steps
            ));
        }
        if self.height < 0.0 || self.freq <= 0.0 || self.plateau_slope <= 0.0 {
            return Err("hearts height must be >= 0, freq and plateau_slope > 0".into());
        }
        Ok(())
    }

    /// Terrace height above the radius along the unit `n`: the noise in
    /// `[0, 1]`, floored to `steps` levels with a smooth ramp at the end of
    /// each step.
    pub fn height(&self, n: Vector3) -> f32 {
        let steps = self.steps.max(1.0);
        let x = (0.5 + 0.75 * fbm(n * self.freq, 4, self.seed)).clamp(0.0, 1.0);
        let s = x * steps;
        let floor = s.floor();
        let ramp = smoothstep(1.0 - EDGE, 1.0, s - floor);
        ((floor + ramp) / steps).min(1.0) * self.height
    }

    /// Steepness at the unit `n`: the height gradient's length, radii per
    /// radian (central differences 1e-3 rad wide).
    pub fn slope(&self, n: Vector3) -> f32 {
        const EPS: f32 = 1e-3;
        let t1 = tangent(n);
        let t2 = n.cross(t1);
        let at = |d: Vector3| self.height((n + d * EPS).normalized());
        let du = (at(t1) - at(-t1)) / (2.0 * EPS);
        let dv = (at(t2) - at(-t2)) / (2.0 * EPS);
        du.hypot(dv)
    }

    /// Whether `n` lies on a flat terrace (the city's ground).
    pub fn is_plateau(&self, n: Vector3) -> bool {
        self.slope(n) < self.plateau_slope
    }

    /// Bound of `height`, radii.
    pub fn max_height(&self) -> f32 {
        self.height.max(0.0)
    }

    /// Digest of every parameter, for geometry keys.
    pub fn digest(&self) -> u64 {
        [
            self.steps.to_bits(),
            self.height.to_bits(),
            self.freq.to_bits(),
            self.plateau_slope.to_bits(),
            self.seed,
        ]
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
            (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
}

planet_params! {
    NeonGridParams {
        seed: 11.0, 0.0, 9999.0, "seed of the terraces (rounded)";
        bump: 1.0, 0.0, 1.0, "share of the terrace normal in the lighting";
        line_width: 0.006, 0.0, 0.05, "width of the grid lines, planet radii";
        cells: 24.0, 4.0, 96.0, "grid cells around the equator";
        pulse_speed: 0.5, 0.0, 4.0, "pulses per second running along the lines";
        heart_size: 0.18, 0.0, 0.45, "size of the heart at each crossing, cells";
        glitch: 0.6, 0.0, 1.0, "strength of the glitch bands (0 = off)";
        glitch_rate: 0.08, 0.0, 1.0, "share of latitude bands glitching at a time";
        detail: 1.0, 0.0, 1.0, "share of the grid drawn (lowered with distance when drawn)";
    }
}

impl NeonGridParams {
    /// The terraces of this material's seed.
    pub fn field(&self) -> HeartsField {
        HeartsField::new(self.seed.round().max(0.0) as u32)
    }
}

/// Projected radius (pixels) below which the grid is gone, and from which
/// it is whole: thinner lines than a pixel would shimmer (moire).
pub const GRID_FADE_PX: (f32, f32) = (40.0, 160.0);

/// Share of the grid drawn at `radius_px` pixels of projected radius.
pub fn grid_fade(radius_px: f32) -> f32 {
    if radius_px.is_nan() {
        return 0.0;
    }
    smoothstep(GRID_FADE_PX.0, GRID_FADE_PX.1, radius_px)
}

/// Signed distance to a heart of `size` (negative inside): two circles
/// side by side over a downward triangle, the tip at `(0, -size)`.
pub fn heart_sdf(p: P2, size: f32) -> f32 {
    let s = size;
    let lobes =
        circle(p, (-0.5 * s, 0.25 * s), 0.5 * s).min(circle(p, (0.5 * s, 0.25 * s), 0.5 * s));
    lobes.min(triangle(
        p,
        (-0.95 * s, 0.05 * s),
        (0.95 * s, 0.05 * s),
        (0.0, -s),
    ))
}

const CANYON: Vector3 = hex(0x1A0B2E);
const PLATEAU: Vector3 = hex(0x5A2C85);
/// Glow of the terrace ramps (the steps read on the night side too).
const RAMP: Vector3 = hex(0x2EC4FF);
const LINE: Vector3 = hex(0xFF2E63);
const PULSE: Vector3 = hex(0xFFD1E0);
const HEART: Vector3 = hex(0xFF0A54);
const AMBIENT: f32 = 0.08;

/// The neon grid over the terraces, with its per-frame values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedHearts {
    pub field: HeartsField,
    pub params: NeonGridParams,
    /// Unit direction towards the star, object space.
    pub light: Vector3,
    /// Scene clock (frozen while paused).
    pub time: f32,
}

/// Glitch steps per second: the bands change this often.
pub const GLITCH_STEPS: f32 = 8.0;
/// Latitude bands of the glitch, pole to pole.
pub const GLITCH_BANDS: f32 = 40.0;

fn unit_hash(seed: u32, i: i32, j: i32) -> f32 {
    (crate::shading::noise::hash3(i, j, 0x9117, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// The glitch of the latitude `lat` (radians) at time `t`: the shift of
/// the grid along the longitude and the extra shift of its red channel,
/// both in cells. The clock is cut into `GLITCH_STEPS` steps per second and
/// the sphere into `GLITCH_BANDS` latitude bands; a band glitches during a
/// step when a hash of both falls under `glitch_rate`. Deterministic for a
/// given time (frozen while paused) and `(0, 0)` with `glitch` at 0.
pub fn glitch_offset(lat: f32, t: f32, p: &NeonGridParams) -> (f32, f32) {
    if p.glitch.is_nan() || p.glitch <= 0.0 || !lat.is_finite() || !t.is_finite() {
        return (0.0, 0.0);
    }
    let step = (t * GLITCH_STEPS).floor() as i32;
    let band = ((lat / PI + 0.5) * GLITCH_BANDS).floor() as i32;
    if unit_hash(1, step, band) >= p.glitch_rate {
        return (0.0, 0.0);
    }
    let shift = (unit_hash(2, step, band) - 0.5) * 1.5 * p.glitch;
    (shift, 0.25 * p.glitch)
}

/// Emissive parts of the grid at the unit `n`: the line, the pulse that
/// runs along it and the heart at the crossing, each in `[0, 1]`.
pub fn grid_pattern(n: Vector3, t: f32, p: &NeonGridParams) -> (f32, f32, f32) {
    grid_pattern_shifted(n, t, p, 0.0)
}

/// `grid_pattern` with the grid moved `shift` cells along the longitude.
pub fn grid_pattern_shifted(n: Vector3, t: f32, p: &NeonGridParams, shift: f32) -> (f32, f32, f32) {
    let width = p.line_width;
    if p.detail.is_nan() || p.detail <= 0.0 || width.is_nan() || width <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let cells = p.cells.max(1.0);
    let lat = n.y.clamp(-1.0, 1.0).asin();
    let lon = n.z.atan2(n.x);
    let (u, v) = (lon / TAU * cells + shift, lat / PI * (cells / 2.0));
    let (fu, fv) = (
        (u + 0.5).rem_euclid(1.0) - 0.5,
        (v + 0.5).rem_euclid(1.0) - 0.5,
    );
    // Distances to the nearest meridian and parallel, radians on the
    // sphere (planet radii).
    let cell = TAU / cells;
    let (du, dv) = (fu.abs() * cell * lat.cos(), fv.abs() * cell);
    let line = (1.0 - smoothstep(0.5 * width, width, du.min(dv))) * p.detail;
    // A pulse every four cells, sliding along the nearest line; the time
    // term is wrapped first so a long session keeps its precision.
    let along = if du < dv { v } else { u };
    let shift = (p.pulse_speed * t * 0.25).rem_euclid(1.0);
    let pulse = smoothstep(0.85, 1.0, (along * 0.25 - shift).rem_euclid(1.0)) * line;
    let heart = (1.0 - smoothstep(-0.02, 0.0, heart_sdf((fu, fv), p.heart_size))) * p.detail;
    (line, pulse, heart)
}

impl PreparedHearts {
    /// Color at `object_position`: the terraces lit by the star through
    /// their own normal (canyons darker than plateaus), plus the emissive
    /// grid, pulses and hearts, which glow on the night side too.
    pub fn shade(&self, object_position: Vector3) -> Vector3 {
        let n = object_position.normalized();
        let field = HeightField::Hearts(self.field);
        let terrace = field.normal(n, NORMAL_EPS);
        let normal = (n + (terrace - n) * self.params.bump.clamp(0.0, 1.0)).normalized();
        let lit = AMBIENT + (1.0 - AMBIENT) * lambert(normal, self.light);
        let level = if self.field.height > 0.0 {
            self.field.height(n) / self.field.height
        } else {
            0.0
        };
        let ramp = smoothstep(0.02, 0.15, 1.0 - terrace.dot(n)) * 0.5;
        let base = mix3(CANYON, PLATEAU, level) * lit + RAMP * ramp;
        // The glitch moves the grid of a band and splits its red channel;
        // it never touches the geometry (or the depth).
        let lat = n.y.clamp(-1.0, 1.0).asin();
        let (shift, split) = glitch_offset(lat, self.time, &self.params);
        let glow =
            |(line, pulse, heart): (f32, f32, f32)| LINE * line + PULSE * pulse + HEART * heart;
        let emissive = glow(grid_pattern_shifted(n, self.time, &self.params, shift));
        let red = if split != 0.0 {
            glow(grid_pattern_shifted(
                n,
                self.time,
                &self.params,
                shift + split,
            ))
            .x
        } else {
            emissive.x
        };
        base + Vector3::new(red, emissive.y, emissive.z)
    }
}
