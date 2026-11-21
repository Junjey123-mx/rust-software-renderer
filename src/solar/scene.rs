//! The solar scene: shared meshes, a flat list of body instances and the
//! values every instance shares in a frame.
//!
//! Frame contract: the owner of the framebuffer clears it exactly once per
//! frame; `render_objects_without_clear` never clears color or depth, so the
//! shared depth buffer resolves occlusion between instances whatever their
//! draw order.

use crate::celestial::Body;
use crate::geometry::Vertex;
use crate::pipeline::{
    DirectionalLight, OrbitView, PerspectiveCamera, RenderError, RenderScratch, ShadedOptions,
    ShadedStats, Transform, Uniforms, ViewProjection, render_shaded_with_scratch, sphere_visible,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::solar::attach::landmark::Landmark;
use crate::solar::attach::props::{PROP_SIZE, PropLayers, ScatterRule, bake, scatter};
use crate::solar::attach::traffic::{NEON_SIZE, Traffic};
use crate::solar::attach::waterfalls::WaterfallSet;
use crate::solar::attach::{AttachmentKind, AttachmentSpec, HostPose};
use crate::solar::camera::{CameraRig, FollowRig, orbit_perspective};
use crate::solar::card_ring::CardRing;
use crate::solar::effects::EffectShader;
use crate::solar::flight::{
    FlightInput, FlightMode, FlightScale, LocalTarget, local_step_over, local_step_scaled,
    toggle_local_scaled,
};
use crate::solar::instance::{
    Quality, adapted_material, prepare_instance, projected_radius_px, render_instance_scratch,
};
use crate::solar::nebula::{Nebula, draw_nebula};
use crate::solar::physics::{CollisionShape, GravitySource};
use crate::solar::planet::clover::CloverParams;
use crate::solar::planet::geometry::{LodState, Relief, lod_for, lod_step};
use crate::solar::planet::hearts::NeonGridParams;
use crate::solar::planet::joker::JokerParams;
use crate::solar::planet::{HeightField, PlanetMaterial, PreparedPlanet};
use crate::solar::plasma_loops::{ArcShader, PlasmaLoops, plasma_options};
use crate::solar::royal_crown::RoyalCrown;
use crate::solar::ship::MESH_RADIUS;
use crate::solar::ship::{render_ship_scratch, ship_color, ship_light, ship_options};
use crate::solar::skybox::{DEFAULT_SEED, DEFAULT_STARS, StarDensity, StarField, draw_stars};
use crate::solar::stats::FrameStats;
use crate::solar::system::layout::{FIFTH_SLOT_RESERVED_EXTENT, OrbitLayout};
use crate::solar::system::spec::AttachmentSpec as PlanetAttachment;
use crate::solar::system::{PlanetId, ScaleProfile, SpecError, SystemSpec};
use crate::solar::thrusters::Thrusters;
use crate::solar::{
    BodyInstance, FrameContext, Orbit, SPHERE_RADIUS, ShipState, SimulationClock, SolarMeshes,
};
use raylib::math::Vector3;
use std::cell::{Ref, RefCell, RefMut};

/// Where the Hearts pyramid stands, the planet's own frame (moved to the
/// nearest plateau): high on its northern side, in view from the system
/// camera.
pub const HEARTS_LANDMARK: Vector3 = Vector3::new(0.3, 0.8, 0.5);
/// Angle around the pyramid's foot kept free of buildings, radians (its
/// base is about 0.23 rad wide).
pub const CITY_CLEARANCE: f32 = 0.3;
/// Size of a Hearts building, planet radii.
pub const BUILDING_SIZE: f32 = 0.06;
/// Angle around the pyramid's foot no ship route crosses, radians (the
/// pyramid, its disc and a ship's length).
pub const TRAFFIC_CLEARANCE: f32 = 0.4;

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
    /// Surface props baked into sectors (`None`: none), drawn on the host
    /// an `AttachmentKind::Props` names.
    pub props: Option<PropLayers>,
    /// Waterfalls of the clover planet (`None`: none), drawn on the host an
    /// `AttachmentKind::Waterfalls` names.
    pub waterfalls: Option<WaterfallSet>,
    /// The Hearts pyramid (`None`: none), drawn on the host an
    /// `AttachmentKind::Landmark` names.
    pub landmark: Option<Landmark>,
    /// The Hearts city baked into sectors (`None`: none), drawn on the host
    /// an `AttachmentKind::City` names.
    pub city: Option<PropLayers>,
    /// Neon ships of the planet an `AttachmentKind::Traffic` names
    /// (`None`: none).
    pub traffic: Option<Traffic>,
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
    /// Relief geometry by body index (`set_relief`); a body without an
    /// entry is drawn with the course sphere.
    pub relief: Vec<Option<Relief>>,
    /// Level of detail with hysteresis (`lod_step`, the live viewer); off,
    /// each frame takes `lod_for` of its own sizes (recording, benchmark).
    pub lod_live: bool,
    /// Level of each body across frames when `lod_live` is on.
    lod_states: RefCell<Vec<LodState>>,
    /// One level for every relief body, whatever its size (benchmark A/B).
    pub lod_force: Option<u8>,
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
            props: None,
            waterfalls: None,
            landmark: None,
            city: None,
            traffic: None,
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
            relief: Vec::new(),
            lod_live: false,
            lod_states: RefCell::new(Vec::new()),
            lod_force: None,
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
        let instance = |material: &PlanetMaterial, scale: f32, spin: f32| -> BodyInstance {
            let transform = Transform::new(Vector3::zero(), scale);
            match *material {
                PlanetMaterial::Lab6(material) => BodyInstance {
                    material,
                    ..BodyInstance::new(material.body(), transform, spin)
                },
                // A Project 3 surface: the laboratory 6 material underneath
                // is never drawn (`surface` is), it only fills the slot.
                surface => BodyInstance {
                    surface: Some(surface),
                    ..BodyInstance::new(Body::CrimsonEntrance, transform, spin)
                },
            }
        };
        let mut bodies = vec![BodyInstance {
            mu: spec.star.mu,
            id: spec.star.id,
            name: spec.star.name,
            ..instance(&spec.star.material, spec.star.scale, spec.star.spin)
        }];
        for p in &spec.planets {
            if p.host.is_some() {
                return Err(SpecError::Unsupported("planet.host"));
            }
            let orbit = Orbit::from_linear(p.orbit_radius, p.orbit_speed, p.phase)
                .map_err(|_| SpecError::OutOfRange("planet.orbit_speed", p.orbit_speed))?;
            // The clover's and the Hearts' fields follow the planet seed.
            let material = match p.material {
                PlanetMaterial::Clover(params) => PlanetMaterial::Clover(CloverParams {
                    seed: p.seed as f32,
                    ..params
                }),
                PlanetMaterial::Hearts(params) => PlanetMaterial::Hearts(NeonGridParams {
                    seed: p.seed as f32,
                    ..params
                }),
                PlanetMaterial::Joker(params) => PlanetMaterial::Joker(JokerParams {
                    seed: p.seed as f32,
                    ..params
                }),
                other => other,
            };
            bodies.push(
                BodyInstance {
                    mu: p.mu,
                    id: p.id,
                    name: p.name,
                    ..instance(&material, p.scale, p.spin)
                }
                .with_orbit(orbit),
            );
        }
        let mut scene = Self::new(meshes, view, light_world);
        scene.bodies = bodies;
        for i in 0..scene.bodies.len() {
            match scene.bodies[i].surface {
                Some(PlanetMaterial::Clover(params)) => scene.add_clover_surface(i, &params)?,
                Some(PlanetMaterial::Hearts(params)) => scene.add_hearts_surface(i, &params)?,
                Some(PlanetMaterial::Joker(params)) => scene
                    .set_relief(i, HeightField::Joker(params.field()))
                    .map_err(|_| SpecError::Unsupported("joker relief"))?,
                _ => {}
            }
        }
        for (k, p) in spec.planets.iter().enumerate() {
            for a in &p.attachments {
                let PlanetAttachment::Traffic {
                    ships,
                    cycles,
                    seed,
                } = *a;
                if ships > 0 || cycles > 0 {
                    scene.add_traffic(k + 1, ships, cycles, seed);
                }
            }
        }
        scene.update();
        Ok(scene)
    }

    /// Relief, trees and rocks of the clover planet at body `index`, built
    /// once here (never while drawing).
    fn add_clover_surface(&mut self, index: usize, params: &CloverParams) -> Result<(), SpecError> {
        let field = HeightField::Clover(params.field());
        self.set_relief(index, field)
            .map_err(|_| SpecError::Unsupported("clover relief"))?;
        let seed = params.field().seed;
        let mut poses = scatter(&field, &ScatterRule::TREES, seed);
        poses.extend(scatter(&field, &ScatterRule::ROCKS, seed ^ 0x5eed));
        self.props = Some(bake(
            &poses,
            &field,
            SPHERE_RADIUS,
            PROP_SIZE * SPHERE_RADIUS,
            seed,
        ));
        self.attachments.push(AttachmentSpec {
            kind: AttachmentKind::Props,
            host: self.bodies[index].id,
        });
        let per_mountain = params.waterfalls.round().clamp(0.0, 1.0) as u8;
        if per_mountain > 0 {
            if let HeightField::Clover(clover) = field {
                self.waterfalls = Some(WaterfallSet::for_clover(
                    &clover,
                    per_mountain,
                    seed,
                    SPHERE_RADIUS,
                ));
            }
            self.attachments.push(AttachmentSpec {
                kind: AttachmentKind::Waterfalls,
                host: self.bodies[index].id,
            });
        }
        Ok(())
    }

    /// Terraces, pyramid and city of the Hearts planet at body `index`,
    /// built once here (never while drawing). The pyramid stands on the
    /// plateau nearest `HEARTS_LANDMARK`; no building rises within
    /// `CITY_CLEARANCE` of it.
    fn add_hearts_surface(
        &mut self,
        index: usize,
        params: &NeonGridParams,
    ) -> Result<(), SpecError> {
        let field = HeightField::Hearts(params.field());
        self.set_relief(index, field)
            .map_err(|_| SpecError::Unsupported("hearts relief"))?;
        let host = self.bodies[index].id;
        let landmark = Landmark::pyramid(&field, HEARTS_LANDMARK, SPHERE_RADIUS)
            .map_err(|_| SpecError::Unsupported("hearts landmark"))?;
        let rule = ScatterRule {
            exclude: Some((landmark.dir, CITY_CLEARANCE)),
            ..ScatterRule::BUILDINGS
        };
        let seed = params.field().seed;
        self.city = Some(bake(
            &scatter(&field, &rule, seed),
            &field,
            SPHERE_RADIUS,
            BUILDING_SIZE * SPHERE_RADIUS,
            seed,
        ));
        self.landmark = Some(landmark);
        for kind in [AttachmentKind::City, AttachmentKind::Landmark] {
            self.attachments.push(AttachmentSpec { kind, host });
        }
        Ok(())
    }

    /// `ships` neon ships over body `index`, `NEON_SIZE` of the spacecraft,
    /// and `cycles` lightcycles on its ground, every route clear of its
    /// landmark and the cycles clear of its city.
    fn add_traffic(&mut self, index: usize, ships: u8, cycles: u8, seed: u32) {
        let host = self.bodies[index].id;
        let on_host = |kind| {
            self.attachments
                .iter()
                .any(|a| a.kind == kind && a.host == host)
        };
        let sites = match &self.city {
            Some(city) if on_host(AttachmentKind::City) => city.sites.clone(),
            _ => Vec::new(),
        };
        let field = self.relief_of(index).map_or(HeightField::Flat, |r| r.field);
        let landmark_here = on_host(AttachmentKind::Landmark);
        let exclude = self
            .landmark
            .as_ref()
            .filter(|_| landmark_here)
            .map(|l| (l.dir, TRAFFIC_CLEARANCE));
        let ship_radius = NEON_SIZE * MESH_RADIUS * SHIP_SCALE;
        self.traffic = Some(
            Traffic::new(
                &self.meshes,
                field,
                ships,
                seed,
                ship_radius,
                self.radius_of(index),
                exclude,
            )
            .with_cycles(cycles, seed, &sites, exclude),
        );
        self.attachments.push(AttachmentSpec {
            kind: AttachmentKind::Traffic,
            host,
        });
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
        // The Project 3 sky cycles from purple to blue and back.
        scene.nebula.drift = 1.0;
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
                    AttachmentKind::Props => self
                        .props
                        .as_ref()
                        .map(|p| crate::solar::attach::Attachment::Props(p).extent()),
                    AttachmentKind::Traffic => self.traffic.as_ref().map(|t| t.reach),
                    AttachmentKind::City => self
                        .city
                        .as_ref()
                        .map(|p| crate::solar::attach::Attachment::Props(p).extent()),
                    AttachmentKind::Waterfalls => self
                        .waterfalls
                        .as_ref()
                        .map(|w| crate::solar::attach::Attachment::Waterfalls(w).extent()),
                    AttachmentKind::Landmark => self
                        .landmark
                        .as_ref()
                        .map(|l| crate::solar::attach::Attachment::Landmark(l).extent()),
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

    /// World velocity of body `index` now: along its orbit at the clock's
    /// speed, zero while paused or without an orbit.
    pub fn body_velocity(&self, index: usize) -> Vector3 {
        match self.bodies[index].orbit {
            Some(o) if !self.clock.paused => {
                let r = self.bodies[index].transform.position - self.center;
                r.cross(Vector3::new(0.0, o.speed * self.clock.speed, 0.0))
            }
            _ => Vector3::zero(),
        }
    }

    /// Every body (the star too) as a collision sphere for the ship, for
    /// the frame the clock just advanced. The clock already placed the
    /// bodies at the frame's end and `physics::collide` moves each shape by
    /// its velocity over the frame's substeps, so a shape starts where its
    /// body was at the frame's start (`real_delta` earlier): starting at the
    /// end position would lead the body by one frame of its orbit (0.035 u
    /// inside a planet at 10 FPS before this fix).
    pub fn collision_shapes(&self) -> Vec<CollisionShape> {
        (0..self.bodies.len())
            .map(|i| {
                // A relief body collides at its highest ground in space
                // flight (local flight follows the ground itself).
                let relief = self.relief_of(i).map_or(0.0, |r| r.field.max_height());
                let velocity = self.body_velocity(i);
                CollisionShape::new(
                    self.bodies[i].transform.position - velocity * self.clock.real_delta,
                    self.radius_of(i) * (1.0 + relief),
                    self.ship.radius(),
                    velocity,
                )
            })
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
        let transform = self.bodies[planet].transform;
        match self.relief_of(planet).map(|r| r.field) {
            // Over relief the clearance is kept above the ground below the
            // ship (read in the planet's turning object space).
            Some(field) => local_step_over(
                &mut self.ship,
                &mut self.flight,
                position,
                radius,
                input,
                dt,
                &scale,
                |d| radius * field.surface_radius(transform.to_object_dir(d).normalized()),
            ),
            None => local_step_scaled(
                &mut self.ship,
                &mut self.flight,
                position,
                radius,
                input,
                dt,
                &scale,
            ),
        }
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
        self.nebula.time = time;
        for body in &mut self.bodies {
            body.update(time, center);
        }
        if let Some(traffic) = &mut self.traffic {
            traffic.update_trails(time);
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
            let prepared = match (body.surface, self.relief_of(i)) {
                (Some(surface), _) => surface
                    .adapted(self.quality, self.radius_px(i, &shared.projection, h))
                    .prepare(&drawn, &frame),
                (None, Some(r)) => PreparedPlanet::Relief {
                    base: prepare_instance(&drawn, &frame),
                    field: r.field,
                    bump: r.bump,
                },
                (None, None) => PreparedPlanet::Lab6(prepare_instance(&drawn, &frame)),
            };
            jobs.push(Job::Body(Box::new(prepared), drawn.transform));
            let model = Uniforms {
                model_matrix: drawn.transform.matrix(),
            };
            let tag = (jobs.len() - 1) as u32;
            let (mesh, lod) = self.body_mesh(i, &shared.projection, h);
            let s = scratch.rasterize(
                framebuffer,
                &model,
                mesh,
                &shared.projection,
                mode,
                options,
                tag,
            )?;
            stats.body_lod(i, true, &s, lod);
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
            jobs.push(Job::Effect(draw.shader.clone()));
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
            Job::Body(prepared, transform) => prepared.shade(transform, s, v),
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
        let radius_px = self.radius_px(index, projection, height);
        BodyInstance {
            material: adapted_material(&body.material, self.quality, radius_px),
            ..*body
        }
    }

    /// Projected radius of body `index`, in pixels.
    pub(crate) fn radius_px(&self, index: usize, projection: &ViewProjection, height: u32) -> f32 {
        projected_radius_px(
            projection,
            self.bodies[index].transform.position,
            self.radius_of(index),
            height,
        )
    }

    /// Gives body `index` the relief of `field`, built now (or shared from
    /// the cache of `meshes`) at unit scale; `Flat` goes back to the sphere.
    pub fn set_relief(&mut self, index: usize, field: HeightField) -> Result<(), String> {
        if index >= self.bodies.len() {
            return Err(format!("no body {index}"));
        }
        if self.relief.len() <= index {
            self.relief.resize(index + 1, None);
        }
        self.relief[index] = match field {
            HeightField::Flat => None,
            _ => Some(Relief {
                field,
                lods: self.meshes.relief.get_or_build(SPHERE_RADIUS, &field)?,
                bump: 1.0,
            }),
        };
        Ok(())
    }

    /// Relief of body `index`, if it has one.
    pub fn relief_of(&self, index: usize) -> Option<&Relief> {
        self.relief.get(index).and_then(Option::as_ref)
    }

    /// Level of detail of body `index` at `px` pixels: `None` for a sphere
    /// body. With `lod_live` it moves that body's hysteresis state.
    pub fn lod_level(&self, index: usize, px: f32) -> Option<u8> {
        self.relief_of(index)?;
        if let Some(level) = self.lod_force {
            return Some(level);
        }
        if !self.lod_live {
            return Some(lod_for(px));
        }
        let mut states = self.lod_states.borrow_mut();
        if states.len() < self.bodies.len() {
            states.resize(self.bodies.len(), LodState::default());
        }
        Some(lod_step(&mut states[index], px))
    }

    /// Mesh of body `index` this frame and its level: the relief level, or
    /// the sphere (no level).
    fn body_mesh(
        &self,
        index: usize,
        projection: &ViewProjection,
        height: u32,
    ) -> (&[Vertex], Option<u8>) {
        let px = self.radius_px(index, projection, height);
        match (self.relief_of(index), self.lod_level(index, px)) {
            (Some(relief), Some(level)) => (relief.mesh(level), Some(level)),
            _ => (&self.meshes.sphere, None),
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
        self.bodies.iter().position(|b| b.id == id)
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
        let relief = self.relief_of(index).map_or(0.0, |r| r.field.max_height());
        self.sphere_in_view(
            b.transform.position,
            self.radius_of(index) * (1.0 + relief),
            width,
            height,
        )
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
                    self.body_mesh(i, &shared.projection, h).0,
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
    Body(Box<PreparedPlanet>, Transform),
    /// One plasma arc and the transform of the star it rises from.
    Plasma(ArcShader, Transform),
    /// One draw of an attached effect (`effects`).
    Effect(EffectShader),
    Ship(DirectionalLight),
}
