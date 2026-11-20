//! Project 3 planets: material identity of the solar system, independent of
//! the laboratory 6 `celestial::Body` enum (which keeps its own CLI and
//! viewer).

pub mod clover;
pub mod field;
pub mod geometry;
pub mod icosphere;
pub mod material;

pub use clover::CloverField;
pub use field::{HeightField, TestField};
pub use material::{PlanetMaterial, PreparedPlanet};
