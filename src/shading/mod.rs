//! Procedural shading toolkit (laboratory 6).
//!
//! Pure, allocation-free math shared by every celestial body and reusable by
//! the final project: palettes and ramps, seeded 3D gradient noise, event
//! lifecycles, signed distance fields, sphere frames and lighting models.
//! Nothing here opens a window or knows about a specific body.

pub mod cycle;
pub mod math;
pub mod noise;
pub mod sdf;
pub mod sphere;
