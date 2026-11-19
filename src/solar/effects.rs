//! Extra geometry of the solar scene attached to its bodies (the royal
//! crown on The King's Crown), drawn through the shared CPU pipeline and
//! depth buffer. Each effect lists its draws (a prebuilt mesh, the model
//! matrix of the body it rides on and a shader), so the frame batch and the
//! one-by-one path draw exactly the same thing and `scene.rs` stays a thin
//! dispatcher.

use crate::celestial::Body;
use crate::geometry::Vertex;
use crate::pipeline::{
    FragmentView, NormalSource, RenderError, ShadedOptions, ShadedStats, SurfaceSample, Transform,
    Uniforms, render_shaded_with_scratch,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::solar::SolarScene;
use crate::solar::royal_crown::{CrownPart, CrownShader};
use raylib::math::Vector3;

/// What a draw is shaded with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectShader {
    Crown(CrownShader),
}

impl EffectShader {
    pub fn shade(&self, s: &SurfaceSample, view: &FragmentView) -> Vector3 {
        match self {
            Self::Crown(c) => c.shade(s.object_position, s.face_normal, view.view_direction),
        }
    }
}

/// One mesh of an effect ready to draw this frame.
#[derive(Debug, Clone, Copy)]
pub struct EffectDraw<'a> {
    pub mesh: &'a [Vertex],
    pub model: Uniforms,
    pub options: ShadedOptions,
    pub shader: EffectShader,
}

/// Closed meshes wound outwards with flat normals.
pub fn solid_options(threads: usize) -> ShadedOptions {
    ShadedOptions {
        cull_back_faces: true,
        normal_source: NormalSource::Face,
        threads,
    }
}

impl SolarScene {
    /// The body wearing the royal crown, when the scene has both.
    pub fn crown_host(&self) -> Option<usize> {
        self.crown.as_ref()?;
        self.bodies
            .iter()
            .position(|b| b.body() == Body::KingsCrown)
    }

    /// The crown's transform: the star's, turned by the crown's own spin.
    pub fn crown_transform(&self, host: usize) -> Transform {
        let star = self.bodies[host].transform;
        let spin = self.crown.as_ref().map_or(0.0, |c| c.spin);
        Transform {
            yaw: star.yaw + spin * self.clock.time,
            ..star
        }
    }

    /// Every effect draw that can appear on a `width` x `height` frame.
    pub fn effect_draws(&self, width: u32, height: u32, threads: usize) -> Vec<EffectDraw<'_>> {
        let mut draws = Vec::new();
        if let (Some(host), Some(crown)) = (self.crown_host(), &self.crown) {
            let transform = self.crown_transform(host);
            let reach = self.radius_of(host) * crown.extent;
            if self.sphere_in_view(transform.position, reach, width, height) {
                let model = Uniforms {
                    model_matrix: transform.matrix(),
                };
                for (mesh, part) in [
                    (&crown.gold, CrownPart::Gold),
                    (&crown.jewels, CrownPart::Jewels),
                ] {
                    draws.push(EffectDraw {
                        mesh,
                        model,
                        options: solid_options(threads),
                        shader: EffectShader::Crown(crown.shader(part, transform, self.clock.time)),
                    });
                }
            }
        }
        draws
    }

    /// Draws every effect without clearing, one draw at a time. Returns
    /// the stats of each draw.
    pub fn render_effects_without_clear(
        &self,
        framebuffer: &mut Framebuffer,
        mode: TriangleMode,
        options: ShadedOptions,
    ) -> Result<Vec<ShadedStats>, RenderError> {
        let draws = self.effect_draws(framebuffer.width(), framebuffer.height(), options.threads);
        let projection = self.projection();
        let mut scratch = self.scratch_mut();
        draws
            .iter()
            .map(|d| {
                render_shaded_with_scratch(
                    framebuffer,
                    &d.model,
                    d.mesh,
                    &projection,
                    mode,
                    d.options,
                    &|s, v| d.shader.shade(s, v),
                    &mut scratch,
                )
            })
            .collect()
    }
}
