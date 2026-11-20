//! Clover planet relief: four clover-shaped mountains (three round lobes
//! and a stem, smoothly joined) at the corners of a tetrahedron turned by
//! the seed, rising out of a low fbm seabed. Heights are in planet radii.

use crate::celestial::{ParamSpec, check_range, find_spec};
use crate::shading::lighting::{blinn_phong, fresnel, lambert};
use crate::shading::math::{hex, mix3, smoothstep};
use crate::shading::noise::{fbm, hash3};
use crate::shading::sdf::{P2, circle, smooth_min, triangle};
use crate::solar::planet::field::{HeightField, NORMAL_EPS, tangent};
use raylib::math::Vector3;
use std::f32::consts::TAU;

/// Smallest angle between two clover centers (radians, 70 degrees).
pub const MIN_CENTER_ANGLE: f32 = 70.0 * std::f32::consts::PI / 180.0;
/// Typical peak height, radii (each mountain takes 0.75-1.25 of it).
pub const CLOVER_HEIGHT: f32 = 0.12;
/// Share of a mountain's height given to its rocky noise.
const ROCK_NOISE: f32 = 0.12;
/// Highest ground any seed can raise, radii (`max_height` of the tallest
/// mountain, 1.25 times `CLOVER_HEIGHT` with its noise).
pub const CLOVER_PEAK_BOUND: f32 = CLOVER_HEIGHT * 1.25 * (1.0 + ROCK_NOISE);

/// Height field of the clover planet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloverField {
    /// Unit directions of the four mountains.
    pub centers: [Vector3; 4],
    /// Turn of each clover in its tangent plane, radians.
    pub turns: [f32; 4],
    /// Peak height of each mountain, radii.
    pub heights: [f32; 4],
    /// Radius of a lobe in the tangent plane (gnomonic units).
    pub lobe_r: f32,
    /// Length of the stem below the lobes, same units.
    pub stem: f32,
    /// Water level, radii.
    pub sea: f32,
    pub seed: u32,
}

/// A value in `[0, 1)` from the seed and two indices.
fn unit_hash(seed: u32, i: i32, j: i32) -> f32 {
    (hash3(i, j, 0x51ce, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// `v` turned by `angle` about the unit `axis` (Rodrigues).
fn rotate(v: Vector3, axis: Vector3, angle: f32) -> Vector3 {
    let (s, c) = angle.sin_cos();
    v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c))
}

/// Position of `n` in the plane tangent to the unit `center` (gnomonic
/// projection, `None` at 90 degrees or more so the plane never reaches the
/// far side of the planet).
pub fn tangent_uv(n: Vector3, center: Vector3) -> Option<P2> {
    let d = n.dot(center);
    if d.is_nan() || d <= 1e-3 {
        return None;
    }
    let t1 = tangent(center);
    let t2 = center.cross(t1);
    Some((n.dot(t1) / d, n.dot(t2) / d))
}

/// Signed distance to a clover of lobe radius `lobe_r` (negative inside):
/// three circles at 120 degrees around the origin and a widening stem
/// below, joined with a smooth minimum. A hub inside the lobes keeps the
/// middle high, so no lake is trapped where the lobes meet.
pub fn clover_sdf(p: P2, lobe_r: f32, stem: f32) -> f32 {
    let k = lobe_r * 0.35;
    let mut d = circle(p, (0.0, 0.0), lobe_r * 0.6);
    for a in [90.0f32, 210.0, 330.0] {
        let a = a.to_radians();
        let c = (a.cos() * lobe_r * 0.95, a.sin() * lobe_r * 0.95);
        d = smooth_min(d, circle(p, c, lobe_r), k);
    }
    let bottom = -(lobe_r + stem);
    let tail = triangle(
        p,
        (0.0, 0.0),
        (-lobe_r * 0.28, bottom),
        (lobe_r * 0.28, bottom),
    );
    smooth_min(d, tail, k * 0.5)
}

impl CloverField {
    /// The clover planet of `seed`: a tetrahedron (109.47 degrees between
    /// centers, so never under `MIN_CENTER_ANGLE`) turned by three hashed
    /// angles, with hashed turns and heights.
    pub fn new(seed: u32) -> Self {
        let mut centers = [
            Vector3::new(1.0, 1.0, 1.0),
            Vector3::new(1.0, -1.0, -1.0),
            Vector3::new(-1.0, 1.0, -1.0),
            Vector3::new(-1.0, -1.0, 1.0),
        ];
        let axes = [
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];
        for c in &mut centers {
            *c = c.normalized();
            for (k, axis) in axes.iter().enumerate() {
                *c = rotate(*c, *axis, TAU * unit_hash(seed, 0, k as i32));
            }
            *c = c.normalized();
        }
        let turns = std::array::from_fn(|i| TAU * unit_hash(seed, 1, i as i32));
        let heights =
            std::array::from_fn(|i| CLOVER_HEIGHT * (0.75 + 0.5 * unit_hash(seed, 2, i as i32)));
        Self {
            centers,
            turns,
            heights,
            lobe_r: 0.2,
            stem: 0.16,
            sea: 0.02,
            seed,
        }
    }

    /// Checks a field built by hand: finite values, unit centers at least
    /// `MIN_CENTER_ANGLE` apart (closer mountains would merge).
    pub fn validate(&self) -> Result<(), String> {
        let finite = self
            .centers
            .iter()
            .all(|c| c.x.is_finite() && c.y.is_finite() && c.z.is_finite())
            && self
                .turns
                .iter()
                .chain(&self.heights)
                .all(|v| v.is_finite())
            && [self.lobe_r, self.stem, self.sea]
                .iter()
                .all(|v| v.is_finite());
        if !finite {
            return Err("clover field values must be finite".into());
        }
        for i in 0..4 {
            for j in i + 1..4 {
                let cos = self.centers[i]
                    .normalized()
                    .dot(self.centers[j].normalized());
                if cos.clamp(-1.0, 1.0).acos() < MIN_CENTER_ANGLE {
                    return Err(format!(
                        "clover centers {i} and {j} are closer than 70 degrees"
                    ));
                }
            }
        }
        Ok(())
    }

    /// The low seabed, everywhere below `sea` (from -0.032 to 0.008 R).
    fn seabed(&self, n: Vector3) -> f32 {
        -0.012 + 0.02 * fbm(n * 3.0, 4, self.seed.wrapping_add(7))
    }

    /// How deep `n` lies inside clover `i`, 0 outside to 1 at a lobe's
    /// core (and the plane coordinates).
    pub fn inside(&self, i: usize, n: Vector3) -> f32 {
        let Some((u, v)) = tangent_uv(n, self.centers[i]) else {
            return 0.0;
        };
        let (s, c) = self.turns[i].sin_cos();
        let p = (u * c - v * s, u * s + v * c);
        smoothstep(
            0.0,
            self.lobe_r * 0.9,
            -clover_sdf(p, self.lobe_r, self.stem),
        )
    }

    /// Height above the radius along the unit `n`.
    pub fn height(&self, n: Vector3) -> f32 {
        let mut h = self.seabed(n);
        for i in 0..4 {
            let inside = self.inside(i, n);
            if inside > 0.0 {
                let rock = 1.0 + ROCK_NOISE * fbm(n * 9.0, 3, self.seed.wrapping_add(i as u32));
                h = h.max(self.heights[i] * inside * rock);
            }
        }
        h
    }

    /// Whether the ground at `n` stands above the sea.
    pub fn is_land(&self, n: Vector3) -> bool {
        self.height(n) > self.sea
    }

    /// Signed angular distance from `n` to the coast, radians: positive on
    /// land, negative at sea. Estimated as the height over the sea divided
    /// by the slope (central differences 1e-3 rad wide), so it is exact
    /// close to the coast, where the material and the foam read it.
    pub fn coast_distance(&self, n: Vector3) -> f32 {
        const EPS: f32 = 1e-3;
        let t1 = tangent(n);
        let t2 = n.cross(t1);
        let at = |d: Vector3| self.height((n + d * EPS).normalized());
        let du = (at(t1) - at(-t1)) / (2.0 * EPS);
        let dv = (at(t2) - at(-t2)) / (2.0 * EPS);
        let slope = du.hypot(dv).max(1e-4);
        let d = (self.height(n) - self.sea) / slope;
        if d.is_finite() { d } else { 0.0 }
    }

    /// Bound of `height`, radii.
    pub fn max_height(&self) -> f32 {
        let peak = self.heights.iter().copied().fold(0.0, f32::max) * (1.0 + ROCK_NOISE);
        peak.max(self.sea).max(0.008)
    }

    /// Order-independent digest of every parameter, for geometry keys.
    pub fn digest(&self) -> u64 {
        let mut bits: Vec<u32> = self
            .centers
            .iter()
            .flat_map(|c| [c.x, c.y, c.z])
            .chain(self.turns)
            .chain(self.heights)
            .chain([self.lobe_r, self.stem, self.sea])
            .map(f32::to_bits)
            .collect();
        bits.push(self.seed);
        bits.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
            (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
}

/// The `params!` pattern of the laboratory 6 materials, replicated here so
/// `celestial/` stays untouched: a struct of `f32` fields with a `SPECS`
/// table, `Default`, `get` and range-checked `set`.
macro_rules! planet_params {
    ($ty:ident { $($field:ident : $default:expr, $min:expr, $max:expr, $help:expr;)* }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $ty {
            $(pub $field: f32,)*
        }

        impl Default for $ty {
            fn default() -> Self {
                Self { $($field: $default,)* }
            }
        }

        impl $ty {
            pub const SPECS: &'static [ParamSpec] = &[
                $(ParamSpec {
                    name: stringify!($field),
                    default: $default,
                    min: $min,
                    max: $max,
                    help: $help,
                },)*
            ];

            pub fn get(&self, name: &str) -> Option<f32> {
                match name {
                    $(stringify!($field) => Some(self.$field),)*
                    _ => None,
                }
            }

            pub fn set(&mut self, name: &str, value: f32) -> Result<(), String> {
                let spec = find_spec(Self::SPECS, name)?;
                check_range(spec, value)?;
                match name {
                    $(stringify!($field) => self.$field = value,)*
                    _ => unreachable!("spec table and fields match"),
                }
                Ok(())
            }
        }
    };
}

planet_params! {
    CloverParams {
        seed: 7.0, 0.0, 9999.0, "seed of the clover centers, turns and heights (rounded)";
        bump: 1.0, 0.0, 1.0, "share of the relief normal in the lighting";
        slope_rock: 0.22, 0.0, 1.0, "slope (1 - cos to the vertical) where grass gives way to rock";
        snow_line: 0.86, 0.0, 1.0, "share of the highest peak above which snow lies";
        water_gloss: 90.0, 4.0, 400.0, "specular exponent of the sea";
        wave_speed: 1.0, 0.0, 4.0, "speed of the two wave trains (0 = still water)";
        wave_strength: 0.06, 0.0, 0.3, "tilt of the water normal by the waves";
        foam: 0.8, 0.0, 1.0, "opacity of the foam lines along the coast";
        waterfalls: 1.0, 0.0, 1.0, "waterfalls per mountain (0 = none, rounded)";
    }
}

/// Common period of the waves and the foam, seconds: every phase repeats
/// after it, so time is wrapped into it (no precision loss at large `t`).
pub const WAVE_PERIOD: f32 = 60.0;
/// The two wave trains: direction (object space), spatial frequency on
/// the unit sphere, whole cycles per period and amplitude share.
const WAVES: [(Vector3, f32, f32, f32); 2] = [
    (Vector3::new(0.95, 0.29, 0.1), 70.0, 9.0, 1.0),
    (Vector3::new(-0.19, 0.37, 0.91), 115.0, 14.0, 0.6),
];
/// Width of the foam band off the coast, radians.
pub const FOAM_BAND: f32 = 0.02;
const FOAM_COLOR: Vector3 = hex(0xEAF6FF);

/// `t` scaled by `speed` and wrapped into the wave period (0 if not finite).
fn wave_time(t: f32, speed: f32) -> f32 {
    let t = (t * speed).rem_euclid(WAVE_PERIOD);
    if t.is_finite() { t } else { 0.0 }
}

/// Water normal at the unit `n` and time `t`: two wave trains of different
/// direction and frequency tilt it along their tangential directions.
pub fn wave_normal(n: Vector3, t: f32, p: &CloverParams) -> Vector3 {
    let t = wave_time(t, p.wave_speed);
    let mut tilt = Vector3::zero();
    for (dir, k, cycles, share) in WAVES {
        let along = dir - n * n.dot(dir);
        let phase = k * n.dot(dir) - TAU * cycles * t / WAVE_PERIOD;
        tilt += along * (phase.cos() * p.wave_strength * share);
    }
    let w = (n - tilt).normalized();
    if w.x.is_finite() && w.y.is_finite() && w.z.is_finite() {
        w
    } else {
        n
    }
}

/// Foam at `coast_d` radians from the coast (negative at sea) and time
/// `t`: lines rolling towards the shore inside `FOAM_BAND`, fading out to
/// sea. 0 on land and beyond the band.
pub fn foam(coast_d: f32, t: f32) -> f32 {
    if !(coast_d < 0.0 && coast_d > -FOAM_BAND) {
        return 0.0;
    }
    let x = -coast_d / FOAM_BAND;
    let t = wave_time(t, 1.0);
    let lines = 0.5 + 0.5 * (x * 18.0 + TAU * 6.0 * t / WAVE_PERIOD).sin();
    (1.0 - x) * (0.35 + 0.65 * lines)
}

impl CloverParams {
    /// The height field these parameters describe.
    pub fn field(&self) -> CloverField {
        CloverField::new(self.seed.round().max(0.0) as u32)
    }
}

const GRASS: Vector3 = hex(0x4CAF50);
const MOSS: Vector3 = hex(0x2F6B3A);
const ROCK: Vector3 = hex(0x7D7266);
const SNOW: Vector3 = hex(0xF4F6FA);
const SAND: Vector3 = hex(0xD9C98E);
const SHALLOW: Vector3 = hex(0x1E9FB0);
const DEEP: Vector3 = hex(0x0A2F66);
const SKY: Vector3 = hex(0x9FD4FF);
/// Light that reaches the night side (a dim sky glow).
const AMBIENT: f32 = 0.035;

/// Clover material with its per-frame values: the field, the parameters
/// and the light and time of this frame, in the planet's object space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedClover {
    pub field: CloverField,
    pub params: CloverParams,
    /// Unit direction towards the star, object space.
    pub light: Vector3,
    pub time: f32,
}

impl PreparedClover {
    /// Ground color at the unit `n` with shading normal `normal` and
    /// height `h`: grass low, moss higher, rock on steep slopes and high
    /// up, snow on the gentle tops, sand on the shore.
    pub fn land_albedo(&self, n: Vector3, normal: Vector3, h: f32) -> Vector3 {
        let f = &self.field;
        let top = (f.max_height() - f.sea).max(1e-3);
        let t = ((h - f.sea) / top).clamp(0.0, 1.0);
        let slope = 1.0 - normal.dot(n);
        let p = &self.params;
        let mut c = mix3(GRASS, MOSS, smoothstep(0.25, 0.65, t));
        c = mix3(c, ROCK, smoothstep(0.7, 0.92, t));
        c = mix3(c, ROCK, smoothstep(p.slope_rock * 0.6, p.slope_rock, slope));
        let snow = smoothstep(p.snow_line, p.snow_line + 0.08, t)
            * (1.0 - smoothstep(p.slope_rock, p.slope_rock * 1.6, slope));
        c = mix3(c, SNOW, snow);
        mix3(SAND, c, smoothstep(0.0, 0.004, h - f.sea))
    }

    /// Water color at the unit `n`, `depth` radii under the sea, with
    /// water normal `normal` and unit `eye` direction (object space).
    pub fn water(&self, normal: Vector3, depth: f32, eye: Vector3) -> Vector3 {
        let base = mix3(SHALLOW, DEEP, smoothstep(0.0, 0.03, depth));
        let diffuse = lambert(normal, self.light);
        let rim = fresnel(normal, eye, 5.0) * diffuse.max(0.1);
        let glint = blinn_phong(normal, self.light, eye, self.params.water_gloss);
        base * (AMBIENT + (1.0 - AMBIENT) * diffuse) + SKY * (0.35 * rim) + Vector3::one() * glint
    }

    /// Color of the surface point `object_position` seen from the unit
    /// `eye` direction (object space).
    pub fn shade(&self, object_position: Vector3, eye: Vector3) -> Vector3 {
        let len = object_position.length();
        let n = if len.is_finite() && len > 0.0 {
            object_position / len
        } else {
            Vector3::new(0.0, 1.0, 0.0)
        };
        let h = self.field.height(n);
        if h <= self.field.sea {
            let depth = self.field.sea - h;
            let water = self.water(wave_normal(n, self.time, &self.params), depth, eye);
            // Only the shallow ring pays for the coast distance.
            if depth > 0.01 || self.params.foam <= 0.0 {
                return water;
            }
            let f = foam(self.field.coast_distance(n), self.time) * self.params.foam;
            let lit = FOAM_COLOR * (AMBIENT + (1.0 - AMBIENT) * lambert(n, self.light));
            return mix3(water, lit, f);
        }
        let bumped = HeightField::Clover(self.field).normal(n, NORMAL_EPS);
        let normal = (n + (bumped - n) * self.params.bump.clamp(0.0, 1.0)).normalized();
        let albedo = self.land_albedo(n, normal, h);
        albedo * (AMBIENT + (1.0 - AMBIENT) * lambert(normal, self.light))
    }
}
