//! House of Cards: violet gas giant with diamond storms (gas giant).

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::shading::lighting::wrap_lambert;
use crate::shading::math::hex;
use raylib::math::Vector3;

params! {
    HouseParams {
        seed: 23.0, 0.0, 9999.0, "noise seed";
    }
}

/// Per-frame state of the gas giant shader.
#[derive(Debug, Clone)]
pub struct Frame {
    pub params: HouseParams,
    pub uniforms: ShaderUniforms,
    pub debug: DebugView,
}

pub fn prepare(params: &HouseParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
    }
}

impl Frame {
    pub fn shade(&self, s: &SurfaceSample) -> Vector3 {
        hex(0x8A2BE2) * (0.18 + 0.82 * wrap_lambert(s.normal, self.uniforms.light_direction, 0.35))
    }
}
