//! Joker planet: a theatrical harlequin. The relief is a field of faceted
//! diamond pyramids (the grid is sheared by latitude into a spiral, every
//! diamond rises to its own seeded height, the +X hemisphere stands taller)
//! crowned by six jester-hat spikes. The shader paints it as a costume:
//! four motley panels (emerald harlequin, purple and burgundy harlequin,
//! asymmetric triangles, target diamonds) stitched by zigzag neon seams,
//! a comedy mask grinning on the +X side and a tragedy mask frowning on the
//! -X side, neon ridges with energy pulses and a glitch band that slips the
//! pattern. Heights are in planet radii; the relief never moves (collisions
//! stay consistent), only the colors animate.

use crate::celestial::{ParamSpec, check_range, find_spec};
use crate::shading::lighting::{blinn_phong, lambert};
use crate::shading::math::{hex, mix3, smoothstep};
use crate::shading::noise::hash3;
use crate::solar::planet::clover::planet_params;
use crate::solar::planet::field::{HeightField, NORMAL_EPS};
use raylib::math::Vector3;
use std::f32::consts::{PI, TAU};

/// Height of the tallest relief (a jester-hat spike's tip), radii.
pub const JOKER_HEIGHT: f32 = 0.12;
/// Height of the diamond pyramids, share of the field's height.
const PYRAMID_SHARE: f32 = 0.66;
/// Jester-hat spikes: how many, and the chord radius of each one's base.
pub const JOKER_SPIKES: usize = 6;
const SPIKE_BASE: f32 = 0.24;
/// Latitude and longitude of each spike before its seeded jitter: three
/// leaning over the north, two over the south, one on the equator between
/// the masks.
const SPIKE_SITES: [(f32, f32); JOKER_SPIKES] = [
    (0.95, 0.9),
    (0.88, 3.0),
    (1.02, 5.0),
    (-0.78, 1.9),
    (-0.82, 4.3),
    (0.05, 1.65),
];

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
    /// Unit directions of the jester-hat spikes (from `seed`).
    pub spikes: [Vector3; JOKER_SPIKES],
}

impl JokerField {
    /// Twelve diamonds around the equator, twisted three cells, and six
    /// spikes up to `JOKER_HEIGHT`, for `seed`.
    pub fn new(seed: u32) -> Self {
        let spikes = std::array::from_fn(|k| {
            let (lat, lon) = SPIKE_SITES[k];
            let lon = lon + 0.5 * (unit_hash(seed, k as i32, 0, 0x5b1e) - 0.5);
            Vector3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin())
        });
        Self {
            height: JOKER_HEIGHT,
            cells: 12.0,
            twist: 3.0,
            seed,
            spikes,
        }
    }

    /// Checks a field built by hand: finite values, a height that is not
    /// negative and at least two cells.
    pub fn validate(&self) -> Result<(), String> {
        let spikes = self.spikes.iter().flat_map(|v| [v.x, v.y, v.z]);
        if ![self.height, self.cells, self.twist]
            .into_iter()
            .chain(spikes)
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

    /// How high the jester-hat spikes raise the unit `n`, share of the
    /// field's height: a sharp cone on each spike's base.
    pub fn spike(&self, n: Vector3) -> f32 {
        // Most points lie off every base: compare squared chords first.
        let near = self
            .spikes
            .iter()
            .map(|&s| (n - s).dot(n - s))
            .fold(f32::INFINITY, f32::min);
        if near >= SPIKE_BASE * SPIKE_BASE {
            return 0.0;
        }
        (1.0 - near.sqrt() / SPIKE_BASE).powf(1.6)
    }

    /// Height above the radius along the unit `n`: a four-sided pyramid on
    /// each diamond (sharp ridges, flat facets) scaled by the diamond's
    /// seeded share and flattened near the poles where the diamonds pinch,
    /// or a jester-hat spike where one rises higher; all of it scaled by
    /// the hemisphere (the +X side stands taller).
    pub fn height(&self, n: Vector3) -> f32 {
        let (cell, (fa, fb)) = diamond_cell(n, self.cells.max(2.0), self.twist);
        let pyramid = (1.0 - 2.0 * fa.abs().max(fb.abs())).max(0.0);
        let side = 0.65 + 0.35 * n.x;
        let polar = 1.0 - smoothstep(0.8, 0.97, n.y.abs());
        let diamonds = PYRAMID_SHARE * self.cell_amplitude(cell) * pyramid * polar;
        self.height.max(0.0) * diamonds.max(self.spike(n)) * side
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
const PURPLE: Vector3 = hex(0x4A0E78);
const EMERALD: Vector3 = hex(0x10B67A);
const MAGENTA: Vector3 = hex(0xF0139F);
const BURGUNDY: Vector3 = hex(0x7A0C2C);
/// Neon tones of the seams, ridges and masks (emissive: they glow on the
/// night side too).
const NEON_GREEN: Vector3 = hex(0x2BFFA8);
const NEON_PINK: Vector3 = hex(0xFF2FC0);
const AMBIENT: f32 = 0.10;
/// Motley panels around the planet and the half-width of their seams,
/// panel widths.
const PANELS: f32 = 4.0;
const SEAM: f32 = 0.035;
/// Theatre masks around the equator, comedy and tragedy in turn at
/// staggered latitudes: the unit direction each one faces (object space)
/// and whether it grins (comedy) or frowns (tragedy).
const MASKS: [(Vector3, bool); 4] = [
    (Vector3::new(0.95326, 0.17903, 0.24341), true),
    (Vector3::new(-0.2725, -0.14944, 0.95048), false),
    (Vector3::new(-0.93383, 0.19867, -0.29749), true),
    (Vector3::new(0.28225, -0.099833, -0.95413), false),
];
/// Size of a mask: its oval is 0.62 x 0.56 times this, in the tangent
/// plane of its center.
const MASK_SIZE: f32 = 0.72;
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

/// Which motley panel holds the unit `n` and how far it is from the
/// nearest seam, in panel widths: the panels turn with latitude and their
/// seams zigzag like a jester's collar.
fn panel(n: Vector3, lat: f32, lon: f32) -> (i32, f32) {
    let zig = (lat * 7.0 / PI).rem_euclid(1.0);
    let zig = 0.12 * (2.0 * zig - 1.0).abs();
    let x = (lon / TAU + 0.35 * lat / PI + zig) * PANELS + 0.04 * n.y;
    let f = x - x.floor();
    ((x.floor() as i32).rem_euclid(PANELS as i32), f.min(1.0 - f))
}

/// The mask facing `dir` over the unit `n`, if `n` lies on it: its albedo
/// and its glow. The face is a dark plate with a neon outline, harlequin
/// diamond eyes, a tear diamond and a mouth; the comedy mask grins with
/// teeth, the tragedy mask frowns.
fn mask(n: Vector3, dir: Vector3, comedy: bool, flicker: bool) -> Option<(Vector3, Vector3)> {
    if n.dot(dir) < 0.8 {
        return None;
    }
    let right = Vector3::new(0.0, 1.0, 0.0).cross(dir).normalized();
    let up = dir.cross(right);
    let (u, v) = (n.dot(right) / MASK_SIZE, n.dot(up) / MASK_SIZE - 0.02);
    let oval = (u / 0.62).powi(2) + (v / 0.56).powi(2);
    if oval > 1.0 {
        return None;
    }
    let (ink, glow) = if comedy {
        (NEON_PINK, NEON_GREEN)
    } else {
        (NEON_GREEN, NEON_PINK)
    };
    let (ink, glow) = if flicker { (glow, ink) } else { (ink, glow) };
    let plate = mix3(BLACK, PURPLE, 0.35 + 0.25 * v);
    // The outline: a neon band just inside the oval.
    if oval > 0.84 {
        return Some((BLACK, ink * 1.3));
    }
    // Diamond eyes, slanted up (comedy) or down (tragedy).
    let slant = if comedy { 0.12 } else { -0.12 };
    for side in [-1.0f32, 1.0] {
        let du = u - side * 0.24;
        let dv = v - 0.17 - slant * side * du;
        let eye = du.abs() / 0.15 + dv.abs() / 0.085;
        if eye < 1.0 {
            return Some(if eye < 0.42 {
                (BLACK, Vector3::zero())
            } else {
                (BLACK, glow * (0.6 + 0.4 * eye))
            });
        }
        // A tear diamond under the right eye (comedy) or both (tragedy).
        let tear = du.abs() / 0.04 + (v + 0.03).abs() / 0.085;
        if tear < 1.0 && (side > 0.0 || !comedy) {
            return Some((BURGUNDY, ink * 0.9));
        }
    }
    // The mouth: a lens along a smile (or frown) curve.
    let k = if comedy { 0.85 } else { -0.7 };
    let base = if comedy { -0.3 } else { -0.2 };
    let half = 0.07 * (1.0 - (u / 0.44).powi(2));
    let d = (v - base - k * u * u).abs();
    if half > 0.0 && d < half {
        let teeth = comedy && (u * 16.0).rem_euclid(1.0) < 0.2 && d < 0.75 * half;
        return Some(if teeth {
            (BLACK, Vector3::zero())
        } else {
            (BLACK, ink * 1.1)
        });
    }
    Some((plate, Vector3::zero()))
}

impl PreparedJoker {
    /// Color at `object_position`: a motley of four panels lit by the
    /// star through the facet normals (emerald and black harlequin, purple
    /// and burgundy harlequin, asymmetric triangles in the whole palette,
    /// target diamonds), zigzag neon seams between them, the comedy and
    /// tragedy masks over the panels, black ridges lit by alternating neon
    /// and energy pulses, a burgundy-to-magenta wave sweeping the spiral,
    /// neon spike tips, glitching diamonds and a glitch band that slips the
    /// pattern sideways and swaps the neon.
    pub fn shade(&self, object_position: Vector3, eye: Vector3) -> Vector3 {
        let n = object_position.normalized();
        let f = self.field;
        let p = &self.params;
        let facet = HeightField::Joker(f).normal(n, NORMAL_EPS);
        let normal = (n + (facet - n) * p.bump.clamp(0.0, 1.0)).normalized();
        let lit = AMBIENT + (1.0 - AMBIENT) * lambert(normal, self.light);
        let lat = n.y.clamp(-1.0, 1.0).asin();
        let mut lon = n.z.atan2(n.x);
        // The glitch band: a latitude strip that jumps every glitch step
        // and slips the pattern sideways.
        let step = (self.time * JOKER_GLITCH_STEPS).floor() as i32;
        let band_lat = (unit_hash(p.seed as u32, step, 0, 0xba4d) - 0.5) * 2.4;
        let in_band = p.glitch > 0.0 && (lat - band_lat).abs() < 0.05 * p.glitch;
        if in_band {
            lon += 0.35;
        }
        let slipped = if in_band {
            Vector3::new(lat.cos() * lon.cos(), n.y, lat.cos() * lon.sin())
        } else {
            n
        };
        let (cell, (fa, fb)) = diamond_cell(slipped, f.cells.max(2.0), f.twist);
        let checker = (cell.0 + cell.1).rem_euclid(2) == 0;
        let (panel_id, seam) = panel(n, lat, lon);
        let mut base = match panel_id {
            0 => {
                if checker {
                    EMERALD
                } else {
                    BLACK
                }
            }
            1 => {
                if checker {
                    PURPLE
                } else {
                    BURGUNDY
                }
            }
            2 => {
                // Each diamond split into two triangles of seeded colors.
                let half = if fa > fb { 1 } else { 0 };
                let pick = unit_hash(p.seed as u32, cell.0, cell.1, 0x7e1 + half);
                [BLACK, EMERALD, PURPLE, BURGUNDY, MAGENTA][(pick * 4.99) as usize]
            }
            _ => {
                // Target diamonds: rings around each diamond's center.
                let r = fa.abs().max(fb.abs());
                if (r * 7.0) as i32 % 2 == 0 {
                    if checker { PURPLE } else { EMERALD }
                } else {
                    BLACK
                }
            }
        };
        // The color wave: a band sweeping along the spiral.
        let phase = (lon / TAU + 2.0 * lat / PI - p.wave_speed * self.time).rem_euclid(1.0);
        let wave = smoothstep(0.0, 0.25, phase) * (1.0 - smoothstep(0.25, 0.5, phase));
        base = mix3(base, mix3(BURGUNDY, MAGENTA, phase * 2.0), 0.45 * wave);
        let glitching = diamond_glitch(cell, self.time, p);
        if glitching {
            base = if checker { MAGENTA } else { BURGUNDY };
        }
        let mut glow = Vector3::zero();
        let edge = 1.0 - 2.0 * fa.abs().max(fb.abs());
        let mut ridge = 1.0 - smoothstep(0.02, 0.14, edge);
        for (dir, comedy) in MASKS {
            if let Some((albedo, light)) = mask(n, dir, comedy, glitching) {
                base = albedo;
                glow += light;
                ridge *= 0.25;
            }
        }
        let shine = 0.35 * blinn_phong(normal, self.light, eye, 48.0);
        let color = mix3(base * lit, BLACK, 0.85 * ridge) + Vector3::new(shine, shine, shine);
        // Neon on the ridges, alternating green and pink by diamond, with
        // energy pulses every three cells.
        let along = (cell.0 as f32 + cell.1 as f32) / 3.0;
        let pulse = smoothstep(
            0.8,
            1.0,
            (along - p.pulse_speed * self.time).rem_euclid(1.0),
        );
        let neon = if (cell.0 - cell.1).rem_euclid(3) == 0 {
            NEON_GREEN
        } else {
            NEON_PINK
        };
        glow += neon * (ridge * (0.35 + 0.65 * pulse));
        // Zigzag seams between the panels, green and pink in turn.
        let seam_glow = 1.0 - smoothstep(0.4 * SEAM, SEAM, seam);
        let seam_color = if panel_id % 2 == 0 {
            NEON_GREEN
        } else {
            NEON_PINK
        };
        glow += seam_color * (1.2 * seam_glow);
        // Spike tips burn like the bells of a jester's hat.
        let tip = smoothstep(0.3, 0.75, f.spike(n));
        glow += mix3(NEON_PINK, NEON_GREEN, (cell.0 & 1) as f32) * (1.4 * tip);
        if in_band {
            // The band swaps the neon and tints toward magenta.
            glow = Vector3::new(glow.z, glow.x, glow.y) + MAGENTA * 0.25;
        }
        color + glow
    }
}
