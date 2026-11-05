//! Crimson Entrance: volcanic planet of craters and spade mesas (rocky).
//!
//! The surface is classified from one procedural height field (see
//! `terrain`): obsidian plains, basalt, crimson highlands, slate mesas, ash
//! rims and glowing fissures. Its only symbol is the spade.

pub mod events;
pub mod terrain;

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample, ViewPreset, direction_from_angles};
use crate::shading::lighting::lambert;
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
    }
}

impl Frame {
    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        let n = s.normal;
        let ground = terrain::ground(n, self);
        match self.debug {
            DebugView::Height => {
                return Vector3::one() * (0.5 + 0.5 * ground.height).clamp(0.0, 1.0);
            }
            DebugView::Materials => return ground.material.debug_color(),
            _ => {}
        }
        let albedo = terrain::albedo(ground.material, n, self);
        let mut color = albedo * (0.06 + lambert(n, self.uniforms.light_direction));
        if ground.material == TerrainMaterial::Crack {
            color += hex(0xFF7A00) * 0.15;
        }
        color + mix3(hex(0xD21F2B), hex(0xFF7A00), ground.magma) * (0.4 * ground.magma)
    }
}
