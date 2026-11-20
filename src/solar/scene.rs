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
    ShadedStats, Transform, Uniforms, ViewProjection, render_shaded_with_scratch, sphere_visible,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::solar::attach::{AttachmentKind, AttachmentSpec, HostPose};
use crate::solar::camera::{CameraRig, FollowRig, orbit_perspective};
use crate::solar::card_ring::CardRing;
use crate::solar::effects::EffectShader;
use crate::solar::flight::{
    FlightInput, FlightMode, FlightScale, LocalTarget, local_step_scaled, toggle_local_scaled,
};
use crate::solar::instance::{
    Quality, adapted_material, prepare_instance, projected_radius_px, render_instance_scratch,
    shade_instance,
};
use crate::solar::nebula::{Nebula, draw_nebula};
use crate::solar::physics::GravitySource;
use crate::solar::planet::PlanetMaterial;
use crate::solar::plasma_loops::{ArcShader, PlasmaLoops, plasma_options};
use crate::solar::royal_crown::RoyalCrown;
use crate::solar::ship::{render_ship_scratch, ship_color, ship_light, ship_options};
use crate::solar::skybox::{DEFAULT_SEED, DEFAULT_STARS, StarDensity, StarField, draw_stars};
use crate::solar::stats::FrameStats;
use crate::solar::system::layout::{FIFTH_SLOT_RESERVED_EXTENT, OrbitLayout};
use crate::solar::system::{PlanetId, ScaleProfile, SpecError, SystemSpec};
use crate::solar::thrusters::Thrusters;
use crate::solar::{
    BodyInstance, FrameContext, Orbit, SPHERE_RADIUS, ShipState, SimulationClock, SolarMeshes,
};
use raylib::math::Vector3;
use std::cell::{Ref, RefCell, RefMut};

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
/// Start of the spacecraft for a system whose shells end at `outer`: one
/// unit beyond, slightly above the orbital plane, nose (-Z) towards the
/// star. `SHIP_SPAWN` is `spawn_beyond(8.0)`.
pub fn spawn_beyond(outer: f32) -> Vector3 {
    Vector3::new(0.0, 0.6, outer + 1.0)
}

/// Start of the spacecraft: outside every orbit and the space reserved for
/// a ring around House of Cards (6.2 + 1.7), slightly above the orbital
/// plane, nose (-Z) towards the star.
pub const SHIP_SPAWN: Vector3 = Vector3 {
    x: 0.0,
    y: 0.6,
    z: 9.0,
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
    /// Colored backdrop behind the stars.
    pub nebula: Nebula,
    /// Magnetic arcs that rise out of The King's Crown, drawn with its
    /// transform (no arcs: nothing drawn).
    pub plasma: PlasmaLoops,
    /// Royal crown worn by The King's Crown (`None`: no crown).
    pub crown: Option<RoyalCrown>,
    /// Ring of diamond cards around House of Cards (`None`: no ring).
    pub cards: Option<CardRing>,
    /// What rides on which body, by id, in draw order: the royal crown on
    /// the star and the card ring around House of Cards. The geometry
    /// itself lives in `crown` and `cards`; an attachment without its
    /// geometry, or whose host is missing, draws nothing.
    pub attachments: Vec<AttachmentSpec>,
    /// Exhaust of the spacecraft's nozzles (`None`: cold engines).
    pub thrusters: Option<Thrusters>,
    /// Shader quality policy (full by default).
    pub quality: Quality,
    /// Shaded-stage buffers reused by every draw of this scene.
    scratch: RefCell<RenderScratch>,
    /// Counters of the last `render_frame` (`last_stats`).
    stats: RefCell<FrameStats>,
    /// View test of every object (frustum culling). Off, everything is
    /// drawn and clipped by the rasterizer: the same image with more work.
    pub culling: bool,
    /// Simulated time shared by orbits, spins and material animation.
    pub clock: SimulationClock,
    /// Scale of local flight and quick travel: `None` keeps the
    /// pre-project constants (`FlightScale::legacy`), a profile makes them
    /// relative to each planet's radius.
    pub scale: Option<ScaleProfile>,
    /// Orbit layout of a scaled system (`scaled_from_spec`); `None` for the
    /// pre-project one.
    pub layout: Option<OrbitLayout>,
    /// Gravity of the bodies on the ship in space flight (`G`): on in the
    /// scaled system, off in the pre-project one (whose bodies have no
    /// `mu`).
    pub gravity: bool,
    /// Center of the orbits (the star).
    pub center: Vector3,
    /// Unit direction towards the light, world space.
    pub light_world: Vector3,
}

/// Applies the body values of `sets` to `spec` and returns the shader ones.
fn split_sets(spec: &mut SystemSpec, sets: &[(String, f32)]) -> Result<Vec<(String, f32)>, String> {
    let mut shader_sets = Vec::new();
    for (name, value) in sets {
        let consumed = spec
            .apply_set(name, *value)
            .map_err(|e| format!("--set '{name}={value}': {e}"))?;
        if !consumed {
            shader_sets.push((name.clone(), *value));
        }
    }
    Ok(shader_sets)
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
            stars: StarField::generate_with(DEFAULT_SEED, DEFAULT_STARS, StarDensity::default()),
            nebula: Nebula::new(DEFAULT_SEED, 1.0),
            plasma: PlasmaLoops::crown(),
            crown: Some(RoyalCrown::royal()),
            cards: Some(CardRing::diamonds()),
            attachments: vec![
                AttachmentSpec {
                    kind: AttachmentKind::RoyalCrown,
                    host: PlanetId(Body::KingsCrown.name()),
                },
                AttachmentSpec {
                    kind: AttachmentKind::CardRing,
                    host: PlanetId(Body::HouseOfCards.name()),
                },
            ],
            thrusters: Some(Thrusters::default()),
            scratch: RefCell::new(RenderScratch::new()),
            stats: RefCell::new(FrameStats::default()),
            culling: true,
            quality: Quality::Full,
            clock: SimulationClock::default(),
            scale: None,
            layout: None,
            gravity: false,
            center: Vector3::zero(),
            light_world,
        }
    }

    /// The default system: The King's Crown at the center (scale 1.3 of the
    /// course sphere, radius 1.13, wearing its royal crown, its loops drawn
    /// as external arcs), Crimson Entrance at scale 0.6 on a 3.0 orbit and
    /// House of Cards at scale 0.9 on a 6.2 orbit, placed for time 0. The
    /// gap between the two orbits leaves room for a ring of up to 2.2 radii
    /// around the gas giant (its diamond cards reach 2.13), about one unit
    /// clear of Crimson Entrance.
    pub fn default_system(
        meshes: SolarMeshes,
        view: OrbitView,
        light_world: Vector3,
    ) -> Result<Self, String> {
        // The star's loops are external geometry here (`plasma`): its
        // surface shader keeps the plasma, spots, corona and eclipse, and
        // stops painting loops inside the disk (`--set crown.loops=1` brings
        // them back). The values live in `SystemSpec::preproject3`.
        Self::from_spec(meshes, view, light_world, &SystemSpec::preproject3())
            .map_err(|e| e.to_string())
    }

    /// The scene described by `spec`: the star at the center, then every
    /// planet in the order of the spec (which is the draw order). An
    /// invalid spec is an error, never a partial scene.
    pub fn from_spec(
        meshes: SolarMeshes,
        view: OrbitView,
        light_world: Vector3,
        spec: &SystemSpec,
    ) -> Result<Self, SpecError> {
        let scene = Self::from_spec_unchecked(meshes, view, light_world, spec)?;
        scene.check_spawn()?;
        Ok(scene)
    }

    /// `from_spec` without the spawn check.
    fn from_spec_unchecked(
        meshes: SolarMeshes,
        view: OrbitView,
        light_world: Vector3,
        spec: &SystemSpec,
    ) -> Result<Self, SpecError> {
        spec.validate()?;
        let instance = |material: &PlanetMaterial, scale: f32, spin: f32| {
            let PlanetMaterial::Lab6(material) = *material;
            BodyInstance {
                material,
                ..BodyInstance::new(
                    material.body(),
                    Transform::new(Vector3::zero(), scale),
                    spin,
                )
            }
        };
        let mut bodies = vec![BodyInstance {
            mu: spec.star.mu,
            ..instance(&spec.star.material, spec.star.scale, spec.star.spin)
        }];
        for p in &spec.planets {
            if p.host.is_some() {
                return Err(SpecError::Unsupported("planet.host"));
            }
            let orbit = Orbit::from_linear(p.orbit_radius, p.orbit_speed, p.phase)
                .map_err(|_| SpecError::OutOfRange("planet.orbit_speed", p.orbit_speed))?;
            bodies.push(
                BodyInstance {
                    mu: p.mu,
                    ..instance(&p.material, p.scale, p.spin)
                }
                .with_orbit(orbit),
            );
        }
        let mut scene = Self::new(meshes, view, light_world);
        scene.bodies = bodies;
        scene.update();
        Ok(scene)
    }

    /// The Project 3 system (`SystemSpec::project3`) at the project scale.
    pub fn project3_system(
        meshes: SolarMeshes,
        view: OrbitView,
        light_world: Vector3,
    ) -> Result<Self, SpecError> {
        Self::scaled_from_spec(meshes, view, light_world, &SystemSpec::project3())
    }

    /// The system of `spec` at the project scale: the size ratios are
    /// checked (`ScaleProfile::check`), the orbits must keep the shells
    /// apart (`OrbitLayout::solve`, explicit radii only), the ship starts
    /// beyond the fifth slot in inertial flight at the profile's cruise
    /// speed with gravity on, and local flight scales with each planet.
    pub fn scaled_from_spec(
        meshes: SolarMeshes,
        view: OrbitView,
        light_world: Vector3,
        spec: &SystemSpec,
    ) -> Result<Self, SpecError> {
        let profile = ScaleProfile::project3();
        profile.check(spec)?;
        let mut scene = Self::from_spec_unchecked(meshes, view, light_world, spec)?;
        let eff: Vec<f32> = (1..scene.bodies.len()).map(|i| scene.shell_of(i)).collect();
        let layout = OrbitLayout::solve(spec, &eff, scene.shell_of(0), profile.min_gap)?;
        scene.ship = ShipState::new(
            spawn_beyond(layout.fifth_slot + FIFTH_SLOT_RESERVED_EXTENT),
            SHIP_SCALE,
        );
        scene.ship.move_speed = profile.cruise_speed;
        scene.ship.inertia = true;
        scene.gravity = true;
        scene.scale = Some(profile);
        scene.layout = Some(layout);
        scene.check_spawn()?;
        Ok(scene)
    }

    /// The ship's start stays clear of every shell for the whole orbit: the
    /// star's sphere and the ring each planet's shell sweeps.
    pub fn check_spawn(&self) -> Result<(), SpecError> {
        let p = self.ship.position - self.center;
        for (i, b) in self.bodies.iter().enumerate() {
            let distance = match b.orbit {
                Some(o) => (p.x.hypot(p.z) - o.radius).hypot(p.y),
                None => (self.ship.position - b.transform.position).length(),
            };
            let room = distance - self.shell_of(i) - self.ship.radius();
            if room.is_nan() || room <= 0.0 {
                return Err(SpecError::OutOfRange("ship spawn room", room));
            }
        }
        Ok(())
    }

    /// Radius of the sphere that holds body `index` with its attachments
    /// and, for the star, its plasma loops (`system::layout`).
    pub fn shell_of(&self, index: usize) -> f32 {
        let mut reach: f32 = 1.0;
        for a in &self.attachments {
            if self.body_index(a.host) == Some(index) {
                let extent = match a.kind {
                    AttachmentKind::RoyalCrown => self.crown.as_ref().map(|c| c.extent),
                    AttachmentKind::CardRing => self.cards.as_ref().map(CardRing::extent),
                };
                reach = reach.max(extent.unwrap_or(1.0));
            }
        }
        if self.bodies[index].orbit.is_none() {
            reach = reach.max(self.plasma.extent());
        }
        self.radius_of(index) * reach
    }

    /// Pull of every body with `mu` at its current position (none while
    /// gravity is off).
    pub fn gravity_sources(&self) -> Vec<GravitySource> {
        if !self.gravity {
            return Vec::new();
        }
        self.bodies
            .iter()
            .enumerate()
            .filter(|(_, b)| b.mu > 0.0)
            .map(|(i, b)| GravitySource::new(b.transform.position, b.mu, self.radius_of(i)))
            .collect()
    }

    /// Local flight and quick-travel distances around body `index`.
    pub fn flight_scale(&self, index: usize) -> FlightScale {
        match &self.scale {
            None => FlightScale::legacy(),
            Some(profile) => FlightScale::for_planet(profile, self.radius_of(index)),
        }
    }

    /// The system of `spec` with the command line's `--set` values: body
    /// values (`SPEC_PARAMS`, such as `entrance.radius`) change the spec
    /// before it is validated and built, every other parameter goes to the
    /// body's shader (`apply_overrides`). All or nothing: any error stops
    /// the start with a message naming the option. Changing a value means
    /// starting the program again; there is no reload while it runs.
    pub fn from_spec_with_sets(
        meshes: SolarMeshes,
        view: OrbitView,
        light_world: Vector3,
        mut spec: SystemSpec,
        sets: &[(String, f32)],
    ) -> Result<Self, String> {
        let shader_sets = split_sets(&mut spec, sets)?;
        let mut scene =
            Self::from_spec(meshes, view, light_world, &spec).map_err(|e| format!("--set: {e}"))?;
        scene.apply_overrides(&shader_sets)?;
        Ok(scene)
    }

    /// `from_spec_with_sets` for the Project 3 system at the project scale
    /// (`scaled_from_spec`): the layout is checked again with the values.
    pub fn project3_with_sets(
        meshes: SolarMeshes,
        view: OrbitView,
        light_world: Vector3,
        sets: &[(String, f32)],
    ) -> Result<Self, String> {
        let mut spec = SystemSpec::project3();
        let shader_sets = split_sets(&mut spec, sets)?;
        let mut scene = Self::scaled_from_spec(meshes, view, light_world, &spec)
            .map_err(|e| format!("--set: {e}"))?;
        scene.apply_overrides(&shader_sets)?;
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
        let scale = self.flight_scale(planet);
        local_step_scaled(
            &mut self.ship,
            &mut self.flight,
            position,
            radius,
            input,
            dt,
            &scale,
        );
    }

    /// `L`: toggles local flight; the ship is never moved by the toggle.
    pub fn toggle_local(&mut self) -> Result<FlightMode, String> {
        self.flight = toggle_local_scaled(self.flight, &self.ship, &self.local_targets(), |t| {
            self.flight_scale(t.index)
        })?;
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
        self.light_at(self.ship.position)
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

    /// The body the plasma loops are anchored to: The King's Crown, when it
    /// is in the scene and the scene has arcs.
    pub fn plasma_host(&self) -> Option<usize> {
        if self.plasma.arcs.is_empty() {
            return None;
        }
        self.body_index(PlanetId(Body::KingsCrown.name()))
    }

    /// The host of the loops when the star with its loops (a sphere of
    /// `PlasmaLoops::extent` radii) can appear on the framebuffer.
    fn plasma_in_view(&self, width: u32, height: u32) -> Option<usize> {
        self.plasma_host().filter(|&i| {
            let reach = self.radius_of(i) * self.plasma.extent();
            self.sphere_in_view(self.bodies[i].transform.position, reach, width, height)
        })
    }

    /// Draws the star's external plasma loops without clearing: one tube
    /// per arc with the star's transform, so they turn with it and meet the
    /// bodies in the shared depth buffer. Returns the stats of each arc
    /// (none when the star and its loops are out of view).
    pub fn render_plasma_without_clear(
        &self,
        framebuffer: &mut Framebuffer,
        mode: TriangleMode,
        options: ShadedOptions,
    ) -> Result<Vec<ShadedStats>, RenderError> {
        let Some(host) = self.plasma_in_view(framebuffer.width(), framebuffer.height()) else {
            return Ok(Vec::new());
        };
        let transform = self.bodies[host].transform;
        let model = Uniforms {
            model_matrix: transform.matrix(),
        };
        let projection = self.projection();
        let mut scratch = self.scratch.borrow_mut();
        self.plasma
            .arcs
            .iter()
            .enumerate()
            .map(|(i, arc)| {
                let shader = arc.shader(self.clock.time, SPHERE_RADIUS);
                render_shaded_with_scratch(
                    framebuffer,
                    &model,
                    self.plasma.arc_mesh(i),
                    &projection,
                    mode,
                    plasma_options(options.threads),
                    &|s, v| {
                        shader.shade(s.object_position, transform.to_object_dir(v.view_direction))
                    },
                    &mut scratch,
                )
            })
            .collect()
    }

    /// One frame: clear color and depth once, the star field (color only),
    /// then every body, the star's plasma loops, the attached effects and
    /// the spacecraft, all through the same camera and depth buffer.
    /// Returns the fragments shaded.
    ///
    /// Filled modes run as one frame batch: every visible instance is
    /// rasterized first (depth written at once), all samples are shaded by a
    /// single thread pool and the colors are resolved by depth. The image is
    /// identical to drawing the instances one by one
    /// (`render_objects_without_clear`, `render_plasma_without_clear`,
    /// `render_effects_without_clear` and `render_ship_without_clear`), but
    /// idle cores at the end of each instance are avoided. Wireframe draws
    /// the instances one by one.
    pub fn render_frame(
        &self,
        framebuffer: &mut Framebuffer,
        mode: TriangleMode,
        options: ShadedOptions,
    ) -> Result<usize, RenderError> {
        framebuffer.clear();
        let sky = self.sky_camera();
        draw_nebula(framebuffer, &self.nebula, &sky);
        draw_stars(framebuffer, &self.stars, &sky);
        let (w, h) = (framebuffer.width(), framebuffer.height());
        let mut stats = self.stats.borrow_mut();
        stats.clear();
        if mode == TriangleMode::Wireframe {
            let bodies = self.render_objects_without_clear(framebuffer, mode, options)?;
            let plasma = self.render_plasma_without_clear(framebuffer, mode, options)?;
            let effects = self.render_effects_without_clear(framebuffer, mode, options)?;
            let ship = self.render_ship_without_clear(framebuffer, mode, options)?;
            for (i, s) in bodies.iter().enumerate() {
                stats.body(i, self.body_in_view(i, w, h), s);
            }
            if self.plasma_host().is_some() {
                stats.test(!plasma.is_empty());
            }
            plasma.iter().chain(&effects).for_each(|s| stats.add(s));
            let ship_visible = self.sphere_in_view(self.ship.position, self.ship.radius(), w, h);
            stats.test(ship_visible);
            if ship_visible {
                stats.add(&ship);
            }
            return Ok(bodies
                .iter()
                .chain(&plasma)
                .chain(&effects)
                .map(|s| s.fragments_shaded)
                .sum::<usize>()
                + ship.fragments_shaded);
        }
        let shared = self.frame_context(mode, options);
        let mut scratch = self.scratch.borrow_mut();
        scratch.begin_batch();
        let effects = self.effect_draws(w, h, options.threads);
        let mut jobs: Vec<Job> =
            Vec::with_capacity(self.bodies.len() + self.plasma.arcs.len() + effects.len() + 1);
        for (i, body) in self.bodies.iter().enumerate() {
            if !self.body_in_view(i, w, h) {
                stats.body(i, false, &ShadedStats::default());
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
            let s = scratch.rasterize(
                framebuffer,
                &model,
                &self.meshes.sphere,
                &shared.projection,
                mode,
                options,
                tag,
            )?;
            stats.body(i, true, &s);
        }
        let plasma = self.plasma_in_view(w, h);
        if self.plasma_host().is_some() {
            stats.test(plasma.is_some());
        }
        if let Some(host) = plasma {
            let transform = self.bodies[host].transform;
            let model = Uniforms {
                model_matrix: transform.matrix(),
            };
            for (i, arc) in self.plasma.arcs.iter().enumerate() {
                jobs.push(Job::Plasma(
                    arc.shader(shared.time, SPHERE_RADIUS),
                    transform,
                ));
                let tag = (jobs.len() - 1) as u32;
                stats.add(&scratch.rasterize(
                    framebuffer,
                    &model,
                    self.plasma.arc_mesh(i),
                    &shared.projection,
                    mode,
                    plasma_options(options.threads),
                    tag,
                )?);
            }
        }
        for draw in &effects {
            jobs.push(Job::Effect(draw.shader));
            let tag = (jobs.len() - 1) as u32;
            stats.add(&scratch.rasterize(
                framebuffer,
                &draw.model,
                draw.mesh,
                &shared.projection,
                mode,
                draw.options,
                tag,
            )?);
        }
        let ship_visible = self.sphere_in_view(self.ship.position, self.ship.radius(), w, h);
        stats.test(ship_visible);
        if ship_visible {
            jobs.push(Job::Ship(ship_light(&self.ship, self.ship_light())));
            let model = Uniforms {
                model_matrix: self.ship.model_matrix(),
            };
            let tag = (jobs.len() - 1) as u32;
            stats.add(&scratch.rasterize(
                framebuffer,
                &model,
                &self.meshes.ship,
                &shared.projection,
                mode,
                ship_options(options.threads),
                tag,
            )?);
        }
        scratch.shade_batch(options.threads, |tag, s, v| match &jobs[tag as usize] {
            Job::Body(prepared, transform) => shade_instance(prepared, transform, s, v),
            Job::Plasma(arc, transform) => {
                arc.shade(s.object_position, transform.to_object_dir(v.view_direction))
            }
            Job::Effect(effect) => effect.shade(s, v),
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
        self.light_at(body.transform.position)
    }

    /// Unit direction from world point `p` towards the star (the scene
    /// light at the center itself).
    pub fn light_at(&self, p: Vector3) -> Vector3 {
        let to_star = self.center - p;
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
    /// rectangle under the orthographic orbit camera). Always true with
    /// `culling` off.
    pub fn sphere_in_view(&self, center: Vector3, radius: f32, width: u32, height: u32) -> bool {
        if !self.culling {
            return true;
        }
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

    /// Index of the body with stable id `id`.
    pub fn body_index(&self, id: PlanetId) -> Option<usize> {
        self.bodies.iter().position(|b| b.body().name() == id.0)
    }

    /// Pose of the body with id `id` this frame: its transform (center,
    /// spin, mesh scale) and radius.
    pub fn host_pose(&self, id: PlanetId) -> Option<HostPose> {
        let i = self.body_index(id)?;
        Some(HostPose {
            transform: self.bodies[i].transform,
            radius: self.radius_of(i),
        })
    }

    /// Host index of the first attachment of `kind`.
    pub fn attachment_host(&self, kind: AttachmentKind) -> Option<usize> {
        let spec = self.attachments.iter().find(|a| a.kind == kind)?;
        self.body_index(spec.host)
    }

    /// Whether body `index` passes the view test.
    pub fn body_in_view(&self, index: usize, width: u32, height: u32) -> bool {
        let b = &self.bodies[index];
        self.sphere_in_view(b.transform.position, self.radius_of(index), width, height)
    }

    /// The shaded-stage buffers of this scene, for draws in other modules.
    pub(crate) fn scratch_mut(&self) -> RefMut<'_, RenderScratch> {
        self.scratch.borrow_mut()
    }

    /// Counters of the last `render_frame`.
    pub fn last_stats(&self) -> Ref<'_, FrameStats> {
        self.stats.borrow()
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
    /// Boxed: a prepared shader is large and a frame has only a few jobs
    /// (at most the bodies, the star's arcs, the effect draws and the
    /// spacecraft).
    Body(Box<PreparedShader>, Transform),
    /// One plasma arc and the transform of the star it rises from.
    Plasma(ArcShader, Transform),
    /// One draw of an attached effect (`effects`).
    Effect(EffectShader),
    Ship(DirectionalLight),
}
