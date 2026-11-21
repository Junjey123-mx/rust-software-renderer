//! The spacecraft as the unit of measure of the Project 3 system: planet
//! and star sizes are checked as ratios of world radii (never of mesh
//! scales: `world radius = transform scale x mesh radius`).

use crate::solar::SPHERE_RADIUS;
use crate::solar::scene::SHIP_SCALE;
use crate::solar::ship::MESH_RADIUS;
use crate::solar::system::{SpecError, SystemSpec};

/// Relative tolerance of the ratio checks.
pub const RATIO_TOLERANCE: f32 = 1e-3;

/// Proportions and speeds of the system, in units of the spacecraft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleProfile {
    /// Bounding radius of the spacecraft, world units (`MESH_RADIUS x
    /// SHIP_SCALE`, about 0.319).
    pub ship_radius: f32,
    /// Smallest and largest planet radius, in spacecraft radii.
    pub planet_to_ship: (f32, f32),
    /// Smallest and largest star radius, in radii of the largest planet.
    pub star_to_planet: (f32, f32),
    /// Least free space between the effective shells of neighbours.
    pub min_gap: f32,
    /// Spacecraft speed in space flight, world units per second.
    pub cruise_speed: f32,
    /// Fastest linear orbital speed, world units per second (so the ship
    /// can always catch a planet without quick travel).
    pub max_orbit_speed: f32,
    /// Acceleration of full thrust, world units per second squared.
    pub thrust_accel: f32,
    /// Where the ship starts: `false` just beyond the fifth planet's slot
    /// (Project 3), `true` one gap beyond the outermost shell.
    pub spawn_beyond_layout: bool,
    /// Size of the comets' ellipses and tails against `CometField::project3`.
    pub comet_scale: f32,
}

impl ScaleProfile {
    /// The Project 3 proportions: planets 3-6 times the spacecraft, the
    /// star 3-5 times the largest planet, shells 3 units apart, cruise at
    /// 3 u/s and orbits no faster than 1 u/s.
    pub fn project3() -> Self {
        Self {
            ship_radius: MESH_RADIUS * SHIP_SCALE,
            planet_to_ship: (3.0, 6.0),
            star_to_planet: (3.0, 5.0),
            min_gap: 3.0,
            cruise_speed: 3.0,
            max_orbit_speed: 1.0,
            thrust_accel: crate::solar::flight::THRUST_ACCEL,
            spawn_beyond_layout: false,
            comet_scale: 1.0,
        }
    }

    /// The Grand proportions of the live viewer (second pass,
    /// 2026-11-20): planets 15-30 times the spacecraft (five times the
    /// project scale, so the ship is small against them), the star 4-6
    /// times the largest planet, shells 6 units apart, twice the cruise
    /// and thrust so the larger system stays quick to cross, orbits up to
    /// 2 u/s, the ship starting beyond the outermost shell and the comets
    /// four times wider.
    pub fn grand() -> Self {
        Self {
            ship_radius: MESH_RADIUS * SHIP_SCALE,
            planet_to_ship: (15.0, 30.0),
            star_to_planet: (4.0, 6.0),
            min_gap: 6.0,
            cruise_speed: 6.0,
            max_orbit_speed: 2.0,
            thrust_accel: 2.0 * crate::solar::flight::THRUST_ACCEL,
            spawn_beyond_layout: true,
            comet_scale: 4.0,
        }
    }

    /// Transform scale that gives a mesh of local radius
    /// `mesh_local_radius` the world radius `world_radius`.
    pub fn world_scale_for(world_radius: f32, mesh_local_radius: f32) -> f32 {
        world_radius / mesh_local_radius
    }

    /// Checks every planet radius against the spacecraft, the star against
    /// the largest planet and every orbital speed against the limit, with
    /// `RATIO_TOLERANCE`.
    pub fn check(&self, spec: &SystemSpec) -> Result<(), SpecError> {
        let within = |field, ratio: f32, (low, high): (f32, f32)| {
            if ratio >= low * (1.0 - RATIO_TOLERANCE) && ratio <= high * (1.0 + RATIO_TOLERANCE) {
                Ok(())
            } else {
                Err(SpecError::OutOfRange(field, ratio))
            }
        };
        let mut largest = 0.0f32;
        for p in &spec.planets {
            within(
                "planet radius / ship radius",
                p.radius() / self.ship_radius,
                self.planet_to_ship,
            )?;
            if p.orbit_speed.abs() > self.max_orbit_speed * (1.0 + RATIO_TOLERANCE) {
                return Err(SpecError::OutOfRange("planet.orbit_speed", p.orbit_speed));
            }
            largest = largest.max(p.radius());
        }
        if largest > 0.0 {
            let star = spec.star.scale * SPHERE_RADIUS;
            within(
                "star radius / largest planet radius",
                star / largest,
                self.star_to_planet,
            )?;
        }
        Ok(())
    }
}
