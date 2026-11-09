//! One transformed celestial body and the shader inputs it needs.

use crate::celestial::{Body, CelestialMaterial, DebugView};
use crate::geometry::Vertex;
use crate::pipeline::{
    RenderError, ShadedOptions, ShadedStats, ShaderUniforms, Transform, Uniforms, ViewProjection,
    render_shaded_with,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::solar::Orbit;
use raylib::math::Vector3;

/// Shader uniforms of one instance: the laboratory six shaders expect the
/// light and eye directions in object space, so both world directions are
/// rotated back through the instance transform. Non-unit inputs are
/// normalised; a zero vector is kept as zero.
pub fn instance_uniforms(
    transform: &Transform,
    time: f32,
    light_world: Vector3,
    eye_world: Vector3,
) -> ShaderUniforms {
    ShaderUniforms {
        time,
        light_direction: unit(transform.to_object_dir(light_world)),
        eye_direction: unit(transform.to_object_dir(eye_world)),
    }
}

fn unit(v: Vector3) -> Vector3 {
    let len = v.length();
    if len.is_finite() && len > 1e-6 {
        v / len
    } else {
        Vector3::zero()
    }
}

/// A celestial body placed in the world: its own material parameters and
/// transform over the shared sphere mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyInstance {
    pub material: CelestialMaterial,
    pub transform: Transform,
    /// Self-rotation, radians per simulated second (applied to `yaw`).
    pub spin_speed: f32,
    /// Orbit around the scene center; `None` keeps the transform position.
    pub orbit: Option<Orbit>,
    pub debug: DebugView,
}

impl BodyInstance {
    pub fn new(body: Body, transform: Transform, spin_speed: f32) -> Self {
        Self {
            material: CelestialMaterial::for_body(body),
            transform,
            spin_speed,
            orbit: None,
            debug: DebugView::None,
        }
    }

    pub fn with_orbit(mut self, orbit: Orbit) -> Self {
        self.orbit = Some(orbit);
        self
    }

    /// Places the body for simulated `time`: orbital position (when it has
    /// an orbit) around `center` and self-rotation about its own Y axis.
    /// The material animation reads the same time through the uniforms.
    pub fn update(&mut self, time: f32, center: Vector3) {
        if let Some(orbit) = self.orbit {
            self.transform.position = orbit.position_at(time, center);
        }
        self.transform.yaw = self.spin_speed * time;
    }

    pub fn body(&self) -> Body {
        self.material.body()
    }
}

/// Values shared by every instance drawn in one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameContext {
    pub projection: ViewProjection,
    /// Simulated seconds.
    pub time: f32,
    /// Unit direction towards the light, world space.
    pub light_world: Vector3,
    pub mode: TriangleMode,
    pub options: ShadedOptions,
}

/// Unit direction from the instance center towards the eye, world space:
/// the per-frame `eye_direction` of the shader uniforms. Fragments refine it
/// through `FragmentView` under the perspective camera.
pub fn eye_towards(projection: &ViewProjection, world_point: Vector3) -> Vector3 {
    match projection {
        ViewProjection::Orbit(view) => view.camera.eye_direction(),
        ViewProjection::Perspective(camera) => camera.view_direction(world_point),
    }
}

/// Draws one instance into `framebuffer` (never cleared here) through the
/// shaded stage: the model matrix comes from the transform, the light and
/// eye directions are given in world space and every fragment shades with
/// its own direction towards the eye, rotated into object space.
pub fn render_instance(
    framebuffer: &mut Framebuffer,
    mesh: &[Vertex],
    instance: &BodyInstance,
    frame: &FrameContext,
) -> Result<ShadedStats, RenderError> {
    let transform = instance.transform;
    let eye_world = eye_towards(&frame.projection, transform.position);
    let uniforms = instance_uniforms(&transform, frame.time, frame.light_world, eye_world);
    let prepared = instance.material.prepare(&uniforms, instance.debug);
    let model = Uniforms {
        model_matrix: transform.matrix(),
    };
    render_shaded_with(
        framebuffer,
        &model,
        mesh,
        &frame.projection,
        frame.mode,
        frame.options,
        &|s, v| prepared.shade_with_eye(s, unit(transform.to_object_dir(v.view_direction))),
    )
}
