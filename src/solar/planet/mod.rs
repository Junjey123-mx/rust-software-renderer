//! Project 3 planets: material identity of the solar system, independent of
//! the laboratory 6 `celestial::Body` enum (which keeps its own CLI and
//! viewer).

pub mod ace;
pub mod clover;
pub mod field;
pub mod geometry;
pub mod hearts;
pub mod icosphere;
pub mod joker;
pub mod material;
pub mod normal_map;

pub use ace::AceField;
pub use clover::CloverField;
pub use field::{HeightField, TestField};
pub use hearts::HeartsField;
pub use joker::JokerField;
pub use material::{PlanetMaterial, PreparedPlanet};
