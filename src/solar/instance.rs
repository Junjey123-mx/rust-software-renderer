//! One transformed celestial body and the shader inputs it needs.

use crate::celestial::{Body, CelestialMaterial, DebugView, PreparedShader};
use crate::geometry::Vertex;
use crate::pipeline::{
    FragmentView, RenderError, RenderScratch, ShadedOptions, ShadedStats, ShaderUniforms,
    SurfaceSample, Transform, Uniforms, ViewProjection, render_shaded_with_scratch,
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
    /// Gravitational parameter of the body (0: no pull), from the spec.
    pub mu: f32,
}

impl BodyInstance {
    pub fn new(body: Body, transform: Transform, spin_speed: f32) -> Self {
        Self {
            material: CelestialMaterial::for_body(body),
            transform,
            spin_speed,
            orbit: None,
            debug: DebugView::None,
            mu: 0.0,
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
    render_instance_scratch(
        framebuffer,
        mesh,
        instance,
        frame,
        &mut RenderScratch::new(),
    )
}

/// The instance's shader with its per-frame values computed for `frame`.
pub fn prepare_instance(instance: &BodyInstance, frame: &FrameContext) -> PreparedShader {
    let transform = instance.transform;
    let eye_world = eye_towards(&frame.projection, transform.position);
    let uniforms = instance_uniforms(&transform, frame.time, frame.light_world, eye_world);
    instance.material.prepare(&uniforms, instance.debug)
}

/// Color of one fragment of an instance: the shader sees the fragment's
/// own direction towards the eye, rotated into the instance's object space.
pub fn shade_instance(
    prepared: &PreparedShader,
    transform: &Transform,
    sample: &SurfaceSample,
    view: &FragmentView,
) -> Vector3 {
    prepared.shade_with_eye(sample, unit(transform.to_object_dir(view.view_direction)))
}

/// `render_instance` reusing the caller's buffers.
pub fn render_instance_scratch(
    framebuffer: &mut Framebuffer,
    mesh: &[Vertex],
    instance: &BodyInstance,
    frame: &FrameContext,
    scratch: &mut RenderScratch,
) -> Result<ShadedStats, RenderError> {
    let transform = instance.transform;
    let prepared = prepare_instance(instance, frame);
    let model = Uniforms {
        model_matrix: transform.matrix(),
    };
    render_shaded_with_scratch(
        framebuffer,
        &model,
        mesh,
        &frame.projection,
        frame.mode,
        frame.options,
        &|s, v| shade_instance(&prepared, &transform, s, v),
        scratch,
    )
}

/// Shader quality of the solar scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Quality {
    /// Every body with its full laboratory six parameters (default).
    #[default]
    Full,
    /// Opt-in: small bodies on screen use fewer parallax steps and noise
    /// octaves. Near bodies keep full quality.
    Adaptive,
}

impl std::str::FromStr for Quality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "full" => Ok(Self::Full),
            "adaptive" => Ok(Self::Adaptive),
            other => Err(format!(
                "unknown solar quality '{other}' (expected full or adaptive)"
            )),
        }
    }
}

/// Projected radius (pixels) from which a body keeps full quality.
pub const FULL_DETAIL_PX: f32 = 120.0;
/// Projected radius (pixels) at and below which the lowest detail applies.
pub const LOW_DETAIL_PX: f32 = 20.0;

/// Material used to draw a body whose projected radius is `radius_px`
/// under `quality`. `Full`, or any body at least `FULL_DETAIL_PX` across,
/// returns the material unchanged; otherwise the parallax steps and the
/// noise octaves ramp down continuously (no state between frames, so the
/// same instant always renders the same image). The stored material is
/// never modified.
pub fn adapted_material(
    material: &CelestialMaterial,
    quality: Quality,
    radius_px: f32,
) -> CelestialMaterial {
    if quality == Quality::Full || radius_px.is_nan() || radius_px >= FULL_DETAIL_PX {
        return *material;
    }
    let t = ((radius_px - LOW_DETAIL_PX) / (FULL_DETAIL_PX - LOW_DETAIL_PX)).clamp(0.0, 1.0);
    let lerp = |low: f32, full: f32| (low + (full - low) * t).round().min(full);
    let mut m = *material;
    match &mut m {
        CelestialMaterial::Entrance(p) => {
            p.parallax_steps = lerp(2.0, p.parallax_steps);
            p.terrain_octaves = lerp(3.0_f32.min(p.terrain_octaves), p.terrain_octaves);
        }
        CelestialMaterial::Crown(p) => {
            p.plasma_octaves = lerp(3.0_f32.min(p.plasma_octaves), p.plasma_octaves);
        }
        CelestialMaterial::House(_) => {}
    }
    m
}

/// Radius in pixels of a world sphere seen through `projection` on a
/// framebuffer `height` pixels tall (infinite when the eye is inside it).
pub fn projected_radius_px(
    projection: &ViewProjection,
    center: Vector3,
    radius: f32,
    height: u32,
) -> f32 {
    match projection {
        ViewProjection::Orbit(view) => radius * view.scale(),
        ViewProjection::Perspective(camera) => {
            let z = camera.world_to_view(center).z;
            if z <= radius {
                f32::INFINITY
            } else {
                camera.focal_px(height) * radius / z
            }
        }
    }
}
