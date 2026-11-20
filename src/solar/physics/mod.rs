//! Spacecraft physics of Project 3: fixed-step integration of the inertial
//! flight, the planets' gravity and collisions with the bodies.

pub mod collision;
pub mod gravity;
pub mod motion;

pub use collision::{
    COLLISION_MARGIN, CollisionShape, Contact, SurfaceSampler, collide, collide_with, lift_above,
    resolve, sweep,
};
pub use gravity::{GravitySource, design_mu, gravity_accel};
pub use motion::{MAX_SUBSTEPS, PHYSICS_STEP, advance};
