//! House of Cards: violet gas giant with diamond storms (gas giant).
//!
//! Latitude bands warped by fBm slide past each other under a latitude-
//! dependent wind (a rotation of the sampling direction, never an angle
//! with a seam), mapped through violet, magenta and pearl ramps with an
//! atmospheric rim. A migrating storm advects the band sampling domain along
//! the contours of an L1 diamond metric in its tangent plane, so the bands
//! wind into rhombic spirals. Its only symbol is the diamond.

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::pipeline::{ViewPreset, direction_from_angles};
use crate::shading::cycle::{cycle, envelope};
use crate::shading::lighting::{fresnel, wrap_lambert};
use crate::shading::math::{hex, mix3, ramp, saturate, smoothstep};
use crate::shading::noise::fbm;
use crate::shading::sphere::{east_north, from_local, hash_direction, local_plane, rotate_y};
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
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
        seed,
        storms: [
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
        ],
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
        lit + mix3(hex(0xC13CFF), hex(0xF0408A), rim) * (p.rim_strength * rim)
    }
}
