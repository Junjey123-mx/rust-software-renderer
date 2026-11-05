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
use crate::shading::lighting::{bump_normal, lambert, wrap_lambert};
use crate::shading::math::{hex, mix3};
use crate::shading::sphere::hash_direction;
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
    }
}

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

    /// Lit ground color without clouds.
    pub fn surface_color(&self, n: Vector3) -> Vector3 {
        let l = self.uniforms.light_direction;
        // Cloud shadows attenuate direct light only; emission is untouched.
        let shadow = self.clouds.shadow(n, l);
        if self.debug == DebugView::Shadows {
            return Vector3::one() * (1.0 - shadow);
        }
        let ground = terrain::ground(n, self);
        let lit_normal = self.shading_normal(n);
        match self.debug {
            DebugView::Normals => return lit_normal * 0.5 + Vector3::new(0.5, 0.5, 0.5),
            DebugView::Height => {
                return Vector3::one() * (0.5 + 0.5 * ground.height).clamp(0.0, 1.0);
            }
            DebugView::Materials => return ground.material.debug_color(),
            _ => {}
        }
        let albedo = terrain::albedo(ground.material, n, self);
        let mut color = albedo * (0.06 + lambert(lit_normal, l) * shadow);
        if ground.material == TerrainMaterial::Crack {
            color += hex(0xFF7A00) * 0.15;
        }
        color + mix3(hex(0xD21F2B), hex(0xFF7A00), ground.magma) * (0.4 * ground.magma)
    }
}
