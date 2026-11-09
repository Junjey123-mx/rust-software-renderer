//! Camino al Proyecto 3: the solar scene that composes the laboratory six
//! bodies and the spacecraft on the shared CPU pipeline.

pub mod clock;
pub mod instance;
pub mod orbit;
pub mod resources;
pub mod scene;
pub mod ship;
pub mod viewer;

pub use clock::{MAX_STEP, SimulationClock};
pub use instance::{BodyInstance, FrameContext, eye_towards, instance_uniforms, render_instance};
pub use orbit::Orbit;
pub use resources::{SPHERE_RADIUS, SolarMeshes};
pub use scene::{CameraMode, FOV_Y, NEAR, SolarScene};
pub use ship::ShipState;
