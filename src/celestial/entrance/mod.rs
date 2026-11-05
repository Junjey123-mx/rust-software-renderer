//! Crimson Entrance: volcanic planet of craters and spade mesas (rocky).

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::shading::lighting::lambert;
use crate::shading::math::hex;
use raylib::math::Vector3;

params! {
    EntranceParams {
        seed: 11.0, 0.0, 9999.0, "noise seed";
    }
}

/// Per-frame state of the rocky planet shader.
#[derive(Debug, Clone)]
pub struct Frame {
    pub params: EntranceParams,
    pub uniforms: ShaderUniforms,
    pub debug: DebugView,
}

pub fn prepare(params: &EntranceParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
    }
}

impl Frame {
    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        hex(0x8F1018) * (0.06 + lambert(s.normal, self.uniforms.light_direction))
    }
}
