//! The King's Crown: royal star of living plasma (laboratory 6, star).
//!
//! Temperature field: a time-advected, domain-warped fBm of the unit
//! direction, plus ridged golden veins and fine granulation, mapped through
//! an ivory, gold, orange, crimson and wine ramp. The star is emissive: it
//! ignores the external light. Its only symbol is the crown.

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::shading::math::{hex, ramp, saturate, smoothstep};
use crate::shading::noise::{fbm, warp};
use crate::shading::sphere::rotate_y;
use raylib::math::Vector3;

params! {
    CrownParams {
        seed: 7.0, 0.0, 9999.0, "noise seed";
        brightness: 1.0, 0.2, 2.0, "overall emission";
        plasma_scale: 1.6, 0.5, 8.0, "size of the plasma currents (higher = smaller cells)";
        plasma_octaves: 5.0, 1.0, 7.0, "fBm octaves of the temperature (rounded)";
        warp_strength: 0.8, 0.0, 3.0, "domain warp: how much the currents twist";
        flow_speed: 0.18, 0.0, 2.0, "vertical advection of the plasma, units per second";
        swirl_speed: 0.25, 0.0, 2.0, "rotation of the warp domain, radians per second";
        granulation_scale: 22.0, 4.0, 60.0, "frequency of the fine granulation";
        granulation_strength: 0.12, 0.0, 0.5, "amplitude of the granulation";
        vein_width: 0.08, 0.01, 0.3, "width of the golden veins";
    }
}

/// Spreads the normalised fBm (mostly within +-0.4) over the color ramp.
const PLASMA_CONTRAST: f32 = 1.6;

/// Wine, crimson, orange, gold and incandescent ivory.
const PLASMA_RAMP: [(f32, Vector3); 5] = [
    (0.0, hex(0x4A0B10)),
    (0.28, hex(0xD21F2B)),
    (0.55, hex(0xFF7A00)),
    (0.8, hex(0xFFC63A)),
    (1.0, hex(0xFFF2C4)),
];

/// Per-frame state of the star shader.
#[derive(Debug, Clone)]
pub struct Frame {
    pub params: CrownParams,
    pub uniforms: ShaderUniforms,
    pub debug: DebugView,
    seed: u32,
    octaves: u32,
    swirl: f32,
    flow: Vector3,
}

pub fn prepare(params: &CrownParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    let t = uniforms.time;
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
        seed: params.seed as u32,
        octaves: params.plasma_octaves.round() as u32,
        swirl: params.swirl_speed * t,
        flow: Vector3::new(0.0, params.flow_speed * t, 0.0),
    }
}

impl Frame {
    /// Plasma temperature in `[0, 1]` at unit direction `n`.
    pub fn temperature(&self, n: Vector3) -> f32 {
        let p = &self.params;
        let t = self.uniforms.time;
        let q = rotate_y(n, self.swirl) * p.plasma_scale;
        let w = warp(q + self.flow, p.warp_strength, self.seed);
        // Normalised fBm stays near 0; a fixed contrast spreads it over the ramp.
        let base = saturate(0.5 + 0.5 * PLASMA_CONTRAST * fbm(w, self.octaves, self.seed));
        let veins = 1.0
            - smoothstep(
                0.0,
                p.vein_width,
                fbm(w * 1.7, 3, self.seed.wrapping_add(1)).abs(),
            );
        let gran = p.granulation_strength
            * fbm(
                n * p.granulation_scale + Vector3::new(0.4 * t, 0.0, 0.0),
                2,
                self.seed.wrapping_add(2),
            );
        saturate(0.85 * base + 0.35 * veins + gran)
    }

    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        let temperature = self.temperature(s.normal);
        if self.debug == DebugView::Height {
            return Vector3::one() * temperature;
        }
        ramp(&PLASMA_RAMP, temperature) * self.params.brightness
    }
}
