//! Room each body takes in the system: a sphere around its center that
//! holds its surface with the tallest relief and every attachment.

use crate::solar::SPHERE_RADIUS;
use crate::solar::attach::{Attachment, max_extent};
use crate::solar::plasma_loops::PlasmaLoops;
use crate::solar::system::{GeometrySpec, PlanetSpec, SpecError, StarSpec};

/// Reach reserved for the fifth planet's slot, in its radii (design value
/// of document B; the planet itself waits for artistic approval, C70).
pub const FIFTH_SLOT_RESERVED_EXTENT: f32 = 1.95;

/// Tallest relief of a geometry over its radius, in radii.
fn max_relief(geometry: GeometrySpec) -> f32 {
    match geometry {
        GeometrySpec::Sphere => 0.0,
    }
}

/// World radius of the sphere that holds the planet and its attachments.
pub fn effective_radius(spec: &PlanetSpec, attachments: &[Attachment]) -> Result<f32, SpecError> {
    let surface = 1.0 + max_relief(spec.geometry);
    Ok(spec.radius() * surface.max(max_extent(attachments)?))
}

/// World radius of the sphere that holds the star, its plasma loops (their
/// mesh does not move, so the reach is fixed) and its attachments.
pub fn star_shell(
    spec: &StarSpec,
    attachments: &[Attachment],
    loops: &PlasmaLoops,
) -> Result<f32, SpecError> {
    let reach = loops.extent().max(max_extent(attachments)?);
    Ok(spec.scale * SPHERE_RADIUS * reach)
}
