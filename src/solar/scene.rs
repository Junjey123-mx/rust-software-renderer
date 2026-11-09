//! The solar scene: shared meshes, a flat list of body instances and the
//! values every instance shares in a frame.
//!
//! Frame contract: the owner of the framebuffer clears it exactly once per
//! frame; `render_objects_without_clear` never clears color or depth, so the
//! shared depth buffer resolves occlusion between instances whatever their
//! draw order.

use crate::celestial::Body;
use crate::celestial::PreparedShader;
use crate::pipeline::{
    DirectionalLight, OrbitView, PerspectiveCamera, RenderError, RenderScratch, ShadedOptions,
    ShadedStats, Transform, Uniforms, ViewProjection, sphere_visible,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::solar::camera::{CameraRig, FollowRig, orbit_perspective};
use crate::solar::flight::{FlightInput, FlightMode, LocalTarget, local_step, toggle_local};
use crate::solar::instance::{
    Quality, adapted_material, prepare_instance, projected_radius_px, render_instance_scratch,
    shade_instance,
};
use crate::solar::ship::{render_ship_scratch, ship_color, ship_light, ship_options};
use crate::solar::skybox::{DEFAULT_SEED, DEFAULT_STARS, StarField, draw_stars};
use crate::solar::{BodyInstance, FrameContext, Orbit, ShipState, SimulationClock, SolarMeshes};
use raylib::math::Vector3;
use std::cell::RefCell;

/// Which projection the solar scene renders through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraMode {
    /// The orthographic orbit camera of the earlier scenes.
    Orbit,
    /// A perspective camera placed on the orbit camera's eye direction.
    #[default]
    Perspective,
}

impl CameraMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Orbit => "orbit",
            Self::Perspective => "perspective",
        }
    }
}

impl std::str::FromStr for CameraMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "orbit" => Ok(Self::Orbit),
            "perspective" => Ok(Self::Perspective),
            other => Err(format!(
                "unknown camera '{other}' (expected perspective or orbit)"
            )),
        }
    }
}

/// Spacecraft scale: the 3.99-unit mesh becomes about 0.32 units, well
/// below the smallest planet (0.52).
pub const SHIP_SCALE: f32 = 0.08;
/// Start of the spacecraft: outside every orbit, slightly above the orbital
/// plane, nose (-Z) towards the star.
pub const SHIP_SPAWN: Vector3 = Vector3 {
    x: 0.0,
    y: 0.6,
    z: 7.5,
};

/// Vertical field of view of the perspective camera, radians (60 degrees).
pub const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;
/// Near plane of the perspective camera.
pub const NEAR: f32 = 0.05;

#[derive(Debug, Clone)]
pub struct SolarScene {
    pub meshes: SolarMeshes,
    pub bodies: Vec<BodyInstance>,
    /// Orbit camera rig: target, azimuth, elevation and zoom.
    pub view: OrbitView,
    pub camera_mode: CameraMode,
    /// Which rig drives the perspective camera.
    pub rig: CameraRig,
    pub follow: FollowRig,
    pub ship: ShipState,
    pub flight: FlightMode,
    /// Background stars, generated once.
    pub stars: StarField,
    /// Shader quality policy (full by default).
    pub quality: Quality,
    /// Shaded-stage buffers reused by every draw of this scene.
    scratch: RefCell<RenderScratch>,
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
            camera_mode: CameraMode::default(),
            rig: CameraRig::default(),
            follow: FollowRig::default(),
            ship: ShipState::new(SHIP_SPAWN, SHIP_SCALE),
            flight: FlightMode::Space,
            stars: StarField::generate(DEFAULT_SEED, DEFAULT_STARS),
            scratch: RefCell::new(RenderScratch::new()),
            quality: Quality::Full,
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

    /// Ideal-sphere radius of a body instance.
    pub fn radius_of(&self, index: usize) -> f32 {
        crate::solar::SPHERE_RADIUS * self.bodies[index].transform.scale
    }

    /// Bodies local flight may orbit: every orbiting body (never the star).
    pub fn local_targets(&self) -> Vec<LocalTarget> {
        self.bodies
            .iter()
            .enumerate()
            .filter(|(_, b)| b.orbit.is_some())
            .map(|(index, b)| LocalTarget {
                index,
                position: b.transform.position,
                radius: self.radius_of(index),
            })
            .collect()
    }

    /// Runs the local controller around the body local flight follows, at
    /// its current (already updated) position.
    pub fn local_step(&mut self, input: &FlightInput, dt: f32) {
        let FlightMode::Local { planet, .. } = self.flight else {
            return;
        };
        let (position, radius) = (
            self.bodies[planet].transform.position,
            self.radius_of(planet),
        );
        local_step(
            &mut self.ship,
            &mut self.flight,
            position,
            radius,
            input,
            dt,
        );
    }

    /// `L`: toggles local flight; the ship is never moved by the toggle.
    pub fn toggle_local(&mut self) -> Result<FlightMode, String> {
        self.flight = toggle_local(self.flight, &self.ship, &self.local_targets())?;
        Ok(self.flight)
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

    /// Perspective camera on the orbit rig: the eye sits along the orbit
    /// eye direction at a distance that frames the system at zoom 1 and
    /// approaches with the zoom.
    pub fn perspective_camera(&self) -> PerspectiveCamera {
        orbit_perspective(&self.view.camera, self.extent().max(1.0) * 2.2)
    }

    /// Camera whose rotation and field of view orient the star field: the
    /// frame camera, or the orbit rig's perspective under the orthographic
    /// camera (which has no field of view of its own).
    pub fn sky_camera(&self) -> PerspectiveCamera {
        match self.projection() {
            ViewProjection::Perspective(camera) => camera,
            ViewProjection::Orbit(_) => self.perspective_camera(),
        }
    }

    /// The camera of this frame: orthographic orbit (`--camera orbit`), or
    /// perspective from the orbit rig or behind the spacecraft.
    pub fn projection(&self) -> ViewProjection {
        match (self.camera_mode, self.rig) {
            (CameraMode::Orbit, _) => ViewProjection::Orbit(self.view),
            (CameraMode::Perspective, CameraRig::Orbit) => {
                ViewProjection::Perspective(self.perspective_camera())
            }
            (CameraMode::Perspective, CameraRig::Follow) => {
                ViewProjection::Perspective(self.follow.camera(&self.ship))
            }
        }
    }

    /// Direction from the spacecraft towards the star (scene light when the
    /// ship sits on the center).
    pub fn ship_light(&self) -> Vector3 {
        let d = self.center - self.ship.position;
        let len = d.length();
        if len.is_finite() && len > 1e-6 {
            d / len
        } else {
            self.light_world
        }
    }

    /// Draws the spacecraft without clearing.
    pub fn render_ship_without_clear(
        &self,
        framebuffer: &mut Framebuffer,
        mode: TriangleMode,
        options: ShadedOptions,
    ) -> Result<ShadedStats, RenderError> {
        let (w, h) = (framebuffer.width(), framebuffer.height());
        if !self.sphere_in_view(self.ship.position, self.ship.radius(), w, h) {
            return Ok(ShadedStats::default());
        }
        render_ship_scratch(
            framebuffer,
            &self.meshes.ship,
            &self.ship,
            &self.projection(),
            self.ship_light(),
            mode,
            options.threads,
            &mut self.scratch.borrow_mut(),
        )
    }

    /// One frame: clear color and depth once, the star field (color only),
    /// then every body, then the spacecraft, all through the same camera and
    /// depth buffer. Returns the fragments shaded.
    ///
    /// Filled modes run as one frame batch: every visible instance is
    /// rasterized first (depth written at once), all samples are shaded by a
    /// single thread pool and the colors are resolved by depth. The image is
    /// identical to drawing the instances one by one
    /// (`render_objects_without_clear` + `render_ship_without_clear`), but
    /// idle cores at the end of each instance are avoided. Wireframe draws
    /// the instances one by one.
    pub fn render_frame(
        &self,
        framebuffer: &mut Framebuffer,
        mode: TriangleMode,
        options: ShadedOptions,
    ) -> Result<usize, RenderError> {
        framebuffer.clear();
        draw_stars(framebuffer, &self.stars, &self.sky_camera());
        if mode == TriangleMode::Wireframe {
            let bodies = self.render_objects_without_clear(framebuffer, mode, options)?;
            let ship = self.render_ship_without_clear(framebuffer, mode, options)?;
            return Ok(
                bodies.iter().map(|s| s.fragments_shaded).sum::<usize>() + ship.fragments_shaded
            );
        }
        let (w, h) = (framebuffer.width(), framebuffer.height());
        let shared = self.frame_context(mode, options);
        let mut scratch = self.scratch.borrow_mut();
        scratch.begin_batch();
        let mut jobs: Vec<Job> = Vec::with_capacity(self.bodies.len() + 1);
        for (i, body) in self.bodies.iter().enumerate() {
            if !self.body_in_view(i, w, h) {
                continue;
            }
            let drawn = self.drawn_body(i, &shared.projection, h);
            let frame = FrameContext {
                light_world: self.light_for(body),
                ..shared
            };
            jobs.push(Job::Body(
                Box::new(prepare_instance(&drawn, &frame)),
                drawn.transform,
            ));
            let model = Uniforms {
                model_matrix: drawn.transform.matrix(),
            };
            let tag = (jobs.len() - 1) as u32;
            scratch.rasterize(
                framebuffer,
                &model,
                &self.meshes.sphere,
                &shared.projection,
                mode,
                options,
                tag,
            )?;
        }
        if self.sphere_in_view(self.ship.position, self.ship.radius(), w, h) {
            jobs.push(Job::Ship(ship_light(&self.ship, self.ship_light())));
            let model = Uniforms {
                model_matrix: self.ship.model_matrix(),
            };
            let tag = (jobs.len() - 1) as u32;
            scratch.rasterize(
                framebuffer,
                &model,
                &self.meshes.ship,
                &shared.projection,
                mode,
                ship_options(options.threads),
                tag,
            )?;
        }
        scratch.shade_batch(options.threads, |tag, s, v| match &jobs[tag as usize] {
            Job::Body(prepared, transform) => shade_instance(prepared, transform, s, v),
            Job::Ship(light) => ship_color(light, s),
        });
        scratch.resolve(framebuffer);
        Ok(scratch.batch_len())
    }

    /// Body `index` as drawn this frame: its material adapted to the
    /// quality policy and its projected size.
    fn drawn_body(&self, index: usize, projection: &ViewProjection, height: u32) -> BodyInstance {
        let body = &self.bodies[index];
        let radius_px = projected_radius_px(
            projection,
            body.transform.position,
            self.radius_of(index),
            height,
        );
        BodyInstance {
            material: adapted_material(&body.material, self.quality, radius_px),
            ..*body
        }
    }

    pub fn frame_context(&self, mode: TriangleMode, options: ShadedOptions) -> FrameContext {
        FrameContext {
            projection: self.projection(),
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

    /// Bounding-sphere margin of the view test (relative), against rounding.
    pub const CULL_MARGIN: f32 = 1.02;

    /// Whether a world sphere can appear on a `width` x `height` framebuffer
    /// through this frame's camera (frustum under perspective, the screen
    /// rectangle under the orthographic orbit camera).
    pub fn sphere_in_view(&self, center: Vector3, radius: f32, width: u32, height: u32) -> bool {
        let radius = radius * Self::CULL_MARGIN;
        match self.projection() {
            ViewProjection::Perspective(camera) => {
                sphere_visible(&camera, center, radius, width, height)
            }
            ViewProjection::Orbit(view) => {
                let e = view.camera.world_to_view(center);
                let s = view.scale();
                let (x, y, r) = (
                    width as f32 / 2.0 + e.x * s,
                    height as f32 / 2.0 - e.y * s,
                    radius * s,
                );
                x + r >= 0.0 && x - r <= width as f32 && y + r >= 0.0 && y - r <= height as f32
            }
        }
    }

    /// Whether body `index` passes the view test.
    pub fn body_in_view(&self, index: usize, width: u32, height: u32) -> bool {
        let b = &self.bodies[index];
        self.sphere_in_view(b.transform.position, self.radius_of(index), width, height)
    }

    /// Fragments the reused shaded-stage buffers can hold.
    pub fn scratch_capacity(&self) -> usize {
        self.scratch.borrow().capacity()
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
        let mut scratch = self.scratch.borrow_mut();
        let (w, h) = (framebuffer.width(), framebuffer.height());
        self.bodies
            .iter()
            .enumerate()
            .map(|(i, body)| {
                // Skipped before any vertex is transformed: it cannot add a pixel.
                if !self.body_in_view(i, w, h) {
                    return Ok(ShadedStats::default());
                }
                let frame = FrameContext {
                    light_world: self.light_for(body),
                    ..shared
                };
                let drawn = self.drawn_body(i, &shared.projection, h);
                render_instance_scratch(
                    framebuffer,
                    &self.meshes.sphere,
                    &drawn,
                    &frame,
                    &mut scratch,
                )
            })
            .collect()
    }
}

/// One shading job of a frame batch: what a tagged sample is shaded with.
enum Job {
    /// Boxed: a prepared shader is large and at most four jobs exist.
    Body(Box<PreparedShader>, Transform),
    Ship(DirectionalLight),
}
