//! The solar scene: shared meshes, a flat list of body instances and the
//! values every instance shares in a frame.
//!
//! Frame contract: the owner of the framebuffer clears it exactly once per
//! frame; `render_objects_without_clear` never clears color or depth, so the
//! shared depth buffer resolves occlusion between instances whatever their
//! draw order.

use crate::pipeline::{OrbitView, RenderError, ShadedOptions, ShadedStats};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::solar::{BodyInstance, FrameContext, SimulationClock, SolarMeshes, render_instance};
use raylib::math::Vector3;

#[derive(Debug, Clone)]
pub struct SolarScene {
    pub meshes: SolarMeshes,
    pub bodies: Vec<BodyInstance>,
    pub view: OrbitView,
    /// Simulated time shared by orbits, spins and material animation.
    pub clock: SimulationClock,
    /// Center of the orbits (the star).
    pub center: Vector3,
    /// Unit direction towards the light, world space.
    pub light_world: Vector3,
}

impl SolarScene {
    pub fn new(meshes: SolarMeshes, view: OrbitView, light_world: Vector3) -> Self {
        Self {
            meshes,
            bodies: Vec::new(),
            view,
            clock: SimulationClock::default(),
            center: Vector3::zero(),
            light_world,
        }
    }

    pub fn time(&self) -> f32 {
        self.clock.time
    }

    /// Advances the clock by a real frame delta and places every body at the
    /// new simulated time. Call before drawing; never during.
    pub fn advance(&mut self, real_dt: f32) {
        self.clock.advance(real_dt);
        self.update();
    }

    /// Places every body at the current simulated time.
    pub fn update(&mut self) {
        let (time, center) = (self.clock.time, self.center);
        for body in &mut self.bodies {
            body.update(time, center);
        }
    }

    pub fn frame_context(&self, mode: TriangleMode, options: ShadedOptions) -> FrameContext {
        FrameContext {
            view: self.view,
            time: self.clock.time,
            light_world: self.light_world,
            mode,
            options,
        }
    }

    /// Draws every body into `framebuffer` without clearing it. Returns the
    /// stats of each instance in list order.
    pub fn render_objects_without_clear(
        &self,
        framebuffer: &mut Framebuffer,
        mode: TriangleMode,
        options: ShadedOptions,
    ) -> Result<Vec<ShadedStats>, RenderError> {
        let frame = self.frame_context(mode, options);
        self.bodies
            .iter()
            .map(|body| render_instance(framebuffer, &self.meshes.sphere, body, &frame))
            .collect()
    }
}
