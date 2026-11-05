//! Crimson Entrance: volcanic planet of craters and spade mesas (rocky).
//!
//! The surface is classified from one procedural height field (see
//! `terrain`): obsidian plains, basalt, crimson highlands, slate mesas, ash
//! rims and glowing fissures. Its only symbol is the spade.

pub mod clouds;
pub mod events;
pub mod terrain;

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample, ViewPreset, direction_from_angles};
use crate::shading::lighting::{blinn_phong, bump_normal, lambert, parallax_shift, wrap_lambert};
use crate::shading::math::{hex, mix3, mixf, saturate, smoothstep};
use crate::shading::sphere::{angular_distance, hash_direction};
use raylib::math::Vector3;
use terrain::TerrainMaterial;

params! {
    EntranceParams {
        seed: 11.0, 0.0, 9999.0, "noise seed";
        terrain_scale: 1.8, 0.5, 6.0, "frequency of the hills (higher = smaller)";
        terrain_octaves: 5.0, 1.0, 7.0, "fBm octaves of the detail height (rounded)";
        detail_amplitude: 0.18, 0.0, 0.6, "amplitude of the fine relief";
        basin_depth: 0.55, 0.0, 1.0, "depth of the monumental basin";
        basin_radius: 0.62, 0.2, 1.2, "angular radius of the basin, radians";
        crack_width: 0.035, 0.005, 0.15, "width of the fissures";
        crack_scale: 7.0, 2.0, 20.0, "frequency of the fissure network";
        events: 1.0, 0.0, 1.0, "switch: migrating craters and spade mesas (0 or 1)";
        crater_period: 12.0, 2.0, 60.0, "seconds per crater life cycle";
        crater_radius: 0.28, 0.05, 0.6, "angular radius of a mature crater, radians";
        crater_depth: 0.45, 0.0, 1.0, "depth of a crater bowl";
        spade_period: 16.0, 2.0, 60.0, "seconds per spade mesa life cycle";
        spade_size: 0.42, 0.1, 0.8, "size of the spade mesa in its tangent plane";
        spade_height: 0.6, 0.0, 1.0, "height of the spade mesa at its apogee";
        clouds: 1.0, 0.0, 1.0, "switch: procedural cloud layer (0 or 1)";
        cloud_coverage: 0.45, 0.0, 1.0, "fraction of the sky covered by clouds";
        cloud_scale: 2.4, 0.5, 8.0, "frequency of the cloud masses";
        cloud_speed: 0.07, 0.0, 1.0, "rotation of the cloud layer, radians per second";
        cloud_evolution: 0.05, 0.0, 0.5, "slow change of the cloud shapes per second";
        cloud_softness: 0.12, 0.01, 0.4, "softness of the cloud edges";
        cloud_opacity: 0.85, 0.0, 1.0, "opacity of the densest clouds";
        cloud_shadows: 1.0, 0.0, 1.0, "switch: shadows cast by the clouds (0 or 1)";
        shadow_strength: 0.65, 0.0, 1.0, "how much a dense cloud blocks direct light";
        cloud_altitude: 0.07, 0.0, 0.3, "height of the cloud shell; offsets the shadows";
        bump: 1.0, 0.0, 1.0, "switch: bump-mapped relief (0 or 1)";
        bump_strength: 0.8, 0.0, 4.0, "how strongly the height gradient tilts the normal";
        bump_epsilon: 0.004, 0.0005, 0.05, "finite-difference step of the gradient, radians";
        specular: 1.0, 0.0, 1.0, "switch: glassy highlights on the obsidian plains (0 or 1)";
        specular_strength: 1.4, 0.0, 3.0, "brightness of the obsidian highlights";
        shininess: 24.0, 2.0, 256.0, "Blinn-Phong exponent: higher = tighter highlight";
        night_lights: 1.0, 0.0, 1.0, "switch: fissures glow brighter on the night side (0 or 1)";
        night_glow: 1.0, 0.0, 3.0, "emission of fissures and magma at full night";
        terminator_width: 0.25, 0.02, 0.8, "softness of the day-night transition";
        parallax: 1.0, 0.0, 1.0, "switch: view-dependent parallax of the relief (0 or 1)";
        parallax_scale: 0.06, 0.0, 0.25, "apparent depth of the macro relief";
        parallax_steps: 8.0, 2.0, 32.0, "linear search steps of the parallax (rounded)";
    }
}

/// Emission of fissures and magma on the day side.
pub const DAY_GLOW: f32 = 0.12;

/// Per-frame state of the rocky planet shader.
#[derive(Debug, Clone)]
pub struct Frame {
    pub params: EntranceParams,
    pub uniforms: ShaderUniforms,
    pub debug: DebugView,
    pub seed: u32,
    /// Direction the default camera looks at; events are seeded around it.
    pub focus: Vector3,
    pub basin_center: Vector3,
    /// Craters and spade mesas of this frame (boxed: built once per frame).
    pub events: Box<[events::ActiveEvent; events::CRATER_SLOTS + events::SPADE_SLOTS]>,
    pub clouds: clouds::CloudLayer,
}

pub fn prepare(params: &EntranceParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    let seed = params.seed as u32;
    let (az, el) = ViewPreset::Oblique.angles();
    let focus = direction_from_angles(az, el);
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
        seed,
        focus,
        basin_center: hash_direction(seed, 0, 0, focus, 25.0_f32.to_radians()),
        events: Box::new(events::active_events(params, seed, focus, uniforms.time)),
        clouds: clouds::CloudLayer::new(params, seed, uniforms.time),
    }
}

impl Frame {
    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        let n = s.normal;
        match self.debug {
            DebugView::Clouds => return Vector3::one() * self.clouds.mask(n),
            DebugView::None => {}
            // Ground debug views are shown without clouds.
            _ => return self.surface_color(n),
        }
        let surface = self.surface_color(n);
        // Clouds over the ground, with their own soft lighting and the
        // geometric normal (they float above the relief).
        let l = self.uniforms.light_direction;
        let cloud =
            mix3(hex(0x6E6468), hex(0xD6D0C4), 0.35) * (0.08 + 0.92 * wrap_lambert(n, l, 0.3));
        mix3(surface, cloud, self.clouds.alpha(n))
    }

    /// Lookup direction of the ground seen along the view: the geometric
    /// direction shifted by a stepped search of the macro height field
    /// (parallax). Identity when disabled, at zero scale or facing the eye.
    pub fn parallax_direction(&self, n: Vector3) -> Vector3 {
        let p = &self.params;
        if p.parallax < 0.5 {
            return n;
        }
        parallax_shift(
            n,
            self.uniforms.eye_direction,
            |q| terrain::macro_height(q, self),
            p.parallax_scale,
            p.parallax_steps.round() as u32,
            3,
        )
    }

    /// Normal used for lighting the ground: the geometric normal tilted by
    /// the gradient of the full height field (bump mapping). Night, cloud
    /// and shadow terms keep the geometric normal.
    pub fn shading_normal(&self, n: Vector3) -> Vector3 {
        let p = &self.params;
        if p.bump < 0.5 {
            return n;
        }
        bump_normal(
            n,
            |q| terrain::height(q, self),
            p.bump_epsilon,
            p.bump_strength,
        )
    }

    /// Light reflected by a terrain material: ambient plus diffuse (wrapped
    /// for ash), a warm Blinn-Phong highlight on obsidian, both attenuated
    /// by the cloud `shadow`. Emissive fissures only keep the ambient term,
    /// so they do not depend on the light; their glow is added separately.
    pub fn reflect_light(
        &self,
        material: TerrainMaterial,
        albedo: Vector3,
        lit_normal: Vector3,
        l: Vector3,
        v: Vector3,
        shadow: f32,
    ) -> Vector3 {
        let p = &self.params;
        let response = material.response();
        if response.emissive {
            return albedo * 0.06;
        }
        let diffuse = if response.wrap > 0.0 {
            wrap_lambert(lit_normal, l, response.wrap)
        } else {
            lambert(lit_normal, l)
        };
        let mut color = albedo * (0.06 + diffuse * shadow);
        if p.specular >= 0.5 && response.specular > 0.0 {
            let highlight = blinn_phong(lit_normal, l, v, p.shininess);
            color += hex(0xFFF2C4) * (p.specular_strength * response.specular * highlight * shadow);
        }
        color
    }

    /// Night factor in `[0, 1]` from the geometric normal: 0 on the day side,
    /// rising smoothly across a terminator of width `terminator_width`.
    pub fn night_factor(&self, n: Vector3, l: Vector3) -> f32 {
        smoothstep(0.05, -self.params.terminator_width, n.dot(l))
    }

    /// Self-emission of fissures and molten crater floors: a dim day level
    /// that rises to `night_glow` on the night side. A separate term, never
    /// a darkened diffuse.
    pub fn emission(&self, crack: f32, magma: f32, n: Vector3, l: Vector3) -> Vector3 {
        let p = &self.params;
        let level = if p.night_lights >= 0.5 {
            mixf(DAY_GLOW, p.night_glow, self.night_factor(n, l))
        } else {
            DAY_GLOW
        };
        let core = saturate(crack.max(magma));
        mix3(hex(0xD21F2B), hex(0xFF7A00), core) * ((crack + magma) * level)
    }

    /// Lit ground color without clouds.
    pub fn surface_color(&self, n: Vector3) -> Vector3 {
        let l = self.uniforms.light_direction;
        // Cloud shadows attenuate direct light only; emission is untouched.
        let shadow = self.clouds.shadow(n, l);
        match self.debug {
            DebugView::Shadows => return Vector3::one() * (1.0 - shadow),
            DebugView::Night => return Vector3::one() * self.night_factor(n, l),
            _ => {}
        }
        // Every ground lookup uses the parallax-shifted direction; night,
        // clouds and shadows keep the geometric one.
        let np = self.parallax_direction(n);
        if self.debug == DebugView::Parallax {
            return Vector3::one() * (angular_distance(n, np) * 40.0).min(1.0);
        }
        let ground = terrain::ground(np, self);
        let lit_normal = self.shading_normal(np);
        match self.debug {
            DebugView::Normals => return lit_normal * 0.5 + Vector3::new(0.5, 0.5, 0.5),
            DebugView::Height => {
                return Vector3::one() * (0.5 + 0.5 * ground.height).clamp(0.0, 1.0);
            }
            DebugView::Materials => return ground.material.debug_color(),
            _ => {}
        }
        let albedo = terrain::albedo(ground.material, np, self);
        let v = self.uniforms.eye_direction;
        let color = self.reflect_light(ground.material, albedo, lit_normal, l, v, shadow);
        color + self.emission(ground.crack, ground.magma, n, l)
    }
}
