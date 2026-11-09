//! Camino al Proyecto 3: the solar scene that composes the laboratory six
//! bodies and the spacecraft on the shared CPU pipeline.

pub mod camera;
pub mod clock;
pub mod flight;
pub mod input;
pub mod instance;
pub mod orbit;
pub mod record;
pub mod resources;
pub mod scene;
pub mod ship;
pub mod skybox;
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
