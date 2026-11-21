//! Joker planet: a harlequin of faceted diamond pyramids. The diamond grid
//! is sheared by latitude into a spiral (it seems to twist around the
//! planet), every diamond rises to its own seeded height and one hemisphere
//! stands taller than the other, so the relief is asymmetric. Heights are
//! in planet radii; the relief never moves (collisions stay consistent),
//! only the colors animate.

use crate::celestial::{ParamSpec, check_range, find_spec};
use crate::shading::lighting::{blinn_phong, lambert};
use crate::shading::math::{hex, mix3, smoothstep};
use crate::shading::noise::hash3;
use crate::solar::planet::clover::planet_params;
use crate::solar::planet::field::{HeightField, NORMAL_EPS};
use raylib::math::Vector3;
use std::f32::consts::{PI, TAU};

/// Height of the tallest diamond, radii.
pub const JOKER_HEIGHT: f32 = 0.08;

/// Diamond cell of the unit `n` for `cells` around the equator and the
/// spiral `twist` (cells of longitude shift from pole to pole): the cell
/// index and the position inside it, each in `[-0.5, 0.5)`.
pub fn diamond_cell(n: Vector3, cells: f32, twist: f32) -> ((i32, i32), (f32, f32)) {
    let lat = n.y.clamp(-1.0, 1.0).asin();
    let lon = n.z.atan2(n.x);
    let u = lon / TAU * cells + twist * (lat / PI);
    let v = lat / PI * (cells / 2.0);
    // A square grid turned 45 degrees: diamonds in (u, v).
    let (a, b) = (u + v, u - v);
    let (ia, ib) = (a.floor(), b.floor());
    ((ia as i32, ib as i32), (a - ia - 0.5, b - ib - 0.5))
}

fn unit_hash(seed: u32, i: i32, j: i32, k: i32) -> f32 {
    (hash3(i, j, k, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// Height field of the Joker planet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JokerField {
    /// Height of the tallest diamond, radii.
    pub height: f32,
    /// Diamonds around the equator.
    pub cells: f32,
    /// Longitude shift of the grid from pole to pole, cells.
    pub twist: f32,
    pub seed: u32,
}

impl JokerField {
    /// Twelve diamonds around the equator, twisted three cells, up to
    /// `JOKER_HEIGHT`, for `seed`.
    pub fn new(seed: u32) -> Self {
        Self {
            height: JOKER_HEIGHT,
            cells: 12.0,
            twist: 3.0,
            seed,
        }
    }

    /// Checks a field built by hand: finite values, a height that is not
    /// negative and at least two cells.
    pub fn validate(&self) -> Result<(), String> {
        if ![self.height, self.cells, self.twist]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err("joker field values must be finite".into());
        }
        if self.height < 0.0 || self.cells < 2.0 {
            return Err("joker height must be >= 0 and cells >= 2".into());
        }
        Ok(())
    }

    /// Seeded height share of the diamond `(i, j)`: low most of the time,
    /// a few tall shards.
    fn cell_amplitude(&self, (i, j): (i32, i32)) -> f32 {
        let h = unit_hash(self.seed, i, j, 0x7012);
        if h > 0.82 { 1.0 } else { 0.2 + 0.5 * h }
    }

    /// Height above the radius along the unit `n`: a four-sided pyramid on
    /// each diamond (sharp ridges, flat facets), scaled by the diamond's
    /// seeded share and by the hemisphere (the +X side stands taller),
    /// flattened near the poles where the diamonds pinch.
    pub fn height(&self, n: Vector3) -> f32 {
        let (cell, (fa, fb)) = diamond_cell(n, self.cells.max(2.0), self.twist);
        let pyramid = (1.0 - 2.0 * fa.abs().max(fb.abs())).max(0.0);
        let side = 0.65 + 0.35 * n.x;
        let polar = 1.0 - smoothstep(0.8, 0.97, n.y.abs());
        self.height.max(0.0) * self.cell_amplitude(cell) * pyramid * side * polar
    }

    /// Bound of `height`, radii.
    pub fn max_height(&self) -> f32 {
        self.height.max(0.0)
    }

    /// Digest of every parameter, for geometry keys.
    pub fn digest(&self) -> u64 {
        [
            self.height.to_bits(),
            self.cells.to_bits(),
            self.twist.to_bits(),
            self.seed,
        ]
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
            (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
}

planet_params! {
    JokerParams {
        seed: 23.0, 0.0, 9999.0, "seed of the diamond heights and colors (rounded)";
        bump: 1.0, 0.0, 1.0, "share of the facet normal in the lighting";
        pulse_speed: 0.35, 0.0, 4.0, "energy pulses per second along the spiral";
        wave_speed: 0.12, 0.0, 2.0, "turns per second of the color waves";
        glitch: 0.7, 0.0, 1.0, "strength of the diamond glitch (0 = off)";
        glitch_rate: 0.06, 0.0, 1.0, "share of diamonds glitching at a time";
    }
}

impl JokerParams {
    /// The diamonds of this material's seed.
    pub fn field(&self) -> JokerField {
        JokerField::new(self.seed.round().max(0.0) as u32)
    }
}

const BLACK: Vector3 = hex(0x07040C);
const PURPLE: Vector3 = hex(0x3B0A5C);
const EMERALD: Vector3 = hex(0x0E9F6E);
const MAGENTA: Vector3 = hex(0xE0119D);
const BURGUNDY: Vector3 = hex(0x6E0B25);
const AMBIENT: f32 = 0.10;
/// Glitch steps per second: the glitching diamonds change this often.
pub const JOKER_GLITCH_STEPS: f32 = 6.0;

/// The harlequin over the diamonds, with its per-frame values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedJoker {
    pub field: JokerField,
    pub params: JokerParams,
    /// Unit direction towards the star, object space.
    pub light: Vector3,
    /// Scene clock (frozen while paused).
    pub time: f32,
}

/// Whether the diamond `cell` glitches at time `t`: the clock is cut into
/// `JOKER_GLITCH_STEPS` steps per second and a diamond glitches during a
/// step when a hash of both falls under `glitch_rate`. Deterministic for a
/// given time and never with `glitch` at 0.
pub fn diamond_glitch(cell: (i32, i32), t: f32, p: &JokerParams) -> bool {
    if p.glitch.is_nan() || p.glitch <= 0.0 || !t.is_finite() {
        return false;
    }
    let step = (t * JOKER_GLITCH_STEPS).floor() as i32;
    unit_hash(
        p.seed as u32,
        cell.0 ^ step.wrapping_mul(7919),
        cell.1,
        step,
    ) < p.glitch_rate
}

impl PreparedJoker {
    /// Color at `object_position`: emerald and purple diamonds in a
    /// harlequin checker lit by the star through the facet normals, black
    /// ridges, a burgundy-to-magenta color wave sweeping the spiral, magenta
    /// energy pulses along the ridges (they glow on the night side) and a
    /// few diamonds glitching to magenta or burgundy.
    pub fn shade(&self, object_position: Vector3, eye: Vector3) -> Vector3 {
        let n = object_position.normalized();
        let f = self.field;
        let facet = HeightField::Joker(f).normal(n, NORMAL_EPS);
        let normal = (n + (facet - n) * self.params.bump.clamp(0.0, 1.0)).normalized();
        let lit = AMBIENT + (1.0 - AMBIENT) * lambert(normal, self.light);
        let (cell, (fa, fb)) = diamond_cell(n, f.cells.max(2.0), f.twist);
        let checker = (cell.0 + cell.1).rem_euclid(2) == 0;
        let mut base = if checker { EMERALD } else { PURPLE };
        // The color wave: a band sweeping along the spiral.
        let lat = n.y.clamp(-1.0, 1.0).asin();
        let lon = n.z.atan2(n.x);
        let phase =
            (lon / TAU + 2.0 * lat / PI - self.params.wave_speed * self.time).rem_euclid(1.0);
        let wave = smoothstep(0.0, 0.25, phase) * (1.0 - smoothstep(0.25, 0.5, phase));
        base = mix3(base, mix3(BURGUNDY, MAGENTA, phase * 2.0), 0.55 * wave);
        if diamond_glitch(cell, self.time, &self.params) {
            base = if checker { MAGENTA } else { BURGUNDY };
        }
        let edge = 1.0 - 2.0 * fa.abs().max(fb.abs());
        let ridge = 1.0 - smoothstep(0.02, 0.14, edge);
        let shine = 0.35 * blinn_phong(normal, self.light, eye, 48.0);
        let color = mix3(base * lit, BLACK, 0.85 * ridge) + Vector3::new(shine, shine, shine);
        // Energy pulses run along the ridges, a pulse every three cells.
        let along = (cell.0 as f32 + cell.1 as f32) / 3.0;
        let pulse = smoothstep(
            0.8,
            1.0,
            (along - self.params.pulse_speed * self.time).rem_euclid(1.0),
        );
        color + MAGENTA * (ridge * (0.4 + 0.6 * pulse))
    }
}
