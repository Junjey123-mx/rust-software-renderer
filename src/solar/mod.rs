//! Camino al Proyecto 3: the solar scene that composes the laboratory six
//! bodies and the spacecraft on the shared CPU pipeline.

pub mod clock;
pub mod instance;
pub mod resources;
pub mod scene;
pub mod viewer;

pub use clock::{MAX_STEP, SimulationClock};
pub use instance::{BodyInstance, FrameContext, instance_uniforms, render_instance};
pub use resources::{SPHERE_RADIUS, SolarMeshes};
pub use scene::SolarScene;
