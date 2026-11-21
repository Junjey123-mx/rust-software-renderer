//! Camino al Proyecto 3: the solar scene that composes the laboratory six
//! bodies and the spacecraft on the shared CPU pipeline.

pub mod atmosphere;
pub mod attach;
pub mod audio;
pub mod camera;
pub mod capture;
pub mod card_ring;
pub mod clock;
pub mod comets;
pub mod effects;
pub mod flight;
pub mod hud;
pub mod input;
pub mod instance;
pub mod nebula;
pub mod orbit;
pub mod physics;
pub mod planet;
pub mod plasma_loops;
pub mod record;
pub mod resources;
pub mod royal_crown;
pub mod scene;
pub mod shapes;
pub mod ship;
pub mod ship_lights;
pub mod skybox;
pub mod stats;
pub mod system;
pub mod thrusters;
pub mod travel;
pub mod viewer;

pub use clock::{MAX_STEP, SimulationClock};
pub use instance::{
    BodyInstance, FrameContext, Quality, adapted_material, eye_towards, instance_uniforms,
    prepare_instance, projected_radius_px, render_instance, render_instance_scratch,
    shade_instance,
};
pub use orbit::Orbit;
pub use resources::{SPHERE_RADIUS, SolarMeshes};
pub use scene::{CameraMode, FOV_Y, NEAR, SolarScene};
pub use ship::{ShipState, render_ship, render_ship_scratch};
pub use stats::{BodyStats, FrameStats};
