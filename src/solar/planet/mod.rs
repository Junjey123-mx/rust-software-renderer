//! Project 3 planets: material identity of the solar system, independent of
//! the laboratory 6 `celestial::Body` enum (which keeps its own CLI and
//! viewer).

pub mod field;
pub mod icosphere;
pub mod material;

pub use field::{HeightField, TestField};
pub use material::{PlanetMaterial, PreparedPlanet};
