//! The solar scene: shared meshes, a flat list of body instances and the
//! values every instance shares in a frame.
//!
//! Frame contract: the owner of the framebuffer clears it exactly once per
//! frame; `render_objects_without_clear` never clears color or depth, so the
//! shared depth buffer resolves occlusion between instances whatever their
//! draw order.

use crate::celestial::Body;
use crate::pipeline::{OrbitView, RenderError, ShadedOptions, ShadedStats, Transform};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::solar::{
    BodyInstance, FrameContext, Orbit, SimulationClock, SolarMeshes, render_instance,
};
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

    /// The default system: The King's Crown at the center (radius 1.3 at
    /// scale 1.3 of the course sphere), Crimson Entrance at scale 0.6 on a
    /// 2.6 orbit and House of Cards at scale 0.9 on a 4.4 orbit, placed for
    /// time 0.
    pub fn default_system(
        meshes: SolarMeshes,
        view: OrbitView,
        light_world: Vector3,
    ) -> Result<Self, String> {
        let mut scene = Self::new(meshes, view, light_world);
        scene.bodies = vec![
            BodyInstance::new(Body::KingsCrown, Transform::new(Vector3::zero(), 1.3), 0.05),
            BodyInstance::new(
                Body::CrimsonEntrance,
                Transform::new(Vector3::zero(), 0.6),
                0.6,
            )
            .with_orbit(Orbit::new(2.6, 0.5, 0.0)?),
            BodyInstance::new(
                Body::HouseOfCards,
                Transform::new(Vector3::zero(), 0.9),
                0.35,
            )
            .with_orbit(Orbit::new(4.4, 0.22, std::f32::consts::PI)?),
        ];
        scene.update();
        Ok(scene)
    }

    /// Radius of the outermost orbit plus that body's radius: the world
    /// sphere a camera has to frame.
    pub fn extent(&self) -> f32 {
        self.bodies
            .iter()
            .map(|b| {
                let r = b
                    .orbit
                    .map_or((b.transform.position - self.center).length(), |o| o.radius);
                r + crate::solar::SPHERE_RADIUS * b.transform.scale
            })
            .fold(0.0, f32::max)
    }

    pub fn body_mut(&mut self, body: Body) -> Option<&mut BodyInstance> {
        self.bodies.iter_mut().find(|b| b.body() == body)
    }

    /// Applies `--set body.name=value` overrides: the prefix names the
    /// instance, the rest is validated by its material.
    pub fn apply_overrides(&mut self, sets: &[(String, f32)]) -> Result<(), String> {
        for (name, value) in sets {
            let (prefix, param) = name.split_once('.').ok_or_else(|| {
                format!("--set '{name}' needs a body prefix in the solar scene (crown., entrance. or house.)")
            })?;
            let body: Body = prefix.parse()?;
            let instance = self
                .body_mut(body)
                .ok_or_else(|| format!("body '{prefix}' is not in the scene"))?;
            instance.material.set(param, *value)?;
        }
        Ok(())
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

    /// World light of one body: the unit direction from the body towards
    /// the scene center (the star). A body sitting at the center (the star
    /// itself, emissive) keeps the scene light.
    pub fn light_for(&self, body: &BodyInstance) -> Vector3 {
        let to_star = self.center - body.transform.position;
        let len = to_star.length();
        if len.is_finite() && len > 1e-6 {
            to_star / len
        } else {
            self.light_world
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
        let shared = self.frame_context(mode, options);
        self.bodies
            .iter()
            .map(|body| {
                let frame = FrameContext {
                    light_world: self.light_for(body),
                    ..shared
                };
                render_instance(framebuffer, &self.meshes.sphere, body, &frame)
            })
            .collect()
    }
}
