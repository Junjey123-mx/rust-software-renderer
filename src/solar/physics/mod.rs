//! Spacecraft physics of Project 3: fixed-step integration of the inertial
//! flight and the planets' gravity.

pub mod gravity;
pub mod motion;

pub use gravity::{GravitySource, design_mu, gravity_accel};
pub use motion::{MAX_SUBSTEPS, PHYSICS_STEP, advance};
