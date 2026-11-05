//! The King's Crown: royal star of living plasma (laboratory 6, star).
//!
//! Temperature field: a time-advected, domain-warped fBm of the unit
//! direction, plus ridged golden veins and fine granulation, mapped through
//! an ivory, gold, orange, crimson and wine ramp. Three drifting sunspots
//! breathe, a limb corona pulses, a five-point golden diadem circles the
//! northern cap and a periodic eclipse darkens the core. The star is
//! emissive: it ignores the external light. Its only symbol is the crown.

use crate::celestial::DebugView;
use crate::pipeline::ViewPreset;
use crate::pipeline::direction_from_angles;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::shading::cycle::{cycle, envelope};
use crate::shading::math::{hex, mix3, ramp, saturate, smoothstep};
use crate::shading::noise::{fbm, warp};
use crate::shading::sphere::{angular_distance, hash_direction, rotate_y};
use raylib::math::Vector3;
use std::f32::consts::TAU;

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
        spot_radius: 0.32, 0.05, 0.8, "angular radius of the sunspots, radians";
        spot_breath_period: 9.0, 1.0, 60.0, "seconds for a sunspot to grow and shrink";
        spot_drift_speed: 0.05, 0.0, 1.0, "drift of the sunspots around the axis, radians per second";
        corona_width: 0.45, 0.02, 0.6, "width of the limb corona";
        corona_pulse_period: 4.0, 0.5, 30.0, "seconds per corona pulse";
        corona_pulse_amount: 0.35, 0.0, 1.0, "how much the corona width breathes";
        crown_points: 5.0, 3.0, 9.0, "points of the golden diadem (rounded)";
        crown_base_latitude: 50.0, 20.0, 75.0, "latitude of the diadem band, degrees";
        crown_height: 0.35, 0.0, 0.8, "height of the diadem points, radians of latitude";
        crown_strength: 0.8, 0.0, 1.0, "opacity of the diadem";
        eclipse_period: 24.0, 4.0, 120.0, "seconds between eclipses";
        eclipse_duration: 4.0, 0.0, 30.0, "seconds an eclipse lasts";
        eclipse_depth: 0.75, 0.0, 1.0, "how much the eclipse darkens the core";
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
    /// Current sunspot centers (unit) and angular radii.
    pub spot_centers: [Vector3; 3],
    pub spot_radii: [f32; 3],
    /// Corona and diadem pulse in `[0, 1]`.
    pub pulse: f32,
    /// Eclipse strength in `[0, 1]`.
    pub eclipse: f32,
    points: f32,
    base_latitude: f32,
}

pub fn prepare(params: &CrownParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    let t = uniforms.time;
    let seed = params.seed as u32;
    let (az, el) = ViewPreset::Oblique.angles();
    let focus = direction_from_angles(az, el);
    let spot_centers = std::array::from_fn(|i| {
        let home = hash_direction(seed, i as u32, 0, focus, 50.0_f32.to_radians());
        rotate_y(home, params.spot_drift_speed * t)
    });
    let spot_radii = std::array::from_fn(|i| {
        let phase = TAU * t / params.spot_breath_period + 2.1 * i as f32;
        params.spot_radius * (0.7 + 0.3 * phase.sin())
    });
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
        seed,
        octaves: params.plasma_octaves.round() as u32,
        swirl: params.swirl_speed * t,
        flow: Vector3::new(0.0, params.flow_speed * t, 0.0),
        spot_centers,
        spot_radii,
        pulse: 0.5 + 0.5 * (TAU * t / params.corona_pulse_period).sin(),
        eclipse: eclipse_strength(params, t),
        points: params.crown_points.round(),
        base_latitude: params.crown_base_latitude.to_radians(),
    }
}

/// The eclipse occupies the last `duration / period` of each cycle, so its
/// center falls at `t = period - duration / 2` (22 s by default).
pub fn eclipse_strength(params: &CrownParams, t: f32) -> f32 {
    let active = (params.eclipse_duration / params.eclipse_period).clamp(0.0, 1.0);
    if active <= 0.0 {
        return 0.0;
    }
    let (_, tau) = cycle(t, params.eclipse_period, 0.0);
    let start = 1.0 - active;
    if tau < start {
        return 0.0;
    }
    envelope((tau - start) / active, 0.25, 0.5, 0.25)
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

    /// Umbra and penumbra masks of the nearest sunspot.
    pub fn spots(&self, n: Vector3) -> (f32, f32) {
        let wobble = 0.04 * fbm(n * 6.0, 2, self.seed.wrapping_add(3));
        let (mut umbra, mut penumbra) = (0.0_f32, 0.0_f32);
        for (c, &r) in self.spot_centers.iter().zip(&self.spot_radii) {
            let d = angular_distance(n, *c) + wobble;
            umbra = umbra.max(1.0 - smoothstep(0.45 * r, 0.7 * r, d));
            penumbra = penumbra.max(1.0 - smoothstep(0.7 * r, r, d));
        }
        (umbra, penumbra)
    }

    /// Limb corona band in `[0, 1]`; its width breathes with the pulse.
    pub fn corona(&self, n: Vector3) -> f32 {
        let p = &self.params;
        let rim = 1.0 - saturate(n.dot(self.uniforms.eye_direction));
        let width = p.corona_width * (1.0 + p.corona_pulse_amount * (2.0 * self.pulse - 1.0));
        let flicker = 0.6
            + 0.4
                * fbm(
                    n * 4.0 + Vector3::new(0.0, 0.3 * self.uniforms.time, 0.0),
                    2,
                    self.seed.wrapping_add(4),
                );
        smoothstep(1.0 - width, 1.0, rim) * flicker
    }

    /// Golden diadem mask: a band at the base latitude with `crown_points`
    /// triangular points towards the north pole. The triangle wave has an
    /// integer number of periods, so it is continuous across +-pi.
    pub fn diadem(&self, n: Vector3) -> f32 {
        let p = &self.params;
        let lat = n.y.clamp(-1.0, 1.0).asin();
        let lon = n.z.atan2(n.x);
        let x = self.points * lon / TAU;
        let tri = 1.0 - 2.0 * (x - x.floor() - 0.5).abs();
        let base = self.base_latitude;
        let top = base + p.crown_height * (0.6 + 0.4 * self.pulse) * tri;
        let edge = 0.02;
        let ring = 1.0 - smoothstep(0.03, 0.03 + edge, (lat - base).abs());
        let tooth = smoothstep(base - edge, base, lat) * (1.0 - smoothstep(top - edge, top, lat));
        ring.max(tooth)
    }

    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        let n = s.normal;
        let p = &self.params;
        let mut temperature = self.temperature(n);
        let (umbra, penumbra) = self.spots(n);
        temperature *= 1.0 - 0.45 * penumbra;
        temperature += (0.05 - temperature) * umbra;
        let corona = self.corona(n);
        let diadem = self.diadem(n) * p.crown_strength;
        match self.debug {
            DebugView::Height => return Vector3::one() * temperature,
            DebugView::Materials => {
                let mut c = hex(0xD21F2B) * 0.5;
                c = mix3(c, hex(0x151515), penumbra.max(umbra));
                c = mix3(c, hex(0xFFF2C4), corona);
                return mix3(c, hex(0xFFC63A), diadem);
            }
            _ => {}
        }
        let mut color = ramp(&PLASMA_RAMP, temperature);
        let gold = mix3(hex(0xFFC63A), hex(0xFFF2C4), 0.15 + 0.25 * self.pulse);
        color = mix3(color, gold, diadem);
        let rim = 1.0 - saturate(n.dot(self.uniforms.eye_direction));
        color *= 1.0 - p.eclipse_depth * self.eclipse * (1.0 - rim).powi(2);
        color += mix3(hex(0xFFC63A), hex(0xFFF2C4), corona) * (0.9 * corona);
        color * p.brightness
    }
}
