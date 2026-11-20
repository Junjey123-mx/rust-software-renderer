//! Room each body takes in the system (a sphere around its center that
//! holds its surface with the tallest relief and every attachment) and the
//! orbit radii that keep those shells apart.

use crate::solar::SPHERE_RADIUS;
use crate::solar::attach::{Attachment, max_extent};
use crate::solar::plasma_loops::PlasmaLoops;
use crate::solar::system::spec::AUTO_ORBIT;
use crate::solar::system::{GeometrySpec, PlanetSpec, SpecError, StarSpec, SystemSpec};

/// Effective radius reserved for the fifth planet's slot, world units
/// (design value of document B; the planet itself waits for artistic
/// approval, C70).
pub const FIFTH_SLOT_RESERVED_EXTENT: f32 = 1.95;

/// Tallest relief of a geometry over its radius, in radii.
fn max_relief(geometry: GeometrySpec) -> f32 {
    match geometry {
        GeometrySpec::Sphere => 0.0,
        // The tallest clover peak any seed can raise, and a tree on it.
        GeometrySpec::Clover => {
            crate::solar::planet::clover::CLOVER_PEAK_BOUND
                + 1.2 * crate::solar::attach::props::PROP_SIZE
        }
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

/// Orbit radii of a system whose shells never meet.
#[derive(Debug, Clone, PartialEq)]
pub struct OrbitLayout {
    /// One per planet, in the order of the spec.
    pub radii: Vec<f32>,
    /// Outermost planet shell plus 5: the sphere the cameras frame.
    pub world_radius: f32,
    /// Orbit radius kept for a fifth planet of `FIFTH_SLOT_RESERVED_EXTENT`
    /// (always computed; it counts in `world_radius` only once registered).
    pub fifth_slot: f32,
}

/// Room left around the outermost shell for the cameras.
pub const WORLD_MARGIN: f32 = 5.0;

/// `x` rounded up to a multiple of 0.5.
fn ceil_half(x: f32) -> f32 {
    (x * 2.0).ceil() / 2.0
}

impl OrbitLayout {
    /// Walks the planets in the order of the spec (the order is data):
    /// planet `i` needs `r_i - eff_i >= r_{i-1} + eff_{i-1} + gap`, and the
    /// first one `r_0 - eff_0 >= star_shell + gap`. An explicit radius that
    /// meets it is kept, one that does not is an error; `AUTO_ORBIT` takes
    /// the bound rounded up to 0.5. `eff` holds the effective radii.
    pub fn solve(
        spec: &SystemSpec,
        eff: &[f32],
        star_shell: f32,
        gap: f32,
    ) -> Result<Self, SpecError> {
        if eff.len() != spec.planets.len() {
            return Err(SpecError::OutOfRange("effective radii", eff.len() as f32));
        }
        let mut edge = star_shell;
        let mut radii = Vec::with_capacity(eff.len());
        for (p, &e) in spec.planets.iter().zip(eff) {
            let min = edge + gap + e;
            let r = if p.orbit_radius == AUTO_ORBIT {
                ceil_half(min)
            } else if p.orbit_radius >= min {
                p.orbit_radius
            } else {
                return Err(SpecError::OrbitOverlap {
                    planet: p.id,
                    radius: p.orbit_radius,
                    min,
                });
            };
            edge = r + e;
            radii.push(r);
        }
        Ok(Self {
            radii,
            world_radius: edge + WORLD_MARGIN,
            fifth_slot: ceil_half(edge + gap + FIFTH_SLOT_RESERVED_EXTENT),
        })
    }

    /// Writes the solved radii into `spec`.
    pub fn apply(&self, spec: &mut SystemSpec) {
        for (p, &r) in spec.planets.iter_mut().zip(&self.radii) {
            p.orbit_radius = r;
        }
    }
}
