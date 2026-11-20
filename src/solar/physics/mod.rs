//! Spacecraft physics of Project 3: fixed-step integration of the inertial
//! flight.

pub mod motion;

pub use motion::{MAX_SUBSTEPS, PHYSICS_STEP, advance};
