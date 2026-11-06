//! The King's Crown: royal star of living plasma (laboratory 6, star).
//!
//! Temperature field: a time-advected, domain-warped fBm of the unit
//! direction, plus ridged golden veins and fine granulation, mapped through
//! an ivory, gold, orange, crimson and wine ramp. Three drifting sunspots
//! breathe, a limb corona pulses and a periodic eclipse darkens the core.
//! Plasma loops gather at a foot point, arc across the surface, flare and
//! drain back into the plasma. Everything stays inside the silhouette of the
//! sphere: the loops are luminous paths on the surface, not geometry. The
//! star is emissive: it ignores the external light. No figure is drawn on
//! its surface; its royalty is the gold of the living plasma.

use crate::celestial::DebugView;
use crate::pipeline::ViewPreset;
use crate::pipeline::direction_from_angles;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::shading::cycle::{cycle, envelope};
use crate::shading::math::{hex, mix3, ramp, saturate, smoothstep};
use crate::shading::noise::{fbm, gradient3, hash3, warp};
use crate::shading::sphere::{angular_distance, east_north, hash_direction, local_plane, rotate_y};
use raylib::math::Vector3;
use std::f32::consts::PI;
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
        eclipse_period: 24.0, 4.0, 120.0, "seconds between eclipses";
        eclipse_duration: 4.0, 0.0, 30.0, "seconds an eclipse lasts";
        eclipse_depth: 0.75, 0.0, 1.0, "how much the eclipse darkens the core";
        loops: 1.0, 0.0, 1.0, "switch: plasma loops that erupt and drain back (0 or 1)";
        loop_period: 9.0, 3.0, 60.0, "seconds per plasma loop life cycle";
        loop_size: 0.6, 0.15, 0.9, "half-span of a plasma loop in its tangent plane";
        loop_width: 0.05, 0.01, 0.12, "thickness of the plasma filament";
        loop_brightness: 1.1, 0.0, 3.0, "emission of the plasma loops";
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

/// Plasma loops alive at once, each on its own cycle.
pub const LOOP_SLOTS: usize = 3;
/// Loop feet are seeded within this angle of the default view direction.
pub const LOOP_CONE_DEG: f32 = 60.0;

/// One plasma loop in its current cycle. In the tangent plane at `center`
/// the loop is the upper half of an ellipse with half-span `a` and height
/// `b`, from foot A (`s = 0`) to foot B (`s = 1`), rotated by `angle`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlasmaLoop {
    pub center: Vector3,
    pub east: Vector3,
    pub north: Vector3,
    pub angle: f32,
    pub a: f32,
    pub b: f32,
    pub tau: f32,
    pub cycle: i64,
    /// Revealed part of the arc: `[tail, head]` in `[0, 1]`.
    pub head: f32,
    pub tail: f32,
    /// Filament brightness envelope in `[0, 1]` (0 at both cycle ends).
    pub intensity: f32,
    /// Near-white heat of the maximum phase in `[0, 1]`.
    pub heat: f32,
    /// Concentration glow at foot A before and during the birth.
    pub foot: f32,
    /// Thickness multiplier: thins while the loop drains back.
    pub thickness: f32,
    /// Cosine of the angular radius beyond which the loop cannot appear.
    reach_cos: f32,
    seed: u32,
}

/// Life of a plasma loop over its phase `tau`: concentration (0-0.12),
/// birth and expansion (0.12-0.55, the arc grows from foot A to foot B),
/// maximum (0.55-0.7), return (0.7-0.88, it thins, dims and drains into
/// foot B) and stabilization (0.88-1, nothing). Every term is 0 at both
/// ends of the cycle, so relocating never pops.
pub fn plasma_loop(params: &CrownParams, seed: u32, slot: usize, t: f32) -> PlasmaLoop {
    let period = params.loop_period * (1.0 + 0.37 * slot as f32);
    let (k, tau) = cycle(t, period, 0.45 * slot as f32 * period);
    let (az, el) = ViewPreset::Oblique.angles();
    let focus = direction_from_angles(az, el);
    let center = hash_direction(seed, 30 + slot as u32, k, focus, LOOP_CONE_DEG.to_radians());
    let (east, north) = east_north(center);
    let lo = k as i32;
    let hashed = |salt: u32| {
        hash3(slot as i32, lo, salt as i32, seed ^ 0x51ed_270b) as f32 / u32::MAX as f32
    };
    let a = params.loop_size * (0.75 + 0.5 * hashed(1));
    let b = a * (0.55 + 0.35 * hashed(2));
    let on = if params.loops >= 0.5 { 1.0 } else { 0.0 };
    let intensity = on * smoothstep(0.12, 0.3, tau) * (1.0 - smoothstep(0.7, 0.88, tau));
    let reach = (1.6 * a.max(b)).atan() + 0.1;
    PlasmaLoop {
        center,
        east,
        north,
        angle: PI * hashed(3),
        a,
        b,
        tau,
        cycle: k,
        head: smoothstep(0.12, 0.55, tau),
        tail: smoothstep(0.68, 0.9, tau),
        intensity,
        heat: on * smoothstep(0.45, 0.6, tau) * (1.0 - smoothstep(0.66, 0.8, tau)),
        foot: on * smoothstep(0.0, 0.12, tau) * (1.0 - smoothstep(0.3, 0.55, tau)),
        thickness: 1.0 - 0.5 * smoothstep(0.7, 0.88, tau),
        reach_cos: reach.min(1.5).cos(),
        seed: hash3(slot as i32, lo, 7, seed),
    }
}

/// Light of the plasma loops at one direction.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LoopGlow {
    /// Bright filament core in `[0, 1]`.
    pub core: f32,
    /// Soft halo around the filament.
    pub halo: f32,
    /// Near-white heat of the core at the maximum phase.
    pub heat: f32,
    /// Concentration glow at a foot point.
    pub foot: f32,
}

impl PlasmaLoop {
    /// Plane point of the (undistorted) arc at progress `s` in `[0, 1]`.
    pub fn arc_point(&self, s: f32) -> (f32, f32) {
        let theta = PI * (1.0 - s);
        let (x, y) = (self.a * theta.cos(), self.b * theta.sin());
        let (sa, ca) = self.angle.sin_cos();
        (x * ca - y * sa, x * sa + y * ca)
    }

    /// Glow of this loop at unit direction `n`.
    pub fn glow(&self, n: Vector3, time: f32, width: f32) -> LoopGlow {
        if (self.intensity <= 0.0 && self.foot <= 0.0) || n.dot(self.center) < self.reach_cos {
            return LoopGlow::default();
        }
        let Some((u, v)) = local_plane(n, self.center, self.east, self.north) else {
            return LoopGlow::default();
        };
        // Into the loop frame, with an organic wobble of the filament.
        let (sa, ca) = self.angle.sin_cos();
        let wobble = Vector3::new(0.0, 0.3 * time, 0.0);
        let x = u * ca + v * sa + 0.08 * self.a * fbm(n * 7.0 + wobble, 2, self.seed);
        let y = -u * sa + v * ca + 0.08 * self.a * fbm(n * 7.0 - wobble, 2, self.seed ^ 0x9e37);
        let foot_distance = ((x + self.a).powi(2) + y * y).sqrt() / (0.12 * self.a + 0.04);
        let foot = self.foot * (-foot_distance * foot_distance).exp();
        let (ex, ey) = (x / self.a, y / self.b);
        if ey < -0.25 {
            return LoopGlow {
                foot,
                ..LoopGlow::default()
            };
        }
        let theta = ey.atan2(ex).clamp(0.0, PI);
        let s = 1.0 - theta / PI;
        let rho = (ex * ex + ey * ey).sqrt();
        let radius = ((self.a * theta.cos()).powi(2) + (self.b * theta.sin()).powi(2)).sqrt();
        let distance = (rho - 1.0).abs() * radius;
        let w = width
            * self.thickness
            * (0.7 + 0.6 * (0.5 + 0.5 * gradient3(n * 11.0 + wobble, self.seed ^ 0x5bd1)));
        let reveal = smoothstep(self.tail - 0.03, self.tail + 0.03, s)
            * (1.0 - smoothstep(self.head - 0.03, self.head + 0.03, s));
        let knots = 0.75 + 0.25 * (20.0 * s - 7.0 * time).sin();
        let core = (-(distance / w).powi(2)).exp() * reveal * knots * self.intensity;
        let halo = (-(distance / (3.5 * w)).powi(2)).exp() * reveal * self.intensity * 0.4;
        LoopGlow {
            core,
            halo,
            heat: core * self.heat,
            foot,
        }
    }
}

/// Clamps every channel to `[0, max]`.
fn clamp_color(c: Vector3, max: f32) -> Vector3 {
    Vector3::new(
        c.x.clamp(0.0, max),
        c.y.clamp(0.0, max),
        c.z.clamp(0.0, max),
    )
}

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
    /// Corona pulse in `[0, 1]`.
    pub pulse: f32,
    /// Eclipse strength in `[0, 1]`.
    pub eclipse: f32,
    /// Plasma loops of this frame (boxed: built once per frame).
    pub loops: Box<[PlasmaLoop; LOOP_SLOTS]>,
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
        loops: Box::new(std::array::from_fn(|slot| {
            plasma_loop(params, seed, slot, t)
        })),
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

    /// Combined glow of every plasma loop at unit direction `n`.
    pub fn loop_glow(&self, n: Vector3) -> LoopGlow {
        let mut total = LoopGlow::default();
        for l in self.loops.iter() {
            let g = l.glow(n, self.uniforms.time, self.params.loop_width);
            total.core = total.core.max(g.core);
            total.halo = total.halo.max(g.halo);
            total.heat = total.heat.max(g.heat);
            total.foot = total.foot.max(g.foot);
        }
        total
    }

    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        let n = s.normal;
        let p = &self.params;
        let mut temperature = self.temperature(n);
        let (umbra, penumbra) = self.spots(n);
        temperature *= 1.0 - 0.45 * penumbra;
        temperature += (0.05 - temperature) * umbra;
        // The plasma heats up around the loops and their feet.
        let loops = self.loop_glow(n);
        temperature = saturate(temperature + 0.3 * (loops.halo + loops.foot));
        let corona = self.corona(n);
        match self.debug {
            DebugView::Height => return Vector3::one() * temperature,
            DebugView::Materials => {
                let mut c = hex(0xD21F2B) * 0.5;
                c = mix3(c, hex(0x151515), penumbra.max(umbra));
                c = mix3(c, hex(0xFFF2C4), corona);
                return mix3(c, Vector3::one(), saturate(loops.core.max(loops.foot)));
            }
            _ => {}
        }
        let mut color = ramp(&PLASMA_RAMP, temperature);
        // Loop filaments: a dark crimson sheath separates them from the
        // plasma, then a gold to near-white core burns on top.
        let core = saturate(loops.core);
        let sheath = saturate(2.2 * loops.halo - core) * saturate(p.loop_brightness);
        color = mix3(color, hex(0x4A0B10), 0.55 * sheath);
        color += (mix3(hex(0xFFC63A), hex(0xFFF2C4), 0.35 + 0.65 * core) * (1.2 * core)
            + hex(0xFFF2C4) * loops.heat
            + mix3(hex(0xFF7A00), hex(0xFFF2C4), loops.foot) * loops.foot)
            * p.loop_brightness;
        let rim = 1.0 - saturate(n.dot(self.uniforms.eye_direction));
        color *= 1.0 - p.eclipse_depth * self.eclipse * (1.0 - rim).powi(2);
        color += mix3(hex(0xFFC63A), hex(0xFFF2C4), corona) * (0.9 * corona);
        clamp_color(color * p.brightness, 2.0)
    }
}
