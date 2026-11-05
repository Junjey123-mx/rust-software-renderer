//! House of Cards: violet gas giant with diamond storms (gas giant).
//!
//! Latitude bands warped by fBm slide past each other under a latitude-
//! dependent wind (a rotation of the sampling direction, never an angle
//! with a seam), mapped through violet, magenta and pearl ramps with an
//! atmospheric rim. Its only symbol is the diamond.

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::shading::lighting::{fresnel, wrap_lambert};
use crate::shading::math::{hex, mix3, ramp, saturate};
use crate::shading::noise::fbm;
use crate::shading::sphere::rotate_y;
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
}

pub fn prepare(params: &HouseParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
        seed: params.seed as u32,
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

    /// Cloud color of a sampling direction, before lighting.
    pub fn albedo(&self, n: Vector3) -> Vector3 {
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

    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        let n = s.normal;
        if self.debug == DebugView::Height {
            return Vector3::one() * self.band_value(n);
        }
        let p = &self.params;
        let u = &self.uniforms;
        let lit = self.albedo(n) * (0.18 + 0.82 * wrap_lambert(n, u.light_direction, p.wrap));
        let rim = fresnel(n, u.eye_direction, 3.0);
        lit + mix3(hex(0xC13CFF), hex(0xF0408A), rim) * (p.rim_strength * rim)
    }
}
