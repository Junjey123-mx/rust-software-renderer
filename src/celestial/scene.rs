//! A celestial body on the course sphere with its camera, time and light.

use crate::celestial::{Body, CelestialMaterial, DebugView};
use crate::config::SPHERE_MODEL_PATH;
use crate::geometry::Vertex;
use crate::io::load_obj;
use crate::pipeline::{
    OrbitCamera, OrbitView, RenderError, ShadedOptions, ShadedStats, ShaderUniforms, Uniforms,
    ViewPreset, Viewport, direction_from_angles, model_bounds, render_shaded,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use std::error::Error;
use std::path::Path;

/// Default light: azimuth -75 degrees, elevation 20 degrees, so the oblique
/// view shows the terminator.
pub const DEFAULT_LIGHT_AZIMUTH_DEG: f32 = -75.0;
pub const DEFAULT_LIGHT_ELEVATION_DEG: f32 = 20.0;

#[derive(Debug, Clone)]
pub struct CelestialScene {
    pub material: CelestialMaterial,
    /// The expanded course sphere (576 corners of 192 triangles).
    pub vertices: Vec<Vertex>,
    pub view: OrbitView,
    /// Simulated seconds.
    pub time: f32,
    /// Radians.
    pub light_azimuth: f32,
    /// Radians.
    pub light_elevation: f32,
    pub debug: DebugView,
}

impl CelestialScene {
    /// Loads the course sphere once and frames it for a `width` x `height`
    /// framebuffer.
    pub fn new(
        body: Body,
        preset: ViewPreset,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn Error>> {
        let mesh = load_obj(Path::new(SPHERE_MODEL_PATH))?;
        let vertices = mesh.expand_vertices(body.accent());
        let bounds = model_bounds(&vertices).ok_or("course sphere has no finite geometry")?;
        let camera = OrbitCamera::new(bounds.center, preset);
        let view = OrbitView::new(camera, &bounds, Viewport::new(width, height, 0.1))?;
        Ok(Self {
            material: CelestialMaterial::for_body(body),
            vertices,
            view,
            time: 0.0,
            light_azimuth: DEFAULT_LIGHT_AZIMUTH_DEG.to_radians(),
            light_elevation: DEFAULT_LIGHT_ELEVATION_DEG.to_radians(),
            debug: DebugView::None,
        })
    }

    pub fn body(&self) -> Body {
        self.material.body()
    }

    /// Switches body (default parameters); camera, time and light are kept.
    pub fn set_body(&mut self, body: Body) {
        self.material = CelestialMaterial::for_body(body);
        let accent = body.accent();
        for v in &mut self.vertices {
            v.color = accent;
        }
    }

    pub fn uniforms(&self) -> ShaderUniforms {
        ShaderUniforms {
            time: self.time,
            light_direction: direction_from_angles(self.light_azimuth, self.light_elevation),
            eye_direction: self.view.camera.eye_direction(),
        }
    }

    /// Applies `--set name=value` overrides, validating name and range.
    pub fn apply_overrides(&mut self, sets: &[(String, f32)]) -> Result<(), String> {
        for (name, value) in sets {
            self.material.set(name, *value)?;
        }
        Ok(())
    }

    /// Renders the body into `framebuffer` (not cleared here).
    pub fn render(
        &self,
        framebuffer: &mut Framebuffer,
        mode: TriangleMode,
        options: ShadedOptions,
    ) -> Result<ShadedStats, RenderError> {
        let prepared = self.material.prepare(&self.uniforms(), self.debug);
        render_shaded(
            framebuffer,
            &Uniforms::default(),
            &self.vertices,
            self.view,
            mode,
            options,
            &|s| prepared.shade(s),
        )
    }
}
