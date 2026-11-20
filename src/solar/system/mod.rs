//! Project 3 system description: validated data for the star and every
//! planet (identity, orbit, spin, seed, material, geometry and gravity).

pub mod spec;

pub use spec::{
    AttachmentSpec, GeometrySpec, PlanetId, PlanetSpec, SPEC_PARAMS, SpecError, StarSpec,
    SystemSpec,
};
