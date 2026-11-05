//! The King's Crown: royal star of living plasma (laboratory 6, star).

use crate::celestial::DebugView;
use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::shading::math::hex;
use raylib::math::Vector3;

params! {
    CrownParams {
        seed: 7.0, 0.0, 9999.0, "noise seed";
        brightness: 1.0, 0.2, 2.0, "overall emission";
    }
}

/// Per-frame state of the star shader.
#[derive(Debug, Clone)]
pub struct Frame {
    pub params: CrownParams,
    pub uniforms: ShaderUniforms,
    pub debug: DebugView,
}

pub fn prepare(params: &CrownParams, uniforms: &ShaderUniforms, debug: DebugView) -> Frame {
    Frame {
        params: *params,
        uniforms: *uniforms,
        debug,
    }
}

impl Frame {
    pub fn shade(&self, _s: &SurfaceSample) -> Vector3 {
        hex(0xFFC63A) * self.params.brightness
    }
}
